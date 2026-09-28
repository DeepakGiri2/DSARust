// The assistant's conversations, and the one reply being streamed.
//
// This lives outside React, as the desktop's `Assistant` lives outside any one
// frame: closing the panel, switching problem or leaving the page and coming
// back keeps every conversation, and a reply keeps streaming into the
// conversation it was started for (the desktop's `Target`) — never into
// whichever mode happens to be on screen when its tokens arrive.
//
// Conversations are kept per (problem, mode) for the browser session, in
// sessionStorage, and belong to one account: another user signing in on this
// tab starts from nothing.

import { useSyncExternalStore } from 'react'
import { postSse, type SseEvent } from '@/api/sse'
import type { AiChatRequest, AiMode } from '@/api/types'
import {
  FIX_REQUEST,
  finishFix,
  historyFor,
  type AssistantTurn,
  type ChatTurn,
  type FixOutcome,
  type FixResolution,
  type FixRun,
  type ReplyStatus,
  type UserTurn,
} from './conversation'
import {
  DROPPED,
  isAssistError,
  isRecord,
  readDone,
  readError,
  readToken,
  toAssistError,
  type AssistError,
} from './events'
import { MODE_ORDER, type ChatMode } from './modes'

/** `postSse`'s shape — injected so the store can be driven without a network. */
export type Transport = (
  path: string,
  body: unknown,
  onEvent: (e: SseEvent) => void,
  signal?: AbortSignal,
) => Promise<void>

/** What a reply changes outside the panel, wired by whoever sends it. */
export interface StreamHooks {
  /** The session ended (`unauthorized`) or its CSRF token went stale (`csrf`). */
  onAuthIssue?: () => void
  /** Requests left today after a reply; `null` when it changed by an unknown amount. */
  onQuota?: (remaining: number | null) => void
}

export type ActiveStream =
  | { kind: 'chat'; key: string; slug: string; mode: ChatMode }
  | { kind: 'fix'; key: string; slug: string; lang: string }

export interface AssistantState {
  /** The account these conversations belong to. */
  readonly owner: string | null
  readonly mode: AiMode
  /** "attach my code" for Interview and Guide; Fix always sends the code. */
  readonly attach: boolean
  readonly chats: Readonly<Record<string, readonly ChatTurn[]>>
  readonly fixes: Readonly<Record<string, FixRun>>
  /** The one reply being streamed, and where its tokens go. */
  readonly active: ActiveStream | null
}

export const chatKey = (slug: string, mode: ChatMode) => `chat:${slug}:${mode}`
/** A fix is written against one language's solution, so it is kept per language. */
export const fixKey = (slug: string, lang: string) => `fix:${slug}:${lang}`

export interface ChatRequest {
  slug: string
  lang: string
  mode: ChatMode
  code: string
  runContext: string
}

export interface FixRequest {
  slug: string
  lang: string
  /** Optional pointer for the reviewer. */
  note: string
  code: string
  runContext: string
}

const INITIAL: AssistantState = {
  owner: null,
  mode: 'interview',
  attach: false,
  chats: {},
  fixes: {},
  active: null,
}

const STORAGE_KEY = 'dsa.assistant.v1'
const PERSIST_DELAY_MS = 300

interface Running {
  readonly controller: AbortController
  readonly target: ActiveStream
  /** The chat turn or fix run the tokens belong to. */
  readonly id: string
  settled: boolean
}

type Ending = { status: 'done' } | { status: 'stopped' } | { status: 'error'; error: AssistError }

export class AssistantStore {
  private state: AssistantState
  private readonly listeners = new Set<() => void>()
  private running: Running | null = null
  private persistTimer: ReturnType<typeof setTimeout> | null = null
  private readonly transport: Transport
  private readonly storage: Storage | null
  private readonly now: () => number

  constructor(deps: { transport: Transport; storage?: Storage | null; now?: () => number }) {
    this.transport = deps.transport
    this.storage = deps.storage ?? null
    this.now = deps.now ?? Date.now
    this.state = this.load()
    // A closing tab must not lose the tokens that arrived since the last write.
    if (this.storage) window.addEventListener('pagehide', () => this.flush())
  }

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  getState = (): AssistantState => this.state

  /** Hand the store to an account. Another account's conversations — and any reply in flight — go. */
  adopt(owner: string): void {
    if (this.state.owner === owner) return
    this.stop()
    this.commit({ ...INITIAL, owner })
  }

  setMode(mode: AiMode): void {
    if (mode !== this.state.mode) this.commit({ ...this.state, mode })
  }

