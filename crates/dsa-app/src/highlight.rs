//! A small, dependency-free syntax highlighter.
//!
//! The alternative was `syntect`, which pulls in a regex engine and a bundle
//! of theme files to colour twenty-line snippets. This handles the four
//! languages the content ships with, is driven by the same `languages.toml`
//! data as everything else (via the `syntax` field), and costs nothing to
//! cross-compile.

use egui::text::LayoutJob;
use egui::{Color32, FontId, TextFormat};

pub struct Palette {
    pub text: Color32,
    pub keyword: Color32,
    pub type_name: Color32,
    pub string: Color32,
    pub number: Color32,
    pub comment: Color32,
    pub punct: Color32,
}

const GO: &[&str] = &[
    "func",
    "var",
    "const",
    "return",
    "if",
    "else",
    "for",
    "range",
    "switch",
    "case",
    "default",
    "break",
    "continue",
    "type",
    "struct",
    "map",
    "make",
    "append",
    "len",
    "nil",
    "true",
    "false",
    "package",
    "import",
    "go",
    "defer",
    "chan",
    "select",
    "interface",
];
const CPP: &[&str] = &[
    "int",
    "long",
    "char",
    "bool",
    "void",
    "auto",
    "const",
    "return",
    "if",
    "else",
    "for",
    "while",
    "switch",
    "case",
    "default",
    "break",
    "continue",
    "struct",
    "class",
    "public",
    "private",
    "true",
    "false",
    "nullptr",
    "new",
    "delete",
    "using",
    "namespace",
    "template",
    "typename",
    "static",
    "include",
    "vector",
    "string",
    "unordered_map",
    "unordered_set",
    "pair",
    "size_t",
];
const JAVA: &[&str] = &[
    "class",
    "interface",
    "public",
    "private",
    "protected",
    "static",
    "final",
    "void",
    "int",
    "long",
    "char",
    "boolean",
    "double",
    "return",
    "if",
    "else",
    "for",
    "while",
    "switch",
    "case",
    "default",
    "break",
    "continue",
    "new",
    "null",
    "true",
    "false",
    "import",
    "package",
    "extends",
    "implements",
    "this",
    "super",
    "throws",
];
const PYTHON: &[&str] = &[
    "def", "return", "if", "elif", "else", "for", "while", "in", "not", "and", "or", "None",
    "True", "False", "class", "import", "from", "as", "with", "lambda", "yield", "break",
    "continue", "pass", "self",
];

fn keywords(syntax: &str) -> &'static [&'static str] {
    match syntax {
        "go" => GO,
        "cpp" | "c++" | "c" => CPP,
        "java" => JAVA,
        "python" | "py" => PYTHON,
        _ => GO,
    }
}

fn comment_prefix(syntax: &str) -> &'static str {
    match syntax {
        "python" | "py" => "#",
        _ => "//",
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Tok {
    Word,
    Keyword,
    Type,
    Str,
    Num,
    Comment,
    Punct,
    Space,
}

/// Split one line into classified spans. Line-oriented on purpose: the code
/// panel draws row by row so it can put a breakpoint gutter beside each line.
fn tokenize(line: &str, syntax: &str) -> Vec<(Tok, String)> {
    let kw = keywords(syntax);
    let comment = comment_prefix(syntax);
    let chars: Vec<char> = line.chars().collect();
    let mut out: Vec<(Tok, String)> = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if line[byte_index(line, i)..].starts_with(comment) {
            out.push((Tok::Comment, chars[i..].iter().collect()));
            break;
        }

        if c == '"' || c == '\'' || c == '`' {
            let quote = c;
            let start = i;
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' {
                    i += 2;
                    continue;
                }
                if chars[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push((Tok::Str, chars[start..i.min(chars.len())].iter().collect()));
            continue;
        }

        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '.') {
                i += 1;
            }
            out.push((Tok::Num, chars[start..i].iter().collect()));
            continue;
        }

        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let kind = if kw.contains(&word.as_str()) {
                Tok::Keyword
            } else if word.chars().next().is_some_and(|c| c.is_uppercase()) {
                Tok::Type
            } else {
                Tok::Word
            };
            out.push((kind, word));
            continue;
        }

        if c.is_whitespace() {
            let start = i;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            out.push((Tok::Space, chars[start..i].iter().collect()));
            continue;
        }

        out.push((Tok::Punct, c.to_string()));
        i += 1;
    }
    out
}

