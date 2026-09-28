// The smaller pieces of the Visualize tab: the "practice first" gate, the
// note bar above the canvas and the Problem/Approach card below it.

import { memo } from 'react'
import clsx from 'clsx'
import type { Problem } from '@/api/types'
import type { Step } from '@/trace/types'
import ui from '../shared/ui.module.css'
import styles from './Visualize.module.css'

/**
 * Opening a problem lands on Practice, and the walkthrough stays behind an
 * explicit reveal, so clicking a problem never spoils it (`Visualize::gate`).
 */
export function Gate({
  langLabel,
  onPractice,
  onReveal,
}: {
  langLabel: string
  onPractice: () => void
  onReveal: () => void
}) {
  return (
    <div className={styles.gateWrap}>
      <section className={clsx(ui.card, styles.gate)} aria-labelledby="viz-gate-title">
        <h2 id="viz-gate-title">🎓 Practice first</h2>
        <p>
          This tab plays the full solution step by step — reading it before trying robs you of the struggle that
          makes it stick. Attempt it yourself in Practice: write real {langLabel}, run it against the tests, get
          stuck, think.
        </p>
        <p className={styles.gateHint}>
          Stuck on the underlying data structure? The 📘 helper explains it (with syntax and complexity) without
          spoiling this problem.
        </p>
        <div className={styles.gateActions}>
          <button type="button" className={ui.apply} onClick={onPractice}>
            ✏ Let me solve it first
          </button>
          <button type="button" className="mini-btn" onClick={onReveal}>
            👀 I tried / I&apos;m stuck — show the walkthrough
          </button>
        </div>
      </section>
    </div>
  )
}

const EVENT_GLYPH = { call: '⤵', return: '⤴', stmt: '·' } as const

/** The current step's narration, with the call/return glyph in its colour. */
export const NoteBar = memo(function NoteBar({ step, breakpoint }: { step: Step | null; breakpoint: boolean }) {
  const event = step?.event ?? 'stmt'
  return (
    <div className={styles.note} aria-live="polite">
      <span className={clsx(styles.noteGlyph, styles[`ev_${event}`])} aria-hidden>
        {EVENT_GLYPH[event]}
      </span>
      <span>{step?.note ?? ''}</span>
      {breakpoint && <span className={clsx(ui.pill, ui.red)}>● breakpoint</span>}
    </div>
  )
})

export const About = memo(function About({ problem }: { problem: Problem }) {
  return (
    <section className={clsx(ui.card, styles.about)} aria-label="About this problem">
      <h3>Problem.</h3>
      <p>{problem.description}</p>
      <h3>Approach.</h3>
      <p>{problem.approach}</p>
    </section>
  )
})
