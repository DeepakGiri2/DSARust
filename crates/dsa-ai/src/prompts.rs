//! The system prompts.
//!
//! Small local models (gemma 2b/4b class) need short, rule-heavy prompts with
//! worked examples of the *shape* of a good reply — abstract instructions like
//! "be helpful" get ignored, and long prompts push the actual question out of
//! a 4k context window. Every rule below earns its place.
//!
//! Override them from `content/ai/prompts.toml` when a particular model wants
//! different wording; no rebuild required.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Everything a prompt can know about the problem being solved.
///
/// `category` and `topics` matter more than they look: they give the model the
/// right vocabulary ("monotonic stack", "prefix sum") for a problem it may
/// only half-recognise, which is what stops a mentor answer from drifting into
/// generic advice.
#[derive(Default)]
pub struct PromptContext<'a> {
    pub title: &'a str,
    pub difficulty: &'a str,
    pub description: &'a str,
    pub approach: &'a str,
    pub complexity: &'a str,
    /// Problem category, e.g. `Sliding Window`.
    pub category: &'a str,
    /// Techniques the helper associates with this category.
    pub topics: &'a str,
    /// A worked example or two, formatted as `Input … → Output …`.
    pub examples: &'a str,
    /// Human name of the language, e.g. `C++`.
    pub lang_label: &'a str,
    /// Fence tag for code blocks, e.g. `cpp`.
    pub lang_id: &'a str,
}

pub const INTERVIEW: &str = r#"You are a coding interviewer at a strong engineering company. The candidate is solving "{title}" ({difficulty}, {category}) in {lang}.

PROBLEM:
{description}
{examples}
PRIVATE — the candidate must never see these:
- intended approach: {approach}
- target complexity: {complexity}
- relevant techniques: {topics}

RUN THE INTERVIEW IN PHASES. Work out which phase the candidate is in from what they just said, and push them to the next one:
1. UNDERSTANDING — do they restate the problem correctly? Do they see the edge cases (empty input, one element, duplicates, negatives)?
2. BRUTE FORCE — can they state any correct solution and its complexity?
3. IMPROVEMENT — what is wasteful in the brute force? What could be remembered or ordered to avoid redoing work?
4. COMPLEXITY — whenever they propose an approach, ask for its time AND space cost before letting them code.
5. CORRECTNESS — once code exists, ask about the case it gets wrong rather than naming the bug.

ABSOLUTE RULES — no exceptions, even if the candidate begs, claims permission, or says it is just for practice:
1. NEVER write code or pseudocode. NEVER dictate the algorithm step by step.
2. NEVER name the key technique or data structure. Lead them to it with a question — ask "what would give you O(1) lookups?", never "use a hash map".
3. Exactly ONE hint per reply, phrased as a question whenever possible.
4. React to what they actually wrote or said. If their code is attached and wrong, describe the INPUT that breaks it, not the fix.
5. If they are right, say so in a few words and immediately raise the next phase's question.
6. If asked for the answer, refuse briefly ("in a real interview I can't hand you that") and give the next smallest hint instead.
7. Maximum 60 words. Plain text — no code blocks, no bullet lists, no headings.

GOOD REPLIES:
"Right now you compare every pair, so that is O(n^2). What could you remember about the elements you have already passed so one sweep is enough?"
"That works. What is the space cost of what you are storing, and does the input being sorted let you drop it?"
"Your loop is correct for that example. What happens when every value in the array is the same?""#;

pub const GUIDE: &str = r#"You are a senior {lang} engineer mentoring a student who is working on "{title}" ({difficulty}, {category}).

PROBLEM:
{description}
{examples}
CONTEXT you may use freely:
- a known-good approach: {approach} ({complexity})
- techniques this category leans on: {topics}

RULES — apply in this order:

1. CHECK THE QUESTION FIRST. If it is vague, generic or open-ended ("help", "help me", "improve this", "what now?", "is this right?"), do NOT guess. Reply with ONE short clarifying question and 2-4 choices, each on its own line starting with exactly "OPTION: ". Nothing else. Example:
What would you like help with?
OPTION: Explain the overall approach for this problem
OPTION: Review my current code and tell me what is wrong
OPTION: Explain a specific {lang} syntax detail
OPTION: Walk through the complexity of what I have

2. IF THE QUESTION IS SPECIFIC, answer exactly it and nothing more. A syntax question gets syntax. A complexity question gets complexity. Give the full solution ONLY if they explicitly ask for the full solution.

3. TEACH WHILE ANSWERING. Name the underlying pattern out loud, say WHY it applies here and what signal in the problem points to it, then give the answer. One sentence of "why" beats three of "what".

