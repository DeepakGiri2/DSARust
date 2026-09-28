// Small formatters shared by the account-side screens.
//
// Relative times are hand-rolled rather than `Intl.RelativeTimeFormat`, whose
// narrow style reads differently between ICU builds ("3m ago" / "3 min. ago");
// a timestamp column that changes shape between browsers looks broken.

const MINUTE = 60_000
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR

/** "just now", "5m ago", "3h ago", "2d ago", then a date. */
export function timeAgo(iso: string, now: number = Date.now()): string {
  const t = Date.parse(iso)
  if (Number.isNaN(t)) return ''
  const d = now - t
  // Also covers a server clock a little ahead of this one.
  if (d < 45_000) return 'just now'
  if (d < HOUR) return `${Math.max(1, Math.floor(d / MINUTE))}m ago`
  if (d < DAY) return `${Math.floor(d / HOUR)}h ago`
  if (d < 7 * DAY) return `${Math.floor(d / DAY)}d ago`
  return formatDate(iso, now)
}

/** "Sep 3", or "Sep 3, 2025" when it is not this year. */
export function formatDate(iso: string, now: number = Date.now()): string {
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) return ''
  const sameYear = date.getFullYear() === new Date(now).getFullYear()
  return date.toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
    ...(sameYear ? {} : { year: 'numeric' }),
  })
}

/** A full timestamp for `title` tooltips next to a relative one. */
export function formatDateTime(iso: string): string {
  const date = new Date(iso)
  return Number.isNaN(date.getTime())
    ? ''
    : date.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })
}

/** How long to wait, for "try again in …" — rounded up, never "0 seconds". */
export function formatWait(secs: number): string {
  const s = Math.max(1, Math.ceil(secs))
  if (s < 60) return s === 1 ? '1 second' : `${s} seconds`
  const m = Math.ceil(s / 60)
  if (m < 60) return m === 1 ? 'a minute' : `${m} minutes`
  const h = Math.ceil(m / 60)
  return h === 1 ? 'an hour' : `${h} hours`
}

/** "1 problem", "3 problems". */
export function plural(n: number, one: string, many = `${one}s`): string {
  return `${n} ${n === 1 ? one : many}`
}
