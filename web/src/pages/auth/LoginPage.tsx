// Sign in. A real <form> with `autocomplete` hints, so password managers can
// fill and save it; OAuth providers appear when the server has them. Once the
// session says "authenticated" the page sends itself on — to `?next=`, through
// the profile picker when one has to be chosen.

import { useRef, useState, type FormEvent } from 'react'
import { Link, Navigate, useLocation, useSearchParams } from 'react-router'
import { useMeta } from '@/api/hooks'
import { usePageTitle } from '@/pages/shell/usePageTitle'
import { useSession } from '@/state/session'
import { PageSpinner } from '@/ui'
import { AuthCard, FormAlert, PasswordField, TextField } from './AuthLayout'
import { describeError, NO_ERROR, type FormError } from './errors'
import { focusFirstError } from './focus'
import { OAuthButtons, oauthErrorMessage } from './OAuthButtons'
import { afterAuth } from './redirect'
import { validateCurrentPassword, validateEmail } from './validation'
import styles from './auth.module.css'

export function Component() {
  usePageTitle('Sign in')
  const session = useSession()
  const [params] = useSearchParams()
  if (session.status === 'loading') return <PageSpinner />
  if (session.status === 'authenticated') {
    return <Navigate to={afterAuth(params.get('next'), session.needsProfilePick)} replace />
  }
  return <LoginForm />
}

function LoginForm() {
  const { login } = useSession()
  const meta = useMeta()
  const location = useLocation()
  const [params] = useSearchParams()
  const next = params.get('next')
  // Carried in router state from "forgot password?" and back — never in the URL.
  const [email, setEmail] = useState(() => (location.state as { email?: string } | null)?.email ?? '')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<FormError>(NO_ERROR)
  const [pending, setPending] = useState(false)
  const emailRef = useRef<HTMLInputElement>(null)
  const passwordRef = useRef<HTMLInputElement>(null)
  const order = [
    ['email', emailRef],
    ['password', passwordRef],
  ] as const
  const oauthError = params.get('error')
  const nextQuery = next ? `?next=${encodeURIComponent(next)}` : ''

  const submit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    const fields: Record<string, string> = {}
    const emailProblem = validateEmail(email)
    if (emailProblem) fields.email = emailProblem
    const passwordProblem = validateCurrentPassword(password)
    if (passwordProblem) fields.password = passwordProblem
    if (Object.keys(fields).length > 0) {
      setError({ message: null, fields })
      focusFirstError(fields, order)
      return
    }
    setPending(true)
    setError(NO_ERROR)
    try {
      await login({ email: email.trim(), password })
    } catch (err) {
      const described = describeError(err, {
        unauthorized: 'That email and password don’t match an account.',
      })
      setError(described)
      setPending(false)
      focusFirstError(described.fields, order)
    }
  }

  return (
    <AuthCard
      title="Welcome"
      accent="back"
      subtitle="Sign in to pick up where you left off."
      footer={
        meta.data?.features.signup !== false && (
          <>
            New here?{' '}
            <Link className="link" to={`/signup${nextQuery}`}>
              Create a free account
            </Link>
          </>
        )
      }
    >
      {oauthError && (
        <p className={styles.alert} role="alert">
          {oauthErrorMessage(oauthError)}
        </p>
      )}
      <OAuthButtons oauth={meta.data?.features.oauth} next={afterAuth(next, false)} />
      <form className={styles.form} onSubmit={submit} noValidate>
        <TextField
          id="login-email"
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
          id="login-password"
          label="Password"
          autoComplete="current-password"
          value={password}
          onChange={setPassword}
          error={error.fields.password}
          inputRef={passwordRef}
          aside={
            <Link className={`link ${styles.aside}`} to="/forgot-password" state={{ email: email.trim() }}>
              forgot password?
            </Link>
          }
        />
        <FormAlert message={error.message} />
        <button type="submit" className={`btn btn-primary btn-block ${styles.submit}`} disabled={pending}>
          {pending ? 'Signing in…' : 'Sign in'}
        </button>
      </form>
    </AuthCard>
  )
}
