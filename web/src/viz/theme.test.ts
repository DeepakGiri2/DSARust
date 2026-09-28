import { parseColor, rgb } from './color'
import { DARK_THEME, LIGHT_THEME, contrastOn, fnv1a, themeFromDocument } from './theme'

describe('pointer colours', () => {
  it('hash names with 32-bit FNV-1a, as theme.rs does', () => {
    // Reference vectors for FNV-1a 32.
    expect(fnv1a('')).toBe(0x811c9dc5)
    expect(fnv1a('a')).toBe(0xe40c292c)
    expect(fnv1a('foobar')).toBe(0xbf9cf968)
  })

  it('are stable per name and distinct for i and j', () => {
    expect(DARK_THEME.pointerColor('i')).toBe(DARK_THEME.pointerColor('i'))
    expect(DARK_THEME.pointerColor('i')).not.toEqual(DARK_THEME.pointerColor('j'))
    for (const name of ['i', 'j', 'lo', 'hi', 'slow', 'fast', 'ñ']) {
      expect(DARK_THEME.pointerColor(name)).toBe(DARK_THEME.pointerPalette[fnv1a(name) % 6])
      expect(LIGHT_THEME.pointerColor(name)).toBe(LIGHT_THEME.pointerPalette[fnv1a(name) % 6])
    }
  })
})

describe('contrast rule', () => {
  it('flips the ink with the background luminance', () => {
    expect(contrastOn(rgb(0xffffff))).toEqual(rgb(0x10141c))
    expect(contrastOn(rgb(0x000000))).toEqual(rgb(0xf2f5fa))
    expect(DARK_THEME.on(rgb(0xffffff))[0]).toBe(0x10)
  })

  it('puts light ink on the dark theme current-node fill (luma 125)', () => {
    expect(contrastOn(rgb(0x9f7d26))).toEqual(rgb(0xf2f5fa))
  })
})

describe('themeFromDocument', () => {
  afterEach(() => {
    delete document.documentElement.dataset.theme
    document.head.querySelectorAll('style[data-test]').forEach((s) => s.remove())
  })

  it('defaults to the dark palette when the page has no tokens', () => {
    const theme = themeFromDocument()
    expect(theme.dark).toBe(true)
    expect(theme.cell).toEqual(DARK_THEME.cell)
    expect(theme.fonts).toEqual(DARK_THEME.fonts)
  })

  it('switches palette with data-theme', () => {
    document.documentElement.dataset.theme = 'light'
    const theme = themeFromDocument()
    expect(theme.dark).toBe(false)
    expect(theme.cell).toEqual(rgb(0xedf0f6))
    expect(theme.pointerPalette).toEqual(LIGHT_THEME.pointerPalette)
  })

  it('lets --viz-* tokens and the font tokens override the built-in values', () => {
    const style = document.createElement('style')
    style.dataset.test = ''
    style.textContent = `:root { --viz-cell: #102030; --viz-good: rgb(1, 2, 3); --mono: 'Test Mono', monospace; }`
    document.head.append(style)
    const theme = themeFromDocument()
    expect(theme.cell).toEqual(rgb(0x102030))
    expect(theme.good).toEqual(rgb(0x010203))
    expect(theme.bad).toEqual(DARK_THEME.bad)
    expect(theme.fonts.mono).toBe("'Test Mono', monospace")
  })
})

describe('parseColor', () => {
  it('reads the notations tokens use and rejects the rest', () => {
    expect(parseColor(' #0b0e14 ')).toEqual(rgb(0x0b0e14))
    expect(parseColor('#abc')).toEqual(rgb(0xaabbcc))
    expect(parseColor('#7c6cff22')).toEqual([0x7c, 0x6c, 0xff, 0x22])
    expect(parseColor('rgb(11 14 20 / 0.5)')).toEqual([11, 14, 20, 128])
    expect(parseColor('rgba(11, 14, 20, 50%)')).toEqual([11, 14, 20, 128])
    expect(parseColor('')).toBeNull()
    expect(parseColor('tomato')).toBeNull()
  })
})
