// The canvas side of egui's `Painter`: the shapes dsa-viz paints, drawn with
// the geometry rules epaint 0.33's tessellator applies, so an outline lands on
// the same pixel it does on the desktop.
//
// What is carried over, and why it matters:
//  * Rects snap to physical pixels before anything else (epaint's
//    `round_rects_to_pixels`), which keeps 1-point cell borders crisp at any
//    devicePixelRatio.
//  * Rect strokes sit Inside or Outside the rect, never centred on its edge,
//    and a rounded corner gets epaint's +0.4 radius so small radii read round.
//  * Circle strokes sit outside the radius (epaint's `circle_stroke`).
//  * Axis-aligned line segments snap to a pixel centre when their width is an
//    odd number of pixels (`round_line_segments_to_pixels`).
//  * Text follows egui's layout model: one row per line, row height = ascent
//    + descent + line gap of the font, the row box anchored by `Align2`, the
//    baseline `ascent` below the row top, all rounded to physical pixels. The
//    metrics are those of the fonts egui ships (Hack, Ubuntu Light); the
//    glyphs come from the web fonts, whose cap heights match, so a label sits
//    where the desktop puts it even though the typeface differs.

import { cssColor, type Color } from './color'
import { expand, rectHeight, rectWidth, type Pos2, type Rect } from './geom'
import { roundHalfAway } from './num'
import type { VizFonts } from './theme'

export type FontFamily = 'monospace' | 'proportional'

/** egui `FontId`: a size in points (the em, as in CSS) and a family. */
export interface FontId {
  readonly size: number
  readonly family: FontFamily
}

export function monospace(size: number): FontId {
  return { size, family: 'monospace' }
}

export function proportional(size: number): FontId {
  return { size, family: 'proportional' }
}

/** Where the anchor sits on the text box: 0 = left/top, 0.5 = centre, 1 = right/bottom. */
export interface Align2 {
  readonly x: 0 | 0.5 | 1
  readonly y: 0 | 0.5 | 1
}

export const LEFT_TOP: Align2 = { x: 0, y: 0 }
export const LEFT_CENTER: Align2 = { x: 0, y: 0.5 }
export const CENTER_CENTER: Align2 = { x: 0.5, y: 0.5 }
export const RIGHT_CENTER: Align2 = { x: 1, y: 0.5 }

export interface Stroke {
  readonly width: number
  readonly color: Color
}

export function stroke(width: number, color: Color): Stroke {
  return { width, color }
}

/** Where a rect's outline goes relative to the rect (egui `StrokeKind`; dsa-viz never centres one). */
export type StrokeKind = 'inside' | 'outside'

/** The slice of `CanvasRenderingContext2D` the painter uses — small enough to fake in tests. */
export interface Canvas2D {
  fillStyle: string | CanvasGradient | CanvasPattern
  strokeStyle: string | CanvasGradient | CanvasPattern
  lineWidth: number
  lineCap: CanvasLineCap
  lineJoin: CanvasLineJoin
  font: string
  textAlign: CanvasTextAlign
  textBaseline: CanvasTextBaseline
  setTransform(a: number, b: number, c: number, d: number, e: number, f: number): void
  clearRect(x: number, y: number, w: number, h: number): void
  beginPath(): void
  closePath(): void
  moveTo(x: number, y: number): void
  lineTo(x: number, y: number): void
  rect(x: number, y: number, w: number, h: number): void
  arc(x: number, y: number, radius: number, startAngle: number, endAngle: number): void
  arcTo(x1: number, y1: number, x2: number, y2: number, radius: number): void
  fill(): void
  stroke(): void
  fillText(text: string, x: number, y: number): void
  measureText(text: string): { readonly width: number }
}

interface FontMetrics {
  readonly ascent: number
  readonly descent: number
  readonly lineGap: number
}

/**
 * Vertical metrics of the fonts egui ships, as fractions of the em, from each
 * font's `hhea` table (what ab_glyph reports and egui lays text out with).
 */
const METRICS: Readonly<Record<FontFamily, FontMetrics>> = {
  // Hack Regular: ascender 1901, descender -483, line gap 0, 2048 units per em.
  monospace: { ascent: 1901 / 2048, descent: 483 / 2048, lineGap: 0 },
  // Ubuntu Light: ascender 932, descender -189, line gap 28, 1000 units per em.
  proportional: { ascent: 932 / 1000, descent: 189 / 1000, lineGap: 28 / 1000 },
}

/** egui's proportional face is Ubuntu *Light*; its monospace face is a regular weight. */
const WEIGHT: Readonly<Record<FontFamily, number>> = { monospace: 400, proportional: 300 }

