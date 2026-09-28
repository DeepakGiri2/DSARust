// Renderers for the "row of things" views — a port of
// `crates/dsa-viz/src/linear.rs`: arrays and bar charts, hash maps,
// stacks/queues/deques/heaps, bit rows and free text.

import {
  cellEq,
  type ArrayView,
  type BitsView,
  type KvView,
  type Pointer,
  type StackKind,
  type StackView,
  type TextView,
} from '@/trace/types'
import { easeBack, easeOut, flash, isNew, lerp, lerpRect, mix, withAlpha } from './anim'
import {
  caption,
  cellAsF64,
  cr,
  fitCell,
  glow,
  laneOf,
  pointerLanes,
  pointerMarker,
  shortCellText,
  stateFill,
  valueCell,
  type CellState,
} from './draw'
import {
  add,
  pos2,
  rectCenter,
  rectCenterTop,
  rectFromMinMax,
  rectFromMinSize,
  rectSize,
  rectWidth,
  splat,
  sub,
  translate,
  vec2,
  type Pos2,
  type Rect,
} from './geom'
import { asUsize, clamp, clamp01, fmax } from './num'
import { CENTER_CENTER, LEFT_CENTER, LEFT_TOP, RIGHT_CENTER, monospace, proportional, stroke, type Painter } from './painter'
import type { VizTheme } from './theme'

const EMPTY_ARRAY: ArrayView = { type: 'array', label: '', data: [] }
const NONE: readonly number[] = []

function emptyCaption(p: Painter, rect: Rect, text: string, theme: VizTheme): void {
  caption(p, add(rect.min, vec2(0, theme.labelHeight)), text, theme.dim, 13)
}

// ── Array / bars ────────────────────────────────────────────────────────────

export function arrayHeight(v: ArrayView, theme: VizTheme): number {
  const lanes = pointerLanes(v.pointers ?? [])
  const body = v.bars ? 150 : theme.cellSize
  return theme.labelHeight + body + 18 + lanes * 15 + 6
}

/**
 * Rejection outranks everything: a cell that is both "in the window" and
 * "rejected" must read as rejected, or a binary search looks like it keeps
 * live candidates.
 */
export function arrayCellState(i: number, v: ArrayView): CellState {
  if (v.bad?.includes(i)) return 'bad'
  if (v.hl?.includes(i)) return 'good'
  if (v.done?.includes(i)) return 'done'
  const w = v.window
  if (w && i >= w[0] && i <= w[1]) return 'window'
  return 'plain'
}

export function drawArray(p: Painter, rect: Rect, v: ArrayView, prev: ArrayView | null, t: number, theme: VizTheme): void {
  const was = prev ?? EMPTY_ARRAY
  caption(p, rect.min, v.label, theme.muted, 12)

  const n = v.data.length
  if (n === 0) {
    emptyCaption(p, rect, '(empty)', theme)
    return
  }

  const size = fitCell(n, rectWidth(rect), theme)
  const step = size + theme.gap
  const pointers = v.pointers ?? []
  const top = rect.min.y + theme.labelHeight + pointerLanes(pointers) * 15 + 4
  const cellAt = (i: number): Rect => rectFromMinSize(pos2(rect.min.x + i * step, top), splat(size))

  // The window band slides and stretches rather than jumping, which is what
  // makes a shrinking binary-search range readable.
  const band = (view: ArrayView): Rect | null => {
    if (!view.window) return null
    const lo = clamp(view.window[0], 0, n - 1)
    const hi = clamp(view.window[1], lo, n - 1)
    return rectFromMinMax(sub(cellAt(lo).min, splat(4)), add(cellAt(hi).max, splat(4)))
  }
  const bandNow = band(v)
  if (bandNow) {
    const r = lerpRect(band(was) ?? bandNow, bandNow, t)
    p.rectFilled(r, cr(theme.rounding + 4), withAlpha(theme.window, 0.16))
    p.rectStroke(r, cr(theme.rounding + 4), stroke(1, withAlpha(theme.window, 0.55)), 'inside')
  }

  let barMax = 1
  for (const c of v.data.concat(was.data)) {
    const x = cellAsF64(c)
    if (x !== null) barMax = fmax(barMax, x)
  }

  const hl = v.hl ?? NONE
  const bad = v.bad ?? NONE
  for (let i = 0; i < n; i++) {
    const r = cellAt(i)
    const now = arrayCellState(i, v)
    const before = i < was.data.length ? arrayCellState(i, was) : now
    const changed = i < was.data.length && !cellEq(was.data[i], v.data[i])

    if (v.bars) drawBar(p, rect, r, i, v, was, barMax, now, before, t, theme)
    else valueCell(p, r, shortCellText(v.data[i]), now, before, changed, t, theme)

    // A highlight that appeared this step gets a decaying glow.
    if (isNew(i, hl, was.hl ?? NONE)) glow(p, r, theme.good, flash(t), theme)
    else if (isNew(i, bad, was.bad ?? NONE)) glow(p, r, theme.bad, flash(t), theme)

    if (size > 22) {
      p.text(pos2(rectCenter(r).x, r.max.y + 9), CENTER_CENTER, String(i), monospace(9), theme.dim)
    }
  }

  drawPointers(p, pointers, was.pointers ?? [], t, theme, (i) => rectCenterTop(cellAt(i)))
}

