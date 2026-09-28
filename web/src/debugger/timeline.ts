// The debugger itself: a cursor over a recorded trace plus the clock that
// drives the animation between two adjacent steps.
//
// A line-for-line port of `crates/dsa-core/src/timeline.rs`, constants and
// all, so stepping feels identical in the desktop and the browser. It is pure
// state — no timers, no DOM. The host calls `tick(dt)` once per animation frame
// and reads `transition()` to interpolate the picture.

import type { Trace } from '@/trace/types'

/** Seconds a single visual transition takes when stepping by hand. */
export const MANUAL_ANIM = 0.22
/** Longest a transition may take during playback, whatever the speed. */
export const MAX_PLAY_ANIM = 0.45
/** Base dwell between auto-advanced steps at speed 1.0. */
export const BASE_DWELL = 0.9

export type StepMode = 'in' | 'over' | 'out' | 'back'

export function easeOutCubic(t: number): number {
  const c = Math.min(1, Math.max(0, t))
  return 1 - (1 - c) ** 3
}

export class Timeline {
  private _len = 0
  private _idx = 0
  /** Step the current transition is animating *from*. */
  private _from = 0
  /** 0..=1 progress of that transition. */
  private t = 1
  private animSecs = MANUAL_ANIM
  private _playing = false
  private dwellLeft = 0

  /** Playback multiplier; also shortens the transition during playback. */
  speed = 1
  /** Master switch — off makes every transition instant. */
  animate = true
  breakpoints = new Set<number>()

  constructor(len = 0) {
    this._len = len
  }

  /** Point at a different trace, keeping preferences but dropping position and breakpoints. */
  reset(len: number): void {
    const { speed, animate } = this
    this._len = len
    this._idx = 0
    this._from = 0
    this.t = 1
    this.animSecs = MANUAL_ANIM
    this._playing = false
    this.dwellLeft = 0
    this.breakpoints = new Set()
    this.speed = speed
    this.animate = animate
  }

  /**
   * Re-target after an input edit, keeping the cursor where it was if the new
   * trace is long enough — tweak an input, keep watching the same part.
   */
  retarget(len: number): void {
    const keep = Math.min(this._idx, Math.max(0, len - 1))
    this._len = len
    this._idx = keep
    this._from = keep
    this.t = 1
    this._playing = false
    for (const b of [...this.breakpoints]) if (b >= len) this.breakpoints.delete(b)
  }

  get idx(): number {
    return this._idx
  }
  get fromIdx(): number {
    return this._from
  }
  get len(): number {
    return this._len
  }
  get playing(): boolean {
    return this._playing
  }
  isEmpty(): boolean {
    return this._len === 0
  }
  atEnd(): boolean {
    return this._len === 0 || this._idx + 1 >= this._len
  }
  atStart(): boolean {
    return this._idx === 0
  }

  /** Eased 0..=1 progress of the current transition. 1 means settled. */
  transition(): number {
    return easeOutCubic(this.t)
  }
  /** Raw (un-eased) progress, for effects that want linear time. */
  transitionLinear(): number {
    return this.t
  }
  settled(): boolean {
    return this.t >= 1
  }
  /** True while the visualization still needs repainting. */
  needsRepaint(): boolean {
    return this._playing || !this.settled()
  }

  // ── movement ──────────────────────────────────────────────────────────────

  private goto(target: number, animated: boolean): void {
    if (this._len === 0) return
    const clamped = Math.min(target, this._len - 1)
    if (clamped === this._idx) return
    this._from = this._idx
    this._idx = clamped
    // Jumping more than one step (scrub, continue-to-breakpoint) has no
    // meaningful in-between picture, so it snaps.
    const adjacent = Math.abs(clamped - this._from) === 1
    this.t = this.animate && animated && adjacent ? 0 : 1
    this.animSecs = this._playing ? Math.min((BASE_DWELL / this.speed) * 0.6, MAX_PLAY_ANIM) : MANUAL_ANIM
  }

