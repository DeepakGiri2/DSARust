import type { ArrayView, BitsView, GraphView, GridView, KvView, LinkedListView, StackView, TextView, TreeView, VizView } from '@/trace/types'
import { cellAsF64, fitCell, pointerLanes, shortCellText, stateFill, type CellState } from './draw'
import { pos2, rectFromMinMax } from './geom'
import { arrayCellState, arrayHeight, kvHeight } from './linear'
import { at, graphLayout, gridCellSize, listLayout, treeDepth, treeLayout } from './linked'
import { viewHeight } from './render'
import { DARK_THEME as theme } from './theme'

const RECT = rectFromMinMax(pos2(0, 0), pos2(600, 400))

function tree(spec: [id: number, left: number | null, right: number | null][]): TreeView {
  return { type: 'tree', label: 't', nodes: spec.map(([id, left, right]) => ({ id, val: id, left, right })), root: 0 }
}

describe('tree layout', () => {
  it('places nodes in-order left to right and by depth top to bottom', () => {
    const l = treeLayout(tree([[0, 1, 2], [1, null, null], [2, null, null]]), RECT, theme)
    // 3 slots of 78 centred in 560 usable points; rows 58 apart under a 20 + 22 header.
    expect(l.get(1)).toEqual(pos2(222, 100))
    expect(l.get(0)).toEqual(pos2(300, 42))
    expect(l.get(2)).toEqual(pos2(378, 100))
  })

  it('gives swapped subtrees swapped positions, so the swap animates across', () => {
    const a = treeLayout(tree([[0, 1, 2], [1, null, null], [2, null, null]]), RECT, theme)
    const b = treeLayout(tree([[0, 2, 1], [1, null, null], [2, null, null]]), RECT, theme)
    expect(a.get(1)!.x).toBeLessThan(a.get(2)!.x)
    expect(b.get(1)!.x).toBeGreaterThan(b.get(2)!.x)
  })

  it('survives a cycle instead of hanging', () => {
    const bad = tree([[0, 1, null], [1, 0, null]])
    expect(treeLayout(bad, RECT, theme).size).toBe(2)
    expect(treeDepth(bad)).toBe(2)
  })

  it('stops laying out below 25 levels', () => {
    const chain = tree(Array.from({ length: 40 }, (_, i): [number, number | null, number | null] => [i, i + 1 < 40 ? i + 1 : null, null]))
    expect(treeLayout(chain, RECT, theme).size).toBe(25)
  })
})

describe('interpolated node position', () => {
  it('starts at the old spot, ends at the new one, and appears in place when new', () => {
    const now = new Map([[7, pos2(100, 0)]])
    const old = new Map([[7, pos2(0, 0)]])
    expect(at(now, old, 7, 0).x).toBe(0)
    expect(at(now, old, 7, 1).x).toBe(100)
    expect(at(now, new Map(), 7, 0).x).toBe(100)
  })
})

describe('graph layout', () => {
  const g = (nodes: GraphView['nodes']): GraphView => ({ type: 'graph', label: 'g', nodes, edges: [] })
  const rect = rectFromMinMax(pos2(0, 0), pos2(600, 270))

  it('falls back to a circle starting at 12 o’clock', () => {
    const l = graphLayout(g([0, 1, 2, 3].map((id) => ({ id, label: String(id) }))), rect, theme)
    // Centre (300, 145), radius min(250/2 - 30, 600/2 - 40) = 95.
    const expected = [pos2(300, 50), pos2(395, 145), pos2(300, 240), pos2(205, 145)]
    expected.forEach((p, id) => {
      expect(l.get(id)!.x).toBeCloseTo(p.x, 6)
      expect(l.get(id)!.y).toBeCloseTo(p.y, 6)
    })
  })

  it('is deterministic and honours explicit unit coordinates', () => {
    const v = g([
      { id: 0, label: 'a', x: 0, y: 0 },
      { id: 1, label: 'b', x: 1, y: 1 },
      { id: 2, label: 'c' },
    ])
    const a = graphLayout(v, rect, theme)
    expect(graphLayout(v, rect, theme)).toEqual(a)
    expect(a.get(0)).toEqual(pos2(30, 40))
    expect(a.get(1)).toEqual(pos2(570, 240))
  })
})

describe('list layout', () => {
  it('spaces nodes by their clamped width plus the arrow gap', () => {
    const v: LinkedListView = {
      type: 'list',
      label: 'l',
      nodes: [0, 1, 2].map((id) => ({ id, val: id, next: id < 2 ? id + 1 : null })),
    }
    const { layout, nodeW } = listLayout(v, RECT, theme)
    expect(nodeW).toBe(62)
    expect([0, 1, 2].map((id) => layout.get(id))).toEqual([pos2(39, 64), pos2(135, 64), pos2(231, 64)])
  })
})

