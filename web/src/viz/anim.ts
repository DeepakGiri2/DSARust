// Interpolation helpers — a port of `crates/dsa-viz/src/anim.rs`, the
// difference between a slideshow and a debugger you can actually follow.
//
// Every renderer is handed the previous step's view alongside the current
// one and a progress value `t`. Anything positional (a pointer index, a bar
// height, a cursor cell) is interpolated; anything categorical (a cell turned
// green) is cross-faded, with a brief overshoot so the eye is drawn to what
// *changed* rather than having to diff two frames itself.

import type { VizView } from '@/trace/types'
import type { Color } from './color'
import { expand, pos2, rectCenter, rectFromCenterSize, rectFromMinMax, rectSize, scale, type Pos2, type Rect } from './geom'
import { asU8, clamp01 } from './num'

/** `t` is clamped, so an overshooting easing never pushes a lerp past its target. */
export function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * clamp01(t)
}

export function lerpPos(a: Pos2, b: Pos2, t: number): Pos2 {
  return pos2(lerp(a.x, b.x, t), lerp(a.y, b.y, t))
}

export function lerpRect(a: Rect, b: Rect, t: number): Rect {
  return rectFromMinMax(lerpPos(a.min, b.min, t), lerpPos(a.max, b.max, t))
}

/**
 * Straight RGBA mix, rounded per channel. Good enough for UI accents and
 * avoids the gamma games that make two-colour fades look muddy in the middle.
 */
export function mix(a: Color, b: Color, t: number): Color {
  const k = clamp01(t)
  const f = (x: number, y: number) => asU8(Math.round(x + (y - x) * k))
  return [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2]), f(a[3], b[3])]
}

/** Replace the alpha; `a * 255` truncates, as `as u8` does. */
export function withAlpha(c: Color, a: number): Color {
  return [c[0], c[1], c[2], asU8(clamp01(a) * 255)]
}

/**
 * A short bright pulse at the start of a transition, decaying to nothing.
 * Used for "this just happened" emphasis: a stored map key, a flipped bit.
 */
export function flash(t: number): number {
  const k = 1 - clamp01(t)
  return k * k
}

/** Slight overshoot then settle — a push landing on a stack, an entry dropping into a map. */
export function easeBack(t: number): number {
  const k = clamp01(t)
  const c1 = 1.70158
  const c3 = c1 + 1
  return 1 + c3 * (k - 1) ** 3 + c1 * (k - 1) ** 2
}

export function easeOut(t: number): number {
  return 1 - (1 - clamp01(t)) ** 3
}

/** Grow a rect about its centre — the "pop" for a value that just changed. */
export function scaleRect(r: Rect, s: number): Rect {
  return rectFromCenterSize(rectCenter(r), scale(rectSize(r), s))
}

export function inflate(r: Rect, by: number): Rect {
  return expand(r, by)
}

/** `VizView::anim_key`: kind and label. The kind names contain no `:`, so the pair stays unambiguous. */
export function animKey(view: VizView): string {
  return `${view.type}:${view.label}`
}

export interface ViewPair {
  readonly view: VizView
  /** The previous step's view with the same kind and label, if there was one. */
  readonly before: VizView | null
}

/**
 * Pair each current view with its counterpart from the previous step.
 *
 * Views only animate against a view of the same kind *and* label, so an
 * author who keeps labels stable gets motion for free, and one who rebuilds a
 * label every step gets a clean cut instead of nonsense tweening. Each
 * previous view is used at most once, first match first.
 */
export function pair(prev: readonly VizView[] | null | undefined, cur: readonly VizView[]): ViewPair[] {
  const pool = prev ?? []
  const used = pool.map(() => false)
  return cur.map((view) => {
    const key = animKey(view)
    const i = pool.findIndex((p, j) => !used[j] && animKey(p) === key)
    if (i < 0) return { view, before: null }
    used[i] = true
    return { view, before: pool[i] }
  })
}

/**
 * `true` when `idx` is newly present in `now` compared with `was` — the cue
 * for a flash rather than a steady highlight.
 */
export function isNew(idx: number, now: readonly number[], was: readonly number[]): boolean {
  return now.includes(idx) && !was.includes(idx)
}
