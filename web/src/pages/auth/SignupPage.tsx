// Create an account. The server makes the first profile (named after the
// display name) and sends a verification email; `signup()` adds the browser's
// time zone so streaks count this person's days from the start.

import { useRef, useState, type FormEvent } from 'react'
import { Link, Navigate, useSearchParams } from 'react-router'
import { isApiError } from '@/api/client'
import { useMeta } from '@/api/hooks'
import { usePageTitle } from '@/pages/shell/usePageTitle'
import { useSession } from '@/state/session'
import { PageSpinner, useToast } from '@/ui'
import { AuthCard, FormAlert, PasswordField, TextField } from './AuthLayout'
import { describeError, NO_ERROR, type FormError } from './errors'
import { focusFirstError } from './focus'
import { OAuthButtons } from './OAuthButtons'
import { afterAuth } from './redirect'
import { validateDisplayName, validateEmail, validateNewPassword } from './validation'
import styles from './auth.module.css'

export function Component() {
  usePageTitle('Create an account')
  const session = useSession()
  const meta = useMeta()
  const [params] = useSearchParams()
  if (session.status === 'loading') return <PageSpinner />
  if (session.status === 'authenticated') {
    return <Navigate to={afterAuth(params.get('next'), session.needsProfilePick)} replace />
  }
  if (meta.data?.features.signup === false) {
    return (
      <AuthCard title="Sign-ups are" accent="closed" subtitle="New accounts can’t be created right now.">
        <div className={styles.actions}>
          <Link className="btn btn-primary" to="/login">
            Sign in instead
          </Link>
          <Link className="btn btn-ghost" to="/">
            Browse the problems
          </Link>
        </div>
      </AuthCard>
    )
  }
  return <SignupForm />
}

function SignupForm() {
  const { signup } = useSession()
  const meta = useMeta()
  const toast = useToast()
  const [params] = useSearchParams()
  const next = params.get('next')
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<FormError>(NO_ERROR)
  const [pending, setPending] = useState(false)
  const nameRef = useRef<HTMLInputElement>(null)
  const emailRef = useRef<HTMLInputElement>(null)
  const passwordRef = useRef<HTMLInputElement>(null)
  const order = [
    ['display_name', nameRef],
    ['email', emailRef],
    ['password', passwordRef],
  ] as const

  const submit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    const fields: Record<string, string> = {}
    const nameProblem = validateDisplayName(name)
    if (nameProblem) fields.display_name = nameProblem
    const emailProblem = validateEmail(email)
    if (emailProblem) fields.email = emailProblem
    const passwordProblem = validateNewPassword(password)
    if (passwordProblem) fields.password = passwordProblem
    if (Object.keys(fields).length > 0) {
      setError({ message: null, fields })
      focusFirstError(fields, order)
      return
    }
    setPending(true)
    setError(NO_ERROR)
    try {
      await signup({ display_name: name.trim(), email: email.trim(), password })
      toast.success(`Welcome! We sent a link to ${email.trim()} to verify your email.`)
    } catch (err) {
      // A taken email belongs next to the email field, with the way out.
      const described = isApiError(err, 'conflict')
        ? { message: null, fields: { email: 'An account with this email already exists — sign in instead.' } }
        : describeError(err)
      setError(described)
      setPending(false)
      focusFirstError(described.fields, order)
    }
  }

  const nextQuery = next ? `?next=${encodeURIComponent(next)}` : ''
  return (
    <AuthCard
      title="Create your"
      accent="account"
      subtitle="Free. Progress, favourites and playlists that follow you to every device."
      footer={
        <>
          Already have an account?{' '}
          <Link className="link" to={`/login${nextQuery}`}>
            Sign in
          </Link>
        </>
      }
    >
      <OAuthButtons oauth={meta.data?.features.oauth} next={afterAuth(next, false)} />
      <form className={styles.form} onSubmit={submit} noValidate>
        <TextField
          id="signup-name"
          label="Your name"
          name="name"
          autoComplete="name"
          required
          maxLength={80}
          value={name}
          onChange={(e) => setName(e.target.value)}
          error={error.fields.display_name}
          hint="Also the name of your first profile — you can change it later."
          inputRef={nameRef}
        />
        <TextField
          id="signup-email"
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
          error={error.fields.email}
          inputRef={emailRef}
        />
        <PasswordField
          id="signup-password"
          label="Password"
          name="new-password"
          autoComplete="new-password"
          value={password}
          onChange={setPassword}
          error={error.fields.password}
          inputRef={passwordRef}
          strength
        />
        <FormAlert message={error.message} />
        <button type="submit" className={`btn btn-primary btn-block ${styles.submit}`} disabled={pending}>
          {pending ? 'Creating your account…' : 'Create account'}
        </button>
      </form>
    </AuthCard>
  )
}
