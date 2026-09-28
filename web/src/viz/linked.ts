// Renderers for the structural views — a port of
// `crates/dsa-viz/src/linked.rs`: linked lists, trees, grids and graphs.
//
// These all share one trick. Rather than special-casing every kind of
// structural change, each renderer lays out the *previous* step and the
// *current* step independently and interpolates each node's position by id.
// A swapped pair of subtrees, a node appended to a list, an edge relaxed in a
// graph — they all animate for free, because the layout moved and the drawing
// followed it.

import { cellEq, cellText, type GraphView, type GridView, type LinkedListView, type RC, type TreeNode, type TreeView } from '@/trace/types'
import { easeOut, flash, inflate, lerpPos, lerpRect, mix, withAlpha } from './anim'
import { arrow, caption, cr, glow, laneOf, monoCentered, pointerMarker, shortCellText, valueCell, type CellState } from './draw'
import {
  ZERO,
  add,
  normalized,
  pos2,
  rectCenter,
  rectFromCenterSize,
  rectFromMinSize,
  rectHeight,
  rectWidth,
  scale,
  splat,
  sub,
  vec2,
  type Pos2,
  type Rect,
} from './geom'
import { clamp } from './num'
import { CENTER_CENTER, LEFT_CENTER, monospace, stroke, type Painter } from './painter'
import type { VizTheme } from './theme'

/** Node id → centre. */
export type Layout = ReadonlyMap<number, Pos2>

const EMPTY_LAYOUT: Layout = new Map()

/** Where node `id` is at progress `t`, gliding from its old spot to its new one. */
export function at(layout: Layout, was: Layout, id: number, t: number): Pos2 {
  const now = layout.get(id) ?? ZERO
  const before = was.get(id)
  // A node that did not exist before appears in place rather than flying in
  // from the origin.
  return before ? lerpPos(before, now, easeOut(t)) : now
}

// ── Linked list ─────────────────────────────────────────────────────────────

export function listHeight(theme: VizTheme): number {
  return theme.labelHeight + 88
}

export function listLayout(v: LinkedListView, rect: Rect, theme: VizTheme): { layout: Layout; nodeW: number } {
  const n = Math.max(v.nodes.length, 1)
  const nodeW = clamp((rectWidth(rect) - 40) / n - 26, 26, 62)
  const step = nodeW + 34
  const y = rect.min.y + theme.labelHeight + 44
  const layout = new Map<number, Pos2>()
  v.nodes.forEach((node, i) => layout.set(node.id, pos2(rect.min.x + 8 + i * step + nodeW * 0.5, y)))
  return { layout, nodeW }
}

export function drawList(p: Painter, rect: Rect, v: LinkedListView, was: LinkedListView | null, t: number, theme: VizTheme): void {
  caption(p, rect.min, v.label, theme.muted, 12)
  const { layout, nodeW } = listLayout(v, rect, theme)
  const old = was ? listLayout(was, rect, theme).layout : EMPTY_LAYOUT
  const reversed = v.reversed ?? []

  // Arrows first so nodes draw over their endpoints.
  for (const node of v.nodes) {
    const from = at(layout, old, node.id, t)
    const flipped = reversed.includes(node.id)
    const color = flipped ? theme.good : theme.muted
    if (node.next !== null && layout.has(node.next)) {
      const to = at(layout, old, node.next, t)
      const [a, b] =
        to.x >= from.x
          ? [add(from, vec2(nodeW * 0.5 + 2, 0)), sub(to, vec2(nodeW * 0.5 + 6, 0))]
          : // A backwards arrow runs under the row so it is not hidden behind
            // the nodes it passes.
            [add(from, vec2(-nodeW * 0.5 - 2, 6)), add(to, vec2(nodeW * 0.5 + 6, 6))]
      arrow(p, a, b, color, flipped ? 2.2 : 1.4)
    } else {
      const a = add(from, vec2(nodeW * 0.5 + 2, 0))
      const b = add(a, vec2(20, 0))
      arrow(p, a, b, color, 1.4)
      p.text(add(b, vec2(4, 0)), LEFT_CENTER, 'nil', monospace(10), theme.dim)
    }
  }

  const wasReversed = was?.reversed ?? []
  for (const node of v.nodes) {
    const r = rectFromCenterSize(at(layout, old, node.id, t), vec2(nodeW, 32))
    const flipped = reversed.includes(node.id)
    const wasFlipped = wasReversed.includes(node.id)
    const state: CellState = flipped ? 'good' : 'plain'
    const before: CellState = wasFlipped ? 'good' : 'plain'
    valueCell(p, r, shortCellText(node.val), state, before, false, t, theme)
    if (flipped && !wasFlipped) glow(p, r, theme.good, flash(t), theme)
  }

  // Pointer tabs travel between nodes with the same easing as array cursors.
  // Both ends are looked up in the *current* layout (as on the desktop): a tab
  // starts from where its old node sits now.
  const pointers = v.pointers ?? []
  const nilSpot = pos2(rect.max.x - 26, rect.min.y + theme.labelHeight + 44)
  pointers.forEach(([name, target], laneI) => {
    const prior = was?.pointers?.find(([n]) => n === name)
    const from = prior ? prior[1] : target
    const start = (from !== null ? layout.get(from) : undefined) ?? nilSpot
    const end = (target !== null ? layout.get(target) : undefined) ?? nilSpot
    const tip = sub(lerpPos(start, end, easeOut(t)), vec2(0, 18))
    pointerMarker(p, tip, name, theme.pointerColor(name), laneOf(pointers, laneI), theme)
  })
}

