import { describe, expect, it } from 'vitest'
import { describeDevice } from '@/pages/account/userAgent'
import { formatPrice, yearlySavings } from '@/pages/pricing/pricing'
import { formatWait, plural, timeAgo } from './format'
import { colorName, FALLBACK_COLOR, profileColor, withAlpha } from './profileColor'

describe('timeAgo', () => {
  const now = Date.parse('2026-09-26T12:00:00Z')
  const ago = (ms: number) => timeAgo(new Date(now - ms).toISOString(), now)

  it('reads the same everywhere', () => {
    expect(ago(10_000)).toBe('just now')
    expect(ago(-5_000)).toBe('just now') // a server clock slightly ahead
    expect(ago(90_000)).toBe('1m ago')
    expect(ago(59 * 60_000)).toBe('59m ago')
    expect(ago(3 * 3_600_000)).toBe('3h ago')
    expect(ago(2 * 86_400_000)).toBe('2d ago')
    expect(ago(30 * 86_400_000)).not.toMatch(/ago/)
    expect(timeAgo('not a date', now)).toBe('')
  })
})

describe('formatWait', () => {
  it('rounds up and never says zero', () => {
    expect(formatWait(0)).toBe('1 second')
    expect(formatWait(12)).toBe('12 seconds')
    expect(formatWait(60)).toBe('a minute')
    expect(formatWait(61)).toBe('2 minutes')
    expect(formatWait(3600)).toBe('an hour')
    expect(formatWait(7201)).toBe('3 hours')
  })

  it('pluralises', () => {
    expect(plural(1, 'problem')).toBe('1 problem')
    expect(plural(2, 'entry', 'entries')).toBe('2 entries')
  })
})

describe('profile colours (profiles.rs parse_color)', () => {
  it('parses the stored form', () => {
    expect(profileColor('#22D3EE')).toBe('#22d3ee')
    expect(profileColor('22d3ee')).toBe('#22d3ee')
  })

  it('falls back to the accent instead of going black', () => {
    expect(profileColor('')).toBe(FALLBACK_COLOR)
    expect(profileColor('#zzzzzz')).toBe(FALLBACK_COLOR)
    expect(profileColor('#fff')).toBe(FALLBACK_COLOR)
    expect(profileColor(undefined)).toBe(FALLBACK_COLOR)
  })

  it('adds the desktop’s alpha and names the swatches', () => {
    expect(withAlpha('#7c6cff', 0x2e)).toBe('#7c6cff2e')
    expect(withAlpha('#7c6cff', 300)).toBe('#7c6cffff')
    expect(colorName('#22d3ee')).toBe('cyan')
    expect(colorName('#123456')).toBe('#123456')
  })
})

describe('describeDevice', () => {
  it('names what a person would recognise', () => {
    const chromeWin =
      'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36'
    const edge = `${chromeWin} Edg/140.0`
    const iphone =
      'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1'
    const android = 'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Mobile Safari/537.36'
    expect(describeDevice(chromeWin)).toBe('Chrome on Windows')
    expect(describeDevice(edge)).toBe('Edge on Windows')
    expect(describeDevice(iphone)).toBe('Safari on iOS')
    expect(describeDevice(android)).toBe('Chrome on Android')
    expect(describeDevice('curl/8.0')).toBe('Unknown device')
    expect(describeDevice(null)).toBe('Unknown device')
  })
})

describe('pricing', () => {
  it('works out the yearly saving', () => {
    expect(yearlySavings(10, 96)).toBe(20)
    expect(yearlySavings(10, 120)).toBe(0)
    expect(yearlySavings(10, 130)).toBe(0)
    expect(yearlySavings(undefined, 96)).toBe(0)
  })

  it('shows cents only when there are some', () => {
    expect(formatPrice(9)).toBe('$9')
    expect(formatPrice(8.25)).toBe('$8.25')
  })
})
