import { act, render, screen } from '@testing-library/react'
import type { Trace, VizView } from '@/trace/types'
import { describeView } from './a11y'
import { viewHeight } from './render'
import { installCanvasMock, type CanvasMock } from './testing/recordingContext'
import { DARK_THEME } from './theme'
import { VizCanvas } from './VizCanvas'

const fixtures = import.meta.glob<Trace>('./fixtures/*.json', { eager: true, import: 'default' })

/** Width every observed element reports; tests change it to exercise narrow layouts. */
let observedWidth = 600

class FakeResizeObserver implements ResizeObserver {
  constructor(private readonly callback: ResizeObserverCallback) {}
  observe(): void {
    const entry = { contentRect: { width: observedWidth } } as unknown as ResizeObserverEntry
    this.callback([entry], this)
  }
  unobserve(): void {}
  disconnect(): void {}
}

let canvas: CanvasMock

beforeAll(() => {
  vi.stubGlobal('ResizeObserver', FakeResizeObserver)
  canvas = installCanvasMock()
})

afterAll(() => {
  canvas.restore()
  vi.unstubAllGlobals()
})

afterEach(() => {
  observedWidth = 600
  delete document.documentElement.dataset.theme
})

function canvases(container: HTMLElement): HTMLCanvasElement[] {
  return [...container.querySelectorAll('canvas')]
}

describe('every fixture, every step', () => {
  const entries = Object.entries(fixtures)

  it('has fixtures to render', () => {
    expect(entries.length).toBeGreaterThanOrEqual(30)
  })

  it.each(entries.map(([path, trace]) => [path.replace('./fixtures/', ''), trace] as const))(
    '%s paints each view at t = 0, 0.5 and 1 without remounting',
    (name, trace) => {
      const { container, rerender } = render(<VizCanvas views={trace.steps[0].views} t={1} />)
      trace.steps.forEach((step, i) => {
        const prev = i > 0 ? trace.steps[i - 1].views : null
        let mounted: HTMLCanvasElement[] | null = null
        for (const t of [0, 0.5, 1]) {
          rerender(<VizCanvas views={step.views} prev={prev} t={t} />)
          const now = canvases(container)
          expect(now).toHaveLength(step.views.length)
          if (mounted) now.forEach((c, k) => expect(c, `${name} #${i}: canvas ${k} remounted`).toBe(mounted![k]))
          mounted = now
          now.forEach((c, k) => {
            const ctx = canvas.contextOf(c)
            const where = `${name} step ${i} view ${k} (${step.views[k].type}) t=${t}`
            expect(ctx, where).toBeDefined()
            expect(ctx!.paints, where).toBeGreaterThan(0)
            expect(ctx!.nonFinite, where).toEqual([])
          })
        }
      })
    },
  )

  it.each(entries.map(([path, trace]) => [path.replace('./fixtures/', ''), trace] as const))(
    '%s also paints at a narrow 280pt width',
    (_name, trace) => {
      observedWidth = 280
      const { container, rerender } = render(<VizCanvas views={trace.steps[0].views} t={1} />)
      trace.steps.forEach((step, i) => {
        rerender(<VizCanvas views={step.views} prev={i > 0 ? trace.steps[i - 1].views : null} t={0.5} />)
        for (const c of canvases(container)) expect(canvas.contextOf(c)!.nonFinite).toEqual([])
      })
    },
  )
})

describe('VizCanvas', () => {
  const twoSum = fixtures['./fixtures/two-sum.json']
  const step = twoSum.steps[3]

  it('says so when a step draws no picture', () => {
    render(<VizCanvas views={[]} t={1} />)
    expect(screen.getByText('This step draws no picture.')).toBeInTheDocument()
  })

  it('sizes each canvas to its view and labels it for assistive technology', () => {
    const { container } = render(<VizCanvas views={step.views} t={1} />)
    const [nums, seen] = canvases(container)
    expect(nums).toHaveAttribute('role', 'img')
    expect(nums).toHaveAccessibleName('nums (target = 14): [2, 7, 11, 15, 3]; i → 0')
    expect(seen).toHaveAccessibleName('seen — value → index: empty')
    expect(nums.style.width).toBe('600px')
    expect(nums.style.height).toBe(`${viewHeight(step.views[0], 600, DARK_THEME)}px`)
    expect([nums.width, nums.height]).toEqual([600, 103])
  })

  it('scales the backing store by devicePixelRatio', () => {
    const original = window.devicePixelRatio
    Object.defineProperty(window, 'devicePixelRatio', { configurable: true, value: 2 })
    try {
      const { container } = render(<VizCanvas views={step.views} t={1} />)
      const [nums] = canvases(container)
      expect([nums.width, nums.height]).toEqual([1200, 206])
      expect(canvas.contextOf(nums)!.transform).toEqual([2, 0, 0, 2, 0, 0])
    } finally {
      Object.defineProperty(window, 'devicePixelRatio', { configurable: true, value: original })
    }
  })

  it('repaints only when something it draws from changes', () => {
    const { container, rerender } = render(<VizCanvas views={step.views} t={1} />)
    const [nums] = canvases(container)
    const ctx = canvas.contextOf(nums)!
    const frames = ctx.frames
    rerender(<VizCanvas views={step.views} t={1} className="x" />)
    expect(ctx.frames).toBe(frames)
    rerender(<VizCanvas views={step.views} prev={twoSum.steps[2].views} t={0.5} className="x" />)
    expect(ctx.frames).toBe(frames + 1)
  })

  it('repaints in the other palette when data-theme changes', async () => {
    const { container } = render(<VizCanvas views={step.views} t={1} />)
    const [nums] = canvases(container)
    const ctx = canvas.contextOf(nums)!
    expect(ctx.colors).toContain('rgb(22, 27, 40)') // dark cell
    await act(async () => {
      document.documentElement.dataset.theme = 'light'
    })
    expect(ctx.colors).toContain('rgb(237, 240, 246)') // light cell
    expect(ctx.colors).not.toContain('rgb(22, 27, 40)')
    await act(async () => {
      delete document.documentElement.dataset.theme
    })
    expect(ctx.colors).toContain('rgb(22, 27, 40)')
  })

  it('describes every kind of view', () => {
    const views: VizView[] = [
      { type: 'stack', label: 'st', kind: 'stack', items: [1, 2], pushed: true },
      { type: 'list', label: 'l', nodes: [{ id: 5, val: 'a', next: null }], pointers: [['cur', 5], ['prev', null]] },
      { type: 'tree', label: 't', nodes: [{ id: 0, val: 2, left: 1, right: null }, { id: 1, val: 1, left: null, right: null }], root: 0, cur: 1 },
      { type: 'grid', label: 'g', data: [[1, 0], [0, 1]], cur: [1, 0] },
      { type: 'graph', label: 'G', nodes: [{ id: 0, label: 'A' }, { id: 1, label: 'B' }], edges: [{ from: 0, to: 1 }], active: [[0, 1]] },
      { type: 'bits', label: 'b', rows: [{ label: 'x', value: 5, width: 4 }] },
      { type: 'text', label: 'why', lines: ['a + b', 'c'] },
    ]
    expect(views.map(describeView)).toEqual([
      'st (stack): [1, 2]; pushed 2',
      'l: list [a]; cur → a, prev → nil',
      't: tree, root 2, in order [1, 2]; current 1',
      'g: 2×2 grid; rows 1 0 / 0 1; cursor row 1 column 0',
      'G: graph, 2 nodes, 1 edges; traversing A → B',
      'b: x = 0101 (5)',
      'why: a + b / c',
    ])
  })
})
