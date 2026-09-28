// The right column (`Visualize::inspector`): variables of the selected frame,
// the call stack, and the logs so far.

import { memo, useState } from 'react'
import clsx from 'clsx'
import type { LogEntry, Step } from '@/trace/types'
import { varIsComposite } from '@/trace/types'
import ui from '../shared/ui.module.css'
import { LogList } from './Logs'
import { CHILD_LIMIT, childLines, varRows, type VarRow } from './vars'
import styles from './Visualize.module.css'

export interface InspectorProps {
  slug: string
  step: Step | null
  /** The step before `step`, for change highlighting. */
  prevStep: Step | null
  /** Index into `step.frames` whose variables are shown. */
  frameIdx: number
  onSelectFrame: (i: number) => void
  isWatched: (slug: string, name: string) => boolean
  onToggleWatch: (slug: string, name: string) => void
  logs: LogEntry[]
  showLogs: boolean
  onShowLogs: (show: boolean) => void
}

export const Inspector = memo(function Inspector({
  slug,
  step,
  prevStep,
  frameIdx,
  onSelectFrame,
  isWatched,
  onToggleWatch,
  logs,
  showLogs,
  onShowLogs,
}: InspectorProps) {
  // Expansion is remembered per variable name across steps, so opening `seen`
  // once keeps it open while you step.
  const [expanded, setExpanded] = useState<Record<string, boolean>>({})
  if (!step) return <p className={ui.empty}>No trace.</p>

  const frames = step.frames
  const frame = frames[frameIdx]
  const rows = varRows(frame, prevStep?.frames[frameIdx], (name) => isWatched(slug, name))
  const toggle = (name: string, open: boolean) => setExpanded((e) => ({ ...e, [name]: !open }))

  return (
    <div className={styles.inspectorBody}>
      <div className={styles.varsHead}>
        <span>variables</span>
        {frame && frameIdx !== frames.length - 1 && <span className={styles.frameNote}>(frame: {frame.fn})</span>}
      </div>
      <div className={styles.vars}>
        {rows.length === 0 ? (
          <p className={ui.empty}>no variables in this frame</p>
        ) : (
          rows.map((r) => (
            <VarItem
              key={r.name}
              row={r}
              // The desktop always lists a map's or set's entries; arrays are
              // already whole in their one-line summary, so they start closed.
              open={expanded[r.name] ?? r.value.kind !== 'arr'}
              onToggleOpen={toggle}
              onToggleWatch={() => onToggleWatch(slug, r.name)}
            />
          ))
        )}
      </div>

      <h3 className={clsx('section-label', ui.head, ui.asIs)}>call stack · depth {step.depth}</h3>
      {frames.length === 0 ? (
        <p className={ui.empty}>call stack empty (finished)</p>
      ) : (
        <ul className={styles.stack}>
          {frames
            .map((f, i) => ({ f, i }))
            .reverse()
            .map(({ f, i }) => (
              <li key={i}>
                <button
                  type="button"
                  className={clsx(styles.frame, i === frameIdx && styles.frameSel, i === frames.length - 1 && styles.frameTop)}
                  style={{ marginLeft: `${i * 2}ch` }}
                  aria-pressed={i === frameIdx}
                  title="Inspect this frame's variables"
                  onClick={() => onSelectFrame(i)}
                >
                  {f.fn}
                </button>
              </li>
            ))}
        </ul>
      )}

      {showLogs ? (
        <>
          <div className={styles.logsHead}>
            <h3 className={clsx('section-label', ui.head, ui.asIs)}>logs</h3>
            <button
              type="button"
              className={styles.linkBtn}
              onClick={() => onShowLogs(false)}
              title="Hide the logs here — ⌨ console still shows them"
            >
              hide
            </button>
          </div>
          <LogList entries={logs} />
        </>
      ) : (
        <button type="button" className={clsx(styles.linkBtn, styles.showLogs)} onClick={() => onShowLogs(true)}>
          show logs
        </button>
      )}
    </div>
  )
})

function VarItem({
  row,
  open,
  onToggleOpen,
  onToggleWatch,
}: {
  row: VarRow
  open: boolean
  onToggleOpen: (name: string, open: boolean) => void
  onToggleWatch: () => void
}) {
  const composite = varIsComposite(row.value)
  const lines = composite && open ? childLines(row.value) : []
  return (
    <div className={styles.varItem}>
      <div className={styles.varRow}>
        <button
          type="button"
          className={clsx(styles.star, row.watched && styles.starOn)}
          aria-pressed={row.watched}
          aria-label={`Watch ${row.name}`}
          title="pin to watch"
          onClick={onToggleWatch}
        >
          {row.watched ? '★' : '☆'}
        </button>
        {composite ? (
          <button
            type="button"
            className={styles.disclosure}
            aria-expanded={open}
            aria-label={`${open ? 'Collapse' : 'Expand'} ${row.name}`}
            onClick={() => onToggleOpen(row.name, open)}
          >
            {open ? '▾' : '▸'}
          </button>
        ) : (
          <span className={styles.disclosure} aria-hidden />
        )}
        <span className={styles.varName}>{row.name}</span>
        <span className={clsx(styles.varValue, row.changed && styles.changed)}>
          {row.summary}
          {row.changed && <span className="visually-hidden"> (changed)</span>}
        </span>
      </div>
      {lines.length > 0 && (
        <ul className={styles.children}>
          {lines.slice(0, CHILD_LIMIT).map((l, i) => (
            <li key={i} className={clsx(l.hot && styles.hot)}>
              <span aria-hidden>{l.hot ? '▸' : ' '}</span> {l.text}
            </li>
          ))}
          {lines.length > CHILD_LIMIT && <li className={styles.more}>… {lines.length - CHILD_LIMIT} more</li>}
        </ul>
      )}
    </div>
  )
}