fn byte_index(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

/// Build a laid-out, coloured line ready to hand to `ui.label`.
pub fn line_job(line: &str, syntax: &str, size: f32, p: &Palette) -> LayoutJob {
    let mut job = LayoutJob::default();
    let font = FontId::monospace(size);
    for (kind, text) in tokenize(line, syntax) {
        let color = match kind {
            Tok::Keyword => p.keyword,
            Tok::Type => p.type_name,
            Tok::Str => p.string,
            Tok::Num => p.number,
            Tok::Comment => p.comment,
            Tok::Punct => p.punct,
            _ => p.text,
        };
        job.append(
            &text,
            0.0,
            TextFormat {
                font_id: font.clone(),
                color,
                ..Default::default()
            },
        );
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str, syntax: &str) -> Vec<Tok> {
        tokenize(line, syntax)
            .into_iter()
            .map(|(k, _)| k)
            .filter(|k| *k != Tok::Space)
            .collect()
    }

    #[test]
    fn go_keywords_and_literals_are_classified() {
        let k = kinds("for i := 0; i < 10; i++ {", "go");
        assert_eq!(k[0], Tok::Keyword);
        assert!(k.contains(&Tok::Num));
    }

    #[test]
    fn comments_swallow_the_rest_of_the_line() {
        let toks = tokenize("x := 1 // set x to one", "go");
        let comment = toks.iter().find(|(k, _)| *k == Tok::Comment).unwrap();
        assert_eq!(comment.1, "// set x to one");
        assert!(!toks.iter().any(|(k, t)| *k == Tok::Word && t == "set"));
    }

    #[test]
    fn python_uses_hash_comments_not_slashes() {
        let toks = tokenize("seen = {}  # a map", "python");
        assert!(toks
            .iter()
            .any(|(k, t)| *k == Tok::Comment && t == "# a map"));
        let go_style = tokenize("x = 1 // not a python comment", "python");
        assert!(!go_style.iter().any(|(k, _)| *k == Tok::Comment));
    }

    #[test]
    fn strings_survive_escapes_and_embedded_comment_markers() {
        let toks = tokenize(r#"s := "a//b\" c" + t"#, "go");
        let s = toks.iter().find(|(k, _)| *k == Tok::Str).unwrap();
        assert_eq!(s.1, r#""a//b\" c""#);
        assert!(!toks.iter().any(|(k, _)| *k == Tok::Comment));
    }

    #[test]
    fn an_unterminated_string_does_not_loop_forever() {
        let toks = tokenize("s := \"oops", "go");
        assert!(toks.iter().any(|(k, _)| *k == Tok::Str));
    }

    #[test]
    fn capitalised_identifiers_read_as_types() {
        let toks = tokenize("Map<Integer, Integer> seen = new HashMap<>();", "java");
        assert!(toks.iter().any(|(k, t)| *k == Tok::Type && t == "Map"));
        assert!(toks.iter().any(|(k, t)| *k == Tok::Keyword && t == "new"));
    }

    #[test]
    fn non_ascii_content_does_not_panic_or_lose_text() {
        let line = "// π ≈ 3.14 — done";
        let toks = tokenize(line, "go");
        let joined: String = toks.iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(joined, line);
    }

    #[test]
    fn every_character_is_accounted_for() {
        let line = "if (seen.count(need)) { return {seen[need], i}; }";
        let joined: String = tokenize(line, "cpp")
            .iter()
            .map(|(_, t)| t.as_str())
            .collect();
        assert_eq!(joined, line);
    }
}
