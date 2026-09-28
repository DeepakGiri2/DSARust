// Dispatch — the `crates/dsa-viz/src/lib.rs` part of the port: how tall each
// view is at a given width, and which renderer draws it.
//
// Nothing here knows about problems or the debugger: it draws two snapshots
// and the motion between them, which is what turns a sequence of pictures into
// something you can actually watch an algorithm happen in.

import type { VizView } from '@/trace/types'
import type { Rect } from './geom'
import { arrayHeight, bitsHeight, drawArray, drawBits, drawKv, drawStack, drawText, kvHeight, stackHeight, textHeight } from './linear'
import { drawGraph, drawGrid, drawList, drawTree, graphHeight, gridHeight, listHeight, treeHeight } from './linked'
import type { Painter } from './painter'
import type { VizTheme } from './theme'

/**
 * Vertical gap between two stacked views, in points. `show_views` adds 10
 * after each view and egui's vertical layout adds the desktop's
 * `item_spacing.y` (6) between allocations, so the desktop shows 16 — measured
 * on its screenshots, not just read off the code.
 */
export const VIEW_SPACING = 16

/** `view_height`: the height a view needs at the given width. */
export function viewHeight(view: VizView, width: number, theme: VizTheme): number {
  switch (view.type) {
    case 'array':
      return arrayHeight(view, theme)
    case 'kv':
      return kvHeight(view, width, theme)
    case 'stack':
      return stackHeight(view, theme)
    case 'bits':
      return bitsHeight(view, theme)
    case 'text':
      return textHeight(view, theme)
    case 'list':
      return listHeight(theme)
    case 'tree':
      return treeHeight(view, theme)
    case 'grid':
      return gridHeight(view, width, theme)
    case 'graph':
      return graphHeight()
  }
}

/**
 * Draw one view into `rect`, tweening from `before` (its partner from the
 * previous step, see `pair`) at eased progress `t`: 0 shows the previous
 * frame, 1 the settled current one. A partner of another kind is ignored, as
 * the desktop's casters do.
 */
export function drawView(p: Painter, rect: Rect, view: VizView, before: VizView | null, t: number, theme: VizTheme): void {
  switch (view.type) {
    case 'array':
      return drawArray(p, rect, view, before?.type === 'array' ? before : null, t, theme)
    case 'kv':
      return drawKv(p, rect, view, before?.type === 'kv' ? before : null, t, theme)
    case 'stack':
      return drawStack(p, rect, view, before?.type === 'stack' ? before : null, t, theme)
    case 'bits':
      return drawBits(p, rect, view, before?.type === 'bits' ? before : null, t, theme)
    case 'text':
      return drawText(p, rect, view, theme)
    case 'list':
      return drawList(p, rect, view, before?.type === 'list' ? before : null, t, theme)
    case 'tree':
      return drawTree(p, rect, view, before?.type === 'tree' ? before : null, t, theme)
    case 'grid':
      return drawGrid(p, rect, view, before?.type === 'grid' ? before : null, t, theme)
    case 'graph':
      return drawGraph(p, rect, view, before?.type === 'graph' ? before : null, t, theme)
  }
}