  setAttach(attach: boolean): void {
    if (attach !== this.state.attach) this.commit({ ...this.state, attach })
  }

  /** Ask the next question. Refused while any reply is streaming: one at a time, as on the desktop. */
  sendChat(text: string, req: ChatRequest, hooks: StreamHooks = {}): boolean {
    const content = text.trim()
    if (content === '' || this.state.active) return false
    const key = chatKey(req.slug, req.mode)
    const asked: UserTurn = {
      id: newId(),
      role: 'user',
      content,
      codeAttached: this.state.attach && req.code.trim() !== '',
    }
    this.startChat(key, this.state.chats[key] ?? [], asked, req, hooks)
    return true
  }

  /** Ask the last question again, in place of the reply that failed. */
  retryChat(req: ChatRequest, hooks: StreamHooks = {}): boolean {
    if (this.state.active) return false
    const key = chatKey(req.slug, req.mode)
    const turns = this.state.chats[key] ?? []
    const failed = turns.at(-1)
    const asked = turns.at(-2)
    if (failed?.role !== 'assistant' || failed.status !== 'error' || asked?.role !== 'user') return false
    this.startChat(key, turns.slice(0, -2), asked, req, hooks)
    return true
  }

  private startChat(
    key: string,
    before: readonly ChatTurn[],
    asked: UserTurn,
    req: ChatRequest,
    hooks: StreamHooks,
  ): void {
    const turns = [...before, asked]
    const reply: AssistantTurn = {
      id: newId(),
      role: 'assistant',
      content: '',
      thinking: '',
      status: 'streaming',
      error: null,
    }
    this.commit({ ...this.state, chats: { ...this.state.chats, [key]: [...turns, reply] } })

    // Only the turn being sent carries the code snapshot; the server attaches
    // it to that turn, which keeps the context small.
    const attach = asked.codeAttached && req.code.trim() !== ''
    const request: AiChatRequest = {
      slug: req.slug,
      lang: req.lang,
      mode: req.mode,
      messages: historyFor(turns),
      ...(attach ? { code: req.code } : {}),
      // Run output only means something next to the code that produced it.
      ...(attach && req.runContext.trim() !== '' ? { run_context: req.runContext } : {}),
    }
    this.run({ kind: 'chat', key, slug: req.slug, mode: req.mode }, reply.id, request, hooks)
  }

  /**
   * Review the current code. Fix always sends the code and the last run, and
   * each analysis replaces the previous one for that problem and language.
   */
  analyze(req: FixRequest, hooks: StreamHooks = {}): boolean {
    if (this.state.active || req.code.trim() === '') return false
    const key = fixKey(req.slug, req.lang)
    const note = req.note.trim()
    const run: FixRun = {
      id: newId(),
      note,
      snapshot: req.code,
      content: '',
      thinking: '',
      status: 'streaming',
      error: null,
      outcome: null,
      accepted: null,
      resolution: { kind: 'open' },
    }
    this.commit({ ...this.state, fixes: { ...this.state.fixes, [key]: run } })
    const request: AiChatRequest = {
      slug: req.slug,
      lang: req.lang,
      mode: 'fix',
      messages: [{ role: 'user', content: note || FIX_REQUEST }],
      code: req.code,
      run_context: req.runContext,
    }
    this.run({ kind: 'fix', key, slug: req.slug, lang: req.lang }, run.id, request, hooks)
    return true
  }

  /** Stop the reply in flight. What already arrived stays. */
  stop(): void {
    if (this.running) this.settle(this.running, { status: 'stopped' })
  }

  clearChat(slug: string, mode: ChatMode): void {
    const key = chatKey(slug, mode)
    if (this.state.active?.key === key || !(key in this.state.chats)) return
    const chats = { ...this.state.chats }
    delete chats[key]
    this.commit({ ...this.state, chats })
  }

  clearFix(slug: string, lang: string): void {
    const key = fixKey(slug, lang)
    if (this.state.active?.key === key || !(key in this.state.fixes)) return
    const fixes = { ...this.state.fixes }
    delete fixes[key]
    this.commit({ ...this.state, fixes })
  }

  /** The ticks of the diff on screen, one per change group. */
  setAccepted(slug: string, lang: string, accepted: readonly boolean[]): void {
    this.commit(withFix(this.state, fixKey(slug, lang), null, () => ({ accepted })))
  }

