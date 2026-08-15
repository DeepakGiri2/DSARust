//! Turning a small local model's free-form reply into something the UI can
//! render.
//!
//! These are the fiddly bits of the assistant and the ones most likely to
//! break silently on a model that formats slightly differently, so they are
//! pure functions with tests rather than inline string poking in the UI.

/// Some models emit reasoning inline as `<think>…</think>` instead of using
/// Ollama's separate thinking channel. Pull it out so it renders collapsed
/// either way.
pub fn extract_think(content: &str) -> (String, String) {
    let mut thinking = String::new();
    let mut out = String::with_capacity(content.len());
    let mut rest = content;

    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        let after = &rest[start + "<think>".len()..];
        match after.find("</think>") {
            Some(end) => {
                thinking.push_str(&after[..end]);
                rest = &after[end + "</think>".len()..];
            }
            None => {
                // Unterminated: the model is still streaming its reasoning.
                thinking.push_str(after);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    (thinking, out.trim().to_string())
}

/// A fenced code block and the text around it.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub code: bool,
    pub text: String,
}

/// Split a reply into plain-text and fenced-code segments, in order.
/// An unterminated final fence still yields its (partial) code, which is what
/// makes streaming look right rather than hiding the block until it closes.
pub fn segments(text: &str) -> Vec<Segment> {
    let mut out = Vec::new();
    let mut rest = text;

    while let Some(open) = rest.find("```") {
        if open > 0 {
            out.push(Segment {
                code: false,
                text: rest[..open].to_string(),
            });
        }
        let after = &rest[open + 3..];
        // Skip the optional language tag on the fence line.
        let body_start = after.find('\n').map(|i| i + 1).unwrap_or(after.len());
        let body = &after[body_start..];
        match body.find("```") {
            Some(close) => {
                out.push(Segment {
                    code: true,
                    text: body[..close].trim_end().to_string(),
                });
                rest = &body[close + 3..];
            }
            None => {
                out.push(Segment {
                    code: true,
                    text: body.to_string(),
                });
                rest = "";
            }
        }
    }
    if !rest.is_empty() {
        out.push(Segment {
            code: false,
            text: rest.to_string(),
        });
    }
    out
}

/// The last fenced code block plus everything else, which is how the fix mode
/// separates "issues" prose from the corrected function.
pub fn extract_last_code_block(text: &str) -> (Option<String>, String) {
    let segs = segments(text);
    let last_code = segs.iter().rposition(|s| s.code);
    match last_code {
        None => (None, text.to_string()),
        Some(i) => {
            let rest: String = segs
                .iter()
                .enumerate()
                .filter(|(j, s)| *j != i && !s.code)
                .map(|(_, s)| s.text.as_str())
                .collect();
            let rest = rest
                .trim_end()
                .trim_end_matches("FIXED CODE:")
                .trim_end_matches("Fixed code:")
                .trim()
                .to_string();
            (Some(segs[i].text.clone()), rest)
        }
    }
}

/// Guide mode asks clarifying questions by emitting `OPTION: …` lines; those
/// become clickable buttons instead of text.
pub fn parse_options(content: &str) -> (String, Vec<String>) {
    let mut options = Vec::new();
    let mut body = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim_start();
        match trimmed.strip_prefix("OPTION:") {
            Some(rest) => options.push(rest.trim().to_string()),
            None => body.push(line),
        }
    }
    (body.join("\n").trim().to_string(), options)
}

/// Small models often reformat indentation (tabs vs spaces), which would make
/// the editor diff flag every single line. Convert the fix's leading
/// whitespace back to the original's style.
pub fn match_indent(fixed: &str, original: &str) -> String {
    let orig_tabs = original.lines().any(|l| l.starts_with('\t'));
    let fixed_tabs = fixed.lines().any(|l| l.starts_with('\t'));
    if orig_tabs == fixed_tabs {
        return fixed.to_string();
    }
    fixed
        .lines()
        .map(|line| {
            if fixed_tabs {
                let tabs = line.len() - line.trim_start_matches('\t').len();
                format!("{}{}", "    ".repeat(tabs), line.trim_start_matches('\t'))
            } else {
                let spaces = line.len() - line.trim_start_matches(' ').len();
                format!(
                    "{}{}",
                    "\t".repeat(spaces / 4),
                    &line[spaces - (spaces % 4)..]
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_think_tags_are_lifted_out() {
        let (think, body) = extract_think("<think>hmm maybe</think>The answer is 4.");
        assert_eq!(think, "hmm maybe");
        assert_eq!(body, "The answer is 4.");
    }

    #[test]
    fn an_unterminated_think_block_is_all_reasoning() {
        let (think, body) = extract_think("<think>still going");
        assert_eq!(think, "still going");
        assert_eq!(body, "");
    }

    #[test]
    fn text_without_think_tags_is_untouched() {
        let (think, body) = extract_think("just an answer");
        assert!(think.is_empty());
        assert_eq!(body, "just an answer");
    }

    #[test]
    fn segments_alternate_between_prose_and_code() {
        let segs = segments("before\n```go\nx := 1\n```\nafter");
        assert_eq!(segs.len(), 3);
        assert!(!segs[0].code);
        assert!(segs[1].code);
        assert_eq!(segs[1].text, "x := 1");
        assert!(!segs[2].code);
    }

    #[test]
    fn a_streaming_unterminated_fence_still_shows_its_code() {
        let segs = segments("here:\n```go\nfunc f() {");
        assert!(segs[1].code);
        assert_eq!(segs[1].text, "func f() {");
    }

    #[test]
    fn the_last_code_block_wins_and_prose_is_kept() {
        let reply = "ISSUES:\n- off by one\n\nFIXED CODE:\n```go\nfunc f() {}\n```";
        let (code, rest) = extract_last_code_block(reply);
        assert_eq!(code.unwrap(), "func f() {}");
        assert!(rest.contains("off by one"));
        assert!(!rest.contains("FIXED CODE"));
    }

    #[test]
    fn two_code_blocks_take_the_second() {
        let reply = "```go\nold\n```\nand the fix:\n```go\nnew\n```";
        let (code, _) = extract_last_code_block(reply);
        assert_eq!(code.unwrap(), "new");
    }

    #[test]
    fn a_reply_with_no_code_reports_none() {
        let (code, rest) = extract_last_code_block("- none found");
        assert!(code.is_none());
        assert_eq!(rest, "- none found");
    }

    #[test]
    fn option_lines_become_choices() {
        let (body, options) = parse_options(
            "What would you like help with?\nOPTION: Explain the approach\n  OPTION:  Review my code ",
        );
        assert_eq!(body, "What would you like help with?");
        assert_eq!(options, vec!["Explain the approach", "Review my code"]);
    }

    #[test]
    fn text_without_options_keeps_every_line() {
        let (body, options) = parse_options("line one\nline two");
        assert_eq!(body, "line one\nline two");
        assert!(options.is_empty());
    }

    #[test]
    fn indentation_is_converted_to_the_originals_style() {
        let original = "func f() {\n\treturn 1\n}";
        let fixed = "func f() {\n    return 2\n}";
        let out = match_indent(fixed, original);
        assert!(out.contains("\treturn 2"), "{out:?}");
    }

    #[test]
    fn matching_indentation_is_left_alone() {
        let original = "func f() {\n    return 1\n}";
        let fixed = "func f() {\n    return 2\n}";
        assert_eq!(match_indent(fixed, original), fixed);
    }
}