// ── Tree ────────────────────────────────────────────────────────────────────

/** First node per id — what the desktop's `nodes.iter().find(..)` returns. */
function nodeIndex(v: TreeView): Map<number, TreeNode> {
  const index = new Map<number, TreeNode>()
  for (const n of v.nodes) if (!index.has(n.id)) index.set(n.id, n)
  return index
}

export function treeHeight(v: TreeView, theme: VizTheme): number {
  return theme.labelHeight + Math.max(treeDepth(v), 1) * 58 + 30
}

/**
 * Levels below the root, visiting each id once. The visited set is shared
 * across branches (left first), so a malformed DAG or cycle terminates.
 */
export function treeDepth(v: TreeView): number {
  const nodes = nodeIndex(v)
  const seen = new Set<number>()
  const walk = (id: number | null): number => {
    if (id == null || seen.has(id)) return 0
    seen.add(id)
    const n = nodes.get(id)
    if (!n) return 0
    const left = walk(n.left)
    return 1 + Math.max(left, walk(n.right))
  }
  return walk(v.root)
}

/** `[id, depth]` in in-order, each id once, at most 25 levels deep. */
export function treeOrder(v: TreeView): [id: number, depth: number][] {
  const nodes = nodeIndex(v)
  const order: [number, number][] = []
  const seen = new Set<number>()
  const walk = (id: number | null, depth: number): void => {
    if (id == null) return
    if (seen.has(id) || depth > 24) return // malformed content must not hang the UI
    seen.add(id)
    const n = nodes.get(id)
    if (!n) return
    walk(n.left, depth + 1)
    order.push([id, depth])
    walk(n.right, depth + 1)
  }
  walk(v.root, 0)
  return order
}

/**
 * In-order x, depth y — the layout everyone draws binary trees with, and the
 * one that makes a BST read left-to-right in sorted order.
 */
export function treeLayout(v: TreeView, rect: Rect, theme: VizTheme): Layout {
  const order = treeOrder(v)
  const n = Math.max(order.length, 1)
  const usable = Math.max(rectWidth(rect) - 40, 60)
  const step = Math.min(usable / n, 78)
  const left = rect.min.x + 20 + Math.max(usable - step * n, 0) * 0.5
  const top = rect.min.y + theme.labelHeight + 22
  return new Map(order.map(([id, depth], slot) => [id, pos2(left + (slot + 0.5) * step, top + depth * 58)]))
}

