//! `POST /ai/chat`: the assistant, streamed as server-sent events.
//!
//! The conversation is assembled exactly as the desktop assembles it
//! (`crates/dsa-app/src/assistant.rs`): the mode's system prompt filled from
//! the problem brief, the last `HISTORY_TURNS` turns, and the code snapshot
//! attached to the *newest* user turn only when the user ticked "attach code".
//! Fix mode is a single review turn over the code and the latest run output.
//!
//! Metering: a request reserves one unit of the daily quota atomically before
//! the model is called, and gets it back if the provider fails before
//! producing anything — a student is never charged for an outage.

use crate::ai::{ChatRequest, Chunk, Role};
use crate::dto;
use crate::error::{ApiError, ApiResult};
use crate::extract::{Authed, Json};
use crate::routes::common::{limit, premium_entitled, MINUTE};
use crate::state::AppState;
use crate::store::metering;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::{Stream, StreamExt};
use serde_json::json;
use std::convert::Infallible;
use std::time::Duration;

const MAX_TURN_CHARS: usize = 8_000;

/// What the panel sends as the Fix turn when the student wrote no note.
const FIX_DEFAULT_NOTE: &str = "Review my current solution and fix what is wrong.";

/// A history window cut from the end can start on the model's turn, and a
/// failed reply can leave two questions in a row. Models want the transcript
/// to open with the user and alternate, so drop a leading reply and fold
/// same-speaker neighbours together.
fn normalize_turns(turns: Vec<(Role, String)>) -> Vec<(Role, String)> {
    let mut out: Vec<(Role, String)> = Vec::with_capacity(turns.len());
    for (role, text) in turns.into_iter().skip_while(|(r, _)| *r == Role::Assistant) {
        match out.last_mut() {
            Some((prev, acc)) if *prev == role => {
                acc.push_str("\n\n");
                acc.push_str(&text);
            }
            _ => out.push((role, text)),
        }
    }
    out
}

fn daily_limit(state: &AppState, authed: &Authed) -> u32 {
    if premium_entitled(&authed.user) {
        state.cfg.policy.ai_pro_daily
    } else {
        state.cfg.policy.ai_free_daily
    }
}

pub async fn status(
    State(state): State<AppState>,
    authed: Authed,
) -> ApiResult<axum::Json<dto::AiStatus>> {
    Ok(axum::Json(dto::AiStatus {
        enabled: state.ai.is_some(),
        provider: state
            .ai
            .as_ref()
            .map(|a| a.provider_name.to_string())
            .unwrap_or_default(),
        model: state
            .ai
            .as_ref()
            .map(|a| a.model.clone())
            .unwrap_or_default(),
        daily_limit: daily_limit(&state, &authed),
        used_today: metering::ai_used_today(&state.db, authed.user.id)
            .await?
            .max(0) as u32,
    }))
}

/// `Input: nums = [2,7,11,15], target = 9 → Output: 0 1` — the desktop's
/// LeetCode-style example lines (`fmt_value`).
fn fmt_value(v: &dsa_core::problem::InputValue) -> String {
    use dsa_core::problem::InputValue as V;
    match v {
        V::List(items) => format!(
            "[{}]",
            items.iter().map(fmt_value).collect::<Vec<_>>().join(",")
        ),
        V::Str(s) => format!("\"{s}\""),
        other => other.to_editable(),
    }
}

