//! Building a runnable program for a pack that never shipped one.
//!
//! Eighty-five packs carry a hand-written `practice/<lang>.txt`; the other two
//! hundred carry only the solution function. Pressing Run on one of those used
//! to hand the compiler a bare function and get back
//! `expected 'package', found 'func'` — a message about the harness, shown to
//! someone who was looking at their own code.
//!
//! Where the problem's declared inputs line up with the entry function's
//! parameters, the surrounding program is derivable: read the fields in schema
//! order exactly as [`crate::problem`] serialises them, call the function,
//! print the result the way the tests spell it. Where they do not line up —
//! a `string` input that the solution takes as `[][]int`, a tree, a grid — no
//! amount of guessing is safe, and this returns `None` so the caller can say so
//! plainly instead of emitting something that compiles into a wrong answer.
//!
//! Bailing is always the safe answer here. A missing harness is a message; a
//! wrong one is a bug the user will blame on their own code.

use crate::problem::{InputField, InputType, InputValue, ProblemMeta, TestCase};

/// The shapes a generated harness can read, pass and print.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ty {
    Int,
    Ints,
    Str,
    Bool,
    /// `[][]int`, spelled by a `string` field. See [`RowStyle`] — the tree
    /// uses two encodings and the pack's own values say which.
    Rows,
    /// Anything else: a tree, a grid, a linked list.
    Other,
}

/// The entry function, as far as its first line can be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    pub name: String,
    params: Vec<Ty>,
    returns: Ty,
}

/// Read the first top-level function's signature.
///
/// The entry point is the first function in the file: every pack in the tree is
/// written that way, helpers below the function that uses them.
pub fn signature(reference: &str, lang: &str) -> Option<Signature> {
    let head = first_signature(reference)?;
    let open = head.find('(')?;
    let close = matching_paren(&head, open)?;
    let (before, params, after) = (
        &head[..open],
        &head[open + 1..close],
        head[close + 1..].trim().trim_end_matches('{').trim(),
    );

    let (name, ret_text) = match lang {
        // `func twoSum(...) []int {` — the name follows `func`, the result
        // follows the parameter list.
        "go" => (before.trim().strip_prefix("func")?.trim(), after),
        // `vector<int> twoSum(...) {` / `static int[] twoSum(...) {` — the name
        // is the last word before the parameters, the type everything before it.
        _ => {
            let before = before.trim();
            let cut = before.rfind(|c: char| c.is_whitespace() || c == '*' || c == '&')?;
            (before[cut + 1..].trim(), before[..=cut].trim())
        }
    };
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }

    Some(Signature {
        name: name.to_string(),
        params: split_params(params)
            .into_iter()
            .map(|p| param_ty(&p, lang))
            .collect(),
        returns: ty(ret_text, lang),
    })
}

fn first_signature(reference: &str) -> Option<String> {
    let mut lines = reference.lines().peekable();
    while let Some(line) = lines.next() {
        let mut head = line.to_string();
        let mut depth = super::practice::paren_delta(line);
        while depth > 0 {
            let next = lines.next()?;
            depth += super::practice::paren_delta(next);
            head.push(' ');
            head.push_str(next.trim());
        }
        if super::practice::opens_function(&head) {
            return Some(head);
        }
    }
    None
}

