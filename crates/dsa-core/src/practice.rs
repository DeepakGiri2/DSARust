//! What the Practice tab writes into, and what it hands the compiler.
//!
//! A pack ships the same solution twice: `code/<lang>.txt`, the `//@tag`-marked
//! reference the animation is annotated against, and — for packs that have one
//! — `practice/<lang>.txt`, a complete program that reads stdin, calls the
//! solution and prints the answer. Practice wants neither of them verbatim:
//!
//! * Opening the tab must not hand over the answer, so the editor starts from
//!   the reference with every function *body* replaced by a placeholder. What
//!   survives is the shape — signatures, receivers, the struct a design problem
//!   is built around — which is what LeetCode gives you and what the harness
//!   needs in order to link.
//! * Running what the user wrote has to produce a whole program, so the
//!   reference is located inside the harness and their text put in its place.
//!
//! Both are text transforms over content rather than anything about the GUI, so
//! they live here, where they can be tested against the real packs.
//!
//! The scanner assumes what every `code/<lang>.txt` in the tree actually is:
//! braces for blocks, `//` for comments, no block comments and no brace inside
//! a multi-line raw string. It is not a parser and does not need to be — a
//! source it cannot make sense of falls back to being left alone.

/// The body a blanked function gets: a comment saying what to do, and a
/// statement that type-checks whatever the signature promised to return.
fn todo_body(lang: &str) -> &'static str {
    match lang {
        "cpp" => "// write your code here\nthrow runtime_error(\"todo\");",
        "java" => "// write your code here\nthrow new UnsupportedOperationException(\"todo\");",
        "python" => "# write your code here\nraise NotImplementedError",
        // Go's `panic` is a terminating statement, so this compiles whatever
        // the function's result list says.
        _ => "// write your code here\npanic(\"todo\")",
    }
}

/// Net `open` minus `close` on a line, ignoring both inside string and
/// character literals and after a `//` comment.
fn delta(line: &str, open: char, close: char) -> i32 {
    let mut depth = 0;
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            // A backtick string is Go's raw form: no escapes inside it.
            Some(q) => {
                if c == '\\' && q != '`' {
                    chars.next();
                } else if c == q {
                    quote = None;
                }
            }
            None => match c {
                '"' | '\'' | '`' => quote = Some(c),
                '/' if chars.peek() == Some(&'/') => break,
                _ if c == open => depth += 1,
                _ if c == close => depth -= 1,
                _ => {}
            },
        }
    }
    depth
}

pub(crate) fn brace_delta(line: &str) -> i32 {
    delta(line, '{', '}')
}

pub(crate) fn paren_delta(line: &str) -> i32 {
    delta(line, '(', ')')
}

/// True when a declaration head opens a function body rather than a type body.
///
/// The parameter list is the whole distinction: `func (m *Map) get(k int) int {`
/// and `static int[] twoSum(int[] nums, int target) {` open something whose body
/// is the answer, while `type MyHashMap struct {` and `class Main {` open
/// something whose body is scaffolding and has to survive.
///
/// `head` may be several source lines joined together — a C++ signature wide
/// enough to wrap is still one declaration.
pub(crate) fn opens_function(head: &str) -> bool {
    let head = match head.trim_end().strip_suffix('{') {
        Some(h) => h,
        None => return false,
    };
    match (head.find('('), head.rfind(')')) {
        (Some(open), Some(close)) => close > open,
        _ => false,
    }
}

fn indent_of(line: &str) -> &str {
    &line[..line.len() - line.trim_start().len()]
}

/// Byte index of the first `target` that is not inside a literal, stopping at a
/// `//` comment. `None` if there is none.
fn first_unquoted(line: &str, target: char) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut chars = line.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match quote {
            Some(q) => {
                if c == '\\' && q != '`' {
                    chars.next();
                } else if c == q {
                    quote = None;
                }
            }
            None => match c {
                '"' | '\'' | '`' => quote = Some(c),
                '/' if chars.peek().map(|(_, c)| *c) == Some('/') => return None,
                _ if c == target => return Some(i),
                _ => {}
            },
        }
    }
    None
}

