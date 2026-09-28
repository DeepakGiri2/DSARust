// A sliver of emath (egui's geometry crate): just the point and rect
// operations the renderers use, with the same meaning, so every renderer reads
// like the Rust it was ported from. Coordinates are canvas points, which are
// CSS pixels — one egui point on the desktop.

export interface Pos2 {
  readonly x: number
  readonly y: number
}

/** An offset rather than a place. Same shape as `Pos2`, as in emath. */
export type Vec2 = Pos2

/** Axis-aligned rectangle: `min` is the top-left corner, `max` the bottom-right. */
export interface Rect {
  readonly min: Pos2
  readonly max: Pos2
}

export const ZERO: Pos2 = { x: 0, y: 0 }

export function pos2(x: number, y: number): Pos2 {
  return { x, y }
}

export const vec2 = pos2

export function splat(v: number): Vec2 {
  return { x: v, y: v }
}

export function add(a: Pos2, b: Vec2): Pos2 {
  return { x: a.x + b.x, y: a.y + b.y }
}

export function sub(a: Pos2, b: Vec2): Pos2 {
  return { x: a.x - b.x, y: a.y - b.y }
}

export function scale(v: Vec2, s: number): Vec2 {
  return { x: v.x * s, y: v.y * s }
}

export function length(v: Vec2): number {
  return Math.hypot(v.x, v.y)
}

/**
 * Unit vector along `v`. A zero vector comes back unchanged (emath's
 * `normalized`), so an edge between two coincident nodes draws nothing
 * instead of a line full of NaN.
 */
export function normalized(v: Vec2): Vec2 {
  const len = length(v)
  return len > 0 ? { x: v.x / len, y: v.y / len } : v
}

export function rectFromMinMax(min: Pos2, max: Pos2): Rect {
  return { min, max }
}

export function rectFromMinSize(min: Pos2, size: Vec2): Rect {
  return { min, max: add(min, size) }
}

export function rectFromCenterSize(center: Pos2, size: Vec2): Rect {
  const half = scale(size, 0.5)
  return { min: sub(center, half), max: add(center, half) }
}

export function rectWidth(r: Rect): number {
  return r.max.x - r.min.x
}

export function rectHeight(r: Rect): number {
  return r.max.y - r.min.y
}

export function rectSize(r: Rect): Vec2 {
  return { x: rectWidth(r), y: rectHeight(r) }
}

export function rectCenter(r: Rect): Pos2 {
  return { x: (r.min.x + r.max.x) * 0.5, y: (r.min.y + r.max.y) * 0.5 }
}

export function rectCenterTop(r: Rect): Pos2 {
  return { x: (r.min.x + r.max.x) * 0.5, y: r.min.y }
}

/** Grow a rect by `by` on every side (emath `Rect::expand`). */
export function expand(r: Rect, by: number): Rect {
  return { min: sub(r.min, splat(by)), max: add(r.max, splat(by)) }
}

export function translate(r: Rect, by: Vec2): Rect {
  return { min: add(r.min, by), max: add(r.max, by) }
}