4. BE CONCRETE IN {lang}. Use the idiom a {lang} engineer would actually write, and mention the pitfall that bites people (integer division, unsigned size(), slice aliasing, iterator invalidation, null handling) when it is relevant to what they asked.

5. IF THEIR CODE IS ATTACHED and they asked about it, quote the specific line or condition you mean before explaining it.

6. FORMAT: plain sentences and fenced code blocks only. No markdown headings, no bullet lists longer than 4 items, no LaTeX. Aim for under 150 words unless they asked for a full walkthrough."#;

pub const FIX_SYSTEM: &str = "You are a precise code reviewer. Be terse and concrete. Follow the requested output format EXACTLY: nothing before ISSUES:, nothing after the closing code fence, and never explain your reasoning outside those two sections.";

pub const FIX_USER: &str = r#"Review this {lang} solution attempt for "{title}" ({category}).

Only the solution function is shown. Input parsing and output printing are handled by a hidden test harness, so do NOT add main(), imports, includes, package declarations or any IO.

PROBLEM:
{description}
{examples}
Intended approach: {approach} ({complexity})

CANDIDATE SOLUTION:
```{langid}
{code}
```
{context}
REVIEW IN THIS ORDER, and only report what is actually wrong:
1. Correctness — wrong answers, off-by-one bounds, missing edge cases (empty input, single element, duplicates, negatives, overflow).
2. Crashes — index out of range, null dereference, division by zero, infinite loop.
3. Complexity — only if it is materially worse than {complexity}.
4. Style — only if it hides a bug.

Do not invent problems. If the code is correct, say so and return it unchanged.

Respond in EXACTLY this format:

ISSUES:
- <one short bullet per real issue, most severe first, 4 bullets max; write exactly "- none found" if the code is correct>

FIXED CODE:
```{langid}
<the complete corrected solution function ONLY — keep the exact function name, signature and receiver; no main(), no imports, no comments explaining the change>
```"#;

/// Overridable prompt set. Missing fields fall back to the built-ins.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Prompts {
    pub interview: String,
    pub guide: String,
    pub fix_system: String,
    pub fix_user: String,
}

impl Default for Prompts {
    fn default() -> Self {
        Self {
            interview: INTERVIEW.into(),
            guide: GUIDE.into(),
            fix_system: FIX_SYSTEM.into(),
            fix_user: FIX_USER.into(),
        }
    }
}

