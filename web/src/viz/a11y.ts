// Text alternatives for the canvases: what a screen reader announces for a
// view. A canvas is opaque to assistive technology, so each one carries a
// one-line summary of the same state the picture shows — the values, where
// the pointers are, what is highlighted — e.g. `nums: [2, 7, 11, 15, 3]; i → 1`.

import { cellText, type Cell, type VizView } from '@/trace/types'
import { treeOrder } from './linked'

/** Grids up to this many cells are read out in full; bigger ones by size only. */
const GRID_READ_LIMIT = 64

const join = (xs: readonly (string | number)[]): string => xs.join(', ')
const cells = (xs: readonly Cell[]): string => `[${join(xs.map(cellText))}]`

function binary(value: number, width: number): string {
  const w = Math.min(Math.max(Math.trunc(width), 1), 64)
  const v = Number.isFinite(value) ? BigInt.asUintN(w, BigInt(Math.trunc(value))) : 0n
  return v.toString(2).padStart(w, '0')
}

export function describeView(view: VizView): string {
  const head = view.label.replace(/\s+/g, ' ').trim() || view.type
  const parts: string[] = []
  switch (view.type) {
    case 'array':
      parts.push(`${head}${view.bars ? ' (bars)' : ''}: ${cells(view.data)}`)
      if (view.pointers?.length) parts.push(join(view.pointers.map(([name, i]) => `${name} → ${i}`)))
      if (view.window) parts.push(`window ${view.window[0]} to ${view.window[1]}`)
      if (view.hl?.length) parts.push(`highlighted ${join(view.hl)}`)
      if (view.bad?.length) parts.push(`rejected ${join(view.bad)}`)
      if (view.done?.length) parts.push(`done ${join(view.done)}`)
      break
    case 'kv':
      parts.push(
        view.entries.length
          ? `${head}: {${join(view.entries.map(([k, v]) => `${k} → ${cellText(v)}`))}}`
          : `${head}: empty`,
      )
      if (view.hlKeys?.length) parts.push(`highlighted ${join(view.hlKeys)}`)
      if (view.badKeys?.length) parts.push(`rejected ${join(view.badKeys)}`)
      break
    case 'stack':
      parts.push(`${head} (${view.kind}): ${cells(view.items)}`)
      if (view.pushed && view.items.length) parts.push(`pushed ${cellText(view.items[view.items.length - 1])}`)
      if (view.popped !== undefined) parts.push(`popped ${cellText(view.popped)}`)
      if (view.bad) parts.push('last item rejected')
      break
    case 'list': {
      const val = new Map(view.nodes.map((n) => [n.id, cellText(n.val)]))
      parts.push(`${head}: list ${cells(view.nodes.map((n) => n.val))}`)
      if (view.pointers?.length) {
        parts.push(join(view.pointers.map(([name, id]) => `${name} → ${id === null ? 'nil' : (val.get(id) ?? 'nil')}`)))
      }
      break
    }
    case 'tree': {
      const val = new Map(view.nodes.map((n) => [n.id, cellText(n.val)]))
      const order = treeOrder(view).map(([id]) => val.get(id) ?? '?')
      parts.push(
        view.nodes.length
          ? `${head}: tree, root ${view.root === null ? 'none' : (val.get(view.root) ?? 'none')}, in order [${join(order)}]`
          : `${head}: empty tree`,
      )
      if (view.cur !== undefined) parts.push(`current ${val.get(view.cur) ?? view.cur}`)
      break
    }
    case 'grid': {
      const rows = view.data.length
      const cols = view.data[0]?.length ?? 0
      parts.push(`${head}: ${rows}×${cols} grid`)
      if (rows * cols <= GRID_READ_LIMIT && rows > 0) {
        parts.push(`rows ${view.data.map((r) => r.map(cellText).join(' ')).join(' / ')}`)
      }
      if (view.cur) parts.push(`cursor row ${view.cur[0]} column ${view.cur[1]}`)
      break
    }
    case 'graph': {
      const label = new Map(view.nodes.map((n) => [n.id, n.label]))
      const name = (id: number) => label.get(id) ?? String(id)
      parts.push(`${head}: graph, ${view.nodes.length} nodes, ${view.edges.length} edges`)
      if (view.cur !== undefined) parts.push(`current ${name(view.cur)}`)
      if (view.active?.length) parts.push(`traversing ${join(view.active.map(([a, b]) => `${name(a)} → ${name(b)}`))}`)
      break
    }
    case 'bits':
      parts.push(`${head}: ${join(view.rows.map((r) => `${r.label} = ${binary(r.value, r.width)} (${r.value})`))}`)
      break
    case 'text':
      parts.push(`${head}: ${view.lines.join(' / ')}`)
      break
  }
  return parts.join('; ')
}