/// A whole function written on one line — `int hash(int k) { return k % 8; }` —
/// re-emitted with its body blanked.
///
/// Rare (four sources in the tree) but the leak is not: `design-hashmap`'s
/// hash function is the one idea that problem is about. An *empty* one-line
/// body is left alone, because a C++ constructor like `Map() : buckets(8) {}`
/// hides nothing and has to keep working.
///
/// The literal-aware scan is what separates this from an initializer: the `{`
/// in `map<char, char> pairs = {{')','('}}` has no parameter list before it.
fn blank_one_liner(line: &str, lang: &str) -> Option<String> {
    if brace_delta(line) != 0 {
        return None;
    }
    let core = line.trim_end().trim_end_matches(';').trim_end();
    if !core.ends_with('}') {
        return None;
    }
    let open = first_unquoted(line, '{')?;
    let close = core.len() - 1;
    if close <= open || !opens_function(&line[..=open]) || line[open + 1..close].trim().is_empty() {
        return None;
    }

    let indent = indent_of(line);
    let mut out = format!("{}\n", &line[..=open]);
    for body_line in todo_body(lang).lines() {
        out.push_str(indent);
        out.push_str("    ");
        out.push_str(body_line);
        out.push('\n');
    }
    out.push_str(indent);
    out.push('}');
    out.push_str(line[close + 1..].trim_end()); // a trailing `;`, if there was one
    out.push('\n');
    Some(out)
}

/// The reference with every function body replaced by [`todo_body`].
///
/// A source with no function in it at all is returned unchanged — there is
/// nothing to blank, and an empty editor would be worse than a visible one.
pub fn starter(reference: &str, lang: &str) -> String {
    let mut out = String::with_capacity(reference.len());
    let mut lines = reference.lines().peekable();
    let mut blanked = false;

    while let Some(line) = lines.next() {
        if let Some(one_liner) = blank_one_liner(line, lang) {
            out.push_str(&one_liner);
            blanked = true;
            continue;
        }

        // A declaration head runs until its parameter list closes, so a
        // signature too wide for one line is still recognised as one.
        let mut head = vec![line];
        let mut open_parens = paren_delta(line);
        while open_parens > 0 {
            match lines.next() {
                Some(l) => {
                    open_parens += paren_delta(l);
                    head.push(l);
                }
                None => break,
            }
        }
        for l in &head {
            out.push_str(l);
            out.push('\n');
        }
        if !opens_function(&head.join(" ")) {
            continue;
        }

        let indent = indent_of(head[0]).to_string();
        // Consume the real body. The line that closes it is kept verbatim
        // rather than rebuilt, because in C++ it is often `};` and not `}`.
        let mut depth: i32 = head.iter().map(|l| brace_delta(l)).sum();
        let mut closer = format!("{indent}}}");
        while depth > 0 {
            match lines.next() {
                Some(l) => {
                    depth += brace_delta(l);
                    closer = l.to_string();
                }
                None => break,
            }
        }

        for body_line in todo_body(lang).lines() {
            out.push_str(&indent);
            out.push_str("    ");
            out.push_str(body_line);
            out.push('\n');
        }
        out.push_str(&closer);
        out.push('\n');
        blanked = true;
    }

    if blanked {
        out
    } else {
        reference.to_string()
    }
}

/// Where `reference` sits inside `harness`: the line range it occupies and the
/// extra indent the harness wraps it in (four spaces, for a Java pack whose
/// solution lives inside `class Main`).
///
/// Lines are compared trimmed, so a harness that re-indents the solution — or
/// differs from it by trailing whitespace — still matches.
fn locate(harness: &str, reference: &str) -> Option<(usize, usize, String)> {
    let hay: Vec<&str> = harness.lines().collect();
    let mut needle: Vec<&str> = reference
        .lines()
        .skip_while(|l| l.trim().is_empty())
        .collect();
    while needle.last().is_some_and(|l| l.trim().is_empty()) {
        needle.pop();
    }
    if needle.is_empty() || needle.len() > hay.len() {
        return None;
    }

    'candidate: for start in 0..=hay.len() - needle.len() {
        for (k, want) in needle.iter().enumerate() {
            if hay[start + k].trim() != want.trim() {
                continue 'candidate;
            }
        }
        let outer = indent_of(hay[start]).len();
        let inner = indent_of(needle[0]).len();
        if outer < inner {
            continue;
        }
        let indent = hay[start][..outer - inner].to_string();
        return Some((start, start + needle.len(), indent));
    }
    None
}

