// Shared drawing primitives — a port of `crates/dsa-viz/src/draw.rs` — so a
// "cell" looks and behaves identically whether it is an array slot, a grid
// square or a stack entry.

import { cellText, type Cell } from '@/trace/types'
import { inflate, mix, scaleRect, withAlpha } from './anim'
import type { Color } from './color'
import { add, length, pos2, rectCenter, rectFromCenterSize, rectHeight, scale, sub, vec2, type Pos2, type Rect } from './geom'
import { clamp } from './num'
import { CENTER_CENTER, LEFT_TOP, monospace, proportional, stroke, type Painter } from './painter'
import type { VizTheme } from './theme'

/** egui `CornerRadius::same(v as u8)`: corner radii are whole points, truncated. */
export function cr(v: number): number {
  return Number.isNaN(v) ? 0 : Math.trunc(clamp(v, 0, 255))
}

/**
 * What a cell is currently *meaning*. Renderers compute this for the previous
 * and the current step and cross-fade between the two.
 *
 * - `done` — finished / no longer a candidate
 * - `good` — matched, chosen, part of the answer
 * - `bad` — rejected, mismatched
 * - `cur` — under the cursor right now
 * - `window` — inside the live window
 */
export type CellState = 'plain' | 'done' | 'good' | 'bad' | 'cur' | 'window'

export function stateFill(state: CellState, theme: VizTheme): Color {
  switch (state) {
    case 'plain':
      return theme.cell
    case 'done':
      return mix(theme.cell, theme.bg, 0.55)
    case 'good':
      return mix(theme.cell, theme.good, 0.75)
    case 'bad':
      return mix(theme.cell, theme.bad, 0.7)
    case 'cur':
      return mix(theme.cell, theme.cur, 0.55)
    case 'window':
      return mix(theme.cell, theme.window, 0.35)
  }
}

export function stateText(state: CellState, theme: VizTheme): Color {
  switch (state) {
    case 'done':
      return theme.dim
    case 'good':
    case 'bad':
    case 'cur':
      return theme.on(stateFill(state, theme))
    case 'plain':
    case 'window':
      return theme.text
  }
}

/**
 * Draw one value cell, cross-fading from `was` to `now` and popping briefly
 * when the value itself changed.
 */
export function valueCell(
  p: Painter,
  rect: Rect,
  text: string,
  now: CellState,
  was: CellState,
  changed: boolean,
  t: number,
  theme: VizTheme,
): void {
  const fill = mix(stateFill(was, theme), stateFill(now, theme), t)
  // A changed value grows past its final size and settles back, which reads
  // as "look here" without any colour change at all.
  const r = changed ? scaleRect(rect, 1 + 0.18 * (1 - t)) : rect

  p.rectFilled(r, cr(theme.rounding), fill)
  const strokeColor = now === 'plain' ? theme.cellStroke : mix(theme.cellStroke, stateFill(now, theme), 0.6)
  p.rectStroke(r, cr(theme.rounding), stroke(1, strokeColor), 'inside')

  const size = clamp(rectHeight(r) * 0.42, 9, 17)
  p.text(rectCenter(r), CENTER_CENTER, text, monospace(size), mix(stateText(was, theme), stateText(now, theme), t))
}

/** A soft glow behind a cell — used for "this just happened". */
export function glow(p: Painter, rect: Rect, color: Color, strength: number, theme: VizTheme): void {
  if (strength <= 0.01) return
  for (let i = 0; i < 3; i++) {
    const grow = 2 + i * 3
    p.rectFilled(inflate(rect, grow), cr(theme.rounding + grow), withAlpha(color, 0.1 * strength))
  }
}

export function caption(p: Painter, at: Pos2, text: string, color: Color, size: number): void {
  p.text(at, LEFT_TOP, text, proportional(size), color)
}

export function monoCentered(p: Painter, at: Pos2, text: string, color: Color, size: number): void {
  p.text(at, CENTER_CENTER, text, monospace(size), color)
}

