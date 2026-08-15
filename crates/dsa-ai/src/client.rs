//! Ollama client.
//!
//! Everything runs against a local server (`http://localhost:11434` by
//! default) — no API keys, no data leaving the machine. Streaming happens on a
//! worker thread and reaches the UI as events, because a GUI that blocks for
//! twenty seconds while a model thinks is not a GUI.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

/// Thinking models return reasoning on a separate channel so the UI can
/// collapse it instead of showing it as the answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Content,
    Thinking,
}

#[derive(Clone, Debug)]
pub enum Event {
    Token(Channel, String),
    Done,
    Error(String),
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        // No global timeout: a slow model streaming for minutes is normal.
        // The connect timeout is what makes "Ollama is not running" fail fast.
        .timeout_connect(Some(Duration::from_secs(4)))
        .build()
        .new_agent()
}

fn trim_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

/// Models the local server has pulled. Doubles as the reachability check.
pub fn list_models(url: &str) -> Result<Vec<String>, String> {
    let url = trim_url(url);
    let mut res = agent()
        .get(format!("{url}/api/tags"))
        .call()
        .map_err(|e| format!("cannot reach Ollama at {url} ({e})"))?;
    let value: serde_json::Value = res
        .body_mut()
        .read_json()
        .map_err(|e| format!("unexpected response from Ollama: {e}"))?;
    Ok(parse_models(&value))
}

