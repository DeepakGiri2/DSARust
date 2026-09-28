// The animated visualization — `dsa_viz::show_views` for the browser.
//
// Each view of the current step gets its own <canvas>, as wide as the
// container and as tall as its renderer asks for (`viewHeight`), stacked top
// to bottom. During a transition the parent re-renders with a new `t` every
// animation frame, so canvases are keyed by the view's animation identity
// (kind + label + occurrence) and never remount mid-transition, and each one
// repaints — in a layout effect, before the browser paints — only when
// something it draws from changed: its view or partner, `t`, the width, the
// device pixel ratio, the theme or the fonts.

import clsx from 'clsx'
import { memo, useLayoutEffect, useMemo, useRef } from 'react'
import type { VizView } from '@/trace/types'
import { describeView } from './a11y'
import { animKey, pair } from './anim'
import { rectFromMinSize, vec2, ZERO } from './geom'
import { useContentWidth, useDevicePixelRatio, useFontGeneration, useVizTheme } from './hooks'
import { Painter } from './painter'
import { VIEW_SPACING, drawView, viewHeight } from './render'
import type { VizTheme } from './theme'
import styles from './VizCanvas.module.css'

export interface VizCanvasProps {
  /** Views of the current step, drawn top to bottom. */
  views: VizView[]
  /**
   * Views of the step being animated *from*. Each current view tweens from the
   * previous view of the same kind and label (dsa-viz `pair`). Null/undefined
   * when there is nothing to tween from.
   */
  prev?: VizView[] | null
  /** Eased transition progress: 0 shows `prev`, 1 the settled `views`. */
  t: number
  className?: string
}

export function VizCanvas({ views, prev, t, className }: VizCanvasProps) {
  const theme = useVizTheme()
  const fonts = useFontGeneration()
  const dpr = useDevicePixelRatio()
  const [ref, containerWidth] = useContentWidth<HTMLDivElement>()
  // Whole device pixels, so each canvas's backing store maps 1:1 onto the screen.
  const width = Math.floor(containerWidth * dpr) / dpr
  const pairs = useMemo(() => pair(prev, views), [prev, views])
  const keys = useMemo(() => canvasKeys(views), [views])

  return (
    <div ref={ref} className={clsx(styles.root, className)} style={{ gap: VIEW_SPACING }}>
      {pairs.length === 0 ? (
        <p className={styles.empty}>This step draws no picture.</p>
      ) : (
        pairs.map(({ view, before }, i) => (
          <ViewCanvas
            key={keys[i]}
            view={view}
            before={before}
            t={t}
            width={width}
            dpr={dpr}
            theme={theme}
            fonts={fonts}
          />
        ))
      )}
    </div>
  )
}

/** `kind:label#n` — the n-th view with that animation key keeps its canvas from step to step. */
function canvasKeys(views: readonly VizView[]): string[] {
  const seen = new Map<string, number>()
  return views.map((view) => {
    const key = animKey(view)
    const n = seen.get(key) ?? 0
    seen.set(key, n + 1)
    return `${key}#${n}`
  })
}

interface ViewCanvasProps {
  view: VizView
  before: VizView | null
  t: number
  /** Drawing width in points, already a whole number of device pixels. */
  width: number
  dpr: number
  theme: VizTheme
  /** Font generation; a change means text may now render in its real face. */
  fonts: number
}

const ViewCanvas = memo(function ViewCanvas({ view, before, t, width, dpr, theme, fonts }: ViewCanvasProps) {
  const ref = useRef<HTMLCanvasElement>(null)
  const height = viewHeight(view, width, theme)
  // Free text has no motion; pinning its t keeps a transition from repainting it every frame.
  const drawT = view.type === 'text' ? 1 : t
  const label = useMemo(() => describeView(view), [view])

  useLayoutEffect(() => {
    const canvas = ref.current
    if (canvas) paint(canvas, view, before, drawT, width, height, dpr, theme)
  }, [view, before, drawT, width, height, dpr, theme, fonts])

  return (
    <canvas
      ref={ref}
      role="img"
      aria-label={label}
      className={styles.view}
      style={{ width, height: Math.round(height * dpr) / dpr }}
    />
  )
})

function paint(
  canvas: HTMLCanvasElement,
  view: VizView,
  before: VizView | null,
  t: number,
  width: number,
  height: number,
  dpr: number,
  theme: VizTheme,
): void {
  const pw = Math.round(width * dpr)
  const ph = Math.round(height * dpr)
  // Assigning a canvas dimension reallocates and clears it, even to the same value.
  if (canvas.width !== pw) canvas.width = pw
  if (canvas.height !== ph) canvas.height = ph
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx.clearRect(0, 0, width, height)
  if (pw === 0 || ph === 0) return
  drawView(new Painter(ctx, dpr, theme.fonts), rectFromMinSize(ZERO, vec2(width, height)), view, before, t, theme)
}
