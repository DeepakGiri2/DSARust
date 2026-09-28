// Route: /problems/:slug and /problems/:slug/:tab (tab = practice | visualize).
//
// The problem workspace is full-screen, like the desktop's problem page: its
// own header, no site chrome. This module resolves the route and the problem;
// `Workspace` is the screen itself.

import { Link, Navigate, useParams } from 'react-router'
import { isApiError } from '@/api/client'
import { useProblem } from '@/api/hooks'
import { useActiveProfileId, useSession } from '@/state/session'
import { ErrorState, PageSpinner } from '@/ui'
import { DevMockSession, installDevMock } from './dev/devMock'
import { useDocumentTitle } from './shared/useDocumentTitle'
import { practicePath, Workspace } from './Workspace'
import styles from './ProblemPage.module.css'

// Development only, and only when asked for (localStorage.dsaMock = '1'):
// serve the API from fixtures so the screen can be worked on without a
// backend. The whole module is dropped from production builds.
if (import.meta.env.DEV) installDevMock()

export function Component() {
  const { slug = '', tab } = useParams()
  const { status } = useSession()
  const pid = useActiveProfileId()
  const authed = status === 'authenticated'
  // Wait for the session before fetching: a signed-in user reads the
  // entitlement-aware endpoint, a guest the CDN-cached one.
  const problem = useProblem(status === 'loading' ? undefined : slug, authed)

  const dev = import.meta.env.DEV && <DevMockSession />

  if (tab !== undefined && tab !== 'practice' && tab !== 'visualize') {
    return <Navigate to={practicePath(slug)} replace />
  }
  if (status === 'loading' || problem.isPending) {
    return (
      <main className={styles.full}>
        {dev}
        <PageSpinner label="Loading the problem…" />
      </main>
    )
  }
  if (problem.isError) {
    return (
      <main className={styles.full}>
        {dev}
        {isApiError(problem.error, 'not_found') ? (
          <NotFound slug={slug} />
        ) : (
          <ErrorState error={problem.error} onRetry={() => void problem.refetch()} />
        )}
      </main>
    )
  }
  return (
    <>
      {dev}
      <Workspace
        // A different problem, or a different owner of the progress on it,
        // starts from a clean workspace — as `App::open` does on the desktop.
        key={`${slug}|${pid ?? status}`}
        problem={problem.data}
        tab={tab === 'visualize' ? 'visualize' : 'practice'}
        guest={status === 'guest'}
        pid={pid}
      />
    </>
  )
}

function NotFound({ slug }: { slug: string }) {
  useDocumentTitle('Problem not found · DSA Visualized')
  return (
    <div className={styles.notFound}>
      <h1>No such problem</h1>
      <p className="muted">
        “{slug}” is not in the catalogue — it may have been renamed, or the link has a typo.
      </p>
      <Link className="btn btn-ghost" to="/">
        ← back to the problem list
      </Link>
    </div>
  )
}
