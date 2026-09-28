import { memo } from 'react'
import { Link } from 'react-router'
import clsx from 'clsx'
import type { Problem } from '@/api/types'
import { DifficultyPill } from '@/ui'
import ui from '../shared/ui.module.css'
import { exampleText } from './format'
import styles from './Practice.module.css'

/**
 * The LeetCode-style statement beside the editor (`question_panel`). Also the
 * whole content of a locked problem, whose statement is always free to read.
 */
export const QuestionPanel = memo(function QuestionPanel({
  problem,
  visualizeHref,
}: {
  problem: Problem
  /** Where the closing hint points; null hides it (no walkthrough to switch to). */
  visualizeHref: string | null
}) {
  return (
    <article className={styles.question} aria-label="Problem statement">
      <div className={styles.qTitle}>
        <h2>{problem.title}</h2>
        <DifficultyPill difficulty={problem.difficulty} />
      </div>
      <div className={styles.qCategory}>{problem.category}</div>
      {/* The one thing on this screen read as prose rather than scanned, so it
          is the one thing set at full contrast and a comfortable size. */}
      <p className={styles.qDescription}>{problem.description}</p>
      {problem.complexity.trim() && <p className={styles.qComplexity}>{problem.complexity}</p>}
      {problem.tests.map((test, i) => (
        <section key={test.name || i} className={styles.example}>
          <h3 className={clsx('section-label', ui.head)}>Example {i + 1}</h3>
          <pre className={clsx(ui.codeBlock, styles.exampleBlock)}>{exampleText(problem, test)}</pre>
        </section>
      ))}
      {visualizeHref && (
        <p className={styles.qHint}>
          💡 switch to <Link to={visualizeHref}>⏵ Visualize</Link> for the animated step-by-step walkthrough of
          this problem.
        </p>
      )}
    </article>
  )
})
