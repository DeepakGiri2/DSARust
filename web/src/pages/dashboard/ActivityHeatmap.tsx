// A year of activity, GitHub-style, in the account's local days. The whole map
// is one tab stop: the arrow keys walk days (←/→ a week, ↑/↓ a day) and a live
// region reads each one, so it is not a mouse-only chart.

import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type MouseEvent } from 'react'
import { plural } from '@/pages/shell/format'
import { buildHeatmap, describeCell, todayIn, type ActivityDay } from './heatmap'
import styles from './dashboard.module.css'

const CELL = 11
const STEP = CELL + 3
const LEFT = 30
/** Room above the first row for month labels and a tooltip. */
const TOP = 30
const LEVELS = [0, 1, 2, 3, 4] as const

export function ActivityHeatmap({ activity, timezone }: { activity: readonly ActivityDay[]; timezone: string }) {
  const map = useMemo(() => buildHeatmap(activity, todayIn(timezone)), [activity, timezone])
  const [active, setActive] = useState<number | null>(null)
  const [viaKeyboard, setViaKeyboard] = useState(false)
  const scroller = useRef<HTMLDivElement>(null)

  // Where each day sits, and its index in `days` for the keyboard.
  const layout = useMemo(() => {
    const index = new Map(map.days.map((d, i) => [d.day, i]))
    const at: { x: number; y: number }[] = []
    map.weeks.forEach((week, col) =>
      week.forEach((cell, row) => {
        if (cell) at[index.get(cell.day) ?? 0] = { x: LEFT + col * STEP, y: TOP + row * STEP }
      }),
    )
    return { index, at }
  }, [map])

  // On a narrow screen the map scrolls; start at today, the end that matters.
  useEffect(() => {
    const el = scroller.current
    if (el) el.scrollLeft = el.scrollWidth
  }, [])

  const last = map.days.length - 1
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const i = active ?? last
    const to =
      e.key === 'ArrowLeft' ? i - 7
      : e.key === 'ArrowRight' ? i + 7
      : e.key === 'ArrowUp' ? i - 1
      : e.key === 'ArrowDown' ? i + 1
      : e.key === 'Home' ? 0
      : e.key === 'End' ? last
      : null
    if (to === null) return
    e.preventDefault()
    setViaKeyboard(true)
    setActive(Math.min(last, Math.max(0, to)))
  }

  const onMouseMove = (e: MouseEvent<SVGSVGElement>) => {
    const i = (e.target as Element).getAttribute('data-i')
    if (i === null) return
    setViaKeyboard(false)
    setActive(Number(i))
  }

  const width = LEFT + map.weeks.length * STEP
  const height = TOP + 7 * STEP
  const cell = active === null ? null : map.days[active]
  const pos = active === null ? undefined : layout.at[active]
  const summary =
    map.activeDays === 0
      ? 'No activity in the last year yet'
      : `${plural(map.totalRuns, 'run')} and ${map.totalSolved} solved across ${plural(map.activeDays, 'day')} in the last year`

  return (
    <div className={styles.heat}>
      <div
        ref={scroller}
        className={styles.heatScroll}
        tabIndex={0}
        role="group"
        aria-label={`${summary}. Use the arrow keys to read single days.`}
        onKeyDown={onKeyDown}
        onFocus={() => {
          setViaKeyboard(true)
          setActive((a) => a ?? last)
        }}
        onBlur={() => setActive(null)}
      >
        <div className={styles.heatInner} style={{ width }}>
          <svg
            width={width}
            height={height}
            viewBox={`0 0 ${width} ${height}`}
            aria-hidden
            onMouseMove={onMouseMove}
            onMouseLeave={() => setActive(null)}
          >
            {map.months.map((m) => (
              <text key={`${m.label}-${m.column}`} x={LEFT + m.column * STEP} y={TOP - 8} className={styles.heatLabel}>
                {m.label}
              </text>
            ))}
            {['Mon', 'Wed', 'Fri'].map((d, i) => (
              <text key={d} x={0} y={TOP + (1 + i * 2) * STEP + CELL - 2} className={styles.heatLabel}>
                {d}
              </text>
            ))}
            {map.weeks.map((week, col) =>
              week.map((c, row) =>
                c ? (
                  <rect
                    key={c.day}
                    data-i={layout.index.get(c.day)}
                    x={LEFT + col * STEP}
                    y={TOP + row * STEP}
                    width={CELL}
                    height={CELL}
                    rx={2}
                    className={styles[`l${c.level}`]}
                  />
                ) : null,
              ),
            )}
            {pos && (
              <rect x={pos.x - 1.5} y={pos.y - 1.5} width={CELL + 3} height={CELL + 3} rx={3} className={styles.heatFocus} />
            )}
          </svg>
          {cell && pos && (
            <div className={styles.tip} style={{ left: pos.x + CELL / 2, top: pos.y }} aria-hidden>
              {describeCell(cell)}
            </div>
          )}
        </div>
      </div>
      <div className={styles.heatFoot}>
        <span>{summary}</span>
        <span className={styles.legend} aria-hidden>
          Less
          {LEVELS.map((l) => (
            <i key={l} className={styles[`l${l}`]} />
          ))}
          More
        </span>
      </div>
      <span className="visually-hidden" aria-live="polite">
        {viaKeyboard && cell ? describeCell(cell) : ''}
      </span>
    </div>
  )
}
