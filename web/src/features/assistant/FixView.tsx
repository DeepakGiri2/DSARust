// Fix mode: the review, and the proposed correction as a diff against what the
// user wrote — the desktop's `fix_ui`, `fix_banner` and `diff_view` in one
// column.
//
// The editor is not touched while a proposal is on screen. The user's code
// stays exactly as they left it and the proposal is diffed against it — which
// is the whole point, since a silently replaced buffer hands you working code
// and hides the mistake you made. Discarding is therefore free: there is
// nothing to put back. Only "apply" writes, and only the ticked changes.

import { useMemo, useState, type ReactNode } from 'react'
import clsx from 'clsx'
import { Modal } from '@/ui'
import type { FixResolution, FixRun } from './conversation'
import { diff as lineDiff, type Diff, type DiffLine } from './diff'
import { MODES } from './modes'
import { extractThink } from './parse'
import { RichText } from './RichText'
import { ErrorNote, Thinking } from './Transcript'
import styles from './AssistantPanel.module.css'

/** Untouched lines shown around each change; the rest fold away. */
const CONTEXT = 3

export interface FixViewProps {
  run: FixRun | null
  /** The editor's code right now — what a proposal is diffed against. */
  code: string
  busy: boolean
  onRetry: () => void
  onAccepted: (accepted: readonly boolean[]) => void
  onResolve: (resolution: FixResolution) => void
  onApply: (merged: string) => void
}

export function FixView({ run, code, busy, onRetry, onAccepted, onResolve, onApply }: FixViewProps) {
  const split = useMemo(() => extractThink(run?.content ?? ''), [run?.content])
  if (!run) return <p className={styles.empty}>{MODES.fix.emptyHint}</p>

  const thinking = (run.thinking + split.thinking).trim()
  const streaming = run.status === 'streaming'
  const outcome = run.outcome

  return (
    <>
      {run.note && (
        <article className={styles.turn}>
          <span className={clsx(styles.speaker, styles.you)}>you</span>
          <div className={styles.userBody}>
            <RichText text={run.note} />
          </div>
        </article>
      )}
      {thinking && <Thinking text={thinking} live={streaming && split.body === ''} />}
      {streaming && split.body === '' && !thinking && <p className={styles.note}>⏳ analyzing…</p>}
      {/* Until the reply is complete there is nothing to parse: show it raw. */}
      {!outcome && split.body !== '' && (
        <pre className={styles.stream}>
          {split.body}
          {streaming && (
            <span className={styles.cursor} aria-hidden>
              ▍
            </span>
          )}
        </pre>
      )}
      {run.status === 'stopped' && <p className={styles.note}>■ stopped — the review was cut short.</p>}
      {run.error && <ErrorNote error={run.error} busy={busy} onRetry={onRetry} />}

      {outcome && outcome.analysis && (
        <section className={styles.issues}>
          <h3 className="section-label">issues</h3>
          <RichText text={outcome.analysis} />
        </section>
      )}
      {outcome?.kind === 'unchanged' && <p className={styles.empty}>✔ the model suggested no code changes.</p>}
      {outcome?.kind === 'proposal' && (
        <Proposal
          run={run}
          proposal={outcome.code}
          code={code}
          onAccepted={onAccepted}
          onResolve={onResolve}
          onApply={onApply}
        />
      )}
    </>
  )
}

