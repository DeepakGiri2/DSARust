// The target of the "reset your password" email: `?token=` plus a new
// password. Success ends every session of the account, this tab's included,
// so the page re-reads the session and sends the person to sign in.

import { useRef, useState, type FormEvent } from 'react'
import { Link, useSearchParams } from 'react-router'
import { isApiError } from '@/api/client'
import { usePageTitle } from '@/pages/shell/usePageTitle'
import { useSession } from '@/state/session'
import { AuthCard, FormAlert, PasswordField } from './AuthLayout'
import { describeError } from './errors'
import { useResetPassword } from './hooks'
import { validateNewPassword } from './validation'
import styles from './auth.module.css'

export function Component() {
  usePageTitle('Choose a new password')
  const [params] = useSearchParams()
  const token = params.get('token')
  const { refresh } = useSession()
  const reset = useResetPassword()
  const [password, setPassword] = useState('')
  const [fieldError, setFieldError] = useState<string | undefined>()
  const [message, setMessage] = useState<string | null>(null)
  const [expired, setExpired] = useState(false)
  const [done, setDone] = useState(false)
  const passwordRef = useRef<HTMLInputElement>(null)

  if (!token) {
    return (
      <AuthCard
        title="This link is"
        accent="incomplete"
        subtitle="The reset link is missing its token — your mail app may have cut it short."
      >
        <div className={styles.actions}>
          <Link className="btn btn-primary" to="/forgot-password">
            Send a new link
          </Link>
        </div>
      </AuthCard>
    )
  }

  if (done) {
    return (
      <AuthCard title="Password" accent="updated">
        <div className={styles.body}>
          <p>Every session on this account was signed out, this one included. Sign in with your new password.</p>
        </div>
        <div className={styles.actions}>
          <Link className="btn btn-primary" to="/login">
            Sign in
          </Link>
        </div>
      </AuthCard>
    )
  }

  const submit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    const problem = validateNewPassword(password)
    setFieldError(problem ?? undefined)
    setMessage(null)
    setExpired(false)
    if (problem) {
      passwordRef.current?.focus()
      return
    }
    reset.mutate(
      { token, password },
      {
        onSuccess: () => {
          setDone(true)
          void refresh()
        },
        onError: (err) => {
          const described = describeError(err)
          if (described.fields.password) {
            setFieldError(described.fields.password)
            passwordRef.current?.focus()
          } else if (isApiError(err) && ['bad_request', 'not_found', 'validation'].includes(err.code)) {
            // Anything wrong with the token itself: it expired or was used.
            setExpired(true)
          } else {
            setMessage(described.message)
          }
        },
      },
    )
  }

  return (
    <AuthCard title="Choose a new" accent="password" subtitle="Pick something you don’t use anywhere else.">
      <form className={styles.form} onSubmit={submit} noValidate>
        <PasswordField
          id="reset-password"
          label="New password"
          name="new-password"
          autoComplete="new-password"
          value={password}
          onChange={setPassword}
          error={fieldError}
          inputRef={passwordRef}
          strength
        />
        {expired && (
          <p className={styles.alert} role="alert">
            This reset link has expired or was already used.{' '}
            <Link className="link" to="/forgot-password">
              Send a new one
            </Link>
            .
          </p>
        )}
        <FormAlert message={message} />
        <button type="submit" className={`btn btn-primary btn-block ${styles.submit}`} disabled={reset.isPending}>
          {reset.isPending ? 'Saving…' : 'Save the new password'}
        </button>
      </form>
    </AuthCard>
  )
}