export function drawTree(p: Painter, rect: Rect, v: TreeView, was: TreeView | null, t: number, theme: VizTheme): void {
  caption(p, rect.min, v.label, theme.muted, 12)
  if (v.nodes.length === 0) {
    caption(p, add(rect.min, vec2(0, theme.labelHeight)), '(empty tree)', theme.dim, 13)
    return
  }

  const layout = treeLayout(v, rect, theme)
  const old = was ? treeLayout(was, rect, theme) : EMPTY_LAYOUT
  const radius = 16
  const path = v.path ?? []

  for (const node of v.nodes) {
    if (!layout.has(node.id)) continue
    const from = at(layout, old, node.id, t)
    for (const child of [node.left, node.right]) {
      if (child == null || !layout.has(child)) continue
      const to = at(layout, old, child, t)
      const onPath = path.includes(node.id) && path.includes(child)
      const dir = normalized(sub(to, from))
      p.lineSegment(
        add(from, scale(dir, radius)),
        sub(to, scale(dir, radius)),
        stroke(onPath ? 2.2 : 1.3, onPath ? theme.accent : theme.cellStroke),
      )
    }
  }

  // The swap marker arcs between the two nodes that traded places.
  if (v.swap) {
    const [a, b] = v.swap
    if (layout.has(a) && layout.has(b)) {
      const pa = at(layout, old, a, t)
      const pb = at(layout, old, b, t)
      const mid = pos2((pa.x + pb.x) * 0.5, Math.max(pa.y, pb.y) + 26)
      const pts: Pos2[] = []
      for (let i = 0; i <= 16; i++) {
        const s = i / 16
        pts.push(lerpPos(lerpPos(pa, mid, s), lerpPos(mid, pb, s), s))
      }
      p.line(pts, stroke(2, withAlpha(theme.cur, 0.35 + 0.45 * flash(t))))
    }
  }

  const done = v.done ?? []
  for (const node of v.nodes) {
    if (!layout.has(node.id)) continue
    const c = at(layout, old, node.id, t)
    const isCur = v.cur === node.id
    const wasCur = was !== null && was.cur === node.id

    const fillNow = isCur
      ? mix(theme.cell, theme.cur, 0.6)
      : done.includes(node.id)
        ? mix(theme.cell, theme.good, 0.45)
        : path.includes(node.id)
          ? mix(theme.cell, theme.accent, 0.4)
          : theme.cell
    const fillWas = wasCur ? mix(theme.cell, theme.cur, 0.6) : fillNow
    const fill = mix(fillWas, fillNow, t)

    if (isCur && !wasCur) {
      p.circleFilled(c, radius + 6 * flash(t) + 3, withAlpha(theme.cur, 0.22 * flash(t)))
    }
    p.circleFilled(c, radius, fill)
    p.circleStroke(c, radius, stroke(1.2, mix(theme.cellStroke, fill, 0.5)))
    monoCentered(p, c, shortCellText(node.val), theme.on(fill), 12)
  }
}

// ── Grid ────────────────────────────────────────────────────────────────────

export function gridHeight(v: GridView, width: number, theme: VizTheme): number {
  const rows = Math.max(v.data.length, 1)
  const cols = Math.max(v.data[0]?.length ?? 1, 1)
  const size = gridCellSize(rows, cols, width, theme)
  return theme.labelHeight + rows * (size + 3) + 8
}

export function gridCellSize(rows: number, cols: number, width: number, theme: VizTheme): number {
  const byWidth = (width - 24) / cols - 3
  // Keep tall grids from pushing everything else off the screen.
  const byHeight = 320 / rows - 3
  return clamp(Math.min(byWidth, byHeight), 14, theme.cellSize)
}

const rcKey = (r: number, c: number): string => `${r},${c}`

/** Each decorated cell's state, highest priority winning: bad > hl > path > done. */
function gridStates(view: GridView): Map<string, CellState> {
  const out = new Map<string, CellState>()
  const mark = (cells: readonly RC[] | undefined, state: CellState) => {
    for (const [r, c] of cells ?? []) out.set(rcKey(r, c), state)
  }
  mark(view.done, 'done')
  mark(view.path, 'window')
  mark(view.hl, 'good')
  mark(view.bad, 'bad')
  return out
}

export function drawGrid(p: Painter, rect: Rect, v: GridView, was: GridView | null, t: number, theme: VizTheme): void {
  caption(p, rect.min, v.label, theme.muted, 12)
  const rows = v.data.length
  if (rows === 0) return
  const size = gridCellSize(rows, v.data[0].length, rectWidth(rect), theme)
  const step = size + 3
  const top = rect.min.y + theme.labelHeight
  const cellRect = (r: number, c: number): Rect => rectFromMinSize(pos2(rect.min.x + c * step, top + r * step), splat(size))

  const now = gridStates(v)
  const before = was ? gridStates(was) : null
  v.data.forEach((row, r) => {
    row.forEach((cell, c) => {
      const key = rcKey(r, c)
      const state = now.get(key) ?? 'plain'
      const prior = before ? (before.get(key) ?? 'plain') : state
      const old = was?.data[r]?.[c]
      const changed = old !== undefined && !cellEq(old, cell)
      valueCell(p, cellRect(r, c), shortCellText(cell), state, prior, changed, t, theme)
    })
  })

  // The cursor outline slides from cell to cell.
  if (v.cur) {
    const [r, c] = v.cur
    const [fr, fc] = was?.cur ?? v.cur
    const a = cellRect(Math.max(fr, 0), Math.max(fc, 0))
    const b = cellRect(Math.max(r, 0), Math.max(c, 0))
    p.rectStroke(inflate(lerpRect(a, b, easeOut(t)), 2), cr(theme.rounding), stroke(2, theme.cur), 'outside')
  }
}

