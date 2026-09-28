// Why a Run did not produce output, phrased as something the user can act on.

import { errorMessage, isApiError } from '@/api/client'

export type RunNotice =
  | { kind: 'signin' }
  | { kind: 'profile' }
  | {
      kind: 'error'
      text: string
      details?: string[]
      link?: { to: string; label: string }
      /** Epoch ms before which Run stays disabled (429). */
      retryAt?: number
    }

export function describeRunError(e: unknown, now = Date.now()): RunNotice {
  if (!isApiError(e)) return { kind: 'error', text: errorMessage(e) }
  switch (e.code) {
    case 'unauthorized':
      return { kind: 'signin' }
    case 'rate_limited': {
      const secs = e.retryAfterSecs ?? 10
      return {
        kind: 'error',
        text: 'You are running code faster than your plan allows.',
        retryAt: now + secs * 1000,
      }
    }
    case 'unavailable':
      return {
        kind: 'error',
        text: 'The code runner is busy or offline right now — try again in a moment.',
        details: e.message ? [e.message] : undefined,
      }
    case 'email_unverified':
      return {
        kind: 'error',
        text: 'Verify your email address to run code — the link is in your inbox.',
        link: { to: '/account', label: 'account settings' },
      }
    case 'payment_required':
      return {
        kind: 'error',
        text: e.message || 'Running this problem needs the Pro plan.',
        link: { to: '/pricing', label: 'see plans' },
      }
    case 'validation':
      return { kind: 'error', text: e.message, details: e.errors.length ? e.errors : undefined }
    default:
      return { kind: 'error', text: e.message }
  }
}