fn matching_paren(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.char_indices().skip(open) {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Split a parameter list on commas that are not inside brackets — `Map<K, V>`
/// and `vector<vector<int>>` both contain commas that are not separators.
fn split_params(list: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    for c in list.chars() {
        match c {
            '<' | '[' | '(' => depth += 1,
            '>' | ']' | ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// The declared type of one parameter, with its name removed.
fn param_ty(param: &str, lang: &str) -> Ty {
    let p = param.trim();
    let text = match lang {
        // `nums []int` — the type is everything after the name.
        "go" => match p.split_once(char::is_whitespace) {
            Some((_, t)) => t,
            None => return Ty::Other, // a grouped `a, b int`: not worth guessing
        },
        // `vector<int>& nums` — the type is everything before the name.
        _ => match p.rfind(|c: char| c.is_whitespace() || c == '&' || c == '*') {
            Some(i) => &p[..=i],
            None => return Ty::Other,
        },
    };
    ty(text, lang)
}

/// Words that decorate a type without changing it — `static int` is an `int`.
const MODIFIERS: [&str; 6] = ["static", "public", "private", "protected", "final", "const"];

fn ty(text: &str, lang: &str) -> Ty {
    let t: String = text
        .replace('&', " ")
        .split_whitespace()
        .filter(|w| !MODIFIERS.contains(w))
        .collect::<Vec<_>>()
        .join("");
    match (lang, t.as_str()) {
        ("go", "int") | ("cpp", "int") | ("java", "int") => Ty::Int,
        ("go", "[]int") | ("cpp", "vector<int>") | ("java", "int[]") => Ty::Ints,
        ("go", "string") | ("cpp", "string") | ("java", "String") => Ty::Str,
        ("go", "bool") | ("cpp", "bool") | ("java", "boolean") => Ty::Bool,
        ("go", "[][]int") | ("cpp", "vector<vector<int>>") | ("java", "int[][]") => Ty::Rows,
        _ => Ty::Other,
    }
}

/// Can this field be read into that parameter?
///
/// Mostly one-to-one, with the one documented widening: a `string` field is how
/// the packs spell a matrix, so it also satisfies a `[][]int` parameter.
fn reads_into(field: &InputField, param: Ty) -> bool {
    matches!(
        (field.ty, param),
        (InputType::Int, Ty::Int)
            | (InputType::IntArray, Ty::Ints)
            | (InputType::Str, Ty::Str)
            | (InputType::Str, Ty::Rows)
    )
}

/// How a `string` field spells a matrix.
///
/// The tree uses both spellings, so neither can be assumed:
/// `"1,0 2,0 3,1"` puts rows between spaces and cells between commas, while
/// `"0 30, 5 10, 15 20"` does the opposite. The comma-then-space is the tell,
/// and it is read off the pack's own values rather than guessed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowStyle {
    /// `", "` separates rows; whitespace separates cells.
    CommaRows,
    /// Whitespace separates rows; `,` separates cells.
    SpaceRows,
}

/// The spelling this field uses, if every value it has agrees on one.
fn row_style(meta: &ProblemMeta, field: &str) -> Option<RowStyle> {
    let mut style: Option<RowStyle> = None;
    let mut saw_comma = false;
    let maps = std::iter::once(&meta.default_input).chain(meta.tests.iter().map(|t| &t.input));
    for map in maps {
        let Some(InputValue::Str(s)) = map.get(field) else {
            continue;
        };
        saw_comma |= s.contains(',');
        let seen = if s.contains(", ") {
            RowStyle::CommaRows
        } else {
            RowStyle::SpaceRows
        };
        match style {
            None => style = Some(seen),
            Some(prev) if prev != seen => return None, // inconsistent: do not guess
            _ => {}
        }
    }
    // With no comma anywhere there is nothing to tell the two apart.
    saw_comma.then_some(style?)
}

/// Can the printer for `returns` even spell what the tests expect?
///
/// Some packs' expected values were written against the animation's result
/// line rather than a program's stdout, and say things a return value cannot —
/// `remove-duplicates-from-sorted-array-ii` wants `5 | 1 1 2 2 3` from a
/// function that returns one `int`. Generating for those would fail correct
/// code and blame the user, so the pack's own tests are the last gate.
fn printable(returns: Ty, tests: &[TestCase]) -> bool {
    tests.iter().all(|t| {
        let want = t.expected.trim();
        match returns {
            Ty::Int => want.parse::<i64>().is_ok(),
            Ty::Bool => want == "true" || want == "false",
            Ty::Ints => want.split_whitespace().all(|w| w.parse::<i64>().is_ok()),
            // `subsets-ii` returns [][]int but its tests expect `6` — the
            // number of subsets, from the animation's result line. A real
            // matrix always shows a comma.
            Ty::Rows => {
                (want.is_empty() || want.contains(','))
                    && want
                        .split_whitespace()
                        .all(|row| row.split(',').all(|v| v.parse::<i64>().is_ok()))
            }
            Ty::Str => !want.contains('\n'),
            Ty::Other => false,
        }
    })
}

/// A complete program around `reference`, or `None` when the pack's inputs and
/// its entry function cannot be lined up.
pub fn synthesize(lang: &str, meta: &ProblemMeta, reference: &str) -> Option<String> {
    let (fields, tests) = (&meta.inputs, &meta.tests);
    let sig = signature(reference, lang)?;

    // Every field has to map to the parameter in the same position, and the
    // result has to be something with an agreed spelling.
    if sig.params.len() != fields.len() || sig.params.contains(&Ty::Other) {
        return None;
    }
    if !fields
        .iter()
        .zip(&sig.params)
        .all(|(f, p)| reads_into(f, *p))
    {
        return None;
    }
    if sig.returns == Ty::Other || !printable(sig.returns, tests) {
        return None;
    }

    // A matrix parameter is only readable if the pack spells it consistently.
    let mut style = None;
    for (f, p) in fields.iter().zip(&sig.params) {
        if *p == Ty::Rows {
            let seen = row_style(meta, &f.name)?;
            if style.is_some_and(|s| s != seen) {
                return None; // two fields, two spellings: one reader cannot do both
            }
            style = Some(seen);
        }
    }
    let style = style.unwrap_or(RowStyle::SpaceRows);

    Some(match lang {
        "go" => go_program(&sig, fields, reference, style),
        "cpp" => cpp_program(&sig, fields, reference, style),
        "java" => java_program(&sig, fields, reference, style),
        _ => return None,
    })
}

/// Standard-library packages a solution may reach for, and the import path each
/// one needs. Only these are added, so a method call on a local named `h` is
/// never mistaken for a package.
const STD: [(&str, &str); 9] = [
    ("sort", "sort"),
    ("math", "math"),
    ("bytes", "bytes"),
    ("errors", "errors"),
    ("unicode", "unicode"),
    ("heap", "container/heap"),
    ("list", "container/list"),
    ("bits", "math/bits"),
    ("utf8", "unicode/utf8"),
];

/// The imports the generated Go program needs beyond its own five.
///
/// `strconv` and `strings` are already in, and every one of the fixed five is
/// used by the readers below, so nothing here can produce an unused import.
fn extra_imports(reference: &str) -> String {
    let mut out = String::new();
    for (name, path) in STD {
        let needle = format!("{name}.");
        if reference.contains(&needle) {
            out.push_str(&format!("    \"{path}\"\n"));
        }
    }
    out
}

/// The call arguments, in schema order — the field names double as locals.
fn args(fields: &[InputField]) -> String {
    fields
        .iter()
        .map(|f| f.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn go_program(sig: &Signature, fields: &[InputField], reference: &str, style: RowStyle) -> String {
    let reads: String = fields
        .iter()
        .zip(&sig.params)
        .map(|(f, t)| match t {
            Ty::Int => format!("    {} := readInts(reader)[0]\n", f.name),
            Ty::Ints => format!("    {} := readInts(reader)\n", f.name),
            Ty::Rows => format!("    {} := readRows(reader)\n", f.name),
            _ => format!("    {} := readLine(reader)\n", f.name),
        })
        .collect();

    let print = match sig.returns {
        Ty::Ints => "    parts := make([]string, len(res))\n    for i, v := range res {\n        parts[i] = strconv.Itoa(v)\n    }\n    fmt.Println(strings.Join(parts, \" \"))",
        Ty::Rows => "    parts := make([]string, len(res))\n    for i, row := range res {\n        cells := make([]string, len(row))\n        for j, v := range row {\n            cells[j] = strconv.Itoa(v)\n        }\n        parts[i] = strings.Join(cells, \",\")\n    }\n    fmt.Println(strings.Join(parts, \" \"))",
        _ => "    fmt.Println(res)",
    };

    format!(
        "package main\n\n\
         import (\n    \"bufio\"\n    \"fmt\"\n    \"os\"\n    \"strconv\"\n    \"strings\"\n{extra})\n\n\
         {reference}\n\
         func readLine(reader *bufio.Reader) string {{\n\
         \x20   line, _ := reader.ReadString('\\n')\n\
         \x20   return strings.TrimRight(line, \"\\r\\n\")\n\
         }}\n\n\
         func readInts(reader *bufio.Reader) []int {{\n\
         \x20   fields := strings.Fields(readLine(reader))\n\
         \x20   nums := make([]int, len(fields))\n\
         \x20   for i, f := range fields {{\n\
         \x20       nums[i], _ = strconv.Atoi(f)\n\
         \x20   }}\n\
         \x20   return nums\n\
         }}\n\n\
         {rows}\n\
         func main() {{\n\
         \x20   reader := bufio.NewReader(os.Stdin)\n\
         {reads}\
         \x20   res := {name}({args})\n\
         {print}\n\
         }}\n",
        reference = reference.trim_end(),
        name = sig.name,
        args = args(fields),
        reads = reads,
        print = print,
        extra = extra_imports(reference),
        rows = go_read_rows(style),
    )
}

/// The matrix reader, in whichever spelling this pack uses.
fn go_read_rows(style: RowStyle) -> &'static str {
    match style {
        // "0 30, 5 10" — split on commas, then on spaces.
        RowStyle::CommaRows => {
            "func readRows(reader *bufio.Reader) [][]int {
                 rows := [][]int{}
                 for _, part := range strings.Split(readLine(reader), \",\") {
                     fields := strings.Fields(part)
                     if len(fields) == 0 {
                         continue
                     }
                     row := make([]int, len(fields))
                     for i, c := range fields {
                         row[i], _ = strconv.Atoi(c)
                     }
                     rows = append(rows, row)
                 }
                 return rows
             }
"
        }
        // "1,0 2,0" — split on spaces, then on commas.
        RowStyle::SpaceRows => {
            "func readRows(reader *bufio.Reader) [][]int {
                 rows := [][]int{}
                 for _, part := range strings.Fields(readLine(reader)) {
                     cells := strings.Split(part, \",\")
                     row := make([]int, len(cells))
                     for i, c := range cells {
                         row[i], _ = strconv.Atoi(c)
                     }
                     rows = append(rows, row)
                 }
                 return rows
             }
"
        }
    }
}

fn cpp_program(sig: &Signature, fields: &[InputField], reference: &str, style: RowStyle) -> String {
    let reads: String = fields
        .iter()
        .zip(&sig.params)
        .map(|(f, t)| match t {
            Ty::Int => format!("    int {} = readInts()[0];\n", f.name),
            Ty::Ints => format!("    vector<int> {} = readInts();\n", f.name),
            Ty::Rows => format!("    vector<vector<int>> {} = readRows();\n", f.name),
            _ => format!("    string {} = readLine();\n", f.name),
        })
        .collect();

    // C++ is the one that will not print a bool as `true` on its own.
    let print = match sig.returns {
        Ty::Ints => "    for (size_t i = 0; i < res.size(); i++) {\n        if (i) cout << \" \";\n        cout << res[i];\n    }\n    cout << endl;",
        Ty::Bool => "    cout << (res ? \"true\" : \"false\") << endl;",
        Ty::Rows => "    for (size_t i = 0; i < res.size(); i++) {
        if (i) cout << \" \";
        for (size_t j = 0; j < res[i].size(); j++) {
            if (j) cout << \",\";
            cout << res[i][j];
        }
    }
    cout << endl;",
        _ => "    cout << res << endl;",
    };

    format!(
        "#include <bits/stdc++.h>\nusing namespace std;\n\n\
         {reference}\n\
         static string readLine() {{\n\
         \x20   string line;\n\
         \x20   getline(cin, line);\n\
         \x20   while (!line.empty() && (line.back() == '\\r' || line.back() == '\\n')) line.pop_back();\n\
         \x20   return line;\n\
         }}\n\n\
         static vector<int> readInts() {{\n\
         \x20   istringstream ss(readLine());\n\
         \x20   vector<int> nums;\n\
         \x20   int x;\n\
         \x20   while (ss >> x) nums.push_back(x);\n\
         \x20   return nums;\n\
         }}\n\n\
         {rows}\n\
         int main() {{\n\
         {reads}\
         \x20   auto res = {name}({args});\n\
         {print}\n\
         }}\n",
        reference = reference.trim_end(),
        name = sig.name,
        args = args(fields),
        reads = reads,
        print = print,
        rows = cpp_read_rows(style),
    )
}

/// C++'s matrix reader. Both spellings split on one separator and then the
/// other; only the order differs.
fn cpp_read_rows(style: RowStyle) -> &'static str {
    match style {
        // "0 30, 5 10" — commas end rows, so turn them into newlines and let
        // `getline` do the splitting.
        RowStyle::CommaRows => {
            "static vector<vector<int>> readRows() {\n\
             \x20   vector<vector<int>> rows;\n\
             \x20   string line = readLine();\n\
             \x20   for (char& c : line) if (c == ',') c = '\\n';\n\
             \x20   istringstream ls(line);\n\
             \x20   string part;\n\
             \x20   while (getline(ls, part)) {\n\
             \x20       istringstream cs(part);\n\
             \x20       vector<int> row;\n\
             \x20       int v;\n\
             \x20       while (cs >> v) row.push_back(v);\n\
             \x20       if (!row.empty()) rows.push_back(row);\n\
             \x20   }\n\
             \x20   return rows;\n\
             }\n"
        }
        // "1,0 2,0" — whitespace ends rows, commas separate cells.
        RowStyle::SpaceRows => {
            "static vector<vector<int>> readRows() {\n\
             \x20   vector<vector<int>> rows;\n\
             \x20   istringstream ss(readLine());\n\
             \x20   string tok;\n\
             \x20   while (ss >> tok) {\n\
             \x20       for (char& c : tok) if (c == ',') c = ' ';\n\
             \x20       istringstream cs(tok);\n\
             \x20       vector<int> row;\n\
             \x20       int v;\n\
             \x20       while (cs >> v) row.push_back(v);\n\
             \x20       rows.push_back(row);\n\
             \x20   }\n\
             \x20   return rows;\n\
             }\n"
        }
    }
}

fn java_program(
    sig: &Signature,
    fields: &[InputField],
    reference: &str,
    style: RowStyle,
) -> String {
    let reads: String = fields
        .iter()
        .zip(&sig.params)
        .map(|(f, t)| match t {
            Ty::Int => format!("        int {} = readInts()[0];\n", f.name),
            Ty::Ints => format!("        int[] {} = readInts();\n", f.name),
            Ty::Rows => format!("        int[][] {} = readRows();\n", f.name),
            _ => format!("        String {} = readLine();\n", f.name),
        })
        .collect();

    let print = match sig.returns {
        Ty::Ints => "        StringBuilder sb = new StringBuilder();\n        for (int v : res) {\n            if (sb.length() > 0) sb.append(' ');\n            sb.append(v);\n        }\n        System.out.println(sb);",
        Ty::Rows => "        StringBuilder sb = new StringBuilder();\n        for (int[] row : res) {\n            if (sb.length() > 0) sb.append(' ');\n            for (int j = 0; j < row.length; j++) {\n                if (j > 0) sb.append(',');\n                sb.append(row[j]);\n            }\n        }\n        System.out.println(sb);",
        _ => "        System.out.println(res);",
    };

    // The solution is a static method, so it is indented into `class Main`.
    let body = reference
        .trim_end()
        .lines()
        .map(|l| {
            if l.trim().is_empty() {
                String::new()
            } else {
                format!("    {l}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "import java.util.*;\nimport java.io.*;\n\n\
         class Main {{\n\n\
         {body}\n\n\
         \x20   static BufferedReader br = new BufferedReader(new InputStreamReader(System.in));\n\n\
         \x20   static String readLine() throws Exception {{\n\
         \x20       String line = br.readLine();\n\
         \x20       return line == null ? \"\" : line;\n\
         \x20   }}\n\n\
         \x20   static int[] readInts() throws Exception {{\n\
         \x20       String line = readLine().trim();\n\
         \x20       if (line.isEmpty()) return new int[0];\n\
         \x20       String[] parts = line.split(\"\\\\s+\");\n\
         \x20       int[] nums = new int[parts.length];\n\
         \x20       for (int i = 0; i < parts.length; i++) nums[i] = Integer.parseInt(parts[i]);\n\
         \x20       return nums;\n\
         \x20   }}\n\n\
         {rows}\n\
         \x20   public static void main(String[] args) throws Exception {{\n\
         {reads}\
         \x20       var res = {name}({args});\n\
         {print}\n\
         \x20   }}\n\
         }}\n",
        body = body,
        name = sig.name,
        args = args(fields),
        reads = reads,
        print = print,
        rows = java_read_rows(style),
    )
}

/// Java's matrix reader, in whichever spelling this pack uses.
fn java_read_rows(style: RowStyle) -> &'static str {
    match style {
        RowStyle::CommaRows => {
            "    static int[][] readRows() throws Exception {\n\
             \x20       String line = readLine().trim();\n\
             \x20       if (line.isEmpty()) return new int[0][];\n\
             \x20       String[] parts = line.split(\",\");\n\
             \x20       int[][] rows = new int[parts.length][];\n\
             \x20       for (int i = 0; i < parts.length; i++) {\n\
             \x20           String[] cells = parts[i].trim().split(\"\\\\s+\");\n\
             \x20           rows[i] = new int[cells.length];\n\
             \x20           for (int j = 0; j < cells.length; j++) rows[i][j] = Integer.parseInt(cells[j]);\n\
             \x20       }\n\
             \x20       return rows;\n\
             \x20   }\n"
        }
        RowStyle::SpaceRows => {
            "    static int[][] readRows() throws Exception {\n\
             \x20       String line = readLine().trim();\n\
             \x20       if (line.isEmpty()) return new int[0][];\n\
             \x20       String[] parts = line.split(\"\\\\s+\");\n\
             \x20       int[][] rows = new int[parts.length][];\n\
             \x20       for (int i = 0; i < parts.length; i++) {\n\
             \x20           String[] cells = parts[i].split(\",\");\n\
             \x20           rows[i] = new int[cells.length];\n\
             \x20           for (int j = 0; j < cells.length; j++) rows[i][j] = Integer.parseInt(cells[j]);\n\
             \x20       }\n\
             \x20       return rows;\n\
             \x20   }\n"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::{InputField, InputType};

    fn field(name: &str, ty: InputType) -> InputField {
        InputField {
            name: name.into(),
            label: name.into(),
            ty,
            min: None,
            max: None,
            min_len: None,
            max_len: None,
            charset: None,
            sorted: false,
            unique: false,
            help: None,
        }
    }

    /// A manifest carrying just the input fields — enough for the shapes
    /// these tests exercise.
    fn meta(fields: &[InputField]) -> ProblemMeta {
        ProblemMeta {
            slug: "t".into(),
            title: "T".into(),
            category: "C".into(),
            difficulty: crate::problem::Difficulty::Easy,
            tier: crate::problem::Tier::T50,
            description: String::new(),
            approach: String::new(),
            complexity: String::new(),
            leetcode: None,
            inputs: fields.to_vec(),
            default_input: Default::default(),
            tests: Vec::new(),
            hints: Vec::new(),
            related: Vec::new(),
        }
    }

    /// The same, with one string field's value set — matrix readers are chosen
    /// from the values, so the shape tests have to supply one.
    fn meta_with(fields: &[InputField], field: &str, value: &str) -> ProblemMeta {
        let mut m = meta(fields);
        m.default_input
            .insert(field.into(), InputValue::Str(value.into()));
        m
    }

    const GO: &str = "func twoSum(nums []int, target int) []int {\n    return nil\n}\n";

    #[test]
    fn a_go_signature_is_read_off_its_first_line() {
        let s = signature(GO, "go").unwrap();
        assert_eq!(s.name, "twoSum");
        assert_eq!(s.params, vec![Ty::Ints, Ty::Int]);
        assert_eq!(s.returns, Ty::Ints);
    }

    #[test]
    fn cpp_and_java_signatures_read_the_type_before_the_name() {
        let cpp = signature(
            "vector<int> twoSum(vector<int>& nums, int target) {\n    return {};\n}\n",
            "cpp",
        )
        .unwrap();
        assert_eq!(cpp.name, "twoSum");
        assert_eq!(cpp.params, vec![Ty::Ints, Ty::Int]);
        assert_eq!(cpp.returns, Ty::Ints);

        let java = signature(
            "static boolean isValid(String s) {\n    return true;\n}\n",
            "java",
        )
        .unwrap();
        assert_eq!(java.name, "isValid");
        assert_eq!(java.params, vec![Ty::Str]);
        assert_eq!(java.returns, Ty::Bool);
    }

    #[test]
    fn a_shape_it_cannot_read_becomes_other_rather_than_a_guess() {
        let s = signature(
            "func maxDepth(root *TreeNode, memo map[int]int) int {\n    return 0\n}\n",
            "go",
        )
        .unwrap();
        assert_eq!(s.params, vec![Ty::Other, Ty::Other]);
    }

    #[test]
    fn a_string_field_feeds_a_matrix_parameter() {
        // Course Schedule II: the pack declares `prereqs` a string holding
        // "1,0 2,0 3,1 3,2" and the solution takes [][]int. Rows separated by
        // spaces and values by commas is the tree's documented encoding, so
        // this is derivable rather than guessed.
        let reference =
            "func findOrder(numCourses int, prerequisites [][]int) []int {\n    return nil\n}\n";
        let fields = [
            field("numCourses", InputType::Int),
            field("prereqs", InputType::Str),
        ];
        let m = meta_with(&fields, "prereqs", "1,0 2,0 3,1 3,2");
        let p = synthesize("go", &m, reference).expect("the shape is derivable");
        assert!(p.contains("numCourses := readInts(reader)[0]"));
        assert!(p.contains("prereqs := readRows(reader)"));
        assert!(
            p.contains("strings.Split(part, \",\")"),
            "rows split on commas"
        );
        assert!(p.contains("res := findOrder(numCourses, prereqs)"));
    }

    #[test]
    fn a_matrix_result_is_printed_rows_then_cells() {
        let reference = "func combine(n int, k int) [][]int {\n    return nil\n}\n";
        let fields = [field("n", InputType::Int), field("k", InputType::Int)];
        let p = synthesize("go", &meta(&fields), reference).unwrap();
        assert!(
            p.contains("strings.Join(cells, \",\")"),
            "cells joined by comma"
        );
        assert!(
            p.contains("strings.Join(parts, \" \")"),
            "rows joined by space"
        );
    }

    #[test]
    fn a_field_count_that_does_not_match_generates_nothing() {
        let fields = [field("nums", InputType::IntArray)];
        assert!(synthesize("go", &meta(&fields), GO).is_none());
    }

    #[test]
    fn a_tree_or_grid_input_generates_nothing() {
        let reference = "func maxDepth(root *TreeNode) int {\n    return 0\n}\n";
        let fields = [field("tree", InputType::Tree)];
        assert!(synthesize("go", &meta(&fields), reference).is_none());
    }

    #[test]
    fn the_generated_go_program_is_whole() {
        let fields = [
            field("nums", InputType::IntArray),
            field("target", InputType::Int),
        ];
        let p = synthesize("go", &meta(&fields), GO).expect("two-sum's shape is readable");
        assert!(p.starts_with("package main"));
        assert!(p.contains("func main() {"));
        assert!(p.contains("nums := readInts(reader)"));
        assert!(p.contains("target := readInts(reader)[0]"));
        assert!(p.contains("res := twoSum(nums, target)"));
        assert!(
            p.contains("strings.Join(parts, \" \")"),
            "[]int prints joined"
        );
        assert!(p.contains("func twoSum(nums []int, target int) []int {"));
        // Reading in schema order is what makes it agree with `serialize_input`.
        assert!(p.find("nums :=").unwrap() < p.find("target :=").unwrap());
    }

    #[test]
    fn cpp_spells_a_bool_the_way_the_tests_do() {
        // `cout << true` is `1`; every pack's expected value says `true`.
        let reference = "bool isHappy(int n) {\n    return true;\n}\n";
        let fields = [field("n", InputType::Int)];
        let p = synthesize("cpp", &meta(&fields), reference).unwrap();
        assert!(p.contains("res ? \"true\" : \"false\""));
        assert!(p.contains("#include <bits/stdc++.h>"));
        assert!(p.contains("int main() {"));
    }

    #[test]
    fn java_wraps_the_solution_in_its_class() {
        let reference = "static int f(int n) {\n    return n;\n}\n";
        let fields = [field("n", InputType::Int)];
        let p = synthesize("java", &meta(&fields), reference).unwrap();
        assert!(p.contains("class Main {"));
        assert!(p.contains("    static int f(int n) {"), "indented in:\n{p}");
        assert!(p.contains("public static void main"));
        assert!(p.trim_end().ends_with('}'));
    }

    #[test]
    fn a_string_in_and_a_string_out_needs_no_conversion() {
        let reference = "func decode(s string) string {\n    return s\n}\n";
        let fields = [field("s", InputType::Str)];
        let p = synthesize("go", &meta(&fields), reference).unwrap();
        assert!(p.contains("s := readLine(reader)"));
        assert!(p.contains("fmt.Println(res)"));
    }

    #[test]
    fn an_unknown_language_generates_nothing() {
        let fields = [
            field("nums", InputType::IntArray),
            field("target", InputType::Int),
        ];
        assert!(synthesize("python", &meta(&fields), GO).is_none());
    }
}
