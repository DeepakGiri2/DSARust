// The variables panel's model: which values changed since the previous step,
// watched ones first, and the child lines a composite expands into.

import { cellEq, cellText, varSummary, type Cell, type Frame, type VarVal } from '@/trace/types'

const cellsEq = (a: readonly Cell[], b: readonly Cell[]) =>
  a.length === b.length && a.every((x, i) => cellEq(x, b[i]))

/**
 * `VarVal`'s derived `PartialEq`: same kind, same payload *and* the same
 * highlights — a map whose flashed key moved on has changed, even when its
 * entries did not. An absent `hl` is the empty list, as serde reads it.
 */
export function varEq(a: VarVal, b: VarVal): boolean {
  switch (a.kind) {
    case 'num':
      return b.kind === 'num' && cellEq(a.v, b.v)
    case 'str':
    case 'ptr':
      return b.kind === a.kind && a.v === b.v
    case 'bool':
      return b.kind === 'bool' && a.v === b.v
    case 'null':
      return b.kind === 'null'
    case 'arr': {
      if (b.kind !== 'arr' || !cellsEq(a.v, b.v)) return false
      const ha = a.hl ?? []
      const hb = b.hl ?? []
      return ha.length === hb.length && ha.every((x, i) => x === hb[i])
    }
    case 'map': {
      if (b.kind !== 'map' || a.v.length !== b.v.length) return false
      if (!a.v.every(([k, v], i) => k === b.v[i][0] && cellEq(v, b.v[i][1]))) return false
      const ha = a.hl ?? []
      const hb = b.hl ?? []
      return ha.length === hb.length && ha.every((x, i) => x === hb[i])
    }
    case 'set':
      return b.kind === 'set' && cellsEq(a.v, b.v) && cellsEq(a.hl ?? [], b.hl ?? [])
  }
}

export interface VarRow {
  name: string
  value: VarVal
  summary: string
  changed: boolean
  watched: boolean
}

/**
 * Rows for one frame. `prev` is the same frame index one *step* back (not the
 * step being animated from), exactly as the desktop compares — and with no
 * previous frame everything counts as changed, which is why the first step
 * of a trace shows every value lit.
 */
export function varRows(
  frame: Frame | undefined,
  prev: Frame | undefined,
  isWatched: (name: string) => boolean,
): VarRow[] {
  if (!frame) return []
  const rows = Object.entries(frame.vars).map(([name, value]): VarRow => {
    const old = prev?.vars[name]
    return {
      name,
      value,
      summary: varSummary(value),
      changed: old === undefined || !varEq(old, value),
      watched: isWatched(name),
    }
  })
  // Watched first; otherwise the frame's own order, which never reshuffles
  // between steps so the eye can keep following a value.
  return [...rows.filter((r) => r.watched), ...rows.filter((r) => !r.watched)]
}

export interface ChildLine {
  /** The trace flashed this entry on this step. */
  hot: boolean
  text: string
}

/** How many children a composite shows before summarising the rest. */
export const CHILD_LIMIT = 14

/** The lines an expanded map, set or array shows. */
export function childLines(v: VarVal): ChildLine[] {
  switch (v.kind) {
    case 'map':
      return v.v.map(([k, val]) => ({ hot: (v.hl ?? []).includes(k), text: `${k} → ${cellText(val)}` }))
    case 'set':
      return v.v.map((x) => ({ hot: (v.hl ?? []).some((h) => cellEq(h, x)), text: cellText(x) }))
    case 'arr':
      return v.v.map((x, i) => ({ hot: (v.hl ?? []).includes(i), text: `[${i}] ${cellText(x)}` }))
    default:
      return []
  }
}