/// The harness with the reference solution swapped out for `solution`.
///
/// With no harness — most packs ship only `code/<lang>.txt` — or with one the
/// reference cannot be found in, the user's text *is* the program, which is the
/// same thing the web version does when a problem has no harness entry.
pub fn assemble(solution: &str, reference: &str, harness: Option<&str>) -> String {
    let harness = match harness {
        Some(h) if !h.trim().is_empty() => h,
        _ => return solution.to_string(),
    };
    let (start, end, indent) = match locate(harness, reference) {
        Some(found) => found,
        None => return solution.to_string(),
    };

    let lines: Vec<&str> = harness.lines().collect();
    let mut out = String::with_capacity(harness.len() + solution.len());
    for line in &lines[..start] {
        out.push_str(line);
        out.push('\n');
    }
    for line in solution.lines() {
        if !line.trim().is_empty() {
            out.push_str(&indent);
            out.push_str(line);
        }
        out.push('\n');
    }
    for line in &lines[end..] {
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const GO: &str = "func twoSum(nums []int, target int) []int {\n    seen := map[int]int{}\n    for i, x := range nums {\n        if j, ok := seen[target-x]; ok {\n            return []int{j, i}\n        }\n        seen[x] = i\n    }\n    return nil\n}\n";

    const GO_HARNESS: &str = "package main\n\nimport \"fmt\"\n\nfunc twoSum(nums []int, target int) []int {\n    seen := map[int]int{}\n    for i, x := range nums {\n        if j, ok := seen[target-x]; ok {\n            return []int{j, i}\n        }\n        seen[x] = i\n    }\n    return nil\n}\n\nfunc main() {\n    fmt.Println(twoSum([]int{2, 7}, 9))\n}\n";

    #[test]
    fn the_starter_keeps_the_signature_and_drops_the_answer() {
        let s = starter(GO, "go");
        assert!(s.starts_with("func twoSum(nums []int, target int) []int {"));
        assert!(s.contains("// write your code here"));
        assert!(s.contains("panic(\"todo\")"));
        // The whole point: none of the algorithm survives.
        assert!(!s.contains("seen"));
        assert!(!s.contains("range nums"));
        assert_eq!(s.lines().count(), 4, "signature, two body lines, close");
    }

    #[test]
    fn every_function_is_blanked_not_just_the_first() {
        let src = "func climbStairs(n int) int {\n    return climb(n, map[int]int{})\n}\n\nfunc climb(n int, memo map[int]int) int {\n    return n\n}\n";
        let s = starter(src, "go");
        assert!(s.contains("func climbStairs(n int) int {"));
        assert!(s.contains("func climb(n int, memo map[int]int) int {"));
        assert!(
            !s.contains("return climb("),
            "the helper call is the answer"
        );
        assert_eq!(s.matches("write your code here").count(), 2);
    }

    #[test]
    fn a_design_problems_struct_survives_but_its_methods_do_not() {
        let src = "type MyHashMap struct {\n    buckets [][]entry\n}\n\nfunc (m *MyHashMap) Get(key int) int {\n    return m.buckets[key%8][0].value\n}\n";
        let s = starter(src, "go");
        // Without the field list nothing the user writes can compile.
        assert!(s.contains("buckets [][]entry"));
        assert!(s.contains("func (m *MyHashMap) Get(key int) int {"));
        assert!(!s.contains("key%8"));
    }

    #[test]
    fn a_cpp_class_keeps_its_closing_semicolon() {
        let src =
            "class Solution {\npublic:\n    int f(int x) {\n        return x * 2;\n    }\n};\n";
        let s = starter(src, "cpp");
        assert!(s.contains("class Solution {"));
        assert!(s.contains("throw runtime_error(\"todo\");"));
        assert!(!s.contains("x * 2"));
        assert!(s.trim_end().ends_with("};"), "got:\n{s}");
    }

    #[test]
    fn the_placeholder_matches_the_language() {
        assert!(starter(GO, "go").contains("panic"));
        assert!(starter("int f() {\n    return 1;\n}\n", "cpp").contains("runtime_error"));
        assert!(starter("int f() {\n    return 1;\n}\n", "java").contains("UnsupportedOperation"));
    }

    #[test]
    fn a_source_with_no_function_is_left_alone() {
        let src = "type entry struct{ key, value int }\n";
        assert_eq!(starter(src, "go"), src);
    }

    #[test]
    fn a_brace_in_a_string_does_not_confuse_the_scanner() {
        assert_eq!(brace_delta("fmt.Println(\"}\")"), 0);
        assert_eq!(brace_delta("c := '{'"), 0);
        assert_eq!(brace_delta("x := 1 // }"), 0);
        assert_eq!(brace_delta("if x { // {"), 1);
        assert_eq!(brace_delta("seen := map[int]int{}"), 0);
    }

    #[test]
    fn a_signature_wrapped_over_two_lines_is_still_one_signature() {
        // Both `evaluate-division` and `path-with-maximum-probability` wrap
        // their C++ parameter list, and used to keep their whole body.
        let src = "vector<double> calc(vector<vector<string>>& equations, vector<double>& values,\n                    vector<vector<string>>& queries) {\n    vector<double> ans;\n    return ans;\n}\n";
        let s = starter(src, "cpp");
        assert!(s.contains("vector<vector<string>>& queries) {"));
        assert!(s.contains("write your code here"));
        assert!(!s.contains("return ans;"));
    }

    #[test]
    fn a_function_written_on_one_line_is_blanked_too() {
        let src = "class MyHashMap {\n    vector<vector<int>> buckets;\n    int hash(int key) { return key % (int)buckets.size(); }\npublic:\n    MyHashMap() : buckets(8) {}\n};\n";
        let s = starter(src, "cpp");
        assert!(s.contains("int hash(int key) {"));
        assert!(!s.contains("key % (int)buckets.size()"), "got:\n{s}");
        // An empty one-line body hides nothing and still has to construct.
        assert!(s.contains("MyHashMap() : buckets(8) {}"));
        // The field list is the shell, not the answer.
        assert!(s.contains("vector<vector<int>> buckets;"));
    }

    #[test]
    fn a_braced_initializer_is_not_mistaken_for_a_one_line_function() {
        // The parens here live inside char literals, after the brace.
        for src in [
            "map<char, char> pairs = {{')','('}, {']','['}};\n",
            "pairs := map[byte]byte{')': '(', ']': '['}\n",
            "int dirs[4][2] = {{1, 0}, {-1, 0}, {0, 1}, {0, -1}};\n",
        ] {
            assert_eq!(starter(src, "cpp"), src, "rewrote a declaration: {src}");
        }
    }

    #[test]
    fn a_one_line_constructor_is_not_treated_as_an_opener() {
        assert!(!opens_function(
            "    ListNode(int v) : val(v), next(nullptr) {}"
        ));
        assert!(!opens_function("type MyHashMap struct {"));
        assert!(!opens_function("class Main {"));
        assert!(opens_function("func (m *MyHashMap) Get(key int) int {"));
        assert!(opens_function(
            "static int[] twoSum(int[] nums, int target) {"
        ));
    }

    #[test]
    fn assembling_puts_the_users_code_where_the_reference_was() {
        let mine = "func twoSum(nums []int, target int) []int {\n    return []int{0, 1}\n}\n";
        let program = assemble(mine, GO, Some(GO_HARNESS));
        assert!(program.starts_with("package main"));
        assert!(program.contains("return []int{0, 1}"));
        assert!(program.contains("func main() {"));
        // The reference must be gone, or the file has two twoSum functions.
        assert!(!program.contains("seen := map[int]int{}"));
        assert_eq!(program.matches("func twoSum").count(), 1);
    }

    #[test]
    fn a_java_harness_re_indents_the_solution_into_its_class() {
        let reference = "static int f(int x) {\n    return x;\n}\n";
        let harness =
            "class Main {\n\n    static int f(int x) {\n        return x;\n    }\n\n    public static void main(String[] a) {}\n}\n";
        let mine = "static int f(int x) {\n    return 42;\n}\n";
        let program = assemble(mine, reference, Some(harness));
        assert!(
            program.contains("    static int f(int x) {\n        return 42;\n    }"),
            "indent not restored:\n{program}"
        );
        assert!(program.contains("public static void main"));
    }

    #[test]
    fn a_blank_line_in_the_solution_does_not_collect_trailing_indent() {
        let reference = "static int f() {\n    return 1;\n}\n";
        let harness = "class Main {\n    static int f() {\n        return 1;\n    }\n}\n";
        let mine = "static int f() {\n\n    return 2;\n}\n";
        let program = assemble(mine, reference, Some(harness));
        assert!(
            program.contains("{\n\n"),
            "blank line was padded:\n{program}"
        );
    }

    #[test]
    fn with_no_harness_the_solution_is_the_program() {
        let mine = "func twoSum() {}\n";
        assert_eq!(assemble(mine, GO, None), mine);
        assert_eq!(assemble(mine, GO, Some("   ")), mine);
    }

    #[test]
    fn a_harness_the_reference_is_missing_from_falls_back_rather_than_corrupting() {
        let mine = "func twoSum() {}\n";
        assert_eq!(
            assemble(mine, GO, Some("package main\nfunc main() {}\n")),
            mine
        );
    }

    #[test]
    fn the_starter_round_trips_back_through_the_harness() {
        // What the tab actually does on open: blank the reference, then run it.
        // The result has to still be a whole program.
        let program = assemble(&starter(GO, "go"), GO, Some(GO_HARNESS));
        assert!(program.contains("package main"));
        assert!(program.contains("func main() {"));
        assert!(program.contains("panic(\"todo\")"));
        assert!(!program.contains("seen := map"));
    }
}
