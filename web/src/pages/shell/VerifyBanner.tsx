// "Verify your email" — shown only where it gates something (the server
// requires a verified address to run code), and dismissible for the rest of
// the tab's life, so it informs once instead of nagging on every page.

import { useState } from 'react'
import { useMeta } from '@/api/hooks'
import type { Uuid } from '@/api/types'
import { describeError } from '@/pages/auth/errors'
import { useResendVerification } from '@/pages/auth/hooks'
import { useSession } from '@/state/session'
import { useToast } from '@/ui'
import styles from './AppShell.module.css'

const key = (userId: Uuid) => `dsa.verifyBannerDismissed.${userId}`

function readDismissed(userId: Uuid): boolean {
  try {
    return sessionStorage.getItem(key(userId)) === '1'
  } catch {
    return false
  }
}

export function VerifyBanner() {
  const { user } = useSession()
  const meta = useMeta()
  const resend = useResendVerification()
  const toast = useToast()
  // Keyed by user so signing in as someone else re-reads their own choice.
  const [dismissedFor, setDismissedFor] = useState<Uuid | null>(null)

  if (!user || user.email_verified || !meta.data?.features.email_verification_required) return null
  if (dismissedFor === user.id || readDismissed(user.id)) return null

  const dismiss = () => {
    try {
      sessionStorage.setItem(key(user.id), '1')
    } catch {
      // Storage blocked: it stays dismissed until the page reloads.
    }
    setDismissedFor(user.id)
  }

  const send = () =>
    resend.mutate(undefined, {
      onSuccess: () => toast.success(`Sent — check ${user.email} for the link.`),
      onError: (e) => toast.error(describeError(e).message ?? 'Could not send the email.'),
    })

  return (
    <aside className={styles.banner} aria-label="Email verification">
      <div className={styles.bannerInner}>
        <span aria-hidden>✉</span>
        <span className={styles.bannerText}>
          Verify your email to run code — we sent a link to <strong>{user.email}</strong>.
        </span>
        <button type="button" className="mini-btn" onClick={send} disabled={resend.isPending}>
          {resend.isPending ? 'sending…' : resend.isSuccess ? 'sent ✓ resend' : 'resend link'}
        </button>
        <button
          type="button"
          className={styles.bannerClose}
          onClick={dismiss}
          aria-label="Dismiss for now"
          title="Dismiss for now"
        >
          ✕
        </button>
      </div>
    </aside>
  )
}
