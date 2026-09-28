// Browser state the canvases paint from: the theme (`data-theme` + the
// `--viz-*` tokens), web-font loading, the device pixel ratio and the
// container's width. Each is an external store read with
// useSyncExternalStore (or a ResizeObserver), so a change repaints exactly the
// canvases that depend on it and nothing polls.

import { useLayoutEffect, useRef, useState, useSyncExternalStore, type RefObject } from 'react'
import { DARK_THEME, themeFromDocument, type VizTheme } from './theme'

type Listener = () => void

// ── theme ───────────────────────────────────────────────────────────────────

let themeCache: { readonly key: string | undefined; readonly theme: VizTheme } | null = null

function themeSnapshot(): VizTheme {
  const key = document.documentElement.dataset.theme
  if (themeCache === null || themeCache.key !== key) themeCache = { key, theme: themeFromDocument() }
  return themeCache.theme
}

function subscribeTheme(onChange: Listener): () => void {
  const observer = new MutationObserver(onChange)
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] })
  return () => observer.disconnect()
}

/** The canvas theme, re-read whenever the root element's `data-theme` changes. */
export function useVizTheme(): VizTheme {
  return useSyncExternalStore(subscribeTheme, themeSnapshot, () => DARK_THEME)
}

// ── fonts ───────────────────────────────────────────────────────────────────

/** The part of `FontFaceSet` (`document.fonts`) the canvas needs. */
export interface FontSource {
  load(font: string): Promise<unknown>
  readonly ready: Promise<unknown>
  addEventListener(type: 'loadingdone', listener: () => void): void
}

export interface FontStore {
  subscribe(listener: Listener): () => void
  /** A counter that ticks whenever fonts finish loading. */
  getSnapshot(): number
}

/**
 * Canvas text is rasterised with whatever face is available at that instant
 * and never re-flows by itself, and drawing text on a canvas does not reliably
 * make the browser fetch an @font-face in time for the first paint. So the
 * store asks for the two faces the renderers use (`faces`, as CSS font
 * shorthands), then ticks once they — and anything else in flight — are ready,
 * and again after every later batch of loads (a subset for a glyph first seen
 * on another part of the page, say).
 */
export function createFontStore(source: () => FontSource | undefined, faces: () => readonly string[]): FontStore {
  let generation = 0
  let started = false
  const listeners = new Set<Listener>()
  const bump = () => {
    generation += 1
    for (const listener of listeners) listener()
  }
  const start = () => {
    if (started) return
    started = true
    const set = source()
    if (!set) return
    set.addEventListener('loadingdone', bump)
    void Promise.allSettled(faces().map((face) => set.load(face)))
      .then(() => set.ready)
      .then(bump, bump)
  }
  return {
    subscribe(listener) {
      listeners.add(listener)
      start()
      return () => {
        listeners.delete(listener)
      }
    },
    getSnapshot: () => generation,
  }
}

const fontStore = createFontStore(
  () => (typeof document === 'undefined' ? undefined : document.fonts),
  () => {
    const { mono, sans } = themeFromDocument().fonts
    return [`400 12px ${mono}`, `300 12px ${sans}`]
  },
)

/** Changes whenever web fonts finish loading — a cue to repaint canvas text. */
export function useFontGeneration(): number {
  return useSyncExternalStore(fontStore.subscribe, fontStore.getSnapshot, () => 0)
}

// ── device pixel ratio ──────────────────────────────────────────────────────

function devicePixelRatio(): number {
  return window.devicePixelRatio || 1
}

/**
 * A `resolution` media query matches only the ratio it was built for, so it
 * is rebuilt after every change (browser zoom, a move to another monitor).
 */
function subscribeDpr(onChange: Listener): () => void {
  if (typeof window.matchMedia !== 'function') return () => {}
  let query: MediaQueryList | null = null
  const handle = () => {
    watch()
    onChange()
  }
  const watch = () => {
    query?.removeEventListener('change', handle)
    query = window.matchMedia(`(resolution: ${devicePixelRatio()}dppx)`)
    query.addEventListener('change', handle)
  }
  watch()
  return () => query?.removeEventListener('change', handle)
}

export function useDevicePixelRatio(): number {
  return useSyncExternalStore(subscribeDpr, devicePixelRatio, () => 1)
}

// ── container width ─────────────────────────────────────────────────────────

function contentWidth(el: HTMLElement): number {
  const css = getComputedStyle(el)
  const inset = ['paddingLeft', 'paddingRight', 'borderLeftWidth', 'borderRightWidth'] as const
  const chrome = inset.reduce((sum, prop) => sum + (parseFloat(css[prop]) || 0), 0)
  return Math.max(0, el.getBoundingClientRect().width - chrome)
}

/**
 * The content-box width of the element behind the returned ref. Measured
 * synchronously on mount — so the first paint already has the right width —
 * then tracked with a ResizeObserver.
 */
export function useContentWidth<T extends HTMLElement>(): [RefObject<T | null>, number] {
  const ref = useRef<T>(null)
  const [width, setWidth] = useState(0)
  useLayoutEffect(() => {
    const el = ref.current
    if (!el) return
    setWidth(contentWidth(el))
    if (typeof ResizeObserver === 'undefined') {
      const onResize = () => setWidth(contentWidth(el))
      window.addEventListener('resize', onResize)
      return () => window.removeEventListener('resize', onResize)
    }
    const observer = new ResizeObserver((entries) => {
      const last = entries[entries.length - 1]
      if (last) setWidth(last.contentRect.width)
    })
    observer.observe(el)
    return () => observer.disconnect()
  }, [])
  return [ref, width]
}
