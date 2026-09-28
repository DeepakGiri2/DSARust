// A recording stand-in for CanvasRenderingContext2D, for tests (jsdom has no
// canvas). It implements exactly the surface `Painter` uses and logs every
// call. It throws where a real context throws (a negative arc radius), and it
// collects calls with NaN or infinite coordinates — a browser silently drops
// those, so a renderer that produces them is broken even though nothing
// crashes.

import type { Canvas2D } from '../painter'

export interface DrawCall {
  readonly op: string
  readonly args: readonly unknown[]
}

const PAINT_OPS = new Set(['fill', 'stroke', 'fillText'])

export class RecordingContext implements Canvas2D {
  fillStyle: string | CanvasGradient | CanvasPattern = '#000000'
  strokeStyle: string | CanvasGradient | CanvasPattern = '#000000'
  lineWidth = 1
  lineCap: CanvasLineCap = 'butt'
  lineJoin: CanvasLineJoin = 'miter'
  font = '10px sans-serif'
  textAlign: CanvasTextAlign = 'start'
  textBaseline: CanvasTextBaseline = 'alphabetic'

  /** Calls since the last `clearRect` — the frame currently on the canvas. */
  frame: DrawCall[] = []
  /** Every call ever made with a non-finite number among its arguments. */
  readonly nonFinite: DrawCall[] = []
  /** Number of frames started (`clearRect` calls). */
  frames = 0
  /** The last transform set. */
  transform: readonly number[] = [1, 0, 0, 1, 0, 0]

  /** Fill, stroke and text calls in the current frame. */
  get paints(): number {
    return this.frame.filter((c) => PAINT_OPS.has(c.op)).length
  }

  /** The style strings the current frame painted with. */
  get colors(): Set<string> {
    const out = new Set<string>()
    for (const c of this.frame) if (PAINT_OPS.has(c.op)) out.add(String(c.args[0]))
    return out
  }

  setTransform(a: number, b: number, c: number, d: number, e: number, f: number): void {
    this.transform = [a, b, c, d, e, f]
  }

  clearRect(x: number, y: number, w: number, h: number): void {
    this.frames += 1
    this.frame = []
    this.log('clearRect', [x, y, w, h])
  }

  beginPath(): void {
    this.log('beginPath', [])
  }

  closePath(): void {
    this.log('closePath', [])
  }

  moveTo(x: number, y: number): void {
    this.log('moveTo', [x, y])
  }

  lineTo(x: number, y: number): void {
    this.log('lineTo', [x, y])
  }

  rect(x: number, y: number, w: number, h: number): void {
    this.log('rect', [x, y, w, h])
  }

  arc(x: number, y: number, radius: number, startAngle: number, endAngle: number): void {
    if (radius < 0) throw new DOMException(`arc radius ${radius} is negative`, 'IndexSizeError')
    this.log('arc', [x, y, radius, startAngle, endAngle])
  }

  arcTo(x1: number, y1: number, x2: number, y2: number, radius: number): void {
    if (radius < 0) throw new DOMException(`arcTo radius ${radius} is negative`, 'IndexSizeError')
    this.log('arcTo', [x1, y1, x2, y2, radius])
  }

  fill(): void {
    this.log('fill', [this.fillStyle])
  }

  stroke(): void {
    this.log('stroke', [this.strokeStyle, this.lineWidth])
  }

  fillText(text: string, x: number, y: number): void {
    this.log('fillText', [this.fillStyle, text, x, y, this.font, this.textAlign])
  }

  /** Monospace-ish: 0.6 em per code point, enough for layout that measures. */
  measureText(text: string): { readonly width: number } {
    const size = Number(/(\d+(?:\.\d+)?)px/.exec(this.font)?.[1] ?? 10)
    return { width: [...text].length * size * 0.6 }
  }

  private log(op: string, args: readonly unknown[]): void {
    const call = { op, args }
    if (args.some((a) => typeof a === 'number' && !Number.isFinite(a))) this.nonFinite.push(call)
    this.frame.push(call)
  }
}

export interface CanvasMock {
  /** The context a canvas has handed out, if it has been asked for one. */
  contextOf(canvas: HTMLCanvasElement): RecordingContext | undefined
  restore(): void
}

/** Make every `<canvas>` hand out a `RecordingContext` for `'2d'`. */
export function installCanvasMock(): CanvasMock {
  const contexts = new WeakMap<HTMLCanvasElement, RecordingContext>()
  const original = Object.getOwnPropertyDescriptor(HTMLCanvasElement.prototype, 'getContext')
  Object.defineProperty(HTMLCanvasElement.prototype, 'getContext', {
    configurable: true,
    writable: true,
    value(this: HTMLCanvasElement, kind: string): RecordingContext | null {
      if (kind !== '2d') return null
      let ctx = contexts.get(this)
      if (!ctx) {
        ctx = new RecordingContext()
        contexts.set(this, ctx)
      }
      return ctx
    },
  })
  return {
    contextOf: (canvas) => contexts.get(canvas),
    restore: () => {
      if (original) Object.defineProperty(HTMLCanvasElement.prototype, 'getContext', original)
    },
  }
}