function Proposal({
  run,
  proposal,
  code,
  onAccepted,
  onResolve,
  onApply,
}: {
  run: FixRun
  proposal: string
  code: string
  onAccepted: (accepted: readonly boolean[]) => void
  onResolve: (resolution: FixResolution) => void
  onApply: (merged: string) => void
}) {
  // Against the code as it is now, so "apply" can never undo an edit made
  // after the fix arrived: whatever is in the editor is the left-hand side.
  const d = useMemo(() => lineDiff(code, proposal), [code, proposal])
  const [expanded, setExpanded] = useState(false)
  const res = run.resolution

  if (res.kind === 'applied') {
    return (
      <p className={styles.resolved}>
        ✔ applied {res.kept} of {res.total} change{res.total === 1 ? '' : 's'} to your code.
      </p>
    )
  }
  if (res.kind === 'discarded') {
    return (
      <p className={styles.resolved}>
        ✖ discarded — your code is unchanged.
        <button type="button" className="mini-btn" onClick={() => onResolve({ kind: 'open' })}>
          review it again
        </button>
      </p>
    )
  }
  if (d.isEmpty()) {
    return <p className={styles.resolved}>✔ the AI changed nothing — your code already matches it.</p>
  }

  // Ticks are per change group of *this* diff; if editing reshaped the diff,
  // the old ticks no longer name the same changes, so every change starts ticked.
  const accepted = run.accepted?.length === d.hunks ? run.accepted : Array<boolean>(d.hunks).fill(true)
  const kept = accepted.filter(Boolean).length
  const toggle = (hunk: number) => onAccepted(accepted.map((on, i) => (i === hunk ? !on : on)))
  const setAll = (on: boolean) => onAccepted(Array<boolean>(d.hunks).fill(on))
  const apply = () => {
    setExpanded(false)
    onApply(d.apply(accepted))
    onResolve({ kind: 'applied', kept, total: d.hunks })
  }
  const discard = () => {
    setExpanded(false)
    onResolve({ kind: 'discarded' })
  }

  const review = (expand: ReactNode) => (
    <>
      <div className={styles.banner}>
        <span>the AI's fix, against what you wrote:</span>
        <span className={clsx(styles.pill, styles.minus)}>−{d.removed}</span>
        <span className={clsx(styles.pill, styles.plus)}>+{d.added}</span>
        <span className={styles.kept}>
          · {kept} of {d.hunks} change{d.hunks === 1 ? '' : 's'} kept
        </span>
        {d.hunks > 1 && (
          <>
            <button type="button" className="mini-btn" onClick={() => setAll(true)}>
              keep all
            </button>
            <button type="button" className="mini-btn" onClick={() => setAll(false)}>
              keep none
            </button>
          </>
        )}
        <div className={styles.bannerActions}>
          {expand}
          <button type="button" className="mini-btn" onClick={discard} title="Leave your code exactly as it is">
            ✖ discard
          </button>
          <button
            type="button"
            className={clsx('mini-btn', styles.apply)}
            onClick={apply}
            disabled={kept === 0}
            title="Put the ticked changes into your code"
          >
            ✔ apply selected
          </button>
        </div>
      </div>
      <DiffView diff={d} accepted={accepted} onToggle={toggle} />
    </>
  )

  return (
    <>
      <p className={styles.proposed}>
        ✏ the fix is shown as a diff against your code — red is yours, green is theirs. Keep the changes you want.
      </p>
      {review(
        <button type="button" className="mini-btn" onClick={() => setExpanded(true)} title="Open the diff wide">
          ⤢ expand
        </button>,
      )}
      {/* The column is narrow; the desktop shows this in the editor's place. */}
      <Modal
        open={expanded}
        onClose={() => setExpanded(false)}
        title="🔧 the AI's fix, against what you wrote"
        labelledBy="ai-fix-diff-title"
        width={1000}
      >
        <div className={styles.issues}>{review(null)}</div>
      </Modal>
    </>
  )
}

const MARK: Record<DiffLine['change'], string> = { same: ' ', removed: '−', added: '+' }

/**
 * The fix, line by line, against what the user wrote. Two gutters, as a code
 * host shows them: the left number is the line in your code, the right is the
 * line in the proposal, and a line that exists on only one side has only one
 * number. Each change group carries its own tick, so a fix that corrects one
 * thing and rewrites another can be taken in part.
 */
export function DiffView({
  diff: d,
  accepted,
  onToggle,
}: {
  diff: Diff
  accepted: readonly boolean[]
  onToggle: (hunk: number) => void
}) {
  const rows = useMemo(() => d.rows(CONTEXT), [d])
  const taken = (hunk: number | null) => hunk === null || (accepted[hunk] ?? true)

  const out: ReactNode[] = []
  let drawn: number | null = null
  rows.forEach((row, r) => {
    if (row.kind === 'folded') {
      out.push(
        <tr key={`fold-${r}`} className={styles.fold}>
          <td colSpan={4}>
            ⋯ {row.count} unchanged line{row.count === 1 ? '' : 's'}
          </td>
        </tr>,
      )
      return
    }
    const line = d.lines[row.index]
    // One tick per group, above its first line. A group is an unbroken run, so
    // this fires exactly once for each.
    if (line.hunk !== null && line.hunk !== drawn) {
      drawn = line.hunk
      const hunk = line.hunk
      const on = taken(hunk)
      out.push(
        <tr key={`hunk-${hunk}`} className={styles.hunkRow}>
          <td colSpan={4}>
            <label
              className={clsx(styles.hunkToggle, !on && styles.hunkOff)}
              title={
                on
                  ? 'This change will be applied — untick to keep your version'
                  : "Your version is kept — tick to take the AI's"
              }
            >
              <input type="checkbox" checked={on} onChange={() => onToggle(hunk)} />
              change {hunk + 1} of {d.hunks}
              {!on && ' — keeping yours'}
            </label>
          </td>
        </tr>,
      )
    }
    out.push(
      <tr key={`line-${row.index}`} className={clsx(styles[line.change], !taken(line.hunk) && styles.left)}>
        <td className={styles.num}>{line.oldNo ?? ''}</td>
        <td className={styles.num}>{line.newNo ?? ''}</td>
        <td className={styles.marker} aria-hidden>
          {MARK[line.change]}
        </td>
        <td className={styles.text}>
          {line.change !== 'same' && (
            <span className="visually-hidden">{line.change === 'removed' ? 'yours, removed: ' : 'theirs, added: '}</span>
          )}
          {line.text}
        </td>
      </tr>,
    )
  })

  return (
    <div className={styles.diffScroll}>
      <table className={styles.diffTable}>
        <tbody>{out}</tbody>
      </table>
    </div>
  )
}
