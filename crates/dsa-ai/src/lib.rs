//! Local AI assistant.
//!
//! Three modes, ported from the web version:
//!
//! * **Interview** — nudges with questions, never writes code or names the
//!   technique. Code blocks it does emit are rendered locked.
//! * **Guide** — a mentor that answers exactly what was asked, and responds to
//!   a vague question with clickable `OPTION:` choices.
//! * **Fix** — reviews the current solution and returns issues plus a
//!   corrected function, which the editor opens as a reviewable diff.
//!
//! Everything runs against a local Ollama server; nothing is sent anywhere
//! else, and with no server running the rest of the app is unaffected.

pub mod client;
pub mod parse;
pub mod prompts;

pub use client::{
    chat_stream, list_models, Channel, ChatMessage, ChatOptions, Event, Role, Stream,
    DEFAULT_OLLAMA_URL,
};
pub use parse::{
    extract_last_code_block, extract_think, match_indent, parse_options, segments, Segment,
};
pub use prompts::{attach_code, PromptContext, Prompts};

/// Which assistant tab is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Interview,
    Guide,
    Fix,
}

impl Mode {
    pub fn label(&self) -> &'static str {
        match self {
            Mode::Interview => "Interview",
            Mode::Guide => "Guide",
            Mode::Fix => "Fix",
        }
    }

    /// Who the reply is attributed to in the transcript.
    pub fn speaker(&self) -> &'static str {
        match self {
            Mode::Interview => "interviewer",
            _ => "mentor",
        }
    }

    pub fn temperature(&self) -> f32 {
        match self {
            Mode::Interview => 0.6,
            Mode::Guide => 0.4,
            Mode::Fix => 0.2,
        }
    }

    pub fn placeholder(&self) -> &'static str {
        match self {
            Mode::Interview => "ask the interviewer… (Enter to send)",
            _ => "ask the mentor… (Enter to send)",
        }
    }

    pub fn empty_hint(&self) -> &'static str {
        match self {
            Mode::Interview => "Practice like a real interview — the AI nudges you with questions and tiny hints but never reveals code or names the technique. Try \"am I on the right track?\".",
            Mode::Guide => "A teaching mentor — ask about syntax, patterns, best practices or complexity and it answers exactly what you asked. Vague question? It asks back with clickable choices.",
            Mode::Fix => "Points out what's wrong in your solution, then opens the proposed fix directly in the editor as a diff — accept or discard it there. Run your code or tests first so the AI sees the errors too.",
        }
    }
}

/// How many turns of history are sent back to the model. Local models have
/// small context windows, and the code snapshot is attached separately.
pub const HISTORY_TURNS: usize = 12;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interview_runs_hotter_than_fix() {
        assert!(Mode::Interview.temperature() > Mode::Guide.temperature());
        assert!(Mode::Guide.temperature() > Mode::Fix.temperature());
    }

    #[test]
    fn every_mode_has_a_hint_and_a_speaker() {
        for m in [Mode::Interview, Mode::Guide, Mode::Fix] {
            assert!(!m.empty_hint().is_empty());
            assert!(!m.speaker().is_empty());
            assert!(!m.label().is_empty());
        }
        assert_eq!(Mode::Interview.speaker(), "interviewer");
    }
}
