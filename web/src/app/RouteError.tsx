import { isRouteErrorResponse, Link, useRouteError } from 'react-router'

export function RouteError() {
  const err = useRouteError()
  const message = isRouteErrorResponse(err)
    ? `${err.status} ${err.statusText}`
    : err instanceof Error
      ? err.message
      : 'Something went wrong.'
  return (
    <main style={{ maxWidth: 560, margin: '12vh auto', padding: 24, textAlign: 'center' }}>
      <h1 style={{ fontSize: 22, marginBottom: 10 }}>This page hit a snag</h1>
      <p className="muted" style={{ marginBottom: 18 }}>
        {message}
      </p>
      <Link className="btn btn-ghost" to="/">
        ← back to the problem list
      </Link>
    </main>
  )
}
