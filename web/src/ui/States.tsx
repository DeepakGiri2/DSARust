import type { ReactNode } from 'react'
import { errorMessage } from '@/api/client'

export function Spinner({ label }: { label?: string }) {
  return <span className="spinner" role="progressbar" aria-label={label ?? 'Loading'} />
}

/** Centered spinner for a whole page or panel that is still loading. */
export function PageSpinner({ label }: { label?: string }) {
  return (
    <div style={{ display: 'grid', placeItems: 'center', minHeight: 240, gap: 10 }}>
      <Spinner label={label} />
      {label && <span className="muted" style={{ fontSize: 13 }}>{label}</span>}
    </div>
  )
}

export function EmptyState({ title, children }: { title: ReactNode; children?: ReactNode }) {
  return (
    <div style={{ textAlign: 'center', padding: '40px 16px', color: 'var(--text-dim)' }}>
      <div style={{ fontSize: 15, color: 'var(--text)', marginBottom: 6 }}>{title}</div>
      {children}
    </div>
  )
}

export function ErrorState({ error, onRetry }: { error: unknown; onRetry?: () => void }) {
  return (
    <div style={{ textAlign: 'center', padding: '40px 16px' }}>
      <div style={{ color: 'var(--red)', marginBottom: 10 }}>{errorMessage(error)}</div>
      {onRetry && (
        <button type="button" className="mini-btn" onClick={onRetry}>
          try again
        </button>
      )}
    </div>
  )
}
