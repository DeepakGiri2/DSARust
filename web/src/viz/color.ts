// Colours as the renderers reason about them: straight (unmultiplied) sRGBA
// bytes.
//
// egui stores a Color32 premultiplied, but every colour dsa-viz mixes is
// opaque — where premultiplied and straight are the same bytes — and
// translucency is only ever applied last, by `withAlpha`. egui 0.33
// premultiplies and blends in gamma space, which is exactly what a CSS
// `rgba()` over the page does, so straight bytes reproduce the desktop's
// pixels without a conversion step.

/** `[r, g, b, a]`, each 0–255. */
export type Color = readonly [r: number, g: number, b: number, a: number]

/** An opaque colour from `0xRRGGBB`. */
export function rgb(hex: number): Color {
  return [(hex >> 16) & 0xff, (hex >> 8) & 0xff, hex & 0xff, 0xff]
}

/** The canvas `fillStyle` / `strokeStyle` for a colour. */
export function cssColor([r, g, b, a]: Color): string {
  return a >= 0xff ? `rgb(${r}, ${g}, ${b})` : `rgba(${r}, ${g}, ${b}, ${a / 0xff})`
}

const HEX = /^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i
const RGB_FN = /^rgba?\(\s*([\d.]+)\s*[,\s]\s*([\d.]+)\s*[,\s]\s*([\d.]+)\s*(?:[,/]\s*([\d.]+)(%?)\s*)?\)$/i

const byte = (v: number): number => Math.min(0xff, Math.max(0, Math.round(v)))

/**
 * Parse a CSS colour as design tokens write them: `#rgb`, `#rgba`, `#rrggbb`,
 * `#rrggbbaa`, `rgb()` / `rgba()`. Anything else is `null`, so the caller can
 * fall back to the built-in palette rather than paint garbage.
 */
export function parseColor(input: string): Color | null {
  const s = input.trim()
  const hex = HEX.exec(s)
  if (hex) {
    const digits = hex[1].length <= 4 ? [...hex[1]].map((d) => d + d).join('') : hex[1]
    const at = (i: number) => parseInt(digits.slice(i, i + 2), 16)
    return [at(0), at(2), at(4), digits.length === 8 ? at(6) : 0xff]
  }
  const fn = RGB_FN.exec(s)
  if (fn) {
    const alpha = fn[4] === undefined ? 1 : Number(fn[4]) / (fn[5] ? 100 : 1)
    return [byte(Number(fn[1])), byte(Number(fn[2])), byte(Number(fn[3])), byte(alpha * 0xff)]
  }
  return null
}
