// The auth endpoints `api/hooks.ts` does not wrap yet: email verification and
// the password-reset pair. They go through the same `api` client (CSRF header,
// `ApiError` on failure) and hold no cache, so they are plain mutations.

import { useMutation } from '@tanstack/react-query'
import { api } from '@/api/client'
import type { ForgotPasswordRequest, ResetPasswordRequest, VerifyEmailRequest } from '@/api/types'

/** POST /auth/verify-email → 204. */
export function useVerifyEmail() {
  return useMutation({
    mutationFn: (req: VerifyEmailRequest) => api.post<void>('/auth/verify-email', req),
  })
}

/** POST /auth/verify-email/resend → 202, signed in only. */
export function useResendVerification() {
  return useMutation({ mutationFn: () => api.post<void>('/auth/verify-email/resend') })
}

/** POST /auth/password/forgot → 202 whether or not the account exists. */
export function useForgotPassword() {
  return useMutation({
    mutationFn: (req: ForgotPasswordRequest) => api.post<void>('/auth/password/forgot', req),
  })
}

/** POST /auth/password/reset → 204, and every session of the account ends. */
export function useResetPassword() {
  return useMutation({
    mutationFn: (req: ResetPasswordRequest) => api.post<void>('/auth/password/reset', req),
  })
}
