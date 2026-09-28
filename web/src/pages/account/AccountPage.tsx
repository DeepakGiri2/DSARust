// The account: who you are, how you sign in, what you pay for, your data, and
// the way out. Practice profiles are managed on the picker; this page is the
// login behind them.

import { useMemo, useRef, useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router'
import clsx from 'clsx'
import { useBillingPortal, useDeleteAccount, useMe, useMeta, useUpdateMe } from '@/api/hooks'
import { API_BASE, isApiError } from '@/api/client'
import type { UpdateMeRequest, User } from '@/api/types'
import { RequireAuth } from '@/app/guards'
import { FormAlert, PasswordField, TextField } from '@/pages/auth/AuthLayout'
import { describeError, NO_ERROR, type FormError } from '@/pages/auth/errors'
import { useResendVerification } from '@/pages/auth/hooks'
import { validateDisplayName } from '@/pages/auth/validation'
import { formatDate } from '@/pages/shell/format'
import { usePageTitle } from '@/pages/shell/usePageTitle'
import { useSession } from '@/state/session'
import { useToast } from '@/ui'
import { SecuritySection } from './SecuritySection'
import styles from './account.module.css'

export function Component() {
  return (
    <RequireAuth>
      <Account />
    </RequireAuth>
  )
}

function Account() {
  usePageTitle('Account')
  const session = useSession()
  const me = useMe()
  const meta = useMeta()
  // The session's copy renders at once (so `#billing` links land); /me
  // replaces it with the freshest one.
  const user = me.data ?? session.user
  if (!user) return null
  const billing = meta.data?.features.billing ?? false

  const sections = [
    ['details', 'Your details'],
    ['security', 'Security'],
    ...(billing ? [['billing', 'Plan & billing']] : []),
    ['data', 'Your data'],
    ['danger', 'Delete account'],
  ]

  return (
    <div className={styles.page}>
      <h1 className={styles.title}>
        Your <span className="grad-text">account</span>
      </h1>
      <div className={styles.layout}>
        <nav className={styles.toc} aria-label="Account sections">
          {sections.map(([id, label]) => (
            <Link key={id} to={`#${id}`} className={styles.tocLink}>
              {label}
            </Link>
          ))}
        </nav>
        <div className={styles.sections}>
          <DetailsSection user={user} />
          <SecuritySection user={user} />
          {billing && <BillingSection user={user} />}
          <DataSection />
          <DangerZone user={user} />
        </div>
      </div>
    </div>
  )
}

function timeZones(current: string): string[] {
  let zones: string[] = []
  try {
    zones = Intl.supportedValuesOf('timeZone')
  } catch {
    // Older engines: offer what we know rather than nothing.
  }
  return [...new Set([...zones, current, 'UTC'].filter(Boolean))].sort()
}

function DetailsSection({ user }: { user: User }) {
  const update = useUpdateMe()
  const { refresh } = useSession()
  const toast = useToast()
  const [name, setName] = useState(user.display_name)
  const [zone, setZone] = useState(user.timezone)
  const [error, setError] = useState<FormError>(NO_ERROR)
  const nameRef = useRef<HTMLInputElement>(null)
  const zones = useMemo(() => timeZones(user.timezone), [user.timezone])
  const detected = Intl.DateTimeFormat().resolvedOptions().timeZone
  const dirty = name.trim() !== user.display_name || zone !== user.timezone

  const save = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    const problem = validateDisplayName(name)
    if (problem) {
      setError({ message: null, fields: { display_name: problem } })
      nameRef.current?.focus()
      return
    }
    const patch: UpdateMeRequest = {}
    if (name.trim() !== user.display_name) patch.display_name = name.trim()
    if (zone !== user.timezone) patch.timezone = zone
    setError(NO_ERROR)
    update.mutate(patch, {
      onSuccess: () => {
        toast.success('Saved.')
        void refresh()
      },
      onError: (err) => setError(describeError(err)),
    })
  }

  return (
    <section id="details" className={clsx('card', styles.section)} aria-labelledby="details-title">
      <h2 id="details-title" className={styles.sectionTitle}>
        Your details
      </h2>
      <form className={styles.form} onSubmit={save} noValidate>
        <TextField
          id="acct-name"
          label="Display name"
          name="name"
          autoComplete="name"
          maxLength={80}
          value={name}
          onChange={(e) => setName(e.target.value)}
          error={error.fields.display_name}
          inputRef={nameRef}
        />
        <div className="field">
          <span className={styles.label}>Email</span>
          <EmailLine user={user} />
        </div>
        <div className="field">
          <label htmlFor="acct-zone">Time zone</label>
          <select
            id="acct-zone"
            className={clsx('input', styles.select)}
            value={zone}
            onChange={(e) => setZone(e.target.value)}
            aria-describedby="acct-zone-hint"
          >
            {zones.map((z) => (
              <option key={z} value={z}>
                {z.replaceAll('_', ' ')}
              </option>
            ))}
          </select>
          <span id="acct-zone-hint" className="field-hint">
            Streaks and the activity map count days in this zone.{' '}
            {detected && detected !== zone && (
              <button type="button" className={styles.linkBtn} onClick={() => setZone(detected)}>
                Use this device’s ({detected.replaceAll('_', ' ')})
              </button>
            )}
          </span>
          {error.fields.timezone && <p className="field-error">{error.fields.timezone}</p>}
        </div>
        <FormAlert message={error.message} />
        <div>
          <button type="submit" className="btn btn-primary" disabled={!dirty || update.isPending}>
            {update.isPending ? 'Saving…' : 'Save changes'}
          </button>
        </div>
      </form>
    </section>
  )
}