  /** Applied, discarded, or reopened for review (with every change ticked again). */
  resolveFix(slug: string, lang: string, resolution: FixResolution): void {
    this.commit(
      withFix(this.state, fixKey(slug, lang), null, () =>
        resolution.kind === 'open' ? { resolution, accepted: null } : { resolution },
      ),
    )
  }

  // ── streaming ─────────────────────────────────────────────────────────────

  private run(target: ActiveStream, id: string, request: AiChatRequest, hooks: StreamHooks): void {
    const running: Running = { controller: new AbortController(), target, id, settled: false }
    this.running = running
    this.commit({ ...this.state, active: target })

    const fail = (error: AssistError) => {
      if (error.code === 'unauthorized' || error.code === 'csrf') hooks.onAuthIssue?.()
      // A spent quota (or a plan limit) moved the counter by an unknown amount.
      if (error.code === 'payment_required') hooks.onQuota?.(null)
      this.settle(running, { status: 'error', error })
    }

    this.transport(
      '/ai/chat',
      request,
      (ev) => {
        if (running.settled) return
        if (ev.event === 'token') {
          const token = readToken(ev.data)
          if (token) this.append(running, token.channel, token.text)
        } else if (ev.event === 'done') {
          hooks.onQuota?.(readDone(ev.data)?.remaining_today ?? null)
          this.settle(running, { status: 'done' })
        } else if (ev.event === 'error') {
          fail(readError(ev.data, this.now()))
        }
      },
      running.controller.signal,
    ).then(
      () => {
        if (!running.settled) fail(DROPPED)
      },
      (e: unknown) => {
        if (!running.settled) fail(toAssistError(e, this.now()))
      },
    )
  }

  private append(run: Running, channel: 'content' | 'thinking', text: string): void {
    const { target, id } = run
    this.commit(
      target.kind === 'chat'
        ? withTurn(this.state, target.key, id, (t) =>
            channel === 'content' ? { content: t.content + text } : { thinking: t.thinking + text },
          )
        : withFix(this.state, target.key, id, (f) =>
            channel === 'content' ? { content: f.content + text } : { thinking: f.thinking + text },
          ),
    )
  }

  /** End a reply exactly once, however it ended. */
  private settle(run: Running, ending: Ending): void {
    if (run.settled) return
    run.settled = true
    const status: ReplyStatus = ending.status
    const error = ending.status === 'error' ? ending.error : null
    const { target, id } = run

    let next =
      target.kind === 'chat'
        ? withTurn(this.state, target.key, id, () => ({ status, error }))
        : withFix(this.state, target.key, id, (f) => ({
            status,
            error,
            // A finished fix is parsed exactly once, here.
            outcome: status === 'done' ? finishFix(f.content, f.snapshot) : null,
          }))
    if (this.running === run) {
      this.running = null
      next = { ...next, active: null }
    }
    this.commit(next)
    // After `done` or `error` this closes the connection at once; for a stop it
    // is what tells the server to stop generating.
    run.controller.abort()
  }

  // ── state and persistence ─────────────────────────────────────────────────

  private commit(next: AssistantState): void {
    if (next === this.state) return
    this.state = next
    this.listeners.forEach((l) => l())
    this.persistSoon()
  }

  private persistSoon(): void {
    if (!this.storage || this.persistTimer !== null) return
    this.persistTimer = setTimeout(() => this.flush(), PERSIST_DELAY_MS)
  }

  private flush(): void {
    if (this.persistTimer !== null) clearTimeout(this.persistTimer)
    this.persistTimer = null
    if (!this.storage) return
    const { owner, mode, attach, chats, fixes } = this.state
    const saved: Saved = { v: 1, owner, mode, attach, chats, fixes }
    try {
      this.storage.setItem(STORAGE_KEY, JSON.stringify(saved))
    } catch {
      // Full or blocked storage: the conversations last as long as the page.
    }
  }

  private load(): AssistantState {
    if (!this.storage) return INITIAL
    try {
      const raw = this.storage.getItem(STORAGE_KEY)
      const saved: unknown = raw ? JSON.parse(raw) : null
      return isSaved(saved) ? revive(saved) : INITIAL
    } catch {
      return INITIAL
    }
  }
}

function withTurn(
  state: AssistantState,
  key: string,
  id: string,
  patch: (t: AssistantTurn) => Partial<AssistantTurn>,
): AssistantState {
  const turns = state.chats[key]
  // Gone (cleared, or another account adopted the store): nowhere to write.
  if (!turns) return state
  const i = turns.findLastIndex((t) => t.id === id)
  if (i < 0) return state
  const turn = turns[i]
  if (turn.role !== 'assistant') return state
  const next = [...turns]
  next[i] = { ...turn, ...patch(turn) }
  return { ...state, chats: { ...state.chats, [key]: next } }
}

