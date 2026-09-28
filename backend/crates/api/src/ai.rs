//! AI assist: the desktop's three tutoring modes, served from a hosted model.
//!
//! The prompts are the desktop's own (`dsa_ai::prompts`), filled from the same
//! problem brief — title, statement, approach, the category's technique
//! vocabulary and two worked examples — so a hint reads the same whether it came
//! from a local Ollama or from the cloud. What changes is only the transport:
//!
//! * **anthropic** — the Claude API over HTTPS (Rust has no official SDK, so
//!   this speaks the documented wire format directly: `POST /v1/messages` with
//!   `stream: true`, parsing `content_block_delta` / `message_delta` events).
//!   Adaptive thinking with *summarized* display, so the 💭 section the desktop
//!   renders collapsed actually has something in it; server-side refusal
//!   fallbacks on; the system prompt marked cacheable, since every turn of a
//!   conversation about one problem repeats it verbatim.
//! * **bedrock** — Amazon Bedrock's ConverseStream through the AWS SDK, signed
//!   with the task role: no API key to store or rotate.
//! * **ollama** — a local server, exactly as the desktop uses it (development).
//!
//! Every provider yields the same `Chunk`s, which the route turns into the SSE
//! events the web client parses (`token` / `done` / `error`).

use crate::config::{AiProviderConfig, Config};
use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use serde_json::{json, Value};

/// Default model per provider. Overridden by `AI_MODEL`.
pub const DEFAULT_ANTHROPIC_MODEL: &str = "claude-opus-5";
pub const DEFAULT_BEDROCK_MODEL: &str = "anthropic.claude-opus-5";
pub const DEFAULT_OLLAMA_MODEL: &str = "gemma3:4b";

/// Streaming output ceiling. Replies are short by prompt design; the ceiling
/// is headroom, not a target, and hitting it mid-sentence would be worse.
const MAX_TOKENS: u32 = 64_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

pub struct ChatRequest {
    pub system: String,
    pub messages: Vec<(Role, String)>,
    /// Only honoured by providers whose models accept sampling parameters
    /// (Ollama); current Claude models take their behaviour from the prompt.
    pub temperature: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Chunk {
    Text(String),
    Thinking(String),
    Usage {
        input: u64,
        output: u64,
    },
    /// The model declined (after any configured fallback also declined).
    Refused,
}

pub type ChunkStream = BoxStream<'static, Result<Chunk, String>>;

#[async_trait]
trait Provider: Send + Sync {
    /// Start a completion. Errors before the first byte (auth, quota, network)
    /// come back here; errors mid-stream arrive as `Err` items.
    async fn stream(&self, req: ChatRequest) -> Result<ChunkStream, String>;
}

pub struct Assistant {
    provider: Box<dyn Provider>,
    pub prompts: dsa_ai::Prompts,
    pub provider_name: &'static str,
    pub model: String,
}

impl Assistant {
    pub async fn from_config(
        cfg: &Config,
        http: reqwest::Client,
        content_root: &std::path::Path,
    ) -> Result<Option<Self>> {
        let prompts = dsa_ai::Prompts::load(content_root);
        let effort = std::env::var("AI_EFFORT").ok().filter(|s| !s.is_empty());
        let (provider, name, model): (Box<dyn Provider>, _, String) = match &cfg.ai {
            AiProviderConfig::None => return Ok(None),
            AiProviderConfig::Anthropic { api_key } => {
                let model = cfg
                    .ai_model
                    .clone()
                    .unwrap_or_else(|| DEFAULT_ANTHROPIC_MODEL.into());
                (
                    Box::new(Anthropic {
                        http,
                        api_key: api_key.clone(),
                        base: std::env::var("ANTHROPIC_BASE_URL")
                            .unwrap_or_else(|_| "https://api.anthropic.com".into()),
                        model: model.clone(),
                        effort,
                    }),
                    "anthropic",
                    model,
                )
            }
            AiProviderConfig::Bedrock => {
                let model = cfg
                    .ai_model
                    .clone()
                    .unwrap_or_else(|| DEFAULT_BEDROCK_MODEL.into());
                (bedrock(cfg, &model).await?, "bedrock", model)
            }
            AiProviderConfig::Ollama { url } => {
                let model = cfg
                    .ai_model
                    .clone()
                    .unwrap_or_else(|| DEFAULT_OLLAMA_MODEL.into());
                (
                    Box::new(Ollama {
                        http,
                        url: url.as_str().trim_end_matches('/').to_string(),
                        model: model.clone(),
                    }),
                    "ollama",
                    model,
                )
            }
        };
        Ok(Some(Self {
            provider,
            prompts,
            provider_name: name,
            model,
        }))
    }

