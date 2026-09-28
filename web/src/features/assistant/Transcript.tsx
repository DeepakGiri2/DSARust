// The Interview and Guide transcripts, and the pieces every reply shares:
// the collapsed thinking block and the inline error.

import { memo, useEffect, useMemo, useState } from 'react'
import { Link, useLocation } from 'react-router'
import clsx from 'clsx'
import { nextParam } from '@/app/guards'
import type { AssistantTurn, ChatTurn, UserTurn } from './conversation'
import { errorCopy, type AssistError } from './events'
import { MODES, type ChatMode } from './modes'
import { extractThink, parseOptions } from './parse'
import { RichText } from './RichText'
import styles from './AssistantPanel.module.css'

export interface ChatViewProps {
  mode: ChatMode
  turns: readonly ChatTurn[]
  /** A reply is streaming somewhere, so nothing new can be sent. */
  busy: boolean
  onOption: (text: string) => void
  onRetry: () => void
}

export function ChatView({ mode, turns, busy, onOption, onRetry }: ChatViewProps) {
  if (turns.length === 0) return <p className={styles.empty}>{MODES[mode].emptyHint}</p>
  return turns.map((turn, i) =>
    turn.role === 'user' ? (
      <Question key={turn.id} turn={turn} />
    ) : (
      <Reply
        key={turn.id}
        turn={turn}
        mode={mode}
        last={i === turns.length - 1}
        busy={busy}
        onOption={onOption}
        onRetry={onRetry}
      />
    ),
  )
}

const Question = memo(function Question({ turn }: { turn: UserTurn }) {
  return (
    <article className={styles.turn}>
      <span className={clsx(styles.speaker, styles.you)}>you</span>
      <div className={styles.userBody}>
        <RichText text={turn.content} />
      </div>
      {turn.codeAttached && <span className="chip">📎 code attached</span>}
    </article>
  )
})

interface ReplyProps {
  turn: AssistantTurn
  mode: ChatMode
  last: boolean
  busy: boolean
  onOption: (text: string) => void
  onRetry: () => void
}

const Reply = memo(function Reply({ turn, mode, last, busy, onOption, onRetry }: ReplyProps) {
  const { thinking: inlineThinking, body } = useMemo(() => extractThink(turn.content), [turn.content])
  // Guide mode turns OPTION: lines into choices; elsewhere they are just text.
  const { body: text, options } = useMemo(
    () => (mode === 'guide' ? parseOptions(body) : { body, options: [] }),
    [mode, body],
  )
  const thinking = (turn.thinking + inlineThinking).trim()
  const streaming = turn.status === 'streaming'
  // Choices belong to the latest reply only, once it has finished arriving.
  const choices = last && !streaming ? options.filter((o) => o !== '') : []

  return (
    <article className={styles.turn}>
      <span className={styles.speaker}>{MODES[mode].speaker}</span>
      {thinking && <Thinking text={thinking} live={streaming && body === ''} />}
      {text && <RichText text={text} lockCode={mode === 'interview'} />}
      {streaming && (
        <span className={styles.cursor} aria-hidden>
          ▍
        </span>
      )}
      {turn.status === 'stopped' && <span className={styles.note}>■ stopped</span>}
      {turn.error && <ErrorNote error={turn.error} busy={busy} onRetry={last ? onRetry : undefined} />}
      {choices.length > 0 && (
        <div className={styles.options} role="group" aria-label="Pick one to reply">
          {choices.map((option, i) => (
            <button
              key={i}
              type="button"
              className={styles.option}
              disabled={busy}
              onClick={() => onOption(option)}
            >
              {option}
            </button>
          ))}
        </div>
      )}
    </article>
  )
})

/** Reasoning, collapsed: it is how the model got there, not the answer. */
export function Thinking({ text, live }: { text: string; live: boolean }) {
  return (
    <details className={styles.think}>
      <summary>💭 thinking{live ? '…' : ''}</summary>
      <div className={styles.thinkBody}>{text}</div>
    </details>
  )
}

/** Seconds until `until` (epoch ms), ticking while it is in the future. */
function useSecondsUntil(until: number | null): number {
  const [now, setNow] = useState(Date.now)
  const left = until === null ? 0 : Math.ceil((until - now) / 1000)
  useEffect(() => {
    if (left <= 0) return
    const tick = setTimeout(() => setNow(Date.now()), 1000)
    return () => clearTimeout(tick)
  }, [left])
  return Math.max(0, left)
}

export function ErrorNote({
  error,
  busy,
  onRetry,
}: {
  error: AssistError
  busy: boolean
  /** Offered only where a retry makes sense (the latest reply). */
  onRetry?: () => void
}) {
  const { text, action } = errorCopy(error)
  const wait = useSecondsUntil(error.retryAt)
  const location = useLocation()
  return (
    <div className={styles.error}>
      <span className={styles.errorText}>
        ⚠ {text}
        {wait > 0 && ` Try again in ${wait}s.`}
      </span>
      {action === 'retry' && onRetry && (
        <button type="button" className="mini-btn" onClick={onRetry} disabled={busy || wait > 0}>
          ↻ retry
        </button>
      )}
      {action === 'pricing' && (
        <Link className="link" to="/pricing">
          see plans →
        </Link>
      )}
      {action === 'signin' && (
        <Link className="link" to={`/login?next=${nextParam(location)}`}>
          sign in →
        </Link>
      )}
      {action === 'account' && (
        <Link className="link" to="/account">
          account →
        </Link>
      )}
    </div>
  )
}
