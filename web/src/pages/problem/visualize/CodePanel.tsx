// The reference solution beside the animation (`Visualize::code_panel`).
//
// Drawn row by row rather than in a CodeMirror instance: every line that has
// a `//@tag` gets a real breakpoint button in the gutter, and the row of the
// current step carries the highlight. Rows are memoised, so a step re-renders
// the two rows whose highlight moved, not the file.

import { memo, useLayoutEffect, useRef, type Ref } from 'react'
import clsx from 'clsx'
import type { HlSpan } from '@/features/editor'
import styles from './Visualize.module.css'

export interface CodePanelProps {
  lines: HlSpan[][]
  /** `ProblemSource.line_tags`: 1-based line (as a string) → tag. */
  lineTags: Record<string, string>
  /** 1-based line of the current step, if its tag is on this source. */
  currentLine: number | null
  breakpoints: ReadonlySet<string>
  onToggleBreakpoint: (tag: string) => void
  langLabel: string
}

export const CodePanel = memo(function CodePanel({
  lines,
  lineTags,
  currentLine,
  breakpoints,
  onToggleBreakpoint,
  langLabel,
}: CodePanelProps) {
  const scroller = useRef<HTMLDivElement>(null)
  const currentRow = useRef<HTMLDivElement>(null)

  // Keep the current line in the middle half of the panel. Re-centring on
  // every step (what the desktop's scroll_to_me does) makes fast stepping
  // swim; nudging only when the line nears an edge keeps the code still.
  useLayoutEffect(() => {
    const box = scroller.current
    const row = currentRow.current
    if (!box || !row) return
    const top = row.offsetTop
    const bottom = top + row.offsetHeight
    const margin = box.clientHeight / 4
    if (top < box.scrollTop + margin || bottom > box.scrollTop + box.clientHeight - margin) {
      box.scrollTop = top - (box.clientHeight - row.offsetHeight) / 2
    }
  }, [currentLine, lines])

  return (
    <div className={styles.codePanel}>
      <div className={styles.codeHead}>
        <span>code — {langLabel}</span>
        <span className={styles.bpHint}>click ○ to set a breakpoint</span>
      </div>
      <div ref={scroller} className={styles.code} role="region" aria-label={`Reference solution in ${langLabel}`} tabIndex={0}>
        {lines.map((spans, i) => {
          const n = i + 1
          const tag = lineTags[String(n)]
          return (
            <CodeRow
              key={n}
              n={n}
              spans={spans}
              tag={tag}
              current={currentLine === n}
              breakpoint={tag !== undefined && breakpoints.has(tag)}
              onToggle={onToggleBreakpoint}
              rowRef={currentLine === n ? currentRow : undefined}
            />
          )
        })}
      </div>
    </div>
  )
})

const CodeRow = memo(function CodeRow({
  n,
  spans,
  tag,
  current,
  breakpoint,
  onToggle,
  rowRef,
}: {
  n: number
  spans: HlSpan[]
  tag: string | undefined
  current: boolean
  breakpoint: boolean
  onToggle: (tag: string) => void
  rowRef: Ref<HTMLDivElement> | undefined
}) {
  return (
    <div ref={rowRef} className={clsx(styles.line, current && styles.current)} aria-current={current ? 'step' : undefined}>
      {tag !== undefined ? (
        <button
          type="button"
          className={clsx(styles.bp, breakpoint && styles.bpOn)}
          aria-pressed={breakpoint}
          aria-label={`Breakpoint on line ${n}`}
          title={breakpoint ? 'Remove the breakpoint' : 'Set a breakpoint — ▶ play and ▶▶ continue stop here'}
          onClick={() => onToggle(tag)}
        >
          {breakpoint ? '●' : '○'}
        </button>
      ) : (
        <span className={styles.bpSpace} aria-hidden />
      )}
      <span className={styles.num} aria-hidden>
        {n}
      </span>
      <code className={styles.text}>
        {spans.map((s, j) => (
          <span key={j} className={s.cls ?? undefined}>
            {s.text}
          </span>
        ))}
      </code>
    </div>
  )
})