/** emath's `GUI_ROUNDING`: font metrics are kept on a 1/32-point grid. */
const GUI_ROUNDING = 1 / 32

function roundUi(v: number): number {
  return roundHalfAway(v / GUI_ROUNDING) * GUI_ROUNDING
}

export class Painter {
  private font = ''
  private fillCss = ''
  private strokeCss = ''
  private lineWidth = Number.NaN
  private align: CanvasTextAlign | null = null

  /**
   * @param ctx a context whose transform already maps points to device pixels
   * @param pixelsPerPoint the scale of that transform — what "one pixel" means when snapping
   * @param fonts the CSS font stacks standing in for egui's two families
   */
  constructor(
    private readonly ctx: Canvas2D,
    private readonly pixelsPerPoint: number,
    private readonly fonts: VizFonts,
  ) {
    ctx.lineCap = 'butt'
    ctx.lineJoin = 'miter'
    ctx.textBaseline = 'alphabetic'
  }

  rectFilled(rect: Rect, cornerRadius: number, fill: Color): void {
    if (fill[3] === 0) return
    const x0 = this.snap(rect.min.x)
    const y0 = this.snap(rect.min.y)
    const x1 = this.snap(rect.max.x)
    const y1 = this.snap(rect.max.y)
    if (!(x1 > x0 && y1 > y0)) return
    this.setFill(fill)
    this.roundedRect(x0, y0, x1, y1, cornerRadius > 0 ? Math.max(cornerRadius + 0.4, 0.1) : 0)
    this.ctx.fill()
  }

  rectStroke(rect: Rect, cornerRadius: number, s: Stroke, kind: StrokeKind): void {
    const w = s.width
    if (!(w > 0) || s.color[3] === 0) return
    const grow = kind === 'outside' ? w : 0
    // A stroke that would cover its whole rect is drawn as a fill in the
    // stroke colour, as epaint does, rather than as a self-overlapping ring.
    if (Math.min(rectWidth(rect), rectHeight(rect)) + 2 * grow <= 2 * w + 0.5 / this.pixelsPerPoint) {
      this.rectFilled(grow > 0 ? expand(rect, grow) : rect, cornerRadius, s.color)
      return
    }
    // Snap first, then apply the stroke kind: that keeps the outer edge of an
    // Inside stroke (and the inner edge of an Outside one) on a pixel boundary.
    const x0 = this.snap(rect.min.x) - grow
    const y0 = this.snap(rect.min.y) - grow
    const x1 = this.snap(rect.max.x) + grow
    const y1 = this.snap(rect.max.y) + grow
    const outer = cornerRadius > 0 ? Math.max(cornerRadius + grow + 0.4, w + 0.1) : 0
    const clamped = Math.min(outer, (x1 - x0) / 2, (y1 - y0) / 2)
    const half = w / 2
    this.setStroke(s)
    this.roundedRect(x0 + half, y0 + half, x1 - half, y1 - half, clamped - half)
    this.ctx.stroke()
  }

  circleFilled(center: Pos2, radius: number, fill: Color): void {
    if (!(radius > 0) || fill[3] === 0) return
    this.setFill(fill)
    this.ctx.beginPath()
    this.ctx.arc(center.x, center.y, radius, 0, Math.PI * 2)
    this.ctx.fill()
  }

  /** The stroke sits outside `radius`, so a stroked disc keeps its full fill. */
  circleStroke(center: Pos2, radius: number, s: Stroke): void {
    if (!(radius > 0) || !(s.width > 0) || s.color[3] === 0) return
    this.setStroke(s)
    this.ctx.beginPath()
    this.ctx.arc(center.x, center.y, radius + s.width / 2, 0, Math.PI * 2)
    this.ctx.stroke()
  }

  lineSegment(a: Pos2, b: Pos2, s: Stroke): void {
    if (!(s.width > 0) || s.color[3] === 0) return
    let { x: ax, y: ay } = a
    let { x: bx, y: by } = b
    // The ends are pulled in a quarter pixel before rounding so a segment that
    // ends exactly on a pixel boundary does not spill into the next pixel.
    const quarter = 0.25 / this.pixelsPerPoint
    if (ax === bx) {
      ax = bx = this.strokeCenter(ax, s.width)
      ;[ay, by] =
        ay < by ? [this.snap(ay + quarter), this.snap(by - quarter)] : [this.snap(ay - quarter), this.snap(by + quarter)]
    }
    if (ay === by) {
      ay = by = this.strokeCenter(ay, s.width)
      ;[ax, bx] =
        ax < bx ? [this.snap(ax + quarter), this.snap(bx - quarter)] : [this.snap(ax - quarter), this.snap(bx + quarter)]
    }
    this.setStroke(s)
    this.ctx.beginPath()
    this.ctx.moveTo(ax, ay)
    this.ctx.lineTo(bx, by)
    this.ctx.stroke()
  }

