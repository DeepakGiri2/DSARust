// Password and sessions. Changing the password ends every *other* session (the
// server's rule), so the list below is refreshed afterwards.

import { useRef, useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import clsx from 'clsx'
import { isApiError } from '@/api/client'
import { qk, useChangePassword, useRevokeOtherSessions, useRevokeSession, useSessions } from '@/api/hooks'
import type { SessionRow, User } from '@/api/types'
import { FormAlert, PasswordField } from '@/pages/auth/AuthLayout'
import { describeError, NO_ERROR, type FormError } from '@/pages/auth/errors'
import { focusFirstError } from '@/pages/auth/focus'
import { useForgotPassword } from '@/pages/auth/hooks'
import { validateCurrentPassword, validateNewPassword } from '@/pages/auth/validation'
import { formatDate, formatDateTime, timeAgo } from '@/pages/shell/format'
import { ErrorState, Spinner, useToast } from '@/ui'
import { describeDevice } from './userAgent'
import styles from './account.module.css'

export function SecuritySection({ user }: { user: User }) {
  return (
    <section id="security" className={clsx('card', styles.section)} aria-labelledby="security-title">
      <h2 id="security-title" className={styles.sectionTitle}>
        Security
      </h2>
      {user.has_password ? <ChangePassword user={user} /> : <SetPassword user={user} />}
      <h3 className={styles.subTitle}>Where you’re signed in</h3>
      <Sessions />
    </section>
  )
}

function ChangePassword({ user }: { user: User }) {
  const change = useChangePassword()
  const qc = useQueryClient()
  const toast = useToast()
  const [current, setCurrent] = useState('')
  const [next, setNext] = useState('')
  const [error, setError] = useState<FormError>(NO_ERROR)
  const currentRef = useRef<HTMLInputElement>(null)
  const nextRef = useRef<HTMLInputElement>(null)
  const order = [
    ['current_password', currentRef],
    ['new_password', nextRef],
  ] as const

  const submit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    const fields: Record<string, string> = {}
    const a = validateCurrentPassword(current)
    if (a) fields.current_password = a
    const b = validateNewPassword(next)
    if (b) fields.new_password = b
    if (Object.keys(fields).length) {
      setError({ message: null, fields })
      return focusFirstError(fields, order)
    }
    setError(NO_ERROR)
    change.mutate(
      { current_password: current, new_password: next },
      {
        onSuccess: () => {
          setCurrent('')
          setNext('')
          toast.success('Password changed. Every other session was signed out.')
          void qc.invalidateQueries({ queryKey: qk.sessions })
        },
        onError: (err) => {
          const d = isApiError(err, 'forbidden')
            ? { message: null, fields: { current_password: 'That isn’t your current password.' } }
            : describeError(err)
          setError(d)
          focusFirstError(d.fields, order)
        },
      },
    )
  }

  return (
    <form className={styles.form} onSubmit={submit} noValidate>
      {/* Lets a password manager file the new password under the right account. */}
      <input
        type="text"
        name="username"
        autoComplete="username"
        value={user.email}
        readOnly
        hidden
      />
      <PasswordField
        id="acct-current-password"
        name="current-password"
        label="Current password"
        autoComplete="current-password"
        value={current}
        onChange={setCurrent}
        error={error.fields.current_password}
        inputRef={currentRef}
      />
      <PasswordField
        id="acct-new-password"
        name="new-password"
        label="New password"
        autoComplete="new-password"
        value={next}
        onChange={setNext}
        error={error.fields.new_password}
        inputRef={nextRef}
        strength
      />
      <FormAlert message={error.message} />
      <div>
        <button type="submit" className="btn btn-primary" disabled={change.isPending}>
          {change.isPending ? 'Changing…' : 'Change password'}
        </button>
      </div>
    </form>
  )
}

/** OAuth-only accounts set a first password through the reset email. */
function SetPassword({ user }: { user: User }) {
  const forgot = useForgotPassword()
  const providers = user.oauth_providers.map((p) => p.charAt(0).toUpperCase() + p.slice(1)).join(' and ')
  return (
    <div className={styles.form}>
      <p className={styles.text}>
        You sign in with {providers || 'a connected account'}, so there is no password yet. To be able to
        sign in with your email too, we can send a link to <strong>{user.email}</strong> to set one.
      </p>
      <div>
        <button
          type="button"
          className="btn btn-ghost"
          disabled={forgot.isPending || forgot.isSuccess}
          onClick={() => forgot.mutate({ email: user.email })}
        >
          {forgot.isPending ? 'Sending…' : forgot.isSuccess ? 'Sent — check your inbox' : 'Email me a link to set a password'}
        </button>
      </div>
      {forgot.isError && <FormAlert message={describeError(forgot.error).message} />}
    </div>
  )
}

function Sessions() {
  const sessions = useSessions()
  const revoke = useRevokeSession()
  const revokeOthers = useRevokeOtherSessions()
  const toast = useToast()

  if (sessions.isPending) return <Spinner label="Loading sessions" />
  if (sessions.isError) return <ErrorState error={sessions.error} onRetry={() => void sessions.refetch()} />

  const rows = [...sessions.data].sort(
    (a, b) => Number(b.current) - Number(a.current) || Date.parse(b.last_seen_at) - Date.parse(a.last_seen_at),
  )
  const others = rows.filter((s) => !s.current).length

  return (
    <>
      <ul className={styles.sessions}>
        {rows.map((s) => (
          <SessionItem
            key={s.id}
            session={s}
            revoking={revoke.isPending && revoke.variables === s.id}
            onRevoke={() =>
              revoke.mutate(s.id, {
                onSuccess: () => toast.success(`Signed out ${describeDevice(s.user_agent)}.`),
                onError: (err) => toast.error(describeError(err).message ?? 'Could not sign that session out.'),
              })
            }
          />
        ))}
      </ul>
      {others > 0 && (
        <button
          type="button"
          className={clsx('btn btn-ghost', styles.everywhere)}
          disabled={revokeOthers.isPending}
          onClick={() =>
            revokeOthers.mutate(undefined, {
              onSuccess: () => toast.success('Signed out everywhere else.'),
              onError: (err) => toast.error(describeError(err).message ?? 'Could not sign the other sessions out.'),
            })
          }
        >
          {revokeOthers.isPending ? 'Signing out…' : 'Sign out everywhere else'}
        </button>
      )}
    </>
  )
}

function SessionItem({ session, revoking, onRevoke }: { session: SessionRow; revoking: boolean; onRevoke: () => void }) {
  return (
    <li className={styles.sessionRow}>
      <div className={styles.sessionMain}>
        <span className={styles.device}>
          {describeDevice(session.user_agent)}
          {session.current && <span className={clsx('chip chip-green', styles.thisDevice)}>this device</span>}
        </span>
        <span className={styles.sessionMeta}>
          {session.ip ?? 'unknown address'} · active{' '}
          <time dateTime={session.last_seen_at} title={formatDateTime(session.last_seen_at)}>
            {timeAgo(session.last_seen_at)}
          </time>{' '}
          · signed in {formatDate(session.created_at)}
        </span>
      </div>
      {!session.current && (
        <button type="button" className="mini-btn" onClick={onRevoke} disabled={revoking}>
          {revoking ? 'signing out…' : 'sign out'}
        </button>
      )}
    </li>
  )
}
