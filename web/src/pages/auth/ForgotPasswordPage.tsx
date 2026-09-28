// "Forgot password?" The server answers 202 whether or not the address has an
// account, so this page always ends on "check your inbox" — it must not
// become a way to find out who has signed up.

import { useRef, useState, type FormEvent } from 'react'
import { Link, useLocation } from 'react-router'
import { usePageTitle } from '@/pages/shell/usePageTitle'
import { AuthCard, FormAlert, TextField } from './AuthLayout'
import { describeError } from './errors'
import { useForgotPassword } from './hooks'
import { validateEmail } from './validation'
import styles from './auth.module.css'

export function Component() {
  usePageTitle('Reset your password')
  const location = useLocation()
  const forgot = useForgotPassword()
  const [email, setEmail] = useState(() => (location.state as { email?: string } | null)?.email ?? '')
  const [fieldError, setFieldError] = useState<string | undefined>()
  const [sentTo, setSentTo] = useState<string | null>(null)
  const emailRef = useRef<HTMLInputElement>(null)

  // A failure here is a throttle or the network — never "no such account" —
  // so it is shown as it is.
  const send = (address: string) => forgot.mutate({ email: address }, { onSuccess: () => setSentTo(address) })

  const submit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    const problem = validateEmail(email)
    setFieldError(problem ?? undefined)
    if (problem) {
      emailRef.current?.focus()
      return
    }
    send(email.trim())
  }

  if (sentTo) {
    return (
      <AuthCard title="Check your" accent="inbox">
        <div className={styles.body}>
          <p>
            If an account uses <strong>{sentTo}</strong>, a link to choose a new password is on its way.
          </p>
          <p className="muted">Nothing after a few minutes? Look in spam, or send it again.</p>
        </div>
        <div className={styles.actions}>
          <button type="button" className="btn btn-ghost" onClick={() => send(sentTo)} disabled={forgot.isPending}>
            {forgot.isPending ? 'Sending…' : 'Send it again'}
          </button>
          <Link className="btn btn-primary" to="/login" state={{ email: sentTo }}>
            Back to sign in
          </Link>
        </div>
        {forgot.isError && <FormAlert message={describeError(forgot.error).message} />}
      </AuthCard>
    )
  }

  return (
    <AuthCard
      title="Reset your"
      accent="password"
      subtitle="Enter the email you signed up with and we’ll send a link to choose a new one."
      footer={
        <Link className="link" to="/login" state={{ email: email.trim() }}>
          ← back to sign in
        </Link>
      }
    >
      <form className={styles.form} onSubmit={submit} noValidate>
        <TextField
          id="forgot-email"
          label="Email"
          name="email"
          type="email"
          autoComplete="username"
          inputMode="email"
          autoCapitalize="none"
          spellCheck={false}
          required
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          error={fieldError}
          inputRef={emailRef}
        />
        {forgot.isError && <FormAlert message={describeError(forgot.error).message} />}
        <button type="submit" className={`btn btn-primary btn-block ${styles.submit}`} disabled={forgot.isPending}>
          {forgot.isPending ? 'Sending…' : 'Send the link'}
        </button>
      </form>
    </AuthCard>
  )
}
