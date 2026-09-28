// What a conversation is made of, and the two decisions about it that have to
// be right: which turns go back to the model, and what a finished Fix reply
// turns into. Pure, so both are tested without a server or a DOM.

import type { AiMessage } from '@/api/types'
import type { AssistError } from './events'
import { HISTORY_TURNS } from './modes'
import { extractLastCodeBlock, extractThink, matchIndent } from './parse'
import { trimStartMatches } from './str'

export type ReplyStatus = 'streaming' | 'done' | 'stopped' | 'error'

export interface UserTurn {
  readonly id: string
  readonly role: 'user'
  readonly content: string
  /** The editor's code went with this turn (shown as a pill; never re-sent). */
  readonly codeAttached: boolean
}

export interface AssistantTurn {
  readonly id: string
  readonly role: 'assistant'
  readonly content: string
  /** The thinking channel, kept apart so it is never mistaken for the answer. */
  readonly thinking: string
  readonly status: ReplyStatus
  readonly error: AssistError | null
}

export type ChatTurn = UserTurn | AssistantTurn

/** What a finished Fix reply amounts to. */
export type FixOutcome =
  /** A corrected function that differs from what was sent, re-indented to match it. */
  | { kind: 'proposal'; analysis: string; code: string }
  | { kind: 'unchanged'; analysis: string }
  /** No code block at all: the prose is all there is. */
  | { kind: 'text'; analysis: string }

export type FixResolution =
  | { kind: 'open' }
  | { kind: 'applied'; kept: number; total: number }
  | { kind: 'discarded' }

/**
 * One Fix analysis. Fix is one-shot, as on the desktop: each "analyze" starts
 * afresh from the current code, so there is one of these per problem and
 * language rather than a transcript.
 */
export interface FixRun {
  readonly id: string
  /** What the user typed to point the reviewer somewhere; may be empty. */
  readonly note: string
  /** The code the analysis was asked about. */
  readonly snapshot: string
  readonly content: string
  readonly thinking: string
  readonly status: ReplyStatus
  readonly error: AssistError | null
  /** Set once the reply is complete. */
  readonly outcome: FixOutcome | null
  /**
   * One tick per change group of the diff on screen; `null` means "all
   * ticked", which is how every fix arrives.
   */
  readonly accepted: readonly boolean[] | null
  readonly resolution: FixResolution
}

/** The user turn a Fix request carries when nothing was typed. */
export const FIX_REQUEST = 'Review my current solution and fix what is wrong.'

/** The desktop's fallback, minus "pick a bigger model": the server chooses the model here. */
export const NOTHING_USABLE = 'The model returned nothing usable — try again.'

/**
 * The messages to send for `turns`, which ends with the new user turn.
 *
 * * Only replies with visible content go back: a reply stopped before it said
 *   anything, or refused by the server, never reached the model — and neither
 *   did the question it was answering, unless that question is the new turn.
 *   That also keeps roles alternating, which hosted chat APIs insist on.
 * * Inline `<think>` reasoning is stripped: feeding a model its own reasoning
 *   costs input tokens and tells it nothing.
 * * The last `HISTORY_TURNS` messages, opening on a user turn.
 */
export function historyFor(turns: readonly ChatTurn[]): AiMessage[] {
  const out: AiMessage[] = []
  turns.forEach((turn, i) => {
    if (turn.role === 'assistant') {
      const { body } = extractThink(turn.content)
      if (body !== '') out.push({ role: 'assistant', content: body })
      return
    }
    const reply = turns[i + 1]
    const answered = reply?.role === 'assistant' && extractThink(reply.content).body !== ''
    if (answered || i === turns.length - 1) out.push({ role: 'user', content: turn.content })
  })
  const recent = out.slice(-HISTORY_TURNS)
  while (recent.length > 1 && recent[0].role === 'assistant') recent.shift()
  return recent
}

/**
 * A finished Fix reply: pull the corrected function out — a port of
 * `Assistant::finish_fix`. `snapshot` is the code the request was about; the
 * indentation is matched to it and "no change" is judged against it.
 */
export function finishFix(content: string, snapshot: string): FixOutcome {
  const { body } = extractThink(content)
  const { code, rest } = extractLastCodeBlock(body)
  const analysis = trimStartMatches(rest, 'ISSUES:').trim()

  if (code === null) return { kind: 'text', analysis: analysis || NOTHING_USABLE }
  const aligned = matchIndent(code, snapshot)
  if (aligned.trim() === snapshot.trim()) return { kind: 'unchanged', analysis }
  return { kind: 'proposal', analysis, code: aligned }
}
