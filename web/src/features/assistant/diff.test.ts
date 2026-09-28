// Ported from the tests in crates/dsa-core/src/diff.rs.
import { describe, expect, it } from 'vitest'
import { diff, type Change, type Diff } from './diff'

const kinds = (d: Diff): [Change, string][] => d.lines.map((l) => [l.change, l.text])
const all = (d: Diff, value: boolean) => Array<boolean>(d.hunks).fill(value)

describe('diff', () => {
  it('identical text has nothing to review', () => {
    const src = 'func f() int {\n    return 1\n}\n'
    const d = diff(src, src)
    expect(d.isEmpty()).toBe(true)
    expect(d.removed).toBe(0)
    expect(d.added).toBe(0)
    expect(d.lines.every((l) => l.change === 'same')).toBe(true)
  })

  it('a changed line shows as a removal and an addition', () => {
    const mine = 'func f() int {\n    return 1\n}\n'
    const theirs = 'func f() int {\n    return 2\n}\n'
    const d = diff(mine, theirs)
    expect(kinds(d)).toEqual([
      ['same', 'func f() int {'],
      ['removed', '    return 1'],
      ['added', '    return 2'],
      ['same', '}'],
    ])
    expect([d.removed, d.added, d.hunks]).toEqual([1, 1, 1])
  })

  it('line numbers follow each side independently', () => {
    const d = diff('a\nb\nc\n', 'a\nx\ny\nc\n')
    const byChange = (c: Change) => d.lines.filter((l) => l.change === c).map((l) => [l.oldNo, l.newNo])
    expect(byChange('removed')).toEqual([[2, null]])
    expect(byChange('added')).toEqual([
      [null, 2],
      [null, 3],
    ])
    // The trailing "c" is line 3 on the left and line 4 on the right.
    const last = d.lines[d.lines.length - 1]
    expect([last.oldNo, last.newNo]).toEqual([3, 4])
  })

  it('an insertion removes nothing', () => {
    const d = diff('a\nc\n', 'a\nb\nc\n')
    expect(d.removed).toBe(0)
    expect(d.added).toBe(1)
    expect(d.hunks).toBe(1)
  })

  it('each unbroken run of changes is its own group', () => {
    // Two edits with an untouched line between them are two mistakes, and get
    // a tick each.
    const split = diff('a\nX\nb\nY\nc\n', 'a\n1\nb\n2\nc\n')
    expect(split.hunks).toBe(2)

    // Adjacent edits are one run, and one decision — a replaced line is a
    // removal and an addition, not two changes.
    const together = diff('a\nX\nY\nb\n', 'a\n1\n2\nb\n')
    expect(together.hunks).toBe(1)
    expect([together.removed, together.added]).toEqual([2, 2])
  })

  it('every changed line belongs to exactly one group', () => {
    const d = diff('a\nX\nb\nY\nZ\nc\n', 'a\n1\nb\n2\n3\nc\n')
    for (const l of d.lines) {
      if (l.change === 'same') {
        expect(l.hunk, 'context is in no group').toBeNull()
      } else {
        expect(l.hunk, 'a change is always in a group').not.toBeNull()
        expect(l.hunk!, 'group index out of range').toBeLessThan(d.hunks)
      }
    }
  })

  it('applying everything reproduces the proposal', () => {
    const mine = 'func f() int {\n    x := 1\n    return x\n}\n'
    const theirs = 'func f() int {\n    x := 2\n    y := 3\n    return x + y\n}\n'
    const d = diff(mine, theirs)
    expect(d.apply(all(d, true))).toBe(theirs)
  })

  it('applying nothing gives back exactly what the user wrote', () => {
    const mine = 'func f() int {\n    x := 1\n    return x\n}\n'
    const theirs = 'func f() int {\n    return 99\n}\n'
    const d = diff(mine, theirs)
    expect(d.apply(all(d, false))).toBe(mine)
  })

  it('a single group can be taken while another is left', () => {
    const mine = 'a\nWRONG1\nb\nc\nd\ne\nf\nWRONG2\ng\n'
    const theirs = 'a\nRIGHT1\nb\nc\nd\ne\nf\nRIGHT2\ng\n'
    const d = diff(mine, theirs)
    expect(d.hunks, 'two runs, two decisions').toBe(2)

    const firstOnly = d.apply([true, false])
    expect(firstOnly).toContain('RIGHT1')
    expect(firstOnly).toContain('WRONG2')
    expect(firstOnly).not.toContain('WRONG1')

    const secondOnly = d.apply([false, true])
    expect(secondOnly).toContain('WRONG1')
    expect(secondOnly).toContain('RIGHT2')
  })

  it('an empty accepted list means take it all', () => {
    // The banner's "apply" builds its list from the ticks; a stale or short
    // one must not silently drop the fix.
    const d = diff('a\n', 'b\n')
    expect(d.apply([])).toBe('b\n')
  })

  it('writing from scratch is all additions', () => {
    const d = diff('', 'func f() {}\n')
    expect(d.removed).toBe(0)
    expect(d.added).toBe(1)
    expect(d.apply([true])).toBe('func f() {}\n')
  })

  it('folded context covers every line exactly once', () => {
    const mine = Array.from({ length: 30 }, (_, i) => `line ${i + 1}`).join('\n')
    const theirs = mine.replaceAll('line 15', 'CHANGED')
    const d = diff(mine, theirs)

    const rows = d.rows(3)
    const shown = rows.flatMap((r) => (r.kind === 'line' ? [r.index] : []))
    const hidden = rows.reduce((sum, r) => sum + (r.kind === 'folded' ? r.count : 0), 0)
    expect(shown.length + hidden, 'no line is lost').toBe(d.lines.length)

    // Every changed line survives the fold, and the wall of context does not.
    d.lines.forEach((l, i) => {
      if (l.change !== 'same') expect(shown, `hid a change at ${i}`).toContain(i)
    })
    expect(hidden, 'a 30-line file with one edit folds most of itself').toBeGreaterThan(15)
  })

  it('nothing is folded when everything is near a change', () => {
    const d = diff('a\nb\n', 'x\ny\n')
    expect(d.rows(3).every((r) => r.kind === 'line')).toBe(true)
  })

  it('a pathological input degrades instead of hanging', () => {
    // Past the cell cap the answer is "all of this became all of that", which
    // still applies correctly — it just stops being minimal.
    const a = Array.from({ length: 3000 }, (_, i) => `a${i}\n`).join('')
    const b = Array.from({ length: 3000 }, (_, i) => `b${i}\n`).join('')
    const d = diff(a, b)
    expect(d.removed).toBe(3000)
    expect(d.added).toBe(3000)
    expect(d.apply([true])).toBe(b)
    expect(d.apply([false])).toBe(a)
  })
})