/** Bytes in the UTF-8 encoding of `s` — Rust's `str::len`, which sizes a pointer tab. */
function utf8Length(s: string): number {
  let n = 0
  for (const ch of s) {
    const cp = ch.codePointAt(0) ?? 0
    n += cp < 0x80 ? 1 : cp < 0x800 ? 2 : cp < 0x10000 ? 3 : 4
  }
  return n
}

/**
 * A named pointer marker: a coloured tab with the name, pointing down at a
 * cell. Multiple pointers on the same cell are stacked so none is hidden.
 */
export function pointerMarker(p: Painter, tip: Pos2, name: string, color: Color, lane: number, theme: VizTheme): void {
  const lift = 4 + lane * 15
  const w = Math.max(utf8Length(name) * 7 + 12, 20)
  const h = 14
  const body = rectFromCenterSize(pos2(tip.x, tip.y - lift - h * 0.5), vec2(w, h))
  p.rectFilled(body, cr(4), color)
  p.convexPolygon([pos2(tip.x - 4, body.max.y), pos2(tip.x + 4, body.max.y), pos2(tip.x, body.max.y + 4)], color)
  monoCentered(p, rectCenter(body), name, theme.on(color), 10)
}

/** Pointers sharing a cell are stacked; this returns how many lanes are needed. */
export function pointerLanes<T>(pointers: readonly (readonly [string, T])[]): number {
  let max = 0
  for (let i = 0; i < pointers.length; i++) max = Math.max(max, laneOf(pointers, i) + 1)
  return max
}

/** The lane of pointer `i`: how many earlier pointers aim at the same target. */
export function laneOf<T>(pointers: readonly (readonly [string, T])[], i: number): number {
  let lane = 0
  for (let k = 0; k < i; k++) if (pointers[k][1] === pointers[i][1]) lane++
  return lane
}

/** Straight arrow with a solid head, used by lists and graphs. */
export function arrow(p: Painter, from: Pos2, to: Pos2, color: Color, width: number): void {
  const dir = sub(to, from)
  const len = length(dir)
  if (len < 1) return
  const unit = scale(dir, 1 / len)
  const head = Math.min(7, len * 0.4)
  const base = sub(to, scale(unit, head))
  p.lineSegment(from, base, stroke(width, color))
  const normal = vec2(-unit.y, unit.x)
  p.convexPolygon([to, add(base, scale(normal, head * 0.5)), sub(base, scale(normal, head * 0.5))], color)
}

/** Fit `count` cells into `avail` points, shrinking (never growing) to fit. */
export function fitCell(count: number, avail: number, theme: VizTheme): number {
  if (count === 0) return theme.cellSize
  const needed = count * (theme.cellSize + theme.gap)
  return needed <= avail ? theme.cellSize : Math.max(avail / count - theme.gap, 10)
}

/** A cell's display text, elided to six characters. */
export function shortCellText(c: Cell): string {
  const s = cellText(c)
  // Long strings would overflow their box; the variables panel shows the
  // full value, the cell shows enough to recognise it.
  if (s.length <= 6) return s
  const chars = [...s]
  return chars.length > 6 ? `${chars.slice(0, 5).join('')}…` : s
}

/** Rust's `f64::from_str` grammar: no whitespace, no hex, no empty string — unlike `Number()`. */
const RUST_FLOAT = /^[+-]?(?:inf|infinity|nan|(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)$/i

/** `Cell::as_f64`: numbers as they are, strings only if Rust would parse them. */
export function cellAsF64(c: Cell): number | null {
  if (typeof c === 'number') return c
  if (!RUST_FLOAT.test(c)) return null
  const body = c.replace(/^[+-]/, '').toLowerCase()
  if (body === 'nan') return Number.NaN
  if (body === 'inf' || body === 'infinity') return c.startsWith('-') ? -Infinity : Infinity
  return Number(c)
}