/** Patch the fix run at `key` — only if it is still run `id`, when one is given. */
function withFix(
  state: AssistantState,
  key: string,
  id: string | null,
  patch: (f: FixRun) => Partial<FixRun>,
): AssistantState {
  const run = state.fixes[key]
  if (!run || (id !== null && run.id !== id)) return state
  return { ...state, fixes: { ...state.fixes, [key]: { ...run, ...patch(run) } } }
}

function newId(): string {
  return crypto.randomUUID()
}

// ── what sessionStorage holds ───────────────────────────────────────────────
//
// It is this app's own data, but from an older build or a hand-edited tab, so
// it is checked before it is trusted; anything unreadable starts afresh.

interface Saved {
  v: 1
  owner: string | null
  mode: AiMode
  attach: boolean
  chats: Readonly<Record<string, readonly ChatTurn[]>>
  fixes: Readonly<Record<string, FixRun>>
}

const isStr = (x: unknown): x is string => typeof x === 'string'
const isStatus = (x: unknown): x is ReplyStatus =>
  x === 'streaming' || x === 'done' || x === 'stopped' || x === 'error'
const isErrorOrNull = (x: unknown): x is AssistError | null => x === null || isAssistError(x)

function isTurn(x: unknown): x is ChatTurn {
  if (!isRecord(x) || !isStr(x.id) || !isStr(x.content)) return false
  if (x.role === 'user') return typeof x.codeAttached === 'boolean'
  return x.role === 'assistant' && isStr(x.thinking) && isStatus(x.status) && isErrorOrNull(x.error)
}

function isOutcome(x: unknown): x is FixOutcome | null {
  if (x === null) return true
  if (!isRecord(x) || !isStr(x.analysis)) return false
  return x.kind === 'unchanged' || x.kind === 'text' || (x.kind === 'proposal' && isStr(x.code))
}

function isResolution(x: unknown): x is FixResolution {
  if (!isRecord(x)) return false
  if (x.kind === 'applied') return typeof x.kept === 'number' && typeof x.total === 'number'
  return x.kind === 'open' || x.kind === 'discarded'
}

function isFixRun(x: unknown): x is FixRun {
  return (
    isRecord(x) &&
    [x.id, x.note, x.snapshot, x.content, x.thinking].every(isStr) &&
    isStatus(x.status) &&
    isErrorOrNull(x.error) &&
    isOutcome(x.outcome) &&
    (x.accepted === null || (Array.isArray(x.accepted) && x.accepted.every((a) => typeof a === 'boolean'))) &&
    isResolution(x.resolution)
  )
}

function isSaved(x: unknown): x is Saved {
  return (
    isRecord(x) &&
    x.v === 1 &&
    (x.owner === null || isStr(x.owner)) &&
    MODE_ORDER.some((m) => m === x.mode) &&
    typeof x.attach === 'boolean' &&
    isRecord(x.chats) &&
    Object.values(x.chats).every((turns) => Array.isArray(turns) && turns.every(isTurn)) &&
    isRecord(x.fixes) &&
    Object.values(x.fixes).every(isFixRun)
  )
}

/** A reply that was streaming when the page went away will never finish. */
function revive(saved: Saved): AssistantState {
  const chats = Object.fromEntries(
    Object.entries(saved.chats).map(([key, turns]) => [
      key,
      turns.map((t): ChatTurn => (t.role === 'assistant' && t.status === 'streaming' ? { ...t, status: 'stopped' } : t)),
    ]),
  )
  const fixes = Object.fromEntries(
    Object.entries(saved.fixes).map(([key, f]): [string, FixRun] => [
      key,
      f.status === 'streaming' ? { ...f, status: 'stopped' } : f,
    ]),
  )
  return { owner: saved.owner, mode: saved.mode, attach: saved.attach, chats, fixes, active: null }
}

// ── the app's instance ──────────────────────────────────────────────────────

function sessionStore(): Storage | null {
  try {
    return window.sessionStorage
  } catch {
    // Some privacy modes throw on the mere access.
    return null
  }
}

export const assistantStore = new AssistantStore({ transport: postSse, storage: sessionStore() })

export function useAssistant(): AssistantState {
  return useSyncExternalStore(assistantStore.subscribe, assistantStore.getState)
}
