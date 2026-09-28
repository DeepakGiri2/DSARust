// One catalogue row — `list.rs`'s `row_frame` and its contents.
//
// The whole row is a link (a stretched `::after` on the title), and the star
// sits above it as its own button, so a click on the star can never also open
// the problem — the desktop's `inner_click` rule, without nesting a button in
// an anchor.

import { memo } from 'react'
import { Link } from 'react-router'
import clsx from 'clsx'
import type { CatalogProblem, Difficulty, ProgressStatus } from '@/api/types'
import { DifficultyPill } from '@/ui'
import styles from './CatalogPage.module.css'

const DIFFICULTY_COLOR: Record<Difficulty, string> = {
  Easy: 'var(--green)',
  Medium: 'var(--amber)',
  Hard: 'var(--red)',
}

/**
 * The left edge. A solved problem is edged in green whatever its difficulty:
 * down a long list, "what is left" is the question being asked.
 */
function edgeColor(status: ProgressStatus, problem: CatalogProblem): string {
  if (status === 'solved') return 'var(--green)'
  if (status === 'attempted') return 'var(--amber)'
  return problem.viz ? DIFFICULTY_COLOR[problem.difficulty] : 'var(--border)'
}

function glyph(status: ProgressStatus, viz: boolean): { text: string; color: string } {
  if (status === 'solved') return { text: '✓', color: 'var(--green)' }
  if (status === 'attempted') return { text: '◐', color: 'var(--amber)' }
  return viz ? { text: '▶', color: 'var(--accent)' } : { text: '▷', color: 'var(--text-dim)' }
}

const STATUS_WORDS: Partial<Record<ProgressStatus, string>> = {
  solved: 'solved',
  attempted: 'attempted',
}

export interface ProblemRowProps {
  problem: CatalogProblem
  status: ProgressStatus
  favourite: boolean
  /** Premium, and this viewer is not entitled to it. */
  locked: boolean
  onToggleFavourite: (slug: string, favourite: boolean) => void
}

export const ProblemRow = memo(function ProblemRow({
  problem,
  status,
  favourite,
  locked,
  onToggleFavourite,
}: ProblemRowProps) {
  const g = glyph(status, problem.viz)
  const langs = problem.langs.length
  const words = STATUS_WORDS[status]
  return (
    <div
      className={clsx(styles.row, status === 'solved' && styles.solved, !problem.viz && styles.plain)}
      style={{ ['--edge' as string]: edgeColor(status, problem) }}
    >
      <span className={styles.glyph} style={{ color: g.color }} aria-hidden>
        {g.text}
      </span>
      <Link to={`/problems/${encodeURIComponent(problem.slug)}`} className={styles.rowTitle}>
        {problem.title}
        {words && <span className="visually-hidden"> ({words})</span>}
      </Link>
      <span className={styles.meta}>
        {locked && (
          <span className={styles.lock} title="Part of Pro — the statement is free, the debugger and code need Pro">
            <span aria-hidden>🔒</span> Pro
          </span>
        )}
        {problem.viz ? (
          <span className={styles.badge}>
            interactive{langs > 0 && ` · ${langs} ${langs === 1 ? 'lang' : 'langs'}`}
          </span>
        ) : (
          <span className={styles.badgeDim} title="No animation yet — the statement and practice editor are there">
            not animated
          </span>
        )}
        <DifficultyPill difficulty={problem.difficulty} />
        <button
          type="button"
          className={styles.star}
          aria-pressed={favourite}
          aria-label={favourite ? `Remove ${problem.title} from favourites` : `Add ${problem.title} to favourites`}
          title={favourite ? 'Remove from favourites' : 'Add to favourites'}
          onClick={() => onToggleFavourite(problem.slug, favourite)}
        >
          <span aria-hidden>{favourite ? '★' : '☆'}</span>
        </button>
      </span>
    </div>
  )
})
