// STDIN, OUTPUT and TESTS — the desktop's `io_panel`, one section each.

import { Link } from 'react-router'
import clsx from 'clsx'
import type { Problem, RunResult, TestResult } from '@/api/types'
import ui from '../shared/ui.module.css'
import { TEST_BADGE, compiled, inlineStdin, runErrorText, truncate, type Tone } from './format'
import type { RunNotice } from './runNotice'
import styles from './Practice.module.css'

const pill = (tone: Tone) => clsx(ui.pill, ui[tone])

export interface IoPanelProps {
  problem: Problem
  langLabel: string
  stdin: string
  onStdin: (text: string) => void
  /** A run or test sweep is in flight. */
  running: boolean
  runResult: RunResult | null
  testResult: RunResult | null
  notice: RunNotice | null
  /** Seconds left on a rate-limit cooldown. */
  cooldown: number
  /** `?next=` for the sign-in links. */
  next: string
  /** The last sweep ticked the problem off. */
  solvedNow: boolean
}

export function IoPanel(p: IoPanelProps) {
  return (
    <div className={styles.io}>
      <h2 className={clsx('section-label', ui.head)}>
        <label htmlFor="practice-stdin">stdin</label>
      </h2>
      <textarea
        id="practice-stdin"
        className={clsx('input mono', styles.stdin)}
        value={p.stdin}
        onChange={(e) => p.onStdin(e.target.value)}
        rows={3}
        spellCheck={false}
        autoComplete="off"
        aria-describedby="practice-stdin-hint"
      />
      <span id="practice-stdin-hint" className="visually-hidden">
        Sent to your program on standard input when you press Run: one input per line.
      </span>

      <h2 className={clsx('section-label', ui.head)}>output</h2>
      <div className={styles.output}>
        {p.notice && <NoticeView notice={p.notice} cooldown={p.cooldown} next={p.next} />}
        {p.running ? (
          <p className={ui.empty}>⏳ compiling &amp; running…</p>
        ) : p.runResult ? (
          <RunOutput result={p.runResult} />
        ) : (
          !p.notice && (
            <p className={ui.empty}>press ▶ Run — code executes on a real {p.langLabel} toolchain</p>
          )
        )}
      </div>

      <h2 className={clsx('section-label', ui.head, styles.testsHead)}>
        tests
        {p.testResult && p.testResult.total > 0 && (
          <span
            className={clsx(
              pill(p.testResult.passed === p.testResult.total ? 'green' : 'red'),
              p.solvedNow && styles.celebrate,
            )}
          >
            {p.testResult.passed === p.testResult.total && '✔ '}
            {p.testResult.passed}/{p.testResult.total} passed{p.solvedNow && ' · solved'}
          </span>
        )}
      </h2>
      <TestList problem={p.problem} outcomes={p.testResult?.tests} />
    </div>
  )
}

function NoticeView({ notice, cooldown, next }: { notice: RunNotice; cooldown: number; next: string }) {
  if (notice.kind === 'signin' || notice.kind === 'profile') {
    const signin = notice.kind === 'signin'
    return (
      <div className={ui.notice}>
        <span>
          {signin
            ? 'Sign in to run code. Your solution runs on a real toolchain, and passing every test ticks the problem off.'
            : 'Choose who is practising to run code — runs and progress belong to a profile.'}
        </span>
        <div className={ui.noticeActions}>
          {signin ? (
            <>
              <Link className={ui.apply} to={`/login?next=${next}`}>
                Sign in
              </Link>
              <Link className="mini-btn" to={`/signup?next=${next}`}>
                Create a free account
              </Link>
            </>
          ) : (
            <Link className={ui.apply} to={`/profiles?next=${next}`}>
              Pick a profile
            </Link>
          )}
        </div>
      </div>
    )
  }
  return (
    <div className={clsx(ui.notice, ui.noticeError)}>
      <span>
        {notice.text}
        {cooldown > 0 && ` Try again in ${cooldown}s.`}
      </span>
      {notice.details && (
        <ul className={ui.noticeList}>
          {notice.details.map((d) => (
            <li key={d}>{d}</li>
          ))}
        </ul>
      )}
      {notice.link && (
        <div className={ui.noticeActions}>
          <Link className="mini-btn" to={notice.link.to}>
            {notice.link.label} →
          </Link>
        </div>
      )}
    </div>
  )
}

/** One Run's output, exactly the desktop's order: build output, stdout, stderr, exit code and time. */
function RunOutput({ result }: { result: RunResult }) {
  if (result.status === 'error') {
    return <pre className={clsx(ui.codeBlock, styles.red)}>{runErrorText(result)}</pre>
  }
  const built = compiled(result)
  const exitOk = result.exit_code === 0
  return (
    <div className={styles.runOutput}>
      {result.compile && result.compile.output.trim() !== '' && (
        <>
          <span className={pill(built ? 'dim' : 'red')}>{built ? 'build output' : 'compile error'}</span>
          <pre className={clsx(ui.codeBlock, built ? styles.dimText : styles.red)}>{result.compile.output}</pre>
        </>
      )}
      {built && (
        <>
          {result.stdout?.trim() && <pre className={ui.codeBlock}>{result.stdout.trimEnd()}</pre>}
          {result.stderr?.trim() && (
            <>
              <span className={pill('red')}>stderr</span>
              <pre className={clsx(ui.codeBlock, styles.red)}>{result.stderr.trimEnd()}</pre>
            </>
          )}
          <div className={styles.exitRow}>
            <span className={pill(exitOk ? 'green' : 'red')}>exit code {result.exit_code ?? '—'}</span>
            {result.timed_out && <span className={pill('amber')}>timed out</span>}
            <span className={styles.duration}>{(result.duration_ms / 1000).toFixed(1)}s</span>
          </div>
        </>
      )}
    </div>
  )
}

function TestList({ problem, outcomes }: { problem: Problem; outcomes: TestResult[] | undefined }) {
  if (problem.tests.length === 0) return <p className={ui.empty}>This problem has no test cases.</p>
  return (
    <ol className={styles.tests}>
      {problem.tests.map((c, i) => {
        const o = outcomes?.[i]
        const badge = o ? TEST_BADGE[o.status] : null
        return (
          <li key={c.name || i} className={clsx(styles.case, badge && styles[`case_${badge.tone}`])}>
            <span
              className={clsx(styles.badge, badge ? styles[`text_${badge.tone}`] : styles.dimText)}
              title={badge?.label ?? 'not run yet'}
            >
              {badge?.glyph ?? '·'}
              <span className="visually-hidden">{badge ? `: ${badge.label}` : ': not run yet'}</span>
            </span>
            <div className={styles.caseBody}>
              <div className={styles.caseIn}>
                in: {inlineStdin(c.stdin)}
                {c.edge && <span className={clsx(pill('amber'), styles.edge)}>edge</span>}
              </div>
              <div className={styles.caseWant}>
                want: <code>{c.expected}</code>
                {o?.status === 'fail' && (
                  <>
                    {' '}
                    · got: <code className={styles.red}>{o.actual || '(nothing)'}</code>
                  </>
                )}
              </div>
              {o && o.detail.trim() !== '' && <pre className={styles.detail}>{truncate(o.detail, 400)}</pre>}
            </div>
          </li>
        )
      })}
    </ol>
  )
}