// ── Graph ───────────────────────────────────────────────────────────────────

export function graphHeight(): number {
  return 270
}

export function graphLayout(v: GraphView, rect: Rect, theme: VizTheme): Layout {
  const n = Math.max(v.nodes.length, 1)
  const inner = rectHeight(rect) - theme.labelHeight
  const cx = rectCenter(rect).x
  const cy = rect.min.y + theme.labelHeight + inner * 0.5
  const radius = Math.max(Math.min(inner * 0.5 - 30, rectWidth(rect) * 0.5 - 40), 40)

  const layout = new Map<number, Pos2>()
  v.nodes.forEach((node, i) => {
    if (node.x != null && node.y != null) {
      layout.set(
        node.id,
        pos2(
          rect.min.x + 30 + node.x * (rectWidth(rect) - 60),
          rect.min.y + theme.labelHeight + 20 + node.y * (rectHeight(rect) - theme.labelHeight - 50),
        ),
      )
      return
    }
    // Deterministic circle: the same graph always looks the same, which
    // matters when stepping back and forth.
    const a = (Math.PI * 2 * i) / n - Math.PI / 2
    layout.set(node.id, pos2(cx + radius * Math.cos(a), cy + radius * Math.sin(a)))
  })
  return layout
}

export function drawGraph(p: Painter, rect: Rect, v: GraphView, was: GraphView | null, t: number, theme: VizTheme): void {
  caption(p, rect.min, v.label, theme.muted, 12)
  const layout = graphLayout(v, rect, theme)
  const old = was ? graphLayout(was, rect, theme) : EMPTY_LAYOUT
  const radius = 17
  const active = v.active ?? []

  for (const e of v.edges) {
    if (!layout.has(e.from) || !layout.has(e.to)) continue
    const a = at(layout, old, e.from, t)
    const b = at(layout, old, e.to, t)
    const on = active.some(([x, y]) => (x === e.from && y === e.to) || (x === e.to && y === e.from))
    const color = on ? theme.accent : withAlpha(theme.cellStroke, 0.9)
    const width = on ? 2.4 : 1.3
    const dir = normalized(sub(b, a))
    const s = add(a, scale(dir, radius))
    const end = sub(b, scale(dir, radius))
    if (e.directed) arrow(p, s, end, color, width)
    else p.lineSegment(s, end, stroke(width, color))
    if (on) {
      // A dot runs along the edge in the direction of travel — this is what
      // makes a traversal legible rather than a colour change.
      p.circleFilled(lerpPos(s, end, easeOut(t)), 4, theme.accent)
    }
    if (e.weight != null) {
      p.text(lerpPos(s, end, 0.5), CENTER_CENTER, cellText(e.weight), monospace(10), theme.muted)
    }
  }

  const done = v.done ?? []
  const frontier = v.frontier ?? []
  const bad = v.bad ?? []
  for (const node of v.nodes) {
    const c = at(layout, old, node.id, t)
    const isCur = v.cur === node.id
    const fill = bad.includes(node.id)
      ? mix(theme.cell, theme.bad, 0.7)
      : isCur
        ? mix(theme.cell, theme.cur, 0.6)
        : done.includes(node.id)
          ? mix(theme.cell, theme.good, 0.5)
          : frontier.includes(node.id)
            ? mix(theme.cell, theme.accent, 0.45)
            : theme.cell
    const wasCur = was !== null && was.cur === node.id
    if (isCur && !wasCur) {
      p.circleFilled(c, radius + 8 * flash(t), withAlpha(theme.cur, 0.25 * flash(t)))
    }
    p.circleFilled(c, radius, fill)
    p.circleStroke(c, radius, stroke(1.2, mix(theme.cellStroke, fill, 0.5)))
    monoCentered(p, c, node.label, theme.on(fill), 12)
  }
}
