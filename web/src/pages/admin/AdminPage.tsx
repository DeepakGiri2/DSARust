// The admin overview: a handful of numbers and the content packs that failed
// to load. Anyone but an admin gets the ordinary 404 — the same answer the API
// gives — so the page does not announce that it exists.

import clsx from 'clsx'
import { useAdminOverview } from '@/api/hooks'
import { NotFoundView } from '@/pages/shell/NotFoundView'
import { usePageTitle } from '@/pages/shell/usePageTitle'
import { useSession } from '@/state/session'
import { ErrorState, PageSpinner } from '@/ui'
import styles from './admin.module.css'

export function Component() {
  const session = useSession()
  const isAdmin = session.user?.role === 'admin'
  usePageTitle(isAdmin ? 'Admin' : 'Not found')
  const overview = useAdminOverview(isAdmin)

  if (session.status === 'loading') return <PageSpinner />
  if (!isAdmin) return <NotFoundView />
  if (overview.isPending) return <PageSpinner label="Counting…" />
  if (overview.isError) {
    return (
      <div className={styles.page}>
        <ErrorState error={overview.error} onRetry={() => void overview.refetch()} />
      </div>
    )
  }

  const o = overview.data
  const proShare = o.users ? Math.round((o.users_pro / o.users) * 100) : 0
  const tiles: [string, number, string?][] = [
    ['users', o.users],
    ['on Pro', o.users_pro, `${proShare}% of users`],
    ['sign-ups, 7 days', o.signups_7d],
    ['runs, 24 hours', o.runs_24h],
    ['AI requests, 24 hours', o.ai_requests_24h],
  ]

  return (
    <div className={styles.page}>
      <h1 className={styles.title}>
        Admin <span className="grad-text">overview</span>
      </h1>
      <div className={styles.tiles}>
        {tiles.map(([label, value, sub]) => (
          <div key={label} className={clsx('card', styles.tile)}>
            <span className={styles.value}>{value.toLocaleString()}</span>
            <span className={styles.label}>{label}</span>
            {sub && <span className={styles.sub}>{sub}</span>}
          </div>
        ))}
      </div>
      <section className={clsx('card', styles.panel)} aria-labelledby="content-errors">
        <h2 id="content-errors" className={styles.panelTitle}>
          Content errors
          <span className={clsx('chip', o.content_errors.length ? 'chip-amber' : 'chip-green')}>
            {o.content_errors.length}
          </span>
        </h2>
        {o.content_errors.length === 0 ? (
          <p className={styles.ok}>✓ Every problem pack loaded cleanly.</p>
        ) : (
          <ul className={styles.errors}>
            {o.content_errors.map((e, i) => (
              <li key={i}>{e}</li>
            ))}
          </ul>
        )}
      </section>
    </div>
  )
}
