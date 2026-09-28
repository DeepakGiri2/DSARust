// The target of the verification email: `?token=` is POSTed once, on arrival.
// Doing it from script rather than on a GET means a mail scanner that
// prefetches links cannot use the token up before the person clicks.

import { useEffect, useRef } from 'react'
import { Link, useSearchParams } from 'react-router'
import { usePageTitle } from '@/pages/shell/usePageTitle'
import { useSession } from '@/state/session'
import { Spinner } from '@/ui'
import { AuthCard, FormAlert } from './AuthLayout'
import { describeError } from './errors'
import { useResendVerification, useVerifyEmail } from './hooks'
import styles from './auth.module.css'

export function Component() {
  usePageTitle('Verify your email')
  const [params] = useSearchParams()
  const token = params.get('token')
  const { status, user, refresh } = useSession()
  const { mutate: verify, isSuccess, isError } = useVerifyEmail()
  // Survives StrictMode's second effect run, so the token is spent once.
  const started = useRef(false)

  useEffect(() => {
    if (!token || started.current) return
    started.current = true
    verify({ token }, { onSuccess: () => void refresh() })
  }, [token, verify, refresh])

  if (!token) {
    if (user?.email_verified) return <Verified />
    return (
      <AuthCard title="Check your" accent="inbox">
        <div className={styles.body}>
          {user ? (
            <p>
              We sent a verification link to <strong>{user.email}</strong>. Open it on this device to finish.
            </p>
          ) : (
            <p>This page needs the link from the verification email — open it straight from there.</p>
          )}
        </div>
        {user ? <Resend /> : status !== 'loading' && <SignInToResend />}
      </AuthCard>
    )
  }

  if (isSuccess) return <Verified />

  if (isError) {
    return (
      <AuthCard
        title="That link didn’t"
        accent="work"
        subtitle={
          user?.email_verified
            ? 'Your email is already verified, so there is nothing left to do.'
            : 'It may have expired, or been used already.'
        }
      >
        {user?.email_verified ? (
          <div className={styles.actions}>
            <Link className="btn btn-primary" to="/">
              Go to the problems
            </Link>
          </div>
        ) : user ? (
          <Resend />
        ) : (
          status !== 'loading' && <SignInToResend />
        )}
      </AuthCard>
    )
  }

  return (
    <AuthCard title="Verifying your" accent="email">
      <p className={styles.status}>
        <Spinner label="Verifying" /> One moment…
      </p>
    </AuthCard>
  )
}

function Verified() {
  return (
    <AuthCard title="Email" accent="verified" subtitle="Thanks — running code is unlocked.">
      <div className={styles.actions}>
        <Link className="btn btn-primary" to="/">
          Go to the problems
        </Link>
      </div>
    </AuthCard>
  )
}

function Resend() {
  const resend = useResendVerification()
  return (
    <>
      <div className={styles.actions}>
        <button
          type="button"
          className="btn btn-primary"
          onClick={() => resend.mutate()}
          disabled={resend.isPending || resend.isSuccess}
        >
          {resend.isPending ? 'Sending…' : resend.isSuccess ? 'Sent ✓' : 'Send a new link'}
        </button>
      </div>
      {resend.isSuccess && (
        <p className={styles.status} role="status">
          A fresh link is on its way.
        </p>
      )}
      {resend.isError && <FormAlert message={describeError(resend.error).message} />}
    </>
  )
}

function SignInToResend() {
  return (
    <div className={styles.actions}>
      <Link className="btn btn-primary" to={`/login?next=${encodeURIComponent('/verify-email')}`}>
        Sign in to get a new link
      </Link>
    </div>
  )
}