fn examples(meta: &dsa_core::problem::ProblemMeta) -> String {
    meta.tests
        .iter()
        .take(2)
        .map(|t| {
            let ins = meta
                .inputs
                .iter()
                .map(|f| {
                    format!(
                        "{} = {}",
                        f.name,
                        t.input.get(&f.name).map(fmt_value).unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("Input: {ins} → Output: {}", t.expected)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn event(name: &str, data: serde_json::Value) -> Result<Event, Infallible> {
    Ok(Event::default().event(name).data(data.to_string()))
}

pub async fn chat(
    State(state): State<AppState>,
    authed: Authed,
    Json(req): Json<dto::AiChatRequest>,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    let assistant = state
        .ai
        .as_ref()
        .ok_or_else(|| ApiError::Unavailable("AI assist isn't available on this server.".into()))?;
    let pack = state
        .content
        .pack(&req.slug)
        .ok_or_else(|| ApiError::not_found("problem"))?;
    let lang = state
        .content
        .language(&req.lang)
        .ok_or_else(|| ApiError::not_found("language"))?;
    let mode = match req.mode.as_str() {
        "interview" => dsa_ai::Mode::Interview,
        "guide" => dsa_ai::Mode::Guide,
        "fix" => dsa_ai::Mode::Fix,
        _ => return Err(ApiError::field("mode", "mode is interview, guide or fix")),
    };
    if state.content.is_premium(pack.meta.tier) && !premium_entitled(&authed.user) {
        return Err(ApiError::PaymentRequired(
            "This problem is part of Pro.".into(),
        ));
    }
    let code = req.code.as_deref().unwrap_or("");
    if code.len() > dsa_protocol::MAX_SOURCE_BYTES {
        return Err(ApiError::field("code", "That code is too large to send."));
    }
    let turns: Vec<&dto::AiMessage> = req
        .messages
        .iter()
        .filter(|m| m.role == "user" || (m.role == "assistant" && !m.content.is_empty()))
        .collect();
    if turns
        .iter()
        .any(|m| m.content.chars().count() > MAX_TURN_CHARS)
    {
        return Err(ApiError::field("messages", "That message is too long."));
    }
    let last_is_user = turns.last().is_some_and(|m| m.role == "user");
    if mode != dsa_ai::Mode::Fix && !last_is_user {
        return Err(ApiError::field("messages", "Send a question."));
    }
    if mode == dsa_ai::Mode::Fix && code.trim().is_empty() {
        return Err(ApiError::field("code", "Fix mode needs your code."));
    }

    limit(&state, &format!("ai:{}", authed.user.id), 20, MINUTE).await?;
    let quota = daily_limit(&state, &authed) as i32;
    let used = match metering::reserve_ai_request(&state.db, authed.user.id, quota).await? {
        Some(n) => n,
        None if premium_entitled(&authed.user) => {
            let now = time::OffsetDateTime::now_utc();
            let secs_left = 86_400
                - (now.hour() as u64 * 3600 + now.minute() as u64 * 60 + now.second() as u64);
            return Err(ApiError::RateLimited {
                retry_after_secs: secs_left,
            });
        }
        None => {
            return Err(ApiError::PaymentRequired(format!(
                "You've used today's {quota} AI requests. Pro raises the limit."
            )))
        }
    };

    // ── the brief, exactly as the desktop builds it ────────────────────────
    let meta = &pack.meta;
    let topics = state
        .content
        .lib
        .guide
        .for_category(&meta.category)
        .iter()
        .map(|t| t.title.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let examples = examples(meta);
    let difficulty = match meta.difficulty {
        dsa_core::problem::Difficulty::Easy => "Easy",
        dsa_core::problem::Difficulty::Medium => "Medium",
        dsa_core::problem::Difficulty::Hard => "Hard",
    };
    let cx = dsa_ai::PromptContext {
        title: &meta.title,
        difficulty,
        description: &meta.description,
        approach: &meta.approach,
        complexity: &meta.complexity,
        category: &meta.category,
        topics: &topics,
        examples: &examples,
        lang_label: &lang.label,
        lang_id: &lang.id,
    };
    let prompts = &assistant.prompts;
    let chat = if mode == dsa_ai::Mode::Fix {
        let mut review = prompts.fix_user(&cx, code, req.run_context.as_deref().unwrap_or(""));
        // The panel may send a short note from the student ("it fails on
        // empty input"); it rides after the format rules without changing them.
        if let Some(note) = turns
            .last()
            .filter(|m| m.role == "user")
            .map(|m| m.content.trim())
            .filter(|n| !n.is_empty() && *n != FIX_DEFAULT_NOTE)
        {
            review.push_str(&format!(
                "\n\nThe candidate adds this note (keep the exact response format above): {note}"
            ));
        }
        ChatRequest {
            system: prompts.fix_system.clone(),
            messages: vec![(Role::User, review)],
            temperature: mode.temperature(),
        }
    } else {
        let recent: Vec<&dto::AiMessage> = turns
            .iter()
            .rev()
            .take(dsa_ai::HISTORY_TURNS)
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let last = recent.len().saturating_sub(1);
        let messages = normalize_turns(
            recent
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    if m.role == "user" {
                        let content = if i == last && !code.trim().is_empty() {
                            dsa_ai::attach_code(&m.content, &lang.id, code)
                        } else {
                            m.content.clone()
                        };
                        (Role::User, content)
                    } else {
                        (Role::Assistant, m.content.clone())
                    }
                })
                .collect(),
        );
        ChatRequest {
            system: if mode == dsa_ai::Mode::Interview {
                prompts.interview_system(&cx)
            } else {
                prompts.guide_system(&cx)
            },
            messages,
            temperature: mode.temperature(),
        }
    };

    let upstream = match assistant.stream(chat).await {
        Ok(s) => s,
        Err(msg) => {
            let _ = metering::release_ai_request(&state.db, authed.user.id).await;
            tracing::warn!(error = %msg, "ai provider failed before streaming");
            return Err(ApiError::Unavailable(msg));
        }
    };
    metrics::counter!("dsa_ai_requests_total", "mode" => req.mode.clone()).increment(1);

    let db = state.db.clone();
    let user = authed.user.id;
    let remaining = (quota - used).max(0);
    let stream = async_stream::stream! {
        let mut upstream = upstream;
        let (mut input, mut output, mut produced) = (0u64, 0u64, false);
        while let Some(item) = upstream.next().await {
            match item {
                Ok(Chunk::Text(t)) => {
                    produced = true;
                    yield event("token", json!({ "channel": "content", "text": t }));
                }
                Ok(Chunk::Thinking(t)) => {
                    produced = true;
                    yield event("token", json!({ "channel": "thinking", "text": t }));
                }
                Ok(Chunk::Usage { input: i, output: o }) => {
                    input = input.max(i);
                    output = output.max(o);
                }
                Ok(Chunk::Refused) => {
                    yield event("error", json!({ "code": "forbidden", "message": "The assistant can't help with that request. Try rephrasing it." }));
                    return;
                }
                Err(e) => {
                    if !produced {
                        let _ = metering::release_ai_request(&db, user).await;
                    }
                    yield event("error", json!({ "code": "unavailable", "message": e }));
                    return;
                }
            }
        }
        let _ = metering::add_ai_tokens(&db, user, input as i64, output as i64).await;
        yield event("done", json!({ "input_tokens": input, "output_tokens": output, "remaining_today": remaining }));
    };
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dsa_core::problem::InputValue as V;

    #[test]
    fn history_opens_with_the_user_and_alternates() {
        let t = normalize_turns(vec![
            (Role::Assistant, "stale reply".into()),
            (Role::User, "q1".into()),
            (Role::User, "q1 again".into()),
            (Role::Assistant, "a1".into()),
            (Role::User, "q2".into()),
        ]);
        assert_eq!(
            t,
            vec![
                (Role::User, "q1\n\nq1 again".into()),
                (Role::Assistant, "a1".into()),
                (Role::User, "q2".into()),
            ]
        );
    }

    #[test]
    fn examples_render_leetcode_style_values() {
        assert_eq!(fmt_value(&V::List(vec![V::Int(2), V::Int(7)])), "[2,7]");
        assert_eq!(fmt_value(&V::Str("ab".into())), "\"ab\"");
        assert_eq!(fmt_value(&V::Int(9)), "9");
    }
}
