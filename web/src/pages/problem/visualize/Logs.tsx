// Log lines, as the inspector's "logs" list and as the ⌨ console drawer.

import { memo, useLayoutEffect, useRef, type CSSProperties } from 'react'
import clsx from 'clsx'
import type { LogEntry } from '@/trace/types'
import ui from '../shared/ui.module.css'
import { logColor, logGlyph, logText } from './logStyle'
import styles from './Visualize.module.css'

/** Follow the newest line, like the desktop's `stick_to_bottom`. */
function useStickToBottom(count: number) {
  const box = useRef<HTMLDivElement>(null)
  useLayoutEffect(() => {
    const el = box.current
    if (el) el.scrollTop = el.scrollHeight
  }, [count])
  return box
}

function LogLine({ entry, faded, numbered }: { entry: LogEntry; faded?: boolean; numbered?: boolean }) {
  return (
    <div
      className={clsx(styles.logLine, faded && styles.faded, entry.kind !== 'log' && styles.colored)}
      style={{ '--log': logColor(entry.kind) } as CSSProperties}
    >
      {numbered && <span className={styles.logStep}>{entry.step + 1}</span>}
      <span className={styles.logGlyph} aria-hidden>
        {logGlyph(entry.kind)}
      </span>
      <span className={styles.logText}>{logText(entry)}</span>
    </div>
  )
}

/** The inspector's list: every line up to the current step. */
export const LogList = memo(function LogList({ entries }: { entries: LogEntry[] }) {
  const box = useStickToBottom(entries.length)
  return (
    <div ref={box} className={styles.logs} role="log" aria-label="Logs">
      {entries.length === 0 ? (
        <p className={ui.empty}>no logs yet</p>
      ) : (
        entries.map((e, i) => <LogLine key={i} entry={e} />)
      )}
    </div>
  )
})

/**
 * The ⌨ console (`console_panel`): lines emitted during the current step keep
 * their colour and everything earlier fades, so stepping forward shows at a
 * glance what *this* step did.
 */
export const Console = memo(function Console({ entries, step }: { entries: LogEntry[]; step: number }) {
  const box = useStickToBottom(entries.length)
  const fresh = entries.filter((e) => e.step === step).length
  return (
    <section className={styles.console} aria-label="Console">
      <div className={styles.consoleHead}>
        <span>CONSOLE</span>
        <span className={ui.spacer} />
        {fresh > 0 && <span className={clsx(ui.pill, ui.cyan)}>+{fresh} this step</span>}
        <span className={styles.consoleCount}>
          {entries.length} line{entries.length === 1 ? '' : 's'}
        </span>
      </div>
      <div ref={box} className={styles.consoleBody} role="log" aria-label="Console output">
        {entries.length === 0 ? (
          <p className={ui.empty}>nothing logged yet — lines appear as the animation runs</p>
        ) : (
          entries.map((e, i) => <LogLine key={i} entry={e} faded={e.step !== step} numbered />)
        )}
      </div>
    </section>
  )
})
