// Route guards. Wrap a page's content in these rather than checking the
// session by hand, so "where does a guest go" has one answer.

import type { ReactNode } from 'react'
import { Navigate, useLocation } from 'react-router'
import { useSession } from '@/state/session'
import { PageSpinner } from '@/ui'

/** `?next=` target for after sign-in / profile pick, never an absolute URL. */
export function nextParam(location: { pathname: string; search: string }): string {
  return encodeURIComponent(location.pathname + location.search)
}

/** Only a same-site path is an acceptable redirect target. */
export function safeNext(raw: string | null | undefined, fallback = '/'): string {
  if (!raw) return fallback
  try {
    const decoded = decodeURIComponent(raw)
    return decoded.startsWith('/') && !decoded.startsWith('//') ? decoded : fallback
  } catch {
    return fallback
  }
}

/** Signed-in users only; guests are sent to /login and brought back after. */
export function RequireAuth({ children }: { children: ReactNode }) {
  const { status } = useSession()
  const location = useLocation()
  if (status === 'loading') return <PageSpinner />
  if (status === 'guest') return <Navigate to={`/login?next=${nextParam(location)}`} replace />
  return <>{children}</>
}

/** Signed in *and* a profile chosen; otherwise the picker, then back here. */
export function RequireProfile({ children }: { children: ReactNode }) {
  const { status, needsProfilePick } = useSession()
  const location = useLocation()
  if (status === 'loading') return <PageSpinner />
  if (status === 'guest') return <Navigate to={`/login?next=${nextParam(location)}`} replace />
  if (needsProfilePick) return <Navigate to={`/profiles?next=${nextParam(location)}`} replace />
  return <>{children}</>
}