  /** An open polyline (egui `Shape::line`). */
  line(points: readonly Pos2[], s: Stroke): void {
    if (points.length < 2 || !(s.width > 0) || s.color[3] === 0) return
    this.setStroke(s)
    this.path(points)
    this.ctx.stroke()
  }

  convexPolygon(points: readonly Pos2[], fill: Color): void {
    if (points.length < 3 || fill[3] === 0) return
    this.setFill(fill)
    this.path(points)
    this.ctx.closePath()
    this.ctx.fill()
  }

  /**
   * egui `Painter::text`: lay the text out as a galley, anchor the galley's
   * box at `pos` by `align`, draw. Rows split on `\n`; a multi-row galley is
   * anchored as a block with its rows left-aligned inside it.
   */
  text(pos: Pos2, align: Align2, text: string, font: FontId, color: Color): void {
    if (text === '' || color[3] === 0) return
    const m = METRICS[font.family]
    const ascent = roundUi(m.ascent * font.size)
    const rowHeight = ascent + roundUi(m.descent * font.size) + roundUi(m.lineGap * font.size)
    const rows = text.split('\n')
    const top = this.snap(pos.y - align.y * rowHeight * rows.length)
    this.setFont(font)
    this.setFill(color)
    if (rows.length === 1) {
      this.setAlign(align.x === 0 ? 'left' : align.x === 1 ? 'right' : 'center')
      this.ctx.fillText(text, this.snap(pos.x), top + this.snap(ascent))
      return
    }
    const width = Math.max(...rows.map((row) => this.ctx.measureText(row).width))
    const left = this.snap(pos.x - align.x * width)
    this.setAlign('left')
    rows.forEach((row, i) => this.ctx.fillText(row, left, top + this.snap(i * rowHeight + ascent)))
  }

  /** Nearest physical pixel boundary (egui `round_to_pixels`). */
  private snap(v: number): number {
    return roundHalfAway(v * this.pixelsPerPoint) / this.pixelsPerPoint
  }

  /**
   * Where the centre line of a stroke of `width` goes: on a pixel centre when
   * the stroke covers an odd number of pixels (or less than one), else on a
   * boundary — epaint's `Stroke::round_center_to_pixel`.
   */
  private strokeCenter(v: number, width: number): number {
    const ppp = this.pixelsPerPoint
    const odd = ((width * ppp * 0.5 + 0.25) % 1) > 0.5
    if (width <= 1 / ppp || odd) return (roundHalfAway(v * ppp - 0.5) + 0.5) / ppp
    return this.snap(v)
  }

  private roundedRect(x0: number, y0: number, x1: number, y1: number, radius: number): void {
    const { ctx } = this
    const r = Math.max(0, Math.min(radius, (x1 - x0) / 2, (y1 - y0) / 2))
    ctx.beginPath()
    if (!(r > 0)) {
      ctx.rect(x0, y0, x1 - x0, y1 - y0)
      return
    }
    ctx.moveTo(x0 + r, y0)
    ctx.arcTo(x1, y0, x1, y1, r)
    ctx.arcTo(x1, y1, x0, y1, r)
    ctx.arcTo(x0, y1, x0, y0, r)
    ctx.arcTo(x0, y0, x1, y0, r)
    ctx.closePath()
  }

  private path(points: readonly Pos2[]): void {
    const { ctx } = this
    ctx.beginPath()
    ctx.moveTo(points[0].x, points[0].y)
    for (let i = 1; i < points.length; i++) ctx.lineTo(points[i].x, points[i].y)
  }

  private setFill(c: Color): void {
    const css = cssColor(c)
    if (css !== this.fillCss) {
      this.ctx.fillStyle = css
      this.fillCss = css
    }
  }

  private setStroke(s: Stroke): void {
    const css = cssColor(s.color)
    if (css !== this.strokeCss) {
      this.ctx.strokeStyle = css
      this.strokeCss = css
    }
    if (s.width !== this.lineWidth) {
      this.ctx.lineWidth = s.width
      this.lineWidth = s.width
    }
  }

  private setFont(f: FontId): void {
    const family = f.family === 'monospace' ? this.fonts.mono : this.fonts.sans
    const css = `${WEIGHT[f.family]} ${f.size}px ${family}`
    if (css !== this.font) {
      this.ctx.font = css
      this.font = css
    }
  }

  private setAlign(a: CanvasTextAlign): void {
    if (a !== this.align) {
      this.ctx.textAlign = a
      this.align = a
    }
  }
}