/// Split out so the shape of the response is testable without a server.
pub fn parse_models(value: &serde_json::Value) -> Vec<String> {
    value
        .get("models")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

pub struct ChatOptions {
    pub url: String,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
}

/// A running completion. Poll it from the UI thread; cancel it by dropping it
/// or calling [`Stream::cancel`].
pub struct Stream {
    rx: Receiver<Event>,
    cancel: Arc<AtomicBool>,
    finished: bool,
}

impl Stream {
    /// Drain whatever has arrived since the last call. Never blocks.
    pub fn poll(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(ev) => {
                    if matches!(ev, Event::Done | Event::Error(_)) {
                        self.finished = true;
                    }
                    out.push(ev);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // The worker died without a final event (panic, or the
                    // connection dropped) — do not spin forever waiting.
                    if !self.finished {
                        self.finished = true;
                        out.push(Event::Done);
                    }
                    break;
                }
            }
        }
        out
    }

    pub fn finished(&self) -> bool {
        self.finished
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Start a streaming chat completion on a worker thread.
pub fn chat_stream(opts: ChatOptions) -> Stream {
    let (tx, rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();

    std::thread::spawn(move || {
        let url = trim_url(&opts.url);
        let body = serde_json::json!({
            "model": opts.model,
            "messages": opts.messages,
            "stream": true,
            "keep_alive": "15m",
            "options": { "temperature": opts.temperature, "num_ctx": 4096 },
        });

        let response = agent()
            .post(format!("{url}/api/chat"))
            .header("Content-Type", "application/json")
            .send_json(&body);

        let mut res = match response {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(Event::Error(format!("Ollama request failed: {e}")));
                return;
            }
        };

        let reader = BufReader::new(res.body_mut().as_reader());
        for line in reader.lines() {
            if flag.load(Ordering::Relaxed) {
                break; // dropping the reader closes the connection
            }
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            match parse_chunk(&line) {
                Chunk::Token(channel, text) => {
                    if tx.send(Event::Token(channel, text)).is_err() {
                        return; // the UI dropped the stream
                    }
                }
                Chunk::Error(msg) => {
                    let _ = tx.send(Event::Error(msg));
                    return;
                }
                Chunk::Ignore => {}
            }
        }
        let _ = tx.send(Event::Done);
    });

    Stream {
        rx,
        cancel,
        finished: false,
    }
}

#[derive(Debug, PartialEq)]
pub enum Chunk {
    Token(Channel, String),
    Error(String),
    Ignore,
}

/// Parse one NDJSON line of an Ollama chat stream.
pub fn parse_chunk(line: &str) -> Chunk {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
        return Chunk::Ignore; // partial line or keep-alive noise
    };
    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
        return Chunk::Error(err.to_string());
    }
    let msg = v.get("message");
    if let Some(t) = msg.and_then(|m| m.get("thinking")).and_then(|t| t.as_str()) {
        if !t.is_empty() {
            return Chunk::Token(Channel::Thinking, t.to_string());
        }
    }
    if let Some(t) = msg.and_then(|m| m.get("content")).and_then(|t| t.as_str()) {
        if !t.is_empty() {
            return Chunk::Token(Channel::Content, t.to_string());
        }
    }
    Chunk::Ignore
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_names_are_pulled_out_of_the_tags_response() {
        let v = serde_json::json!({
            "models": [{ "name": "gemma3:4b", "size": 1 }, { "name": "qwen2.5-coder:7b" }]
        });
        assert_eq!(parse_models(&v), vec!["gemma3:4b", "qwen2.5-coder:7b"]);
    }

    #[test]
    fn a_server_with_no_models_is_not_an_error() {
        assert!(parse_models(&serde_json::json!({ "models": [] })).is_empty());
        assert!(parse_models(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn content_and_thinking_arrive_on_different_channels() {
        let c = parse_chunk(r#"{"message":{"content":"hi"},"done":false}"#);
        assert_eq!(c, Chunk::Token(Channel::Content, "hi".into()));
        let t = parse_chunk(r#"{"message":{"thinking":"hmm"},"done":false}"#);
        assert_eq!(t, Chunk::Token(Channel::Thinking, "hmm".into()));
    }

    #[test]
    fn thinking_wins_when_a_chunk_carries_both() {
        // Ollama sends them in separate chunks, but a model that batches them
        // must not have its reasoning rendered as the answer.
        let c = parse_chunk(r#"{"message":{"thinking":"why","content":"because"}}"#);
        assert_eq!(c, Chunk::Token(Channel::Thinking, "why".into()));
    }

    #[test]
    fn an_error_line_is_surfaced() {
        let c = parse_chunk(r#"{"error":"model \"nope\" not found"}"#);
        assert_eq!(c, Chunk::Error("model \"nope\" not found".into()));
    }

    #[test]
    fn empty_tokens_and_final_chunks_are_ignored() {
        assert_eq!(
            parse_chunk(r#"{"message":{"content":""},"done":true}"#),
            Chunk::Ignore
        );
        assert_eq!(parse_chunk(r#"{"done":true}"#), Chunk::Ignore);
    }

    #[test]
    fn a_partial_json_line_does_not_kill_the_stream() {
        assert_eq!(parse_chunk("{\"message\":{\"cont"), Chunk::Ignore);
        assert_eq!(parse_chunk("   "), Chunk::Ignore);
    }

    #[test]
    fn urls_are_normalised_before_use() {
        assert_eq!(
            trim_url("http://localhost:11434/"),
            "http://localhost:11434"
        );
        assert_eq!(trim_url("  http://x:1//  "), "http://x:1");
    }

    #[test]
    fn messages_serialize_with_lowercase_roles() {
        let m = ChatMessage::system("hi");
        let j = serde_json::to_string(&m).unwrap();
        assert!(j.contains(r#""role":"system""#), "{j}");
    }

    #[test]
    fn an_unreachable_server_reports_rather_than_hanging() {
        // Port 1 is reserved and refuses instantly on every platform.
        let err = list_models("http://127.0.0.1:1").unwrap_err();
        assert!(err.contains("cannot reach Ollama"), "{err}");
    }

    #[test]
    fn a_dead_stream_still_reaches_a_finished_state() {
        let (tx, rx) = channel();
        drop(tx);
        let mut s = Stream {
            rx,
            cancel: Arc::new(AtomicBool::new(false)),
            finished: false,
        };
        let events = s.poll();
        assert!(matches!(events.as_slice(), [Event::Done]));
        assert!(s.finished());
    }
}
