// Resizable columns, like the desktop's `SidePanel::resizable` with its
// `width_range`. Widths are a per-device convenience, so they live in
// localStorage rather than in the synced profile settings.

import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent } from 'react'
import { readStore, writeStore } from './storage'
import ui from './ui.module.css'

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v))

export interface ColumnSpec {
  /** Storage key, e.g. `practice.question`. */
  id: string
  initial: number
  min: number
  max: number
}

export function useColumnWidth({ id, initial, min, max }: ColumnSpec): [number, (w: number) => void] {
  const key = `dsa.layout.${id}`
  const [width, setWidth] = useState(() => {
    const saved = Number(readStore('local', key))
    return Number.isFinite(saved) && saved >= min && saved <= max ? saved : initial
  })
  // Persist once the drag settles, not sixty times a second.
  useEffect(() => {
    const t = setTimeout(() => writeStore('local', key, String(width)), 300)
    return () => clearTimeout(t)
  }, [key, width])
  return [width, (w: number) => setWidth(clamp(Math.round(w), min, max))]
}

/**
 * The handle between two columns. `side` is where the resized column sits:
 * a left column grows as the handle moves right, a right column shrinks.
 */
export function Splitter({
  spec,
  width,
  onResize,
  side,
  label,
}: {
  spec: ColumnSpec
  width: number
  onResize: (w: number) => void
  side: 'left' | 'right'
  label: string
}) {
  const drag = useRef<{ x: number; w: number } | null>(null)
  const sign = side === 'left' ? 1 : -1

  const down = (e: PointerEvent<HTMLDivElement>) => {
    e.currentTarget.setPointerCapture(e.pointerId)
    drag.current = { x: e.clientX, w: width }
  }
  const move = (e: PointerEvent<HTMLDivElement>) => {
    if (drag.current) onResize(drag.current.w + sign * (e.clientX - drag.current.x))
  }
  const up = (e: PointerEvent<HTMLDivElement>) => {
    drag.current = null
    if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId)
  }
  const key = (e: KeyboardEvent<HTMLDivElement>) => {
    const step = e.shiftKey ? 64 : 16
    if (e.key === 'ArrowLeft') onResize(width - sign * step)
    else if (e.key === 'ArrowRight') onResize(width + sign * step)
    else if (e.key === 'Home') onResize(spec.min)
    else if (e.key === 'End') onResize(spec.max)
    else return
    e.preventDefault()
  }

  return (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuenow={width}
      aria-valuemin={spec.min}
      aria-valuemax={spec.max}
      tabIndex={0}
      className={ui.splitter}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={up}
      onPointerCancel={up}
      onKeyDown={key}
    />
  )
}
