// The "full program" view: the user's solution spliced into the pack's
// runnable harness. A line-for-line port of `assemble` and `locate` from
// crates/dsa-core/src/practice.rs, so the program shown here is byte for byte
// the one the server builds for a `solution`-mode run.

/** Rust's `str::lines`: split on `\n`, drop one trailing `\r`, no phantom last line. */
export function rustLines(s: string): string[] {
  if (s === '') return []
  const parts = s.split('\n')
  if (parts[parts.length - 1] === '') parts.pop()
  return parts.map((l) => (l.endsWith('\r') ? l.slice(0, -1) : l))
}

const indentOf = (line: string) => line.slice(0, line.length - line.trimStart().length)

/**
 * Where `reference` sits inside `harness`: the line range it occupies and the
 * extra indent the harness wraps it in (four spaces, for a Java pack whose
 * solution lives inside `class Main`).
 *
 * Lines are compared trimmed, so a harness that re-indents the solution — or
 * differs from it by trailing whitespace — still matches.
 */
export function locate(harness: string, reference: string): { start: number; end: number; indent: string } | null {
  const hay = rustLines(harness)
  const needle = rustLines(reference)
  while (needle.length > 0 && needle[0].trim() === '') needle.shift()
  while (needle.length > 0 && needle[needle.length - 1].trim() === '') needle.pop()
  if (needle.length === 0 || needle.length > hay.length) return null

  candidate: for (let start = 0; start <= hay.length - needle.length; start++) {
    for (let k = 0; k < needle.length; k++) {
      if (hay[start + k].trim() !== needle[k].trim()) continue candidate
    }
    const outer = indentOf(hay[start]).length
    const inner = indentOf(needle[0]).length
    if (outer < inner) continue
    return { start, end: start + needle.length, indent: hay[start].slice(0, outer - inner) }
  }
  return null
}

/**
 * The harness with the reference solution swapped out for `solution`.
 *
 * With no harness, or one the reference cannot be found in, the user's text
 * *is* the program — the same fallback the desktop and the server use.
 */
export function assemble(solution: string, reference: string, harness: string | null): string {
  if (harness === null || harness.trim() === '') return solution
  const found = locate(harness, reference)
  if (!found) return solution

  const lines = rustLines(harness)
  let out = ''
  for (const line of lines.slice(0, found.start)) out += `${line}\n`
  for (const line of rustLines(solution)) {
    // A blank line stays blank rather than collecting the harness's indent.
    if (line.trim() !== '') out += found.indent + line
    out += '\n'
  }
  for (const line of lines.slice(found.end)) out += `${line}\n`
  return out
}
