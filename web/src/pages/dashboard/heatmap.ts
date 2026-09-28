// The activity heatmap's arithmetic, kept pure so it can be tested.
//
// Days are the account's *local* days (the server counts streaks the same
// way), carried as `YYYY-MM-DD` strings and stepped with UTC arithmetic — so a
// daylight-saving change can never produce a 23- or 25-hour "day" and skip or
// repeat a cell.

export interface ActivityDay {
  day: string
  runs: number
  solved: number
}

export interface HeatCell {
  day: string
  runs: number
  solved: number
  /** 0 = nothing, 1–4 = relative intensity. */
  level: 0 | 1 | 2 | 3 | 4
}

export interface Heatmap {
  /** Columns of seven (Sunday first); null pads the first and last week. */
  weeks: (HeatCell | null)[][]
  /** Every day in order, oldest first — what keyboard navigation walks. */
  days: HeatCell[]
  /** Month labels and the column they start in. */
  months: { label: string; column: number }[]
  totalRuns: number
  totalSolved: number
  activeDays: number
}

const DAY_MS = 86_400_000

function parseDay(day: string): number {
  const [y, m, d] = day.split('-').map(Number)
  return Date.UTC(y, m - 1, d)
}

function formatDay(ms: number): string {
  const d = new Date(ms)
  const mm = String(d.getUTCMonth() + 1).padStart(2, '0')
  const dd = String(d.getUTCDate()).padStart(2, '0')
  return `${d.getUTCFullYear()}-${mm}-${dd}`
}

/** Today's date in an IANA zone, as `YYYY-MM-DD`; the browser's zone if it is unknown. */
export function todayIn(timeZone: string | undefined, now: Date = new Date()): string {
  const parts = (tz: string | undefined) =>
    new Intl.DateTimeFormat('en-US', { timeZone: tz, year: 'numeric', month: '2-digit', day: '2-digit' }).formatToParts(now)
  let p: Intl.DateTimeFormatPart[]
  try {
    p = parts(timeZone)
  } catch {
    p = parts(undefined)
  }
  const get = (type: string) => p.find((x) => x.type === type)?.value ?? ''
  return `${get('year')}-${get('month')}-${get('day')}`
}

/** Relative intensity: the busiest day in view is 4, anything at all is at least 1. */
export function levelFor(count: number, max: number): HeatCell['level'] {
  if (count <= 0 || max <= 0) return 0
  return Math.min(4, Math.max(1, Math.ceil((count / max) * 4))) as HeatCell['level']
}

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']

/** The last `span` days ending `today`, laid out GitHub-style. */
export function buildHeatmap(activity: readonly ActivityDay[], today: string, span = 365): Heatmap {
  const byDay = new Map(activity.map((a) => [a.day, a]))
  const end = parseDay(today)
  const start = end - (span - 1) * DAY_MS

  const raw: { day: string; runs: number; solved: number }[] = []
  for (let t = start; t <= end; t += DAY_MS) {
    const day = formatDay(t)
    const a = byDay.get(day)
    raw.push({ day, runs: a?.runs ?? 0, solved: a?.solved ?? 0 })
  }
  // A problem ticked off by hand is activity too, not only a run.
  const max = raw.reduce((m, d) => Math.max(m, d.runs + d.solved), 0)
  const days: HeatCell[] = raw.map((d) => ({ ...d, level: levelFor(d.runs + d.solved, max) }))

  const weeks: (HeatCell | null)[][] = []
  const lead = new Date(start).getUTCDay()
  let week: (HeatCell | null)[] = Array.from({ length: lead }, () => null)
  for (const cell of days) {
    week.push(cell)
    if (week.length === 7) {
      weeks.push(week)
      week = []
    }
  }
  if (week.length > 0) weeks.push([...week, ...Array.from({ length: 7 - week.length }, () => null)])

  // A label where a month first appears, unless it would crowd the last one.
  const months: Heatmap['months'] = []
  let lastMonth = -1
  weeks.forEach((w, column) => {
    const first = w.find((c) => c !== null)
    if (!first) return
    const month = Number(first.day.slice(5, 7)) - 1
    if (month !== lastMonth) {
      lastMonth = month
      const prev = months[months.length - 1]
      if (!prev || column - prev.column >= 3) months.push({ label: MONTHS[month], column })
    }
  })

  return {
    weeks,
    days,
    months,
    totalRuns: days.reduce((n, d) => n + d.runs, 0),
    totalSolved: days.reduce((n, d) => n + d.solved, 0),
    activeDays: days.filter((d) => d.level > 0).length,
  }
}

/** "Mon, Sep 22, 2026" for a `YYYY-MM-DD` day, in the viewer's language. */
export function formatHeatDay(day: string): string {
  return new Date(parseDay(day)).toLocaleDateString(undefined, {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
    year: 'numeric',
    timeZone: 'UTC',
  })
}

/** The words for one cell, for its tooltip and for screen readers. */
export function describeCell(c: HeatCell): string {
  const what =
    c.runs === 0 && c.solved === 0
      ? 'No activity'
      : [c.runs > 0 && `${c.runs} ${c.runs === 1 ? 'run' : 'runs'}`, c.solved > 0 && `${c.solved} solved`]
          .filter(Boolean)
          .join(' · ')
  return `${what} — ${formatHeatDay(c.day)}`
}
