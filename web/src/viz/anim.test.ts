import type { ArrayView, KvView } from '@/trace/types'
import { easeBack, easeOut, flash, inflate, isNew, lerp, lerpPos, lerpRect, mix, pair, scaleRect, withAlpha } from './anim'
import { rgb } from './color'
import { pos2, rectFromMinMax } from './geom'

const arr = (label: string): ArrayView => ({ type: 'array', label, data: [] })
const kv = (label: string): KvView => ({ type: 'kv', label, entries: [] })

describe('pair', () => {
  it('pairs views by kind and label', () => {
    const prev = [arr('nums'), kv('seen')]
    const pairs = pair(prev, [arr('nums'), kv('seen')])
    expect(pairs.map((p) => p.before)).toEqual([prev[0], prev[1]])
  })

  it('does not tween a renamed view against the old one', () => {
    expect(pair([arr('nums')], [arr('nums (target = 9)')])[0].before).toBeNull()
  })

  it('does not pair across a kind change', () => {
    expect(pair([arr('x')], [kv('x')])[0].before).toBeNull()
  })

  it('pairs duplicate labels one to one', () => {
    const prev = [arr('a'), arr('a')]
    const pairs = pair(prev, [arr('a'), arr('a'), arr('a')])
    expect(pairs[0].before).toBe(prev[0])
    expect(pairs[1].before).toBe(prev[1])
    expect(pairs[2].before).toBeNull()
  })

  it('matches by identity, not position', () => {
    const prev = [kv('m'), arr('m')]
    const pairs = pair(prev, [arr('m'), kv('m')])
    expect(pairs[0].before).toBe(prev[1])
    expect(pairs[1].before).toBe(prev[0])
  })

  it('has nothing to pair against on the first step', () => {
    expect(pair(null, [arr('a')])[0].before).toBeNull()
    expect(pair(undefined, [arr('a')])[0].before).toBeNull()
  })
})

describe('interpolation', () => {
  it('easing curves hit their endpoints', () => {
    for (const f of [easeOut, easeBack]) {
      expect(f(0)).toBeCloseTo(0, 6)
      expect(f(1)).toBeCloseTo(1, 6)
    }
  })

  it('ease-back overshoots before settling', () => {
    expect(easeBack(0.75)).toBeGreaterThan(1)
  })

  it('clamps its input like the Rust helpers', () => {
    expect(easeOut(2)).toBe(1)
    expect(easeOut(-1)).toBe(0)
    expect(lerp(0, 10, 1.5)).toBe(10)
    expect(lerp(0, 10, -1)).toBe(0)
  })

  it('flash decays from full to nothing', () => {
    expect(flash(0)).toBe(1)
    expect(flash(1)).toBe(0)
    expect(flash(0.5)).toBe(0.25)
  })

  it('interpolates points and rects component-wise', () => {
    expect(lerpPos(pos2(0, 10), pos2(10, 30), 0.5)).toEqual(pos2(5, 20))
    const r = lerpRect(rectFromMinMax(pos2(0, 0), pos2(10, 10)), rectFromMinMax(pos2(10, 10), pos2(30, 30)), 0.5)
    expect(r).toEqual(rectFromMinMax(pos2(5, 5), pos2(20, 20)))
  })

  it('mixes exactly at the endpoints and rounds in between', () => {
    const black = rgb(0x000000)
    const white = rgb(0xffffff)
    expect(mix(black, white, 0)).toEqual(black)
    expect(mix(black, white, 1)).toEqual(white)
    expect(mix(black, white, 0.5)[0]).toBe(128)
    // The desktop's current-node fill for the dark theme, checked against its screenshot.
    expect(mix(rgb(0x161b28), rgb(0xfbbf24), 0.6)).toEqual(rgb(0x9f7d26))
  })

  it('truncates alpha as `as u8` does', () => {
    expect(withAlpha(rgb(0x102030), 0.5)).toEqual([0x10, 0x20, 0x30, 127])
    expect(withAlpha(rgb(0x102030), 2)[3]).toBe(255)
    expect(withAlpha(rgb(0x102030), Number.NaN)[3]).toBe(0)
  })

  it('scales about the centre and inflates on every side', () => {
    const r = rectFromMinMax(pos2(10, 10), pos2(30, 20))
    expect(scaleRect(r, 2)).toEqual(rectFromMinMax(pos2(0, 5), pos2(40, 25)))
    expect(inflate(r, 2)).toEqual(rectFromMinMax(pos2(8, 8), pos2(32, 22)))
  })

  it('detects an index that is newly highlighted', () => {
    expect(isNew(3, [1, 3], [1])).toBe(true)
    expect(isNew(1, [1, 3], [1])).toBe(false)
    expect(isNew(9, [1], [])).toBe(false)
  })
})
