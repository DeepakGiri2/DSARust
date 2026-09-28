import { rgb } from './color'
import { pos2, rectFromMinMax } from './geom'
import { createFontStore, type FontSource } from './hooks'
import { CENTER_CENTER, LEFT_TOP, Painter, monospace, proportional, stroke } from './painter'
import { RecordingContext } from './testing/recordingContext'
import { DARK_THEME } from './theme'

const INK = rgb(0xd6dbe8)

function painter(ppp = 1): [Painter, RecordingContext] {
  const ctx = new RecordingContext()
  return [new Painter(ctx, ppp, DARK_THEME.fonts), ctx]
}

const last = (ctx: RecordingContext, op: string) => ctx.frame.filter((c) => c.op === op).at(-1)?.args

describe('egui text layout', () => {
  it('centres a galley by its row box and sits the baseline one ascent below the top', () => {
    const [p, ctx] = painter()
    // Hack at 17pt: ascent 15.78, row height 19.78 → top 40.1 → 40 px, baseline 40 + 16.
    p.text(pos2(100, 50), CENTER_CENTER, '7', monospace(17), INK)
    expect(last(ctx, 'fillText')).toEqual(['rgb(214, 219, 232)', '7', 100, 56, `400 17px ${DARK_THEME.fonts.mono}`, 'center'])
  })

  it('draws proportional captions in Ubuntu Light from their top-left corner', () => {
    const [p, ctx] = painter(2)
    p.text(pos2(0, 0), LEFT_TOP, 'nums', proportional(12), INK)
    expect(last(ctx, 'fillText')).toEqual(['rgb(214, 219, 232)', 'nums', 0, 11, `300 12px ${DARK_THEME.fonts.sans}`, 'left'])
  })

  it('anchors a multi-row galley as a block', () => {
    const [p, ctx] = painter()
    p.text(pos2(0, 0), LEFT_TOP, 'a\nb', monospace(12), INK)
    const rows = ctx.frame.filter((c) => c.op === 'fillText').map((c) => c.args[3])
    expect(rows).toHaveLength(2)
    expect(Number(rows[1]) - Number(rows[0])).toBe(14)
  })
})

describe('epaint geometry', () => {
  it('keeps an inside stroke inside the pixel-snapped rect, with the +0.4 corner tweak', () => {
    const [p, ctx] = painter()
    p.rectStroke(rectFromMinMax(pos2(10.2, 10), pos2(54, 54)), 6, stroke(1, INK), 'inside')
    const move = ctx.frame.find((c) => c.op === 'moveTo')?.args
    expect(move?.[0]).toBeCloseTo(16.4, 6)
    expect(move?.[1]).toBe(10.5)
    expect(last(ctx, 'stroke')).toEqual(['rgb(214, 219, 232)', 1])
  })

  it('puts an outside stroke around the rect and grows its radius', () => {
    const [p, ctx] = painter()
    p.rectStroke(rectFromMinMax(pos2(0, 0), pos2(44, 44)), 6, stroke(2, INK), 'outside')
    const move = ctx.frame.find((c) => c.op === 'moveTo')?.args
    expect(move?.[0]).toBeCloseTo(6.4, 6)
    expect(move?.[1]).toBe(-1)
  })

  it('strokes circles outside their radius', () => {
    const [p, ctx] = painter()
    p.circleStroke(pos2(50, 50), 16, stroke(1.2, INK))
    expect(last(ctx, 'arc')?.[2]).toBeCloseTo(16.6, 6)
  })

  it('snaps an odd-width horizontal segment to a pixel centre', () => {
    const [p, ctx] = painter()
    p.lineSegment(pos2(0, 64), pos2(100, 64), stroke(1.4, INK))
    expect(last(ctx, 'moveTo')).toEqual([0, 64.5])
    expect(last(ctx, 'lineTo')).toEqual([100, 64.5])
  })

  it('skips fully transparent shapes and degenerate circles', () => {
    const [p, ctx] = painter()
    p.rectFilled(rectFromMinMax(pos2(0, 0), pos2(10, 10)), 0, [1, 2, 3, 0])
    p.circleFilled(pos2(0, 0), 0, INK)
    p.text(pos2(0, 0), LEFT_TOP, '', monospace(12), INK)
    expect(ctx.paints).toBe(0)
  })
})

describe('font store', () => {
  function fakeFonts() {
    const loadingDone: (() => void)[] = []
    let resolveReady: () => void = () => {}
    const source: FontSource & { loads: string[] } = {
      loads: [],
      load(font) {
        source.loads.push(font)
        return Promise.resolve([])
      },
      ready: new Promise<void>((r) => (resolveReady = r)),
      addEventListener: (_type, listener) => loadingDone.push(listener),
    }
    return { source, loadingDone, resolveReady: () => resolveReady() }
  }

  it('requests both faces on first subscription and ticks when they are ready', async () => {
    const fonts = fakeFonts()
    const store = createFontStore(() => fonts.source, () => ['400 12px Mono', '300 12px Sans'])
    const listener = vi.fn()
    store.subscribe(listener)
    store.subscribe(() => {})
    expect(fonts.source.loads).toEqual(['400 12px Mono', '300 12px Sans'])
    expect(store.getSnapshot()).toBe(0)
    fonts.resolveReady()
    await vi.waitFor(() => expect(store.getSnapshot()).toBe(1))
    expect(listener).toHaveBeenCalledTimes(1)
    fonts.loadingDone.forEach((l) => l())
    expect(store.getSnapshot()).toBe(2)
  })

  it('stays at zero without a FontFaceSet (jsdom, workers)', () => {
    const store = createFontStore(() => undefined, () => [])
    store.subscribe(() => {})
    expect(store.getSnapshot()).toBe(0)
  })
})