  /** Advance one recorded step, descending into calls. */
  stepIn(): void {
    this._playing = false
    this.goto(this._idx + 1, true)
  }

  stepBack(): void {
    this._playing = false
    this.goto(Math.max(0, this._idx - 1), true)
  }

  /** Next step at the same depth or shallower — run nested calls to completion. */
  stepOver(trace: Trace): void {
    this._playing = false
    const d = trace.steps[this._idx]?.depth ?? 0
    let target = Math.max(0, this._len - 1)
    for (let j = this._idx + 1; j < this._len; j++) {
      if (trace.steps[j].depth <= d) {
        target = j
        break
      }
    }
    this.goto(target, target === this._idx + 1)
  }

  /** Run until the current frame returns. */
  stepOut(trace: Trace): void {
    this._playing = false
    const d = trace.steps[this._idx]?.depth ?? 0
    let target = Math.max(0, this._len - 1)
    for (let j = this._idx + 1; j < this._len; j++) {
      if (trace.steps[j].depth < d) {
        target = j
        break
      }
    }
    this.goto(target, target === this._idx + 1)
  }

  step(mode: StepMode, trace: Trace): void {
    switch (mode) {
      case 'in':
        return this.stepIn()
      case 'over':
        return this.stepOver(trace)
      case 'out':
        return this.stepOut(trace)
      case 'back':
        return this.stepBack()
    }
  }

  /** VS Code style continue: run to the next breakpoint, else to the end. */
  continueRun(): void {
    this._playing = false
    let target = Math.max(0, this._len - 1)
    let best = Infinity
    for (const b of this.breakpoints) if (b > this._idx && b < best) best = b
    if (best !== Infinity) target = best
    this.goto(target, false)
  }

  restart(): void {
    this._playing = false
    this.goto(0, false)
    this.t = 1
  }

  toEnd(): void {
    this._playing = false
    this.goto(Math.max(0, this._len - 1), false)
  }

  /** Scrubber drag — always instant, the user is driving the picture. */
  jumpTo(i: number): void {
    this._playing = false
    this.goto(Math.max(0, i), false)
  }

  togglePlay(): void {
    if (this.atEnd()) {
      // Play from a finished trace restarts rather than doing nothing.
      this.restart()
      this._playing = true
    } else {
      this._playing = !this._playing
    }
    this.dwellLeft = 0
  }

  pause(): void {
    this._playing = false
  }

  toggleBreakpoint(i: number): void {
    if (!this.breakpoints.delete(i)) this.breakpoints.add(i)
  }

  /** Breakpoints are set on source *tags*: toggling one marks every step on that line. */
  toggleBreakpointTag(tag: string, trace: Trace): void {
    const hits: number[] = []
    trace.steps.forEach((s, i) => s.tag === tag && hits.push(i))
    const any = hits.some((i) => this.breakpoints.has(i))
    for (const i of hits) {
      if (any) this.breakpoints.delete(i)
      else this.breakpoints.add(i)
    }
  }

  hasBreakpointTag(tag: string, trace: Trace): boolean {
    for (const i of this.breakpoints) if (trace.steps[i]?.tag === tag) return true
    return false
  }

  // ── clock ─────────────────────────────────────────────────────────────────

  /** Drive animation and autoplay. `dt` is seconds since the last frame. True when the picture changed. */
  tick(dtRaw: number): boolean {
    let dirty = false
    const dt = Math.min(0.25, Math.max(0, dtRaw)) // survive a stalled frame without a jump

    if (this.t < 1) {
      this.t = Math.min(1, this.t + dt / Math.max(this.animSecs, 0.001))
      dirty = true
    }

    if (this._playing) {
      if (this.atEnd()) {
        this._playing = false
        return true
      }
      this.dwellLeft -= dt
      if (this.dwellLeft <= 0 && this.t >= 1) {
        const next = this._idx + 1
        this.goto(next, true)
        this.dwellLeft = BASE_DWELL / Math.max(this.speed, 0.05)
        if (this.breakpoints.has(next)) this._playing = false
        dirty = true
      }
    }
    return dirty
  }
}
