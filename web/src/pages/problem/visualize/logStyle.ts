// Console lines carry a kind, and the kinds mean genuinely different things:
// call and return are scaffolding, a result is the payoff a trace was built
// to reach. Ported from `log_color` / `log_glyph` / `log_text` in
// crates/dsa-app/src/style.rs.

import type { LogEntry, LogKind } from '@/trace/types'

export function logColor(kind: LogKind): string {
  switch (kind) {
    case 'call':
      return 'var(--accent)'
    case 'return':
      return 'var(--accent-2)'
    case 'result':
      return 'var(--green)'
    case 'log':
      return 'var(--text)'
  }
}

/** The gutter glyph, standing in for the `-> ` / `<- ` prefixes the recorder writes. */
export function logGlyph(kind: LogKind): string {
  switch (kind) {
    case 'call':
      return '→'
    case 'return':
      return '←'
    case 'result':
      return '✔'
    case 'log':
      return '·'
  }
}

/** The text without the recorder's arrow — the glyph draws it now, and both read as a stutter. */
export function logText(entry: LogEntry): string {
  if (entry.text.startsWith('-> ') || entry.text.startsWith('<- ')) return entry.text.slice(3)
  return entry.text
}
