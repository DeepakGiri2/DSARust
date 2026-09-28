// A profile's colour, the way the desktop paints it (profiles.rs): the tile is
// the colour at 0x2e alpha, the ring at 0x66 (0xaa when it is the one in use),
// and the glyph sits on top.

/** The app accent — what a colour that fails to parse falls back to. */
export const FALLBACK_COLOR = '#7c6cff'

/**
 * `#rrggbb` from the server, falling back to the app accent rather than to
 * black — a profile whose colour failed to parse should still look deliberate.
 */
export function profileColor(hex: string | null | undefined): string {
  const h = (hex ?? '').trim().replace(/^#/, '')
  return /^[0-9a-f]{6}$/i.test(h) ? `#${h.toLowerCase()}` : FALLBACK_COLOR
}

/** The same colour at an 8-bit alpha, as `#rrggbbaa` — the desktop's `alpha(c, a)`. */
export function withAlpha(hex: string, alpha: number): string {
  const a = Math.round(Math.min(255, Math.max(0, alpha)))
  return profileColor(hex) + a.toString(16).padStart(2, '0')
}

const NAMES: Record<string, string> = {
  '#7c6cff': 'violet',
  '#22d3ee': 'cyan',
  '#34d399': 'green',
  '#fbbf24': 'amber',
  '#f87171': 'red',
  '#f472b6': 'pink',
}

/** A word for a swatch, so a screen reader hears "cyan" rather than a hex code. */
export function colorName(hex: string): string {
  const c = profileColor(hex)
  return NAMES[c] ?? c
}