function drawBar(
  p: Painter,
  outer: Rect,
  slot: Rect,
  i: number,
  v: ArrayView,
  was: ArrayView,
  max: number,
  now: CellState,
  before: CellState,
  t: number,
  theme: VizTheme,
): void {
  const areaTop = slot.min.y
  const areaBottom = outer.max.y - 22
  const height = Math.max(areaBottom - areaTop, 20)

  const val = (view: ArrayView): number => {
    const c = view.data[i]
    return c === undefined ? 0 : (cellAsF64(c) ?? 0)
  }
  const hNow = clamp(val(v) / max, 0, 1) * height
  const hWas = was.data.length > i ? clamp(val(was) / max, 0, 1) * height : hNow
  const h = lerp(hWas, hNow, easeOut(t))

  const bar = rectFromMinMax(pos2(slot.min.x, areaBottom - h), pos2(slot.max.x, areaBottom))
  const fill = mix(stateFill(before, theme), stateFill(now, theme), t)
  p.rectFilled(bar, cr(theme.rounding), fill)
  p.rectStroke(bar, cr(theme.rounding), stroke(1, mix(theme.cellStroke, fill, 0.5)), 'inside')
  p.text(
    pos2(rectCenter(bar).x, bar.min.y - 8),
    CENTER_CENTER,
    shortCellText(v.data[i]),
    monospace(10),
    now === 'plain' ? theme.muted : theme.text,
  )
}

function drawPointers(
  p: Painter,
  now: readonly Pointer[],
  was: readonly Pointer[],
  t: number,
  theme: VizTheme,
  anchor: (i: number) => Pos2,
): void {
  now.forEach(([name, idx], laneI) => {
    const from = was.find(([n]) => n === name)?.[1] ?? idx
    // Interpolating the *index* (not the pixel) keeps the marker locked to
    // the cell grid while it travels. The float → usize casts saturate at 0
    // exactly as the desktop's do, so an index of -1 draws over cell 0.
    const pos = lerp(from, idx, easeOut(t))
    const base = anchor(asUsize(Math.max(pos, 0)))
    const frac = pos - Math.floor(pos)
    const next = anchor(asUsize(Math.floor(pos)) + 1)
    const at = pos2(lerp(base.x, next.x, frac), base.y)
    pointerMarker(p, at, name, theme.pointerColor(name), laneOf(now, laneI), theme)
  })
}

// ── Hash map / key-value ────────────────────────────────────────────────────

const ROW_H = 26
const ROW_W = 132

function kvPerRow(width: number): number {
  return Math.max(asUsize(Math.floor(width / ROW_W)), 1)
}

export function kvHeight(v: KvView, width: number, theme: VizTheme): number {
  const rows = Math.max(Math.ceil(v.entries.length / kvPerRow(width)), 1)
  return theme.labelHeight + rows * (ROW_H + 4) + 6
}

