// The 🤖 AI assist panel — a port of `crates/dsa-app/src/assistant.rs`, talking
// to the hosted model through `POST /ai/chat` instead of a local Ollama.
//
// Interview never hands over code, Guide answers exactly what was asked and
// offers clickable choices when the question is vague, and Fix reviews the
// current solution and shows its correction as a diff against what the user
// wrote. The server builds the prompts; this panel owns the conversations
// (store.ts), the rendering of replies, and the rule that the editor changes
// only when the user applies a fix.

import {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
} from 'react'
import { Link, useLocation } from 'react-router'
import { useQueryClient } from '@tanstack/react-query'
import clsx from 'clsx'
import { errorMessage } from '@/api/client'
import { qk, useAiStatus } from '@/api/hooks'
import type { AiStatus, Problem } from '@/api/types'
import { nextParam } from '@/app/guards'
import { useSession } from '@/state/session'
import { Seg, Spinner } from '@/ui'
import type { ChatTurn } from './conversation'
import { FixView } from './FixView'
import { MODE_ORDER, MODES, type ChatMode } from './modes'
import {
  assistantStore,
  chatKey,
  fixKey,
  useAssistant,
  type ActiveStream,
  type AssistantState,
  type StreamHooks,
} from './store'
import { ChatView } from './Transcript'
import styles from './AssistantPanel.module.css'

export interface AssistantPanelProps {
  problem: Problem
  /** Active language id. */
  lang: string
  /** Current editor contents (attached only when the user ticks "attach code"). */
  code: string
  /** Latest run/test output as plain text, so Fix mode can see the errors. */
  runContext: string
  /**
   * Replace the editor's contents — Fix mode calls this with the user's code
   * after applying the accepted hunks of the proposed diff.
   */
  onApplyCode: (code: string) => void
  onClose: () => void
}

export function AssistantPanel(props: AssistantPanelProps) {
  const session = useSession()
  const authed = session.status === 'authenticated'
  const userId = session.user?.id ?? null
  const status = useAiStatus(authed)
  const state = useAssistant()

  // The conversations belong to whoever is signed in on this tab.
  useEffect(() => {
    if (userId) assistantStore.adopt(userId)
  }, [userId])

  const ai = authed ? status.data : undefined
  let body: ReactNode
  if (session.status === 'loading') {
    body = <Loading />
  } else if (!authed) {
    body = <SignedOut />
  } else if (status.isError) {
    body = (
      <Notice title="Can't reach AI assist right now">
        <p>{errorMessage(status.error)}</p>
        <button type="button" className="mini-btn" onClick={() => void status.refetch()}>
          ↻ try again
        </button>
      </Notice>
    )
  } else if (!ai || state.owner !== userId) {
    body = <Loading />
  } else if (!ai.enabled) {
    body = (
      <Notice title="AI assist isn't available on this server">
        <p>Everything else — running your code, the tests and the visualizer — works as usual.</p>
      </Notice>
    )
  } else if (ai.daily_limit <= 0) {
    body = (
      <Notice title="AI assist isn't included in your plan">
        <p>Upgrade to get an interviewer, a mentor and a code reviewer on every problem.</p>
        <Link className="btn btn-primary" to="/pricing">
          See plans
        </Link>
      </Notice>
    )
  } else {
    body = <Workspace {...props} state={state} />
  }

  return (
    <aside className={styles.panel} aria-label="AI assist">
      <Header ai={ai} failed={authed && status.isError} onClose={props.onClose} />
      {body}
    </aside>
  )
}

function Header({ ai, failed, onClose }: { ai: AiStatus | undefined; failed: boolean; onClose: () => void }) {
  const live = ai?.enabled === true && ai.daily_limit > 0
  const remaining = ai ? Math.max(0, ai.daily_limit - ai.used_today) : 0
  return (
    <header className={styles.header}>
      <div className={styles.titleRow}>
        <h2 className={styles.title}>🤖 AI assist</h2>
        {failed ? (
          <span className={styles.state}>○ offline</span>
        ) : ai ? (
          <span className={clsx(styles.state, live && styles.online)}>{live ? '● online' : '○ unavailable'}</span>
        ) : null}
        <button type="button" className="mini-btn" onClick={onClose} aria-label="Close AI assist" title="Close">
          ✕
        </button>
      </div>
      {live && (
        <div className={styles.meta}>
          <span className={styles.model} title={`${ai.provider} · ${ai.model}`}>
            {ai.provider} · {ai.model}
          </span>
          <span className={clsx(styles.quota, remaining === 0 && styles.quotaOut)}>
            {remaining} of {ai.daily_limit} left today
          </span>
        </div>
      )}
    </header>
  )
}