function EmailLine({ user }: { user: User }) {
  const resend = useResendVerification()
  const toast = useToast()
  return (
    <div className={styles.emailLine}>
      <span className={styles.email}>{user.email}</span>
      {user.email_verified ? (
        <span className="chip chip-green">✓ verified</span>
      ) : (
        <>
          <span className="chip chip-amber">not verified</span>
          <button
            type="button"
            className="mini-btn"
            disabled={resend.isPending}
            onClick={() =>
              resend.mutate(undefined, {
                onSuccess: () => toast.success(`Sent — check ${user.email}.`),
                onError: (err) => toast.error(describeError(err).message ?? 'Could not send the email.'),
              })
            }
          >
            {resend.isPending ? 'sending…' : 'resend link'}
          </button>
        </>
      )}
    </div>
  )
}

function BillingSection({ user }: { user: User }) {
  const meta = useMeta()
  const portal = useBillingPortal()
  const toast = useToast()
  const plan = meta.data?.plans.find((p) => p.id === user.plan)
  const pro = user.plan === 'pro'
  return (
    <section id="billing" className={clsx('card', styles.section)} aria-labelledby="billing-title">
      <h2 id="billing-title" className={styles.sectionTitle}>
        Plan &amp; billing
      </h2>
      <p className={styles.plan}>
        <span className={clsx('chip', pro ? 'chip-violet' : undefined)}>{plan?.name ?? (pro ? 'Pro' : 'Free')}</span>
        {pro && user.plan_renews_at && <span className={styles.text}>renews {formatDate(user.plan_renews_at)}</span>}
      </p>
      <div className={styles.actions}>
        {pro ? (
          <button
            type="button"
            className="btn btn-ghost"
            disabled={portal.isPending}
            onClick={() =>
              portal.mutate(undefined, {
                onError: (err) => toast.error(describeError(err).message ?? 'Could not open billing.'),
              })
            }
          >
            {portal.isPending ? 'Opening…' : 'Manage billing'}
          </button>
        ) : (
          <Link className="btn btn-primary" to="/pricing">
            Upgrade to Pro
          </Link>
        )}
      </div>
      {pro && <p className={styles.hint}>Invoices, payment method, switching monthly/yearly and cancelling live in the billing portal.</p>}
    </section>
  )
}

function DataSection() {
  return (
    <section id="data" className={clsx('card', styles.section)} aria-labelledby="data-title">
      <h2 id="data-title" className={styles.sectionTitle}>
        Your data
      </h2>
      <p className={styles.text}>
        Everything this account holds — profiles, progress, playlists, drafts and submissions — as one JSON file.
      </p>
      <div className={styles.actions}>
        <a className="btn btn-ghost" href={`${API_BASE}/me/export`} download>
          ⬇ Download my data
        </a>
      </div>
    </section>
  )
}

function DangerZone({ user }: { user: User }) {
  const remove = useDeleteAccount()
  const { logout } = useSession()
  const meta = useMeta()
  const navigate = useNavigate()
  const toast = useToast()
  const [open, setOpen] = useState(false)
  const [confirm, setConfirm] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<FormError>(NO_ERROR)
  const matches = confirm.trim().toLowerCase() === user.email.toLowerCase()

  const submit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    if (!matches) return
    setError(NO_ERROR)
    remove.mutate(
      { confirm_email: confirm.trim(), ...(user.has_password ? { password } : {}) },
      {
        onSuccess: async () => {
          // Off this page first, so its guard never sees the session end.
          navigate('/', { replace: true })
          try {
            await logout()
          } catch {
            // The account and its sessions are gone; only local state was left.
          }
          toast.show('Your account has been deleted.')
        },
        onError: (err) =>
          setError(
            isApiError(err, 'forbidden')
              ? { message: null, fields: { password: 'That password isn’t right.' } }
              : describeError(err),
          ),
      },
    )
  }

  return (
    <section id="danger" className={clsx('card', styles.section, styles.danger)} aria-labelledby="danger-title">
      <h2 id="danger-title" className={styles.sectionTitle}>
        Delete account
      </h2>
      <p className={styles.text}>
        Permanently deletes the account, every profile and all of their progress, playlists, drafts and
        submissions{meta.data?.features.billing ? ', and cancels any subscription' : ''}. This can’t be undone.
      </p>
      {!open ? (
        <div className={styles.actions}>
          <button type="button" className="btn btn-danger" onClick={() => setOpen(true)}>
            Delete my account…
          </button>
        </div>
      ) : (
        <form className={styles.form} onSubmit={submit} noValidate>
          <TextField
            id="delete-confirm"
            label={`Type your email (${user.email}) to confirm`}
            name="confirm-email"
            autoComplete="off"
            spellCheck={false}
            autoCapitalize="none"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
            error={error.fields.confirm_email}
          />
          {user.has_password && (
            <PasswordField
              id="delete-password"
              name="current-password"
              label="Password"
              autoComplete="current-password"
              value={password}
              onChange={setPassword}
              error={error.fields.password}
            />
          )}
          <FormAlert message={error.message} />
          <div className={styles.actions}>
            <button
              type="submit"
              className="btn btn-danger"
              disabled={!matches || (user.has_password && !password) || remove.isPending}
            >
              {remove.isPending ? 'Deleting…' : 'Delete everything'}
            </button>
            <button type="button" className="mini-btn" onClick={() => setOpen(false)}>
              keep my account
            </button>
          </div>
        </form>
      )}
    </section>
  )
}
