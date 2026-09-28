// The Rust `str` methods that parse.rs, diff.rs and assistant.rs are written
// against, with Rust's semantics rather than the nearest JavaScript builtin.
//
// The differences are small and each one is visible: `'…'.split('\n')` yields
// a trailing empty string for text that ends in a newline (a phantom blank line
// at the bottom of every diff) and keeps the `\r` of a CRLF ending (every line
// of a Windows paste flagged as changed); `replace(/^x+/)` is the regex-shaped
// cousin of `trim_start_matches` that is easy to get subtly wrong. So the
// splitting and trimming rules live here, once, with their own tests.

/**
 * `str::lines()`: split at `\n` or `\r\n`. The final line ending is optional,
 * so `"a\n"` is one line and `""` is none; a lone `\r` not followed by `\n` is
 * kept as text.
 */
export function lines(text: string): string[] {
  if (text === '') return []
  const parts = text.split('\n')
  const out: string[] = []
  for (let i = 0; i < parts.length - 1; i++) {
    const part = parts[i]
    out.push(part.endsWith('\r') ? part.slice(0, -1) : part)
  }
  const tail = parts[parts.length - 1]
  if (tail !== '') out.push(tail)
  return out
}

/** `str::trim_start_matches(pat)`: strip every leading repetition of `pat`. */
export function trimStartMatches(text: string, pat: string): string {
  if (pat === '') return text
  let start = 0
  while (text.startsWith(pat, start)) start += pat.length
  return text.slice(start)
}

/** `str::trim_end_matches(pat)`: strip every trailing repetition of `pat`. */
export function trimEndMatches(text: string, pat: string): string {
  if (pat === '') return text
  let end = text.length
  while (end >= pat.length && text.startsWith(pat, end - pat.length)) end -= pat.length
  return text.slice(0, end)
}
