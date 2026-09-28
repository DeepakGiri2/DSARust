// Pieces every account form shares: the centred card, labelled fields that
// wire up `aria-invalid` / `aria-describedby`, a password input with a
// show/hide switch, and the strength meter.

import { useState, type InputHTMLAttributes, type ReactNode, type Ref } from 'react'
import clsx from 'clsx'
import { MIN_PASSWORD, passwordStrength } from './validation'
import styles from './auth.module.css'

export function AuthCard({
  title,
  accent,
  subtitle,
  children,
  footer,
}: {
  title: string
  /** The gradient word, as in "DSA Visualized". */
  accent: string
  subtitle?: ReactNode
  children: ReactNode
  footer?: ReactNode
}) {
  return (
    <div className={styles.page}>
      <div className={clsx('card', styles.card)}>
        <h1 className={styles.title}>
          {title} <span className="grad-text">{accent}</span>
        </h1>
        {subtitle && <p className={styles.subtitle}>{subtitle}</p>}
        {children}
      </div>
      {footer && <p className={styles.footer}>{footer}</p>}
    </div>
  )
}

/** The form-level error, announced when it appears. */
export function FormAlert({ message }: { message: string | null }) {
  if (!message) return null
  return (
    <p className={styles.alert} role="alert">
      {message}
    </p>
  )
}

type InputProps = Omit<InputHTMLAttributes<HTMLInputElement>, 'id' | 'className'>

export function TextField({
  id,
  label,
  error,
  hint,
  inputRef,
  aside,
  ...input
}: InputProps & {
  id: string
  label: string
  error?: string
  hint?: ReactNode
  inputRef?: Ref<HTMLInputElement>
  /** Something to the right of the label, like "forgot password?". */
  aside?: ReactNode
}) {
  const describedBy = [error && `${id}-error`, hint && `${id}-hint`].filter(Boolean).join(' ') || undefined
  return (
    <div className="field">
      <div className={styles.labelRow}>
        <label htmlFor={id}>{label}</label>
        {aside}
      </div>
      <input
        {...input}
        ref={inputRef}
        id={id}
        className="input"
        aria-invalid={error ? true : undefined}
        aria-describedby={describedBy}
      />
      {hint && (
        <div id={`${id}-hint`} className="field-hint">
          {hint}
        </div>
      )}
      {error && (
        <p id={`${id}-error`} className="field-error">
          {error}
        </p>
      )}
    </div>
  )
}

export function PasswordField({
  id,
  label,
  value,
  onChange,
  autoComplete,
  error,
  inputRef,
  aside,
  strength,
  name = 'password',
}: {
  id: string
  label: string
  value: string
  onChange: (value: string) => void
  autoComplete: 'current-password' | 'new-password'
  error?: string
  inputRef?: Ref<HTMLInputElement>
  aside?: ReactNode
  /** Show the strength meter (for choosing a password, not for typing one). */
  strength?: boolean
  name?: string
}) {
  const [shown, setShown] = useState(false)
  const describedBy = [error && `${id}-error`, strength && `${id}-strength`].filter(Boolean).join(' ') || undefined
  return (
    <div className="field">
      <div className={styles.labelRow}>
        <label htmlFor={id}>{label}</label>
        {aside}
      </div>
      <div className={styles.passwordWrap}>
        <input
          ref={inputRef}
          id={id}
          name={name}
          type={shown ? 'text' : 'password'}
          className="input"
          autoComplete={autoComplete}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          required
          minLength={autoComplete === 'new-password' ? MIN_PASSWORD : undefined}
          spellCheck={false}
          autoCapitalize="none"
          aria-invalid={error ? true : undefined}
          aria-describedby={describedBy}
        />
        <button
          type="button"
          className={styles.reveal}
          aria-pressed={shown}
          aria-controls={id}
          aria-label={shown ? 'Hide password' : 'Show password'}
          onClick={() => setShown((s) => !s)}
        >
          {shown ? 'hide' : 'show'}
        </button>
      </div>
      {strength && <StrengthMeter id={`${id}-strength`} password={value} />}
      {error && (
        <p id={`${id}-error`} className="field-error">
          {error}
        </p>
      )}
    </div>
  )
}

function StrengthMeter({ id, password }: { id: string; password: string }) {
  const s = passwordStrength(password)
  const text = !password
    ? `At least ${MIN_PASSWORD} characters — a few unrelated words make a strong one.`
    : s.score === 0
      ? `${MIN_PASSWORD - password.length} more ${MIN_PASSWORD - password.length === 1 ? 'character' : 'characters'} to go`
      : `Strength: ${s.label}`
  return (
    <div id={id} className={styles.strength}>
      <div className={styles.bars} aria-hidden>
        {[1, 2, 3, 4].map((i) => (
          <span key={i} className={clsx(styles.bar, i <= s.score && styles[`s${s.score}`])} />
        ))}
      </div>
      <span className={styles.strengthText} aria-live="polite">
        {text}
      </span>
    </div>
  )
}
