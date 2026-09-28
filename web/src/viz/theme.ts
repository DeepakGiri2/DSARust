// Colours and metrics — a port of `crates/dsa-viz/src/theme.rs`.
//
// One palette drives every renderer so a green cell means the same thing in
// an array, a grid and a graph. Both themes are defined here rather than
// derived from the surrounding page, because the semantic colours (found /
// rejected / current / settled) must stay legible and distinct whatever the
// shell looks like. The page's `--viz-*` tokens carry the same values; when
// they are present they win, so an edit to tokens.css reaches the canvas too.

import { parseColor, rgb, type Color } from './color'

export interface VizFonts {
  /** CSS font stack for values — egui's monospace family (Hack on the desktop). */
  readonly mono: string
  /** CSS font stack for labels — egui's proportional family (Ubuntu Light). */
  readonly sans: string
}

/** `Theme`'s fields, camel-cased (`cell_stroke` → `cellStroke`), plus the font stacks. */
export interface VizThemeValues {
  readonly dark: boolean

  readonly bg: Color
  readonly panel: Color
  readonly cell: Color
  readonly cellStroke: Color
  readonly text: Color
  readonly muted: Color

  /** Pointers and cursors. */
  readonly accent: Color
  /** Found / matched / good. */
  readonly good: Color
  /** Rejected / mismatch. */
  readonly bad: Color
  /** The cell under the cursor right now. */
  readonly cur: Color
  /** Sliding-window / live-range band. */
  readonly window: Color
  /** Finished, no longer interesting. */
  readonly dim: Color

  /** Distinct colours for named pointers; a pointer's name picks its colour. */
  readonly pointerPalette: readonly Color[]

  readonly cellSize: number
  readonly gap: number
  readonly rounding: number
  readonly labelHeight: number

  readonly fonts: VizFonts
}

export interface VizTheme extends VizThemeValues {
  /**
   * Stable colour for a named pointer. Hashing the name (rather than using
   * its position) keeps `i` the same colour even when `j` appears later.
   */
  pointerColor(name: string): Color
  /** Text that stays readable on top of `bg`. */
  on(bg: Color): Color
}

/** The stacks `--mono` / `--sans` declare in tokens.css, for when the page has no tokens (tests). */
const DEFAULT_FONTS: VizFonts = {
  mono: "'JetBrains Mono', 'Cascadia Code', Consolas, 'Courier New', monospace",
  sans: "'Ubuntu', system-ui, -apple-system, 'Segoe UI', sans-serif",
}

const METRICS = { cellSize: 44, gap: 6, rounding: 6, labelHeight: 20 } as const

export const DARK_VALUES: VizThemeValues = {
  dark: true,
  bg: rgb(0x0b0e14),
  panel: rgb(0x11151f),
  cell: rgb(0x161b28),
  cellStroke: rgb(0x232a3b),
  text: rgb(0xd6dbe8),
  muted: rgb(0x8b93a7),
  accent: rgb(0x7c6cff),
  good: rgb(0x34d399),
  bad: rgb(0xf87171),
  cur: rgb(0xfbbf24),
  window: rgb(0x22d3ee),
  dim: rgb(0x4a5163),
  pointerPalette: [rgb(0x7c6cff), rgb(0xff8f5a), rgb(0x34d399), rgb(0x22d3ee), rgb(0xfbbf24), rgb(0xd87cff)],
  ...METRICS,
  fonts: DEFAULT_FONTS,
}

export const LIGHT_VALUES: VizThemeValues = {
  dark: false,
  bg: rgb(0xfafbfd),
  panel: rgb(0xffffff),
  cell: rgb(0xedf0f6),
  cellStroke: rgb(0xc9d1e0),
  text: rgb(0x1b202c),
  muted: rgb(0x666f84),
  accent: rgb(0x146cd8),
  good: rgb(0x0e9f63),
  bad: rgb(0xd6333f),
  cur: rgb(0xb57d00),
  window: rgb(0x714ae0),
  dim: rgb(0xa8b1c2),
  pointerPalette: [rgb(0x146cd8), rgb(0xd4620d), rgb(0x0e9f63), rgb(0x8b3dd6), rgb(0xb57d00), rgb(0x0d919e)],
  ...METRICS,
  fonts: DEFAULT_FONTS,
}

const utf8 = new TextEncoder()

/** 32-bit FNV-1a over the UTF-8 bytes of `s` — the hash `Theme::pointer_color` uses, bit for bit. */
export function fnv1a(s: string): number {
  let h = 0x811c9dc5
  for (const b of utf8.encode(s)) h = Math.imul(h ^ b, 0x01000193) >>> 0
  return h
}

const INK_ON_LIGHT = rgb(0x10141c)
const INK_ON_DARK = rgb(0xf2f5fa)

/** `Theme::on`: dark ink on anything brighter than luma 140, light ink otherwise. */
export function contrastOn(bg: Color): Color {
  const luma = 0.299 * bg[0] + 0.587 * bg[1] + 0.114 * bg[2]
  return luma > 140 ? INK_ON_LIGHT : INK_ON_DARK
}

export function makeTheme(values: VizThemeValues): VizTheme {
  const palette = values.pointerPalette
  return Object.freeze({
    ...values,
    pointerColor: (name: string): Color => palette[fnv1a(name) % palette.length],
    on: contrastOn,
  })
}

export const DARK_THEME: VizTheme = makeTheme(DARK_VALUES)
export const LIGHT_THEME: VizTheme = makeTheme(LIGHT_VALUES)

/**
 * The theme the page is showing: `data-theme` on the root element picks dark
 * or light (dark is the default, as in tokens.css), and each `--viz-*` token
 * that parses overrides the built-in value. `panel` and the pointer palette
 * have no token and always come from theme.rs.
 */
export function themeFromDocument(root: HTMLElement = document.documentElement): VizTheme {
  const base = root.dataset.theme === 'light' ? LIGHT_VALUES : DARK_VALUES
  const css = getComputedStyle(root)
  const color = (token: string, fallback: Color): Color => parseColor(css.getPropertyValue(token)) ?? fallback
  const font = (token: string, fallback: string): string => css.getPropertyValue(token).trim() || fallback
  return makeTheme({
    ...base,
    bg: color('--viz-bg', base.bg),
    cell: color('--viz-cell', base.cell),
    cellStroke: color('--viz-cell-stroke', base.cellStroke),
    text: color('--viz-text', base.text),
    muted: color('--viz-muted', base.muted),
    accent: color('--viz-accent', base.accent),
    good: color('--viz-good', base.good),
    bad: color('--viz-bad', base.bad),
    cur: color('--viz-cur', base.cur),
    window: color('--viz-window', base.window),
    dim: color('--viz-dim', base.dim),
    fonts: { mono: font('--mono', base.fonts.mono), sans: font('--sans', base.fonts.sans) },
  })
}