function Loading() {
  return (
    <div className={styles.center}>
      <Spinner label="Connecting to AI assist" />
    </div>
  )
}

function Notice({ title, children }: { title: string; children?: ReactNode }) {
  return (
    <div className={styles.notice}>
      <p className={styles.noticeTitle}>{title}</p>
      {children}
    </div>
  )
}

function SignedOut() {
  const location = useLocation()
  return (
    <Notice title="Sign in to use AI assist">
      <p>An interviewer, a mentor and a code reviewer who know this problem.</p>
      <div className={styles.noticeActions}>
        <Link className="btn btn-primary" to={`/login?next=${nextParam(location)}`}>
          Sign in
        </Link>
        <Link className="btn btn-ghost" to="/signup">
          Create an account
        </Link>
      </div>
    </Notice>
  )
}

const NO_TURNS: readonly ChatTurn[] = []

function Workspace({
  problem,
  lang,
  code,
  runContext,
  onApplyCode,
  state,
}: AssistantPanelProps & { state: AssistantState }) {
  const slug = problem.slug
  const mode = state.mode
  const chatMode: ChatMode | null = mode === 'fix' ? null : mode
  const { refresh } = useSession()
  const qc = useQueryClient()
  const [draft, setDraft] = useState('')
  const inputRef = useRef<HTMLTextAreaElement>(null)
  const scrollRef = useRef<HTMLDivElement>(null)
  const inputId = useId()

  const hooks = useMemo<StreamHooks>(
    () => ({
      onAuthIssue: () => void refresh(),
      onQuota: (remaining) => {
        if (remaining === null) void qc.invalidateQueries({ queryKey: qk.aiStatus })
        else
          qc.setQueryData<AiStatus>(qk.aiStatus, (s) =>
            s ? { ...s, used_today: Math.max(0, s.daily_limit - remaining) } : s,
          )
      },
    }),
    [qc, refresh],
  )

  const visibleKey = chatMode ? chatKey(slug, chatMode) : fixKey(slug, lang)
  const turns = chatMode ? (state.chats[visibleKey] ?? NO_TURNS) : NO_TURNS
  const run = chatMode ? null : (state.fixes[visibleKey] ?? null)
  const active = state.active
  const busy = active !== null
  const busyHere = active?.key === visibleKey
  const pin = useStickToBottom(scrollRef, chatMode ? turns : run, visibleKey)

  const request = { slug, lang, code, runContext }
  const send = (text: string): boolean => {
    if (!chatMode) return false
    pin()
    return assistantStore.sendChat(text, { ...request, mode: chatMode }, hooks)
  }
  const analyze = (note: string): boolean => {
    pin()
    return assistantStore.analyze({ ...request, note }, hooks)
  }
  const canSend = !busy && (chatMode ? draft.trim() !== '' : code.trim() !== '')
  const submit = () => {
    if (canSend && (chatMode ? send(draft) : analyze(draft))) setDraft('')
  }
  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    // Enter sends and Shift+Enter makes a newline; an Enter that confirms an
    // IME composition is neither.
    if (e.key !== 'Enter' || e.shiftKey || e.nativeEvent.isComposing) return
    e.preventDefault()
    submit()
  }

  // Stable, so a streaming reply re-renders itself and not the whole transcript.
  const onOption = useEvent((text: string) => {
    if (send(text)) inputRef.current?.focus()
  })
  const onRetryChat = useEvent(() => {
    if (!chatMode) return
    pin()
    assistantStore.retryChat({ ...request, mode: chatMode }, hooks)
  })

  const hasContent = chatMode ? turns.length > 0 : run !== null
  const clear = () => (chatMode ? assistantStore.clearChat(slug, chatMode) : assistantStore.clearFix(slug, lang))

  return (
    <>
      <div className={styles.modeRow}>
        <Seg
          value={mode}
          options={MODE_ORDER.map((m) => ({ value: m, label: MODES[m].tab }))}
          onChange={(m) => assistantStore.setMode(m)}
          aria-label="Assistant mode"
          className={styles.modes}
        />
        <button
          type="button"
          className="mini-btn"
          onClick={clear}
          disabled={!hasContent || busyHere}
          aria-label={chatMode ? 'Clear this conversation' : 'Clear this review'}
          title="Start over"
        >
          ↺
        </button>
      </div>

      <div
        ref={scrollRef}
        className={styles.scroll}
        role="log"
        aria-live="polite"
        aria-busy={busyHere}
        aria-label={`${MODES[mode].label} conversation`}
      >
        {chatMode ? (
          <ChatView mode={chatMode} turns={turns} busy={busy} onOption={onOption} onRetry={onRetryChat} />
        ) : (
          <FixView
            run={run}
            code={code}
            busy={busy}
            onRetry={() => run && analyze(run.note)}
            onAccepted={(accepted) => assistantStore.setAccepted(slug, lang, accepted)}
            onResolve={(resolution) => assistantStore.resolveFix(slug, lang, resolution)}
            onApply={onApplyCode}
          />
        )}
      </div>

      <form
        className={styles.composer}
        onSubmit={(e) => {
          e.preventDefault()
          submit()
        }}
      >
        <div className={styles.composerRow}>
          <label className="check" title={chatMode ? undefined : 'Fix always reviews your code'}>
            <input
              type="checkbox"
              checked={!chatMode || state.attach}
              disabled={!chatMode}
              onChange={(e) => assistantStore.setAttach(e.target.checked)}
            />
            📎 attach my code
          </label>
          {active && !busyHere && (
            <span className={styles.elsewhere}>
              ⏳ {describe(active, slug)} is still answering
              <button type="button" className="mini-btn" onClick={() => assistantStore.stop()}>
                ■ stop
              </button>
            </span>
          )}
        </div>
        <div className={styles.inputRow}>
          <label htmlFor={inputId} className="visually-hidden">
            {chatMode ? `Message the ${MODES[chatMode].speaker}` : 'Anything for the reviewer to focus on (optional)'}
          </label>
          <textarea
            id={inputId}
            ref={inputRef}
            className={clsx('input', styles.textarea)}
            rows={2}
            value={draft}
            placeholder={MODES[mode].placeholder}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={onKeyDown}
          />
          {busyHere ? (
            <button type="button" className={clsx('btn btn-ghost', styles.send)} onClick={() => assistantStore.stop()}>
              ■ stop
            </button>
          ) : (
            <button
              type="submit"
              className={clsx('btn btn-primary', styles.send)}
              disabled={!canSend}
              aria-label={chatMode ? 'Send' : 'Analyze my code'}
            >
              {chatMode ? '➤' : '🔍 Analyze'}
            </button>
          )}
        </div>
        {!chatMode && (
          <p className={styles.tip}>
            {runContext.trim() !== ''
              ? "Your code and your last run's output go with it."
              : 'Run your code or tests first so the AI sees the errors too.'}
          </p>
        )}
      </form>
    </>
  )
}