export function drawKv(p: Painter, rect: Rect, v: KvView, was: KvView | null, t: number, theme: VizTheme): void {
  caption(p, rect.min, v.label, theme.muted, 12)
  const old = was?.entries ?? []

  if (v.entries.length === 0) {
    emptyCaption(p, rect, '(empty)', theme)
    return
  }

  const perRow = kvPerRow(rectWidth(rect))
  const hlKeys = v.hlKeys ?? []
  const badKeys = v.badKeys ?? []
  v.entries.forEach(([key, val], i) => {
    const col = i % perRow
    const row = Math.floor(i / perRow)
    const slot = rectFromMinSize(
      pos2(rect.min.x + col * ROW_W, rect.min.y + theme.labelHeight + row * (ROW_H + 4)),
      vec2(ROW_W - 8, ROW_H),
    )

    const isFresh = !old.some(([k]) => k === key)
    const hot = hlKeys.includes(key)
    const isBad = badKeys.includes(key)

    // A brand new entry drops in, bouncing slightly as it lands.
    const appear = isFresh ? easeBack(t) : 1
    const at = translate(slot, vec2(0, (1 - appear) * 12))
    const alpha = isFresh ? clamp01(t) : 1

    const fill = isBad ? mix(theme.cell, theme.bad, 0.6) : hot ? mix(theme.cell, theme.good, 0.55) : theme.cell
    p.rectFilled(at, cr(theme.rounding), withAlpha(fill, alpha))
    p.rectStroke(at, cr(theme.rounding), stroke(1, withAlpha(theme.cellStroke, alpha)), 'inside')
    if (hot || isBad) glow(p, at, isBad ? theme.bad : theme.good, flash(t), theme)

    const fg = hot || isBad ? theme.on(fill) : theme.text
    const mid = rectCenter(at)
    p.text(pos2(at.min.x + 8, mid.y), LEFT_CENTER, key, monospace(12), withAlpha(fg, alpha))
    p.text(mid, CENTER_CENTER, '→', monospace(11), withAlpha(theme.dim, alpha))
    // An updated value is redrawn in place. (The desktop scales a box around
    // it but draws the text at that box's centre in a fixed 12pt font, so an
    // update shows no motion there either.)
    p.text(pos2(at.max.x - 26, mid.y), CENTER_CENTER, shortCellText(val), monospace(12), withAlpha(fg, alpha))
  })
}

// ── Stack / queue / deque / heap ────────────────────────────────────────────

/** Stacks and heaps grow upward; queues and deques run left to right. */
function isVertical(kind: StackKind): boolean {
  return kind === 'stack' || kind === 'heap'
}

const END_LABEL: Readonly<Record<StackKind, string>> = { stack: 'top', queue: 'front', deque: 'front', heap: 'min' }

export function stackHeight(v: StackView, theme: VizTheme): number {
  if (isVertical(v.kind)) return theme.labelHeight + Math.max(v.items.length, 1) * 30 + 24
  return theme.labelHeight + theme.cellSize + 26
}

export function drawStack(p: Painter, rect: Rect, v: StackView, was: StackView | null, t: number, theme: VizTheme): void {
  caption(p, rect.min, v.label, theme.muted, 12)
  const oldLen = was ? was.items.length : v.items.length

  if (v.items.length === 0 && v.popped === undefined) {
    emptyCaption(p, rect, '(empty)', theme)
    return
  }

  const vertical = isVertical(v.kind)
  const n = v.items.length
  const slot = (i: number): Rect => {
    if (vertical) {
      // Grows upward: the last item sits on top, like a real stack.
      const y = rect.min.y + theme.labelHeight + (Math.max(n - 1, 0) - i) * 30
      return rectFromMinSize(pos2(rect.min.x, y), vec2(96, 26))
    }
    const size = fitCell(n, rectWidth(rect), theme)
    return rectFromMinSize(pos2(rect.min.x + i * (size + theme.gap), rect.min.y + theme.labelHeight), splat(size))
  }

  for (let i = 0; i < n; i++) {
    const isTop = i + 1 === n
    const arriving = v.pushed === true && isTop && n > oldLen
    // A pushed item slides in from beyond the open end.
    const off = (1 - easeBack(t)) * 34
    const r = arriving ? translate(slot(i), vertical ? vec2(0, -off) : vec2(off, 0)) : slot(i)

    const state: CellState = v.bad && isTop ? 'bad' : isTop ? 'good' : 'plain'
    valueCell(p, r, shortCellText(v.items[i]), state, state, false, 1, theme)
    if (arriving) glow(p, r, theme.good, flash(t), theme)
    if (isTop) {
      const at = vertical ? pos2(r.max.x + 8, rectCenter(r).y) : pos2(rectCenter(r).x, r.max.y + 10)
      p.text(at, LEFT_CENTER, END_LABEL[v.kind], proportional(10), theme.muted)
    }
  }

  // The popped value keeps drifting away for the length of the transition,
  // so a pop is visible instead of an item simply vanishing. Its slot is the
  // one past the open end. For a stack or heap that still holds items, the
  // desktop computes that slot's row as `(n - 1) - n` in unsigned arithmetic,
  // which wraps and puts the ghost far off-canvas — so there a pop shows only
  // as the shorter column, and so it does here.
  if (v.popped !== undefined && (!vertical || n === 0)) {
    const base = slot(n)
    const off = easeOut(t) * 26
    const r = vertical
      ? rectFromMinSize(pos2(base.min.x, base.min.y - off), vec2(96, 26))
      : rectFromMinSize(pos2(base.min.x + off, base.min.y), rectSize(base))
    const fade = 1 - t
    p.rectFilled(r, cr(theme.rounding), withAlpha(mix(theme.cell, theme.bad, 0.4), fade))
    p.text(rectCenter(r), CENTER_CENTER, shortCellText(v.popped), monospace(12), withAlpha(theme.text, fade))
  }
}