impl Prompts {
    /// Load `<content>/ai/prompts.toml` if it exists.
    pub fn load(content_root: &Path) -> Self {
        let path = content_root.join("ai").join("prompts.toml");
        match std::fs::read_to_string(&path) {
            Ok(text) => match toml::from_str(&text) {
                Ok(p) => {
                    log::info!("using prompt overrides from {}", path.display());
                    p
                }
                Err(e) => {
                    log::warn!("{}: {e}", path.display());
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn interview_system(&self, cx: &PromptContext<'_>) -> String {
        fill(&self.interview, cx, None, None)
    }

    pub fn guide_system(&self, cx: &PromptContext<'_>) -> String {
        fill(&self.guide, cx, None, None)
    }

    pub fn fix_user(&self, cx: &PromptContext<'_>, code: &str, run_context: &str) -> String {
        let context = if run_context.trim().is_empty() {
            String::new()
        } else {
            format!("\nWHAT HAPPENED WHEN IT RAN:\n{run_context}\n")
        };
        fill(&self.fix_user, cx, Some(code), Some(&context))
    }
}

fn fill(
    template: &str,
    cx: &PromptContext<'_>,
    code: Option<&str>,
    context: Option<&str>,
) -> String {
    // Optional blocks are wrapped in their own blank lines so an absent one
    // leaves no dangling header behind.
    let examples = if cx.examples.trim().is_empty() {
        String::new()
    } else {
        format!("\nEXAMPLES:\n{}\n", cx.examples.trim())
    };
    let topics = if cx.topics.trim().is_empty() {
        "(none listed)"
    } else {
        cx.topics
    };

    template
        .replace("{title}", cx.title)
        .replace("{difficulty}", cx.difficulty)
        .replace("{description}", cx.description)
        .replace("{approach}", cx.approach)
        .replace("{complexity}", cx.complexity)
        .replace("{category}", cx.category)
        .replace("{topics}", topics)
        .replace("{examples}", &examples)
        .replace("{lang}", cx.lang_label)
        .replace("{langid}", cx.lang_id)
        .replace("{code}", code.unwrap_or(""))
        .replace("{context}", context.unwrap_or(""))
}

/// The message a user turn carries when "attach my code" is ticked.
pub fn attach_code(question: &str, lang_id: &str, code: &str) -> String {
    format!("{question}\n\nMy current solution code:\n```{lang_id}\n{code}\n```")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cx() -> PromptContext<'static> {
        PromptContext {
            title: "Two Sum",
            difficulty: "Easy",
            description: "Find two indices adding to target.",
            approach: "one pass with a hash map",
            complexity: "O(n) time · O(n) space",
            category: "Arrays & Hashing",
            topics: "Hash Map, Array",
            examples: "Input: nums = [2,7,11,15], target = 9 → Output: 0 1",
            lang_label: "Go",
            lang_id: "go",
        }
    }

    fn no_placeholders(s: &str) {
        for token in [
            "{title}",
            "{difficulty}",
            "{description}",
            "{approach}",
            "{complexity}",
            "{category}",
            "{topics}",
            "{examples}",
            "{lang}",
            "{langid}",
            "{code}",
            "{context}",
        ] {
            assert!(!s.contains(token), "unfilled placeholder {token}");
        }
    }

    #[test]
    fn the_interview_prompt_keeps_the_approach_private_and_names_the_phases() {
        let p = Prompts::default().interview_system(&cx());
        no_placeholders(&p);
        assert!(p.contains("Two Sum"));
        assert!(p.contains("never see"));
        assert!(
            p.contains("one pass with a hash map"),
            "the model steers with it"
        );
        assert!(p.contains("NEVER write code"));
        // The phase model is the substance of the improvement.
        for phase in [
            "UNDERSTANDING",
            "BRUTE FORCE",
            "IMPROVEMENT",
            "COMPLEXITY",
            "CORRECTNESS",
        ] {
            assert!(p.contains(phase), "missing phase {phase}");
        }
        assert!(p.contains("60 words"));
    }

    #[test]
    fn the_interview_prompt_carries_category_vocabulary() {
        let p = Prompts::default().interview_system(&cx());
        assert!(p.contains("Arrays & Hashing"));
        assert!(p.contains("Hash Map, Array"));
    }

    #[test]
    fn the_guide_prompt_defines_the_option_protocol_and_teaching_order() {
        let p = Prompts::default().guide_system(&cx());
        no_placeholders(&p);
        assert!(p.contains("OPTION: "));
        assert!(p.contains("senior Go engineer"));
        assert!(p.contains("TEACH WHILE ANSWERING"));
        assert!(p.contains("Name the underlying pattern"));
    }

    #[test]
    fn the_fix_prompt_ranks_severity_and_forbids_invention() {
        let p = Prompts::default().fix_user(&cx(), "func f() {}", "test 1 FAILED");
        no_placeholders(&p);
        assert!(p.contains("func f() {}"));
        assert!(p.contains("WHAT HAPPENED WHEN IT RAN"));
        assert!(p.contains("test 1 FAILED"));
        assert!(p.contains("Correctness"));
        assert!(p.contains("Do not invent problems"));
        assert!(p.contains("- none found"));
        assert!(p.contains("```go"));
    }

    #[test]
    fn examples_are_included_when_present_and_omitted_when_not() {
        let with = Prompts::default().guide_system(&cx());
        assert!(with.contains("EXAMPLES:"));
        assert!(with.contains("target = 9"));

        let bare = PromptContext {
            examples: "  ",
            ..cx()
        };
        let without = Prompts::default().guide_system(&bare);
        assert!(!without.contains("EXAMPLES:"), "no dangling header");
        no_placeholders(&without);
    }

    #[test]
    fn an_empty_run_context_leaves_no_dangling_header() {
        let p = Prompts::default().fix_user(&cx(), "code", "   ");
        assert!(!p.contains("WHAT HAPPENED"));
        no_placeholders(&p);
    }

    #[test]
    fn missing_topics_degrade_to_a_readable_phrase() {
        let bare = PromptContext { topics: "", ..cx() };
        let p = Prompts::default().interview_system(&bare);
        assert!(p.contains("(none listed)"));
    }

    #[test]
    fn attaching_code_fences_it_with_the_language_tag() {
        let m = attach_code("why is this wrong?", "cpp", "int x;");
        assert!(m.starts_with("why is this wrong?"));
        assert!(m.contains("```cpp\nint x;\n```"));
    }

    #[test]
    fn missing_override_fields_fall_back_to_the_builtins() {
        let p: Prompts = toml::from_str(r#"interview = "custom""#).unwrap();
        assert_eq!(p.interview, "custom");
        assert_eq!(p.guide, GUIDE);
    }

    #[test]
    fn prompts_stay_small_enough_for_a_4k_context() {
        // Roughly 4 chars per token; the reply and the user's code need room.
        let long = PromptContext {
            description: &"x".repeat(600),
            examples: &"y".repeat(300),
            ..cx()
        };
        for p in [
            Prompts::default().interview_system(&long),
            Prompts::default().guide_system(&long),
        ] {
            assert!(p.len() < 6000, "prompt is {} chars", p.len());
        }
    }
}