/** Where the reply that is blocking this conversation is being written. */
function describe(active: ActiveStream, slug: string): string {
  const what = active.kind === 'fix' ? `Fix (${active.lang})` : MODES[active.mode].label
  return active.slug === slug ? what : `${what}, on another problem,`
}

/**
 * Follow a streaming reply while the reader is at the bottom, and leave them be
 * once they scroll up to reread something. Returns `pin`, which re-attaches —
 * for when the user sends, and plainly wants to see the answer.
 */
function useStickToBottom(ref: RefObject<HTMLElement | null>, content: unknown, conversation: string): () => void {
  const stick = useRef(true)
  useEffect(() => {
    const el = ref.current
    if (!el) return
    const onScroll = () => {
      stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 40
    }
    el.addEventListener('scroll', onScroll, { passive: true })
    return () => el.removeEventListener('scroll', onScroll)
  }, [ref])
  // Opening another conversation starts at its latest message.
  useLayoutEffect(() => {
    stick.current = true
  }, [conversation])
  useLayoutEffect(() => {
    const el = ref.current
    if (el && stick.current) el.scrollTop = el.scrollHeight
  }, [ref, content, conversation])
  return useCallback(() => {
    stick.current = true
  }, [])
}

/** A callback with a stable identity that always sees the latest render. */
function useEvent<A extends unknown[]>(fn: (...args: A) => void): (...args: A) => void {
  const latest = useRef(fn)
  useLayoutEffect(() => {
    latest.current = fn
  })
  return useCallback((...args: A) => latest.current(...args), [])
}