    pub async fn stream(&self, req: ChatRequest) -> Result<ChunkStream, String> {
        self.provider.stream(req).await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SSE framing (shared by the Anthropic parser and tests)
// ─────────────────────────────────────────────────────────────────────────────

/// Incremental `text/event-stream` parser: feed bytes, get `(event, data)`.
#[derive(Default)]
pub struct SseFrames {
    buf: String,
}

impl SseFrames {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<(String, String)> {
        self.buf.push_str(&String::from_utf8_lossy(bytes));
        let mut out = Vec::new();
        loop {
            let normalized_end = self
                .buf
                .find("\n\n")
                .map(|i| (i, 2))
                .or_else(|| self.buf.find("\r\n\r\n").map(|i| (i, 4)));
            let Some((end, sep)) = normalized_end else {
                break;
            };
            let frame: String = self.buf.drain(..end + sep).collect();
            let mut event = String::from("message");
            let mut data = Vec::new();
            for line in frame.lines() {
                if let Some(v) = line.strip_prefix("event:") {
                    event = v.trim().to_string();
                } else if let Some(v) = line.strip_prefix("data:") {
                    data.push(v.strip_prefix(' ').unwrap_or(v).to_string());
                }
            }
            if !data.is_empty() {
                out.push((event, data.join("\n")));
            }
        }
        out
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Anthropic (Claude API)
// ─────────────────────────────────────────────────────────────────────────────

struct Anthropic {
    http: reqwest::Client,
    api_key: String,
    base: String,
    model: String,
    effort: Option<String>,
}

impl Anthropic {
    fn body(&self, req: &ChatRequest) -> Value {
        let messages: Vec<Value> = req
            .messages
            .iter()
            .map(|(role, text)| {
                json!({
                    "role": if *role == Role::User { "user" } else { "assistant" },
                    "content": text,
                })
            })
            .collect();
        let mut body = json!({
            "model": self.model,
            "max_tokens": MAX_TOKENS,
            "stream": true,
            // The per-problem system prompt is byte-identical on every turn
            // of a conversation, so it is the cacheable prefix.
            "system": [{ "type": "text", "text": req.system, "cache_control": { "type": "ephemeral" } }],
            "messages": messages,
            // Summarized so the collapsed 💭 section has readable content;
            // the default display on current models is an empty block.
            "thinking": { "type": "adaptive", "display": "summarized" },
            // A policy decline re-runs on a fallback model inside the same call
            // instead of leaving the student with nothing.
            "fallbacks": "default",
        });
        if let Some(effort) = &self.effort {
            body["output_config"] = json!({ "effort": effort });
        }
        body
    }
}

/// Map one Claude streaming event to chunks.
pub fn anthropic_event(event: &str, data: &str) -> Result<Vec<Chunk>, String> {
    let v: Value = serde_json::from_str(data).map_err(|e| format!("bad event from Claude: {e}"))?;
    Ok(match event {
        "message_start" => {
            let u = &v["message"]["usage"];
            let input = u["input_tokens"].as_u64().unwrap_or(0)
                + u["cache_creation_input_tokens"].as_u64().unwrap_or(0)
                + u["cache_read_input_tokens"].as_u64().unwrap_or(0);
            vec![Chunk::Usage { input, output: 0 }]
        }
        "content_block_delta" => match v["delta"]["type"].as_str() {
            Some("text_delta") => vec![Chunk::Text(
                v["delta"]["text"].as_str().unwrap_or("").to_string(),
            )],
            Some("thinking_delta") => vec![Chunk::Thinking(
                v["delta"]["thinking"].as_str().unwrap_or("").to_string(),
            )],
            // Signatures and tool-input deltas carry nothing to show.
            _ => vec![],
        },
        "message_delta" => {
            let mut out = vec![Chunk::Usage {
                input: 0,
                output: v["usage"]["output_tokens"].as_u64().unwrap_or(0),
            }];
            if v["delta"]["stop_reason"].as_str() == Some("refusal") {
                out.push(Chunk::Refused);
            }
            out
        }
        "error" => {
            return Err(v["error"]["message"]
                .as_str()
                .unwrap_or("the model stream failed")
                .to_string())
        }
        // message_stop, content_block_start/stop, ping
        _ => vec![],
    })
}

#[async_trait]
impl Provider for Anthropic {
    async fn stream(&self, req: ChatRequest) -> Result<ChunkStream, String> {
        let resp = self
            .http
            .post(format!("{}/v1/messages", self.base.trim_end_matches('/')))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "server-side-fallback-2026-07-01")
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(&self.body(&req))
            .send()
            .await
            .map_err(|e| format!("could not reach the model: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            tracing::warn!(%status, body = %text.chars().take(500).collect::<String>(), "claude request failed");
            return Err(match status.as_u16() {
                429 | 529 => "the assistant is overloaded right now — try again in a moment".into(),
                _ => format!("the model request failed ({status})"),
            });
        }
        let mut bytes = resp.bytes_stream();
        let stream = async_stream::stream! {
            let mut frames = SseFrames::default();
            while let Some(chunk) = bytes.next().await {
                match chunk {
                    Ok(b) => {
                        for (event, data) in frames.push(&b) {
                            match anthropic_event(&event, &data) {
                                Ok(chunks) => for c in chunks { yield Ok(c) },
                                Err(e) => { yield Err(e); return; }
                            }
                        }
                    }
                    Err(e) => { yield Err(format!("the model stream broke: {e}")); return; }
                }
            }
        };
        Ok(stream.boxed())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Bedrock (ConverseStream, task-role credentials)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "aws")]
async fn bedrock(cfg: &Config, model: &str) -> Result<Box<dyn Provider>> {
    let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
    if let Some(region) = &cfg.aws_region {
        loader = loader.region(aws_config::Region::new(region.clone()));
    }
    let sdk = loader.load().await;
    Ok(Box::new(Bedrock {
        client: aws_sdk_bedrockruntime::Client::new(&sdk),
        model: model.to_string(),
    }))
}

#[cfg(not(feature = "aws"))]
async fn bedrock(_cfg: &Config, _model: &str) -> Result<Box<dyn Provider>> {
    anyhow::bail!("AI_PROVIDER=bedrock needs a build with `--features aws`")
}

#[cfg(feature = "aws")]
struct Bedrock {
    client: aws_sdk_bedrockruntime::Client,
    model: String,
}

#[cfg(feature = "aws")]
#[async_trait]
impl Provider for Bedrock {
    async fn stream(&self, req: ChatRequest) -> Result<ChunkStream, String> {
        use aws_sdk_bedrockruntime::types::{
            ContentBlock, ContentBlockDelta, ConversationRole, ConverseStreamOutput,
            InferenceConfiguration, Message, ReasoningContentBlockDelta, StopReason,
            SystemContentBlock,
        };
        let mut messages = Vec::with_capacity(req.messages.len());
        for (role, text) in &req.messages {
            let m = Message::builder()
                .role(if *role == Role::User {
                    ConversationRole::User
                } else {
                    ConversationRole::Assistant
                })
                .content(ContentBlock::Text(text.clone()))
                .build()
                .map_err(|e| e.to_string())?;
            messages.push(m);
        }
        let out = self
            .client
            .converse_stream()
            .model_id(&self.model)
            .system(SystemContentBlock::Text(req.system.clone()))
            .set_messages(Some(messages))
            .inference_config(
                InferenceConfiguration::builder()
                    .max_tokens(MAX_TOKENS as i32)
                    .build(),
            )
            .send()
            .await
            .map_err(|e| {
                tracing::warn!(error = ?e, "bedrock converse_stream failed");
                match e.as_service_error() {
                    Some(se) if se.is_throttling_exception() => {
                        "the assistant is overloaded right now — try again in a moment".to_string()
                    }
                    _ => "the model request failed".to_string(),
                }
            })?;
        let mut rx = out.stream;
        let stream = async_stream::stream! {
            loop {
                match rx.recv().await {
                    Ok(Some(ConverseStreamOutput::ContentBlockDelta(ev))) => match ev.delta {
                        Some(ContentBlockDelta::Text(t)) => yield Ok(Chunk::Text(t)),
                        Some(ContentBlockDelta::ReasoningContent(ReasoningContentBlockDelta::Text(t))) => {
                            yield Ok(Chunk::Thinking(t))
                        }
                        _ => {}
                    },
                    Ok(Some(ConverseStreamOutput::MessageStop(stop))) => {
                        if stop.stop_reason == StopReason::GuardrailIntervened || stop.stop_reason == StopReason::ContentFiltered {
                            yield Ok(Chunk::Refused);
                        }
                    }
                    Ok(Some(ConverseStreamOutput::Metadata(meta))) => {
                        if let Some(u) = meta.usage {
                            yield Ok(Chunk::Usage { input: u.input_tokens.max(0) as u64, output: u.output_tokens.max(0) as u64 });
                        }
                    }
                    Ok(Some(_)) => {}
                    Ok(None) => return,
                    Err(e) => { yield Err(format!("the model stream broke: {e}")); return; }
                }
            }
        };
        Ok(stream.boxed())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Ollama (development; the desktop's own backend)
// ─────────────────────────────────────────────────────────────────────────────

struct Ollama {
    http: reqwest::Client,
    url: String,
    model: String,
}

/// One NDJSON line from `/api/chat`.
pub fn ollama_line(line: &str) -> Result<Vec<Chunk>, String> {
    let v: Value = serde_json::from_str(line).map_err(|e| format!("bad line from Ollama: {e}"))?;
    if let Some(err) = v["error"].as_str() {
        return Err(err.to_string());
    }
    let mut out = Vec::new();
    if let Some(t) = v["message"]["thinking"].as_str().filter(|t| !t.is_empty()) {
        out.push(Chunk::Thinking(t.to_string()));
    }
    if let Some(t) = v["message"]["content"].as_str().filter(|t| !t.is_empty()) {
        out.push(Chunk::Text(t.to_string()));
    }
    if v["done"].as_bool() == Some(true) {
        out.push(Chunk::Usage {
            input: v["prompt_eval_count"].as_u64().unwrap_or(0),
            output: v["eval_count"].as_u64().unwrap_or(0),
        });
    }
    Ok(out)
}

#[async_trait]
impl Provider for Ollama {
    async fn stream(&self, req: ChatRequest) -> Result<ChunkStream, String> {
        let mut messages = vec![json!({ "role": "system", "content": req.system })];
        for (role, text) in &req.messages {
            messages.push(json!({ "role": if *role == Role::User { "user" } else { "assistant" }, "content": text }));
        }
        let resp = self
            .http
            .post(format!("{}/api/chat", self.url))
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
                "options": { "temperature": req.temperature },
            }))
            .send()
            .await
            .map_err(|e| format!("cannot reach Ollama at {} ({e})", self.url))?;
        if !resp.status().is_success() {
            return Err(format!("Ollama answered {}", resp.status()));
        }
        let mut bytes = resp.bytes_stream();
        let stream = async_stream::stream! {
            let mut buf = String::new();
            while let Some(chunk) = bytes.next().await {
                let Ok(b) = chunk else { yield Err("the Ollama stream broke".to_string()); return; };
                buf.push_str(&String::from_utf8_lossy(&b));
                while let Some(nl) = buf.find('\n') {
                    let line: String = buf.drain(..=nl).collect();
                    let line = line.trim();
                    if line.is_empty() { continue; }
                    match ollama_line(line) {
                        Ok(chunks) => for c in chunks { yield Ok(c) },
                        Err(e) => { yield Err(e); return; }
                    }
                }
            }
        };
        Ok(stream.boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_frames_survive_arbitrary_chunking() {
        let raw = "event: content_block_delta\ndata: {\"a\":1}\n\nevent: ping\ndata: {}\n\n";
        let mut f = SseFrames::default();
        let mut got = Vec::new();
        for b in raw.as_bytes().chunks(7) {
            got.extend(f.push(b));
        }
        assert_eq!(
            got,
            vec![
                ("content_block_delta".to_string(), "{\"a\":1}".to_string()),
                ("ping".to_string(), "{}".to_string())
            ]
        );
    }

    #[test]
    fn claude_events_map_to_chunks() {
        let text = anthropic_event(
            "content_block_delta",
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#,
        )
        .unwrap();
        assert_eq!(text, vec![Chunk::Text("Hi".into())]);

        let think = anthropic_event(
            "content_block_delta",
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"hmm"}}"#,
        )
        .unwrap();
        assert_eq!(think, vec![Chunk::Thinking("hmm".into())]);

        let start = anthropic_event(
            "message_start",
            r#"{"type":"message_start","message":{"usage":{"input_tokens":10,"cache_read_input_tokens":90}}}"#,
        )
        .unwrap();
        assert_eq!(
            start,
            vec![Chunk::Usage {
                input: 100,
                output: 0
            }]
        );

        let end = anthropic_event(
            "message_delta",
            r#"{"type":"message_delta","delta":{"stop_reason":"refusal"},"usage":{"output_tokens":3}}"#,
        )
        .unwrap();
        assert_eq!(
            end,
            vec![
                Chunk::Usage {
                    input: 0,
                    output: 3
                },
                Chunk::Refused
            ]
        );

        assert!(anthropic_event(
            "error",
            r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#
        )
        .is_err());
        assert!(anthropic_event("ping", "{}").unwrap().is_empty());
    }

    #[test]
    fn ollama_lines_carry_thinking_text_and_usage() {
        let c = ollama_line(
            r#"{"message":{"role":"assistant","content":"a","thinking":"t"},"done":false}"#,
        )
        .unwrap();
        assert_eq!(
            c,
            vec![Chunk::Thinking("t".into()), Chunk::Text("a".into())]
        );
        let d = ollama_line(
            r#"{"message":{"content":""},"done":true,"prompt_eval_count":5,"eval_count":7}"#,
        )
        .unwrap();
        assert_eq!(
            d,
            vec![Chunk::Usage {
                input: 5,
                output: 7
            }]
        );
        assert!(ollama_line(r#"{"error":"model not found"}"#).is_err());
    }

    #[test]
    fn the_request_body_enables_caching_thinking_and_fallbacks() {
        let a = Anthropic {
            http: reqwest::Client::new(),
            api_key: String::new(),
            base: String::new(),
            model: DEFAULT_ANTHROPIC_MODEL.into(),
            effort: None,
        };
        let b = a.body(&ChatRequest {
            system: "sys".into(),
            messages: vec![(Role::User, "hi".into())],
            temperature: 0.4,
        });
        assert_eq!(b["model"], "claude-opus-5");
        assert_eq!(b["stream"], true);
        assert_eq!(b["system"][0]["cache_control"]["type"], "ephemeral");
        assert_eq!(b["thinking"]["type"], "adaptive");
        assert_eq!(b["fallbacks"], "default");
        assert!(
            b.get("temperature").is_none(),
            "current Claude models reject sampling params"
        );
        assert!(
            b.get("output_config").is_none(),
            "effort left at the API default unless configured"
        );
    }
}
