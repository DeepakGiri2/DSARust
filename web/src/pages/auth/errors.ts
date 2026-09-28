// `ApiError` → what a form shows. Field messages go next to their inputs;
// everything else becomes one sentence above the submit button.

import { isApiError, errorMessage } from '@/api/client'
import type { ErrorCode } from '@/api/types'
import { formatWait } from '@/pages/shell/format'

export interface FormError {
  /** A sentence for the form as a whole, or null when the fields say it all. */
  message: string | null
  /** Per-field messages keyed by the request's field names. */
  fields: Record<string, string>
}

export const NO_ERROR: FormError = { message: null, fields: {} }

function wait(secs: number | undefined): string {
  return secs === undefined ? 'a little while' : formatWait(secs)
}

/**
 * Describe a failed request for a form. `messages` overrides the text for a
 * code where the form knows better than the server's generic wording (a 409 on
 * signup means "that email has an account", on a profile "that name is taken").
 */
export function describeError(
  e: unknown,
  messages: Partial<Record<ErrorCode, string>> = {},
): FormError {
  if (!isApiError(e)) return { message: errorMessage(e), fields: {} }
  const override = messages[e.code]
  switch (e.code) {
    case 'validation': {
      const fields = e.fieldErrors
      if (Object.keys(fields).length > 0) return { message: override ?? null, fields }
      return { message: override ?? (e.errors.join(' ') || e.message), fields: {} }
    }
    case 'rate_limited':
      return {
        message: override ?? `Too many attempts. Try again in ${wait(e.retryAfterSecs)}.`,
        fields: {},
      }
    case 'account_locked':
      return {
        message:
          override ??
          `This account is locked for ${wait(e.retryAfterSecs)} after too many failed sign-ins. ` +
            'You can reset your password to get back in sooner.',
        fields: {},
      }
    default:
      return { message: override ?? e.message, fields: {} }
  }
}
