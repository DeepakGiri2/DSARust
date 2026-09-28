// Ported from the tests in crates/dsa-core/src/timeline.rs.
import { describe, expect, it } from 'vitest'
import type { Step, Trace } from '@/trace/types'
import { MANUAL_ANIM, Timeline } from './timeline'

function traceWithDepths(depths: number[]): Trace {
  return {
    steps: depths.map(
      (depth, i): Step => ({
        tag: `t${i}`,
        depth,
        event: 'stmt',
        frames: [],
        views: [],
        note: '',
        log_len: 0,
      }),
    ),
    logs: [],
  }
}

describe('Timeline', () => {
  it('step over skips the nested call', () => {
    const tr = traceWithDepths([1, 2, 3, 3, 1])
    const tl = new Timeline(tr.steps.length)
    tl.stepOver(tr)
    expect(tl.idx).toBe(4)
  })

  it('step out leaves the current frame', () => {
    const tr = traceWithDepths([1, 2, 3, 2, 1])
    const tl = new Timeline(tr.steps.length)
    tl.jumpTo(2)
    tl.stepOut(tr)
    expect(tl.idx).toBe(3)
  })

  it('step over at the deepest point runs to the end', () => {
    const tr = traceWithDepths([1, 2, 3])
    const tl = new Timeline(tr.steps.length)
    tl.jumpTo(2)
    tl.stepOver(tr)
    expect(tl.idx).toBe(2)
    expect(tl.atEnd()).toBe(true)
  })

  it('continue stops at the next breakpoint', () => {
    const tr = traceWithDepths(Array(10).fill(1))
    const tl = new Timeline(tr.steps.length)
    tl.breakpoints.add(3)
    tl.breakpoints.add(7)
    tl.continueRun()
    expect(tl.idx).toBe(3)
    tl.continueRun()
    expect(tl.idx).toBe(7)
    tl.continueRun()
    expect(tl.idx).toBe(9)
  })

  it('playback pauses when it hits a breakpoint', () => {
    const tr = traceWithDepths(Array(5).fill(1))
    const tl = new Timeline(tr.steps.length)
    tl.breakpoints.add(1)
    tl.speed = 100
    tl.togglePlay()
    for (let i = 0; i < 20; i++) tl.tick(0.016)
    expect(tl.idx).toBe(1)
    expect(tl.playing).toBe(false)
  })

  it('adjacent steps animate but jumps snap', () => {
    const tr = traceWithDepths(Array(20).fill(1))
    const tl = new Timeline(tr.steps.length)
    tl.stepIn()
    expect(tl.settled()).toBe(false)
    tl.jumpTo(15)
    expect(tl.settled()).toBe(true)
  })

  it('a transition completes in bounded time', () => {
    const tr = traceWithDepths(Array(4).fill(1))
    const tl = new Timeline(tr.steps.length)
    tl.stepIn()
    let elapsed = 0
    while (!tl.settled() && elapsed < 2) {
      tl.tick(0.016)
      elapsed += 0.016
    }
    expect(tl.settled()).toBe(true)
    expect(elapsed).toBeLessThanOrEqual(MANUAL_ANIM + 0.05)
  })

  it('retarget keeps position when possible', () => {
    const tl = new Timeline(50)
    tl.jumpTo(30)
    tl.retarget(40)
    expect(tl.idx).toBe(30)
    tl.retarget(10)
    expect(tl.idx).toBe(9)
  })

  it('tag breakpoints toggle every matching step', () => {
    const tr = traceWithDepths([1, 1, 1, 1])
    const tl = new Timeline(tr.steps.length)
    tl.toggleBreakpointTag('t2', tr)
    expect(tl.breakpoints.has(2)).toBe(true)
    expect(tl.hasBreakpointTag('t2', tr)).toBe(true)
    tl.toggleBreakpointTag('t2', tr)
    expect(tl.breakpoints.size).toBe(0)
  })

  it('play from the end restarts', () => {
    const tr = traceWithDepths([1, 1, 1])
    const tl = new Timeline(tr.steps.length)
    tl.toEnd()
    tl.togglePlay()
    expect(tl.idx).toBe(0)
    expect(tl.playing).toBe(true)
  })

  it('an animation switched off snaps every step', () => {
    const tl = new Timeline(5)
    tl.animate = false
    tl.stepIn()
    expect(tl.settled()).toBe(true)
  })

  it('reset keeps preferences and drops position', () => {
    const tl = new Timeline(10)
    tl.speed = 2.5
    tl.animate = false
    tl.jumpTo(7)
    tl.breakpoints.add(3)
    tl.reset(4)
    expect(tl.idx).toBe(0)
    expect(tl.len).toBe(4)
    expect(tl.breakpoints.size).toBe(0)
    expect(tl.speed).toBe(2.5)
    expect(tl.animate).toBe(false)
  })
})
