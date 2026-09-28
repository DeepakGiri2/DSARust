// A line diff, for showing what the AI changed — a port of
// `crates/dsa-core/src/diff.rs`.
//
// The point of the Fix mode is not to hand over corrected code — it is to show
// you the line you got wrong. A replaced buffer cannot do that: you get working
// code and no idea which part was the mistake. So the proposal is shown against
// what you wrote, removals in red and additions in green, and each change group
// can be taken or left on its own.
//
// Classic LCS, which is O(n·m) in time and memory. Solutions are tens of lines
// and the cap below keeps a pathological input from freezing the tab; beyond it
// the answer degrades to "all of this became all of that", which is both true
// and cheap.

import { lines as splitLines } from './str'

/** Above this many cells the DP table is not worth building. */
const MAX_CELLS = 4_000_000

/**
 * Untouched lines that end a change group.
 *
 * One, so a group is exactly one unbroken run of red and green — which is what
 * the eye reads as "a change" and therefore what a single tick should govern.
 * Merging edits that are two or three lines apart is right for a patch file,
 * where the unit is a hunk to apply; here the unit is a mistake to notice, and
 * two mistakes should be two decisions.
 */
const GAP = 1

export type Change = 'same' | 'removed' | 'added'

export interface DiffLine {
  readonly change: Change
  readonly text: string
  /** 1-based line number in what the user wrote (`same` and `removed`). */
  readonly oldNo: number | null
  /** 1-based line number in the proposal (`same` and `added`). */
  readonly newNo: number | null
  /** Which change group this belongs to; `null` for untouched context. */
  readonly hunk: number | null
}

/** A row in the rendered diff: a line, or a run of context that was folded away. */
export type Row = { kind: 'line'; index: number } | { kind: 'folded'; count: number }

export class Diff {
  readonly lines: readonly DiffLine[]
  /** Number of change groups — what the per-change ticks count. */
  readonly hunks: number
  readonly removed: number
  readonly added: number

  constructor(lines: readonly DiffLine[], hunks: number) {
    this.lines = lines
    this.hunks = hunks
    this.removed = lines.filter((l) => l.change === 'removed').length
    this.added = lines.filter((l) => l.change === 'added').length
  }

  isEmpty(): boolean {
    return this.hunks === 0
  }

  /**
   * The text you get by taking the proposal for the accepted groups and
   * keeping your own code everywhere else.
   *
   * A group not named in `accepted` counts as accepted, so the common call —
   * `apply(Array(hunks).fill(true))` — and a short array both mean "take it all".
   */
  apply(accepted: readonly boolean[]): string {
    const taken = (hunk: number | null) => hunk === null || (accepted[hunk] ?? true)
    let out = ''
    for (const line of this.lines) {
      const keep =
        line.change === 'same' ? true : line.change === 'added' ? taken(line.hunk) : !taken(line.hunk)
      if (keep) out += line.text + '\n'
    }
    return out
  }

  /**
   * Which rows to draw: every changed line, `context` untouched lines around
   * each, and a fold marker standing in for the rest.
   *
   * Without this a two-line fix inside a forty-line function is two red rows
   * lost in a wall of grey, and the reader has to hunt for the thing the
   * screen exists to point at.
   */
  rows(context: number): Row[] {
    const n = this.lines.length
    const near = this.lines.map((_, i) => {
      const lo = Math.max(0, i - context)
      const hi = Math.min(i + context, n - 1)
      for (let j = lo; j <= hi; j++) if (this.lines[j].change !== 'same') return true
      return false
    })

    const rows: Row[] = []
    let folded = 0
    near.forEach((show, index) => {
      if (!show) {
        folded += 1
        return
      }
      if (folded > 0) {
        rows.push({ kind: 'folded', count: folded })
        folded = 0
      }
      rows.push({ kind: 'line', index })
    })
    if (folded > 0) rows.push({ kind: 'folded', count: folded })
    return rows
  }
}

/** Diff `old` against `next`, line by line. */
export function diff(old: string, next: string): Diff {
  const a = splitLines(old)
  const b = splitLines(next)
  const aligned = a.length * b.length > MAX_CELLS ? wholesale(a, b) : walk(a, b, lcsTable(a, b))

  // Group changes: a run of untouched lines shorter than `GAP` is a gap inside
  // one edit, not the space between two.
  let hunks = 0
  let since = Infinity
  const lines = aligned.map((line): DiffLine => {
    if (line.change === 'same') {
      since += 1
      return { ...line, hunk: null }
    }
    if (since >= GAP) hunks += 1
    since = 0
    return { ...line, hunk: hunks - 1 }
  })
  return new Diff(lines, hunks)
}

type Aligned = Omit<DiffLine, 'hunk'>

/**
 * `t[i * (b.length + 1) + j]` = length of the longest common subsequence of
 * `a[i..]` and `b[j..]`. Flat and typed rather than an array of arrays: the
 * table is the whole cost of the diff, and this is the cheap shape for it.
 */
function lcsTable(a: readonly string[], b: readonly string[]): Uint32Array {
  const w = b.length + 1
  const t = new Uint32Array((a.length + 1) * w)
  for (let i = a.length - 1; i >= 0; i--) {
    for (let j = b.length - 1; j >= 0; j--) {
      t[i * w + j] =
        a[i] === b[j] ? t[(i + 1) * w + j + 1] + 1 : Math.max(t[(i + 1) * w + j], t[i * w + j + 1])
    }
  }
  return t
}

function walk(a: readonly string[], b: readonly string[], t: Uint32Array): Aligned[] {
  const w = b.length + 1
  const out: Aligned[] = []
  let i = 0
  let j = 0
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      out.push({ change: 'same', text: a[i], oldNo: i + 1, newNo: j + 1 })
      i += 1
      j += 1
    } else if (t[(i + 1) * w + j] >= t[i * w + j + 1]) {
      out.push({ change: 'removed', text: a[i], oldNo: i + 1, newNo: null })
      i += 1
    } else {
      out.push({ change: 'added', text: b[j], oldNo: null, newNo: j + 1 })
      j += 1
    }
  }
  // Whatever is left is a pure deletion or a pure insertion. Removals first,
  // so a replaced tail reads "this became that" rather than the reverse.
  for (; i < a.length; i++) out.push({ change: 'removed', text: a[i], oldNo: i + 1, newNo: null })
  for (; j < b.length; j++) out.push({ change: 'added', text: b[j], oldNo: null, newNo: j + 1 })
  return out
}

/** The fallback for inputs too large to align: everything out, everything in. */
function wholesale(a: readonly string[], b: readonly string[]): Aligned[] {
  return [
    ...a.map((text, i): Aligned => ({ change: 'removed', text, oldNo: i + 1, newNo: null })),
    ...b.map((text, j): Aligned => ({ change: 'added', text, oldNo: null, newNo: j + 1 })),
  ]
}
