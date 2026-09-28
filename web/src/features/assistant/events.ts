// What the server says during `POST /ai/chat`, and what the panel says about
// it when things go wrong.
//
// Every event's `data` is JSON from another process, so each is checked before
// use rather than cast: a token with no text is skipped, an unknown channel is
// ignored (never shown as the answer), and an error that cannot be read still
// ends the reply as an error instead of hanging it.

import { isApiError } from '@/api/client'
import type { AiDoneEvent, AiTokenEvent, ErrorCode } from '@/api/types'

/** A reply that failed: the server's code, or one of the ways a stream breaks. */
export interface AssistError {
  code: ErrorCode | 'network' | 'dropped'
  /** The server's wording, when it sent any. */
  message: string
  /** Epoch ms before which the server will refuse a retry (rate limits). */
  retryAt: number | null
}

/** The body ended without `done` or `error` — a proxy or the network cut it. */
export const DROPPED: AssistError = { code: 'dropped', message: '', retryAt: null }

const ERROR_CODES: ReadonlySet<string> = new Set<ErrorCode>([
  'bad_request',
  'validation',
  'unauthorized',
  'forbidden',
  'csrf',
  'not_found',
  'conflict',
  'payment_required',
  'rate_limited',
  'email_unverified',
  'account_locked',
  'unavailable',
  'internal',
])

export function isRecord(x: unknown): x is Record<string, unknown> {
  return typeof x === 'object' && x !== null && !Array.isArray(x)
}

export function isAssistError(x: unknown): x is AssistError {
  return (
    isRecord(x) &&
    typeof x.code === 'string' &&
    (ERROR_CODES.has(x.code) || x.code === 'network' || x.code === 'dropped') &&
    typeof x.message === 'string' &&
    (x.retryAt === null || typeof x.retryAt === 'number')
  )
}

function json(data: string): unknown {
  try {
    return JSON.parse(data)
  } catch {
    return undefined
  }
}

export function readToken(data: string): AiTokenEvent | null {
  const t = json(data)
  if (!isRecord(t) || typeof t.text !== 'string' || t.text === '') return null
  return t.channel === 'content' || t.channel === 'thinking' ? { channel: t.channel, text: t.text } : null
}

export function readDone(data: string): AiDoneEvent | null {
  const d = json(data)
  if (!isRecord(d) || typeof d.remaining_today !== 'number') return null
  return {
    input_tokens: typeof d.input_tokens === 'number' ? d.input_tokens : 0,
    output_tokens: typeof d.output_tokens === 'number' ? d.output_tokens : 0,
    remaining_today: d.remaining_today,
  }
}

function assistError(code: string, message: string, retryAfterSecs: unknown, now: number): AssistError {
  return {
    code: ERROR_CODES.has(code) ? (code as ErrorCode) : 'internal',
    message,
    retryAt: typeof retryAfterSecs === 'number' && retryAfterSecs > 0 ? now + retryAfterSecs * 1000 : null,
  }
}

/** `event: error` carries `ApiErrorBody['error']`. */
export function readError(data: string, now: number): AssistError {
  const e = json(data)
  if (!isRecord(e) || typeof e.code !== 'string') return assistError('internal', '', null, now)
  const details = isRecord(e.details) ? e.details : {}
  return assistError(e.code, typeof e.message === 'string' ? e.message : '', details.retry_after_secs, now)
}

/** A refusal before streaming started (an `ApiError`), or a broken connection. */
export function toAssistError(e: unknown, now: number): AssistError {
  if (isApiError(e)) return assistError(e.code, e.message, e.retryAfterSecs, now)
  // fetch() rejects with a TypeError when the server cannot be reached, and
  // the body reader does the same when the connection drops mid-reply.
  return { code: 'network', message: '', retryAt: null }
}

export type ErrorAction = 'retry' | 'pricing' | 'signin' | 'account' | 'none'

/** The sentence the panel shows for an error, and the one thing to do next. */
export function errorCopy(err: AssistError): { text: string; action: ErrorAction } {
  switch (err.code) {
    case 'rate_limited':
      return { text: "You're sending requests faster than AI assist allows.", action: 'retry' }
    case 'payment_required':
      // The server knows which limit it was (daily quota, a Pro problem), so
      // its wording wins; the link is the same either way.
      return { text: err.message || "You've used all of today's AI requests.", action: 'pricing' }
    case 'unavailable':
      return { text: 'The AI service is busy or unavailable right now.', action: 'retry' }
    case 'unauthorized':
      return { text: 'Your session has ended — sign in again to keep going.', action: 'signin' }
    case 'csrf':
      return { text: 'Your session needed refreshing.', action: 'retry' }
    case 'email_unverified':
      return { text: 'Verify your email address to use AI assist.', action: 'account' }
    case 'network':
      return { text: "Couldn't reach the server — check your connection.", action: 'retry' }
    case 'dropped':
      return { text: 'The connection dropped before the reply finished.', action: 'retry' }
    case 'bad_request':
    case 'validation':
    case 'forbidden':
    case 'not_found':
    case 'conflict':
      // Asking again would be refused the same way.
      return { text: err.message || 'The server turned this request down.', action: 'none' }
    default:
      return { text: 'Something went wrong on our side.', action: 'retry' }
  }
}
