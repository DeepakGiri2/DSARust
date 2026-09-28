// The Visualize tab's pure logic: keys, variable changes, log styling (ported
// from crates/dsa-app/src/style.rs tests) and the speed slider.
import { describe, expect, it } from 'vitest'
import type { Frame, LogEntry, VarVal } from '@/trace/types'
import { isTypingTarget, keyAction } from './keys'
import { logColor, logGlyph, logText } from './logStyle'
import { sliderToSpeed, speedToSlider } from './Transport'
import { childLines, varEq, varRows } from './vars'

const key = (k: string, extra: Partial<KeyboardEvent> = {}, target: EventTarget | null = document.body) =>
  keyAction({ key: k, shiftKey: false, ctrlKey: false, metaKey: false, altKey: false, target, ...extra })

describe('keyboard', () => {
  it('maps the desktop and VS Code keys', () => {
    expect(key('ArrowLeft')).toBe('back')
    expect(key('ArrowRight')).toBe('over')
    expect(key('F10')).toBe('over')
    expect(key('ArrowDown')).toBe('in')
    expect(key('F11')).toBe('in')
    expect(key('F11', { shiftKey: true })).toBe('out')
    expect(key('ArrowUp')).toBe('out')
    expect(key(' ')).toBe('play')
    expect(key('r')).toBe('restart')
    expect(key('Home')).toBe('restart')
    expect(key('F5')).toBe('continue')
    expect(key('c')).toBe('continue')
    expect(key('End')).toBe('end')
    expect(key('x')).toBeNull()
  })

  it('leaves browser shortcuts alone', () => {
    expect(key('r', { ctrlKey: true })).toBeNull()
    expect(key('ArrowLeft', { metaKey: true })).toBeNull()
    expect(key('F5', { altKey: true })).toBeNull()
  })

  it('never fires while typing', () => {
    const input = document.createElement('input')
    const area = document.createElement('textarea')
    const editor = document.createElement('div')
    editor.className = 'cm-editor'
    const inner = document.createElement('span')
    editor.append(inner)
    for (const t of [input, area, inner]) expect(key('ArrowRight', {}, t)).toBeNull()
    expect(isTypingTarget(document.body)).toBe(false)
  })

  it('lets Space activate a focused button instead of toggling play', () => {
    const button = document.createElement('button')
    expect(key(' ', {}, button)).toBeNull()
    expect(key('ArrowRight', {}, button)).toBe('over')
  })
})

const frame = (vars: Record<string, VarVal>): Frame => ({ fn: 'f', vars })

describe('variables', () => {
  const num = (v: number): VarVal => ({ kind: 'num', v })

  it('lights everything on the first step, when there is nothing to compare with', () => {
    const rows = varRows(frame({ a: num(1), b: num(2) }), undefined, () => false)
    expect(rows.map((r) => r.changed)).toEqual([true, true])
  })

  it('lights only what changed since the previous step, and new variables', () => {
    const rows = varRows(frame({ a: num(1), b: num(3), c: num(0) }), frame({ a: num(1), b: num(2) }), () => false)
    expect(rows.map((r) => [r.name, r.changed])).toEqual([
      ['a', false],
      ['b', true],
      ['c', true],
    ])
  })

  it('counts a moved highlight as a change, and bridges number and string cells', () => {
    const before: VarVal = { kind: 'map', v: [['2', 0]], hl: ['2'] }
    const after: VarVal = { kind: 'map', v: [['2', 0]] }
    expect(varEq(before, after)).toBe(false)
    expect(varEq({ kind: 'arr', v: [1, 2] }, { kind: 'arr', v: ['1', 2], hl: [] })).toBe(true)
    expect(varEq({ kind: 'num', v: 1 }, { kind: 'str', v: '1' })).toBe(false)
  })

  it('lists watched variables first, otherwise in frame order', () => {
    const rows = varRows(frame({ a: num(1), b: num(2), c: num(3) }), undefined, (n) => n === 'c')
    expect(rows.map((r) => r.name)).toEqual(['c', 'a', 'b'])
    expect(rows[0].watched).toBe(true)
  })

  it('expands composites into child lines with the flashed entries marked', () => {
    expect(childLines({ kind: 'map', v: [['2', 0], ['7', 1]], hl: ['7'] })).toEqual([
      { hot: false, text: '2 → 0' },
      { hot: true, text: '7 → 1' },
    ])
    expect(childLines({ kind: 'arr', v: [5, 6], hl: [1] })[1]).toEqual({ hot: true, text: '[1] 6' })
    expect(childLines({ kind: 'set', v: [3], hl: ['3'] })).toEqual([{ hot: true, text: '3' }])
  })
})

describe('logs', () => {
  it('gives every kind its own colour, a result the payoff green', () => {
    const all = (['log', 'call', 'return', 'result'] as const).map(logColor)
    expect(new Set(all).size).toBe(4)
    expect(logColor('result')).toBe('var(--green)')
  })

  it("replaces the recorder's arrow prefix with the glyph", () => {
    const call: LogEntry = { step: 0, text: '-> enter dfs(1,2)', kind: 'call' }
    const ret: LogEntry = { step: 3, text: '<- dfs returns 9', kind: 'return' }
    expect(logText(call)).toBe('enter dfs(1,2)')
    expect(logText(ret)).toBe('dfs returns 9')
    expect(logGlyph('call')).toBe('→')
    expect(logGlyph('return')).toBe('←')
    expect(logText({ step: 1, text: '2 + 1 = 3', kind: 'log' })).toBe('2 + 1 = 3')
  })
})

describe('speed slider', () => {
  it('is logarithmic over the desktop range and round-trips 1×', () => {
    expect(sliderToSpeed(0)).toBe(0.25)
    expect(sliderToSpeed(1000)).toBe(6)
    expect(sliderToSpeed(speedToSlider(1))).toBe(1)
    expect(speedToSlider(1)).toBeLessThan(500)
  })
})