describe('heights', () => {
  const array = (extra: Partial<ArrayView> = {}): ArrayView => ({ type: 'array', label: 'a', data: [1, 2, 3], ...extra })
  const kv = (n: number): KvView => ({ type: 'kv', label: 'm', entries: Array.from({ length: n }, (_, i) => [String(i), i]) })
  const stack = (kind: StackView['kind'], n: number): StackView => ({ type: 'stack', label: 's', kind, items: Array.from({ length: n }, (_, i) => i) })
  const grid = (rows: number, cols: number): GridView => ({ type: 'grid', label: 'g', data: Array.from({ length: rows }, () => Array.from({ length: cols }, () => 0)) })
  const bits: BitsView = { type: 'bits', label: 'b', rows: [{ label: 'x', value: 5, width: 4 }, { label: 'y', value: 3, width: 4 }] }
  const text: TextView = { type: 'text', label: 't', lines: ['a', 'b', 'c'] }
  const perfect = tree([[0, 1, 2], [1, 3, 4], [2, 5, 6], [3, null, null], [4, null, null], [5, null, null], [6, null, null]])

  it('match view_height for every kind', () => {
    const cases: [VizView, number, number][] = [
      [array(), 600, 88],
      [array({ pointers: [['i', 2], ['j', 2]] }), 600, 118],
      [array({ bars: true }), 600, 194],
      [kv(0), 600, 56],
      [kv(12), 600, 116],
      [kv(12), 300, 206],
      [stack('stack', 3), 600, 134],
      [stack('heap', 0), 600, 74],
      [stack('queue', 3), 600, 90],
      [stack('deque', 9), 600, 90],
      [bits, 600, 86],
      [text, 600, 80],
      [{ type: 'list', label: 'l', nodes: [] }, 600, 108],
      [perfect, 600, 224],
      [grid(3, 3), 600, 169],
      [grid(40, 3), 600, 708],
      [{ type: 'graph', label: 'g', nodes: [], edges: [] }, 600, 270],
    ]
    for (const [view, width, h] of cases) expect(viewHeight(view, width, theme), `${view.type} @ ${width}`).toBe(h)
  })

  it('stay usable for every kind, empty or not (the Rust sanity check)', () => {
    const views: VizView[] = [
      array({ data: [1] }),
      kv(0),
      stack('stack', 0),
      { type: 'list', label: '', nodes: [] },
      { type: 'tree', label: '', nodes: [], root: null },
      grid(3, 3),
      { type: 'graph', label: '', nodes: [], edges: [] },
      { type: 'bits', label: '', rows: [] },
      { type: 'text', label: '', lines: [] },
    ]
    for (const v of views) {
      const h = viewHeight(v, 600, theme)
      expect(h > 10 && h < 700, `${v.type} -> ${h}`).toBe(true)
    }
  })

  it('grow with content', () => {
    expect(kvHeight(kv(12), 300, theme)).toBeGreaterThan(kvHeight(kv(0), 300, theme))
    expect(arrayHeight(array({ bars: true }), theme)).toBeGreaterThan(arrayHeight(array(), theme))
  })

  it('shrink grid cells for large grids, never below 14', () => {
    const small = gridCellSize(3, 3, 600, theme)
    const wide = gridCellSize(3, 40, 600, theme)
    const tall = gridCellSize(40, 3, 600, theme)
    expect(wide).toBeLessThan(small)
    expect(tall).toBeLessThan(small)
    expect(Math.min(wide, tall)).toBeGreaterThanOrEqual(14)
  })
})

describe('cells', () => {
  it('shrink to fit but never grow', () => {
    expect(fitCell(3, 1000, theme)).toBe(theme.cellSize)
    const tight = fitCell(40, 400, theme)
    expect(tight).toBeLessThan(theme.cellSize)
    expect(tight).toBeGreaterThanOrEqual(10)
    expect(fitCell(0, 100, theme)).toBe(theme.cellSize)
  })

  it('elide long values by character, not by UTF-16 unit', () => {
    expect(shortCellText(12)).toBe('12')
    expect(shortCellText(2.5)).toBe('2.5')
    expect(shortCellText('abcdef')).toBe('abcdef')
    expect(shortCellText('abcdefgh')).toBe('abcde…')
    expect(shortCellText('😀😀😀😀😀😀')).toBe('😀😀😀😀😀😀')
    expect(shortCellText('😀😀😀😀😀😀😀')).toBe('😀😀😀😀😀…')
  })

  it('read numbers out of strings the way Rust parses f64', () => {
    expect(cellAsF64(3)).toBe(3)
    expect(cellAsF64('3')).toBe(3)
    expect(cellAsF64('-2.5e1')).toBe(-25)
    expect(cellAsF64('.5')).toBe(0.5)
    expect(cellAsF64('inf')).toBe(Infinity)
    expect(cellAsF64('NaN')).toBeNaN()
    for (const s of ['', ' 3', '0x10', '1e', 'x']) expect(cellAsF64(s), JSON.stringify(s)).toBeNull()
  })

  it('keep every state distinguishable from plain', () => {
    const states: CellState[] = ['good', 'bad', 'cur', 'window', 'done']
    for (const s of states) expect(stateFill(s, theme), s).not.toEqual(stateFill('plain', theme))
  })

  it('put rejection first in the array state priority', () => {
    const view = (hl: number[], bad: number[], done: number[], window?: [number, number]): ArrayView => ({
      type: 'array',
      label: 'a',
      data: [1, 1, 1, 1, 1],
      hl,
      bad,
      done,
      window,
    })
    expect(arrayCellState(2, view([2], [2], [2], [0, 4]))).toBe('bad')
    expect(arrayCellState(2, view([2], [], [2], [0, 4]))).toBe('good')
    expect(arrayCellState(2, view([], [], [2], [0, 4]))).toBe('done')
    expect(arrayCellState(2, view([], [], [], [1, 3]))).toBe('window')
    expect(arrayCellState(0, view([], [], [], [1, 3]))).toBe('plain')
  })

  it('stack pointer lanes only when pointers collide', () => {
    expect(pointerLanes([['i', 0], ['j', 3]])).toBe(1)
    expect(pointerLanes([['i', 2], ['j', 2]])).toBe(2)
    expect(pointerLanes([['i', 2], ['j', 2], ['k', 2]])).toBe(3)
    expect(pointerLanes([])).toBe(0)
  })
})