// ── Bits ────────────────────────────────────────────────────────────────────

export function bitsHeight(v: BitsView, theme: VizTheme): number {
  return theme.labelHeight + Math.max(v.rows.length, 1) * 30 + 6
}

/** A row value as the i64 it is in Rust, so shifts see all 64 bits and the sign. */
function asI64(v: number): bigint {
  return Number.isFinite(v) ? BigInt.asIntN(64, BigInt(Math.trunc(v))) : 0n
}

function bitAt(v: bigint, index: number): boolean {
  return ((v >> BigInt(index)) & 1n) === 1n
}

export function drawBits(p: Painter, rect: Rect, v: BitsView, was: BitsView | null, t: number, theme: VizTheme): void {
  caption(p, rect.min, v.label, theme.muted, 12)

  v.rows.forEach((row, ri) => {
    const y = rect.min.y + theme.labelHeight + ri * 30
    p.text(pos2(rect.min.x, y + 11), LEFT_CENTER, row.label, monospace(11), theme.muted)
    const left = rect.min.x + 86
    const width = clamp(Math.trunc(row.width), 1, 64)
    const bw = clamp((rectWidth(rect) - 130) / width, 7, 20)

    const value = asI64(row.value)
    const priorRow = was?.rows[ri]
    const prior = priorRow ? asI64(priorRow.value) : null
    const marks = row.hl ?? NONE
    for (let b = 0; b < width; b++) {
      // Most significant bit on the left, as written.
      const bitIndex = width - 1 - b
      const on = bitAt(value, bitIndex)
      const flipped = prior !== null && bitAt(prior, bitIndex) !== on
      const marked = marks.includes(bitIndex)

      const r = rectFromMinSize(pos2(left + b * (bw + 2), y), vec2(bw, 22))
      const base = on ? mix(theme.cell, theme.accent, 0.75) : theme.cell
      const fill = flipped || marked ? mix(base, theme.cur, 0.55 * flash(t) + (marked ? 0.25 : 0)) : base
      p.rectFilled(r, cr(3), fill)
      if (bw >= 10) {
        p.text(rectCenter(r), CENTER_CENTER, on ? '1' : '0', monospace(clamp(bw * 0.7, 8, 12)), on ? theme.on(fill) : theme.dim)
      }
      if (flipped) glow(p, r, theme.cur, flash(t), theme)
    }
    p.text(pos2(rect.max.x - 4, y + 11), RIGHT_CENTER, `= ${value}`, monospace(11), theme.text)
  })
}

// ── Text ────────────────────────────────────────────────────────────────────

export function textHeight(v: TextView, theme: VizTheme): number {
  return theme.labelHeight + Math.max(v.lines.length, 1) * 18 + 6
}

/** Free text has no motion, so it takes no previous view and no `t`. */
export function drawText(p: Painter, rect: Rect, v: TextView, theme: VizTheme): void {
  caption(p, rect.min, v.label, theme.muted, 12)
  const hl = v.hl ?? NONE
  v.lines.forEach((line, i) => {
    const y = rect.min.y + theme.labelHeight + i * 18
    const hot = hl.includes(i)
    if (hot) {
      p.rectFilled(rectFromMinSize(pos2(rect.min.x - 4, y - 1), vec2(rectWidth(rect), 18)), cr(4), withAlpha(theme.good, 0.14))
    }
    p.text(pos2(rect.min.x, y), LEFT_TOP, line, monospace(12), hot ? theme.text : theme.muted)
  })
}
