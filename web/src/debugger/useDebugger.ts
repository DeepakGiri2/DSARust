// React binding for `Timeline`.
//
// The timeline is mutable state driven by a clock, which is what React is
// worst at, so it lives outside React: a `Timeline` in a ref, a
// requestAnimationFrame loop that runs only while something is moving
// (`needsRepaint`), and `useSyncExternalStore` publishing an immutable snapshot
// whenever the picture changes. An idle debugger costs zero frames — the same
// property the desktop gets from egui's `request_repaint` discipline.

import { useCallback, useEffect, useMemo, useRef, useSyncExternalStore } from 'react'
import type { Step, Trace } from '@/trace/types'
import { Timeline, type StepMode } from './timeline'

export interface DebuggerSnapshot {
  idx: number
  /** The step the current transition animates from. */
  fromIdx: number
  len: number
  playing: boolean
  /** Eased 0..1 transition progress; 1 = settled. */
  t: number
  speed: number
  animate: boolean
  breakpoints: ReadonlySet<number>
  atStart: boolean
  atEnd: boolean
}

export interface Debugger extends DebuggerSnapshot {
  /** The current step, or null when there is no trace. */
  step: Step | null
  /** The step being animated from (for tweening); equals `step` once settled. */
  fromStep: Step | null
  stepIn(): void
  stepOver(): void
  stepOut(): void
  stepBack(): void
  stepMode(mode: StepMode): void
  continueRun(): void
  restart(): void
  toEnd(): void
  jumpTo(i: number): void
  togglePlay(): void
  pause(): void
  setSpeed(s: number): void
  setAnimate(on: boolean): void
  toggleBreakpointTag(tag: string): void
  hasBreakpointTag(tag: string): boolean
  clearBreakpoints(): void
}

export interface DebuggerOptions {
  /** Changing this resets position and breakpoints (a different problem). A new
   *  trace with the same key only retargets (edited input: keep your place). */
  resetKey?: string
  speed?: number
  animate?: boolean
}

function snapshotOf(tl: Timeline): DebuggerSnapshot {
  return {
    idx: tl.idx,
    fromIdx: tl.fromIdx,
    len: tl.len,
    playing: tl.playing,
    t: tl.transition(),
    speed: tl.speed,
    animate: tl.animate,
    breakpoints: new Set(tl.breakpoints),
    atStart: tl.atStart(),
    atEnd: tl.atEnd(),
  }
}

export function useDebugger(trace: Trace | null, opts: DebuggerOptions = {}): Debugger {
  const tlRef = useRef<Timeline | null>(null)
  if (!tlRef.current) tlRef.current = new Timeline(0)
  const tl = tlRef.current

  const traceRef = useRef<Trace | null>(trace)
  const listeners = useRef(new Set<() => void>())
  const snap = useRef<DebuggerSnapshot>(snapshotOf(tl))
  const raf = useRef<number | null>(null)
  const last = useRef<number | null>(null)

  const emit = useCallback(() => {
    snap.current = snapshotOf(tl)
    listeners.current.forEach((l) => l())
  }, [tl])

  const frame = useCallback(
    (now: number) => {
      const dt = last.current === null ? 0 : (now - last.current) / 1000
      last.current = now
      if (tl.tick(dt)) emit()
      if (tl.needsRepaint()) {
        raf.current = requestAnimationFrame(frame)
      } else {
        raf.current = null
        last.current = null
      }
    },
    [tl, emit],
  )

  const kick = useCallback(() => {
    if (raf.current === null && tl.needsRepaint()) {
      last.current = null
      raf.current = requestAnimationFrame(frame)
    }
  }, [tl, frame])

  useEffect(
    () => () => {
      if (raf.current !== null) cancelAnimationFrame(raf.current)
    },
    [],
  )

  // A new trace: reset for a new problem, retarget for new input.
  const keyRef = useRef<string | undefined>(opts.resetKey)
  useEffect(() => {
    traceRef.current = trace
    const len = trace?.steps.length ?? 0
    if (keyRef.current !== opts.resetKey) {
      keyRef.current = opts.resetKey
      tl.reset(len)
    } else if (len !== tl.len || trace) {
      tl.retarget(len)
    }
    emit()
  }, [trace, opts.resetKey, tl, emit])

  useEffect(() => {
    if (opts.speed !== undefined && opts.speed !== tl.speed) {
      tl.speed = opts.speed
      emit()
    }
  }, [opts.speed, tl, emit])

  useEffect(() => {
    if (opts.animate !== undefined && opts.animate !== tl.animate) {
      tl.animate = opts.animate
      emit()
    }
  }, [opts.animate, tl, emit])

  const subscribe = useCallback((l: () => void) => {
    listeners.current.add(l)
    return () => listeners.current.delete(l)
  }, [])
  const state = useSyncExternalStore(
    subscribe,
    () => snap.current,
    () => snap.current,
  )

  const act = useCallback(
    (f: (t: Trace) => void) => {
      const tr = traceRef.current
      if (!tr) return
      f(tr)
      emit()
      kick()
    },
    [emit, kick],
  )

  const api = useMemo(
    () => ({
      stepIn: () => act(() => tl.stepIn()),
      stepOver: () => act((tr) => tl.stepOver(tr)),
      stepOut: () => act((tr) => tl.stepOut(tr)),
      stepBack: () => act(() => tl.stepBack()),
      stepMode: (mode: StepMode) => act((tr) => tl.step(mode, tr)),
      continueRun: () => act(() => tl.continueRun()),
      restart: () => act(() => tl.restart()),
      toEnd: () => act(() => tl.toEnd()),
      jumpTo: (i: number) => act(() => tl.jumpTo(i)),
      togglePlay: () => act(() => tl.togglePlay()),
      pause: () => act(() => tl.pause()),
      setSpeed: (s: number) =>
        act(() => {
          tl.speed = s
        }),
      setAnimate: (on: boolean) =>
        act(() => {
          tl.animate = on
        }),
      toggleBreakpointTag: (tag: string) => act((tr) => tl.toggleBreakpointTag(tag, tr)),
      hasBreakpointTag: (tag: string) => {
        const tr = traceRef.current
        return tr ? tl.hasBreakpointTag(tag, tr) : false
      },
      clearBreakpoints: () =>
        act(() => {
          tl.breakpoints.clear()
        }),
    }),
    [act, tl],
  )

  const step = trace?.steps[state.idx] ?? null
  const fromStep = state.t < 1 ? (trace?.steps[state.fromIdx] ?? step) : step
  return { ...state, ...api, step, fromStep }
}
