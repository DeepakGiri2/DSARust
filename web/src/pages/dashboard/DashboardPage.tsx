// The dashboard: how far the active profile has got. Everything here is that
// profile's alone — the desktop keeps each profile's history apart, and so
// does this page.

import { useMemo } from 'react'
import { Link } from 'react-router'
import clsx from 'clsx'
import { useCatalog, useStats } from '@/api/hooks'
import type { Difficulty, RunStatus, Stats, SubmissionSummary } from '@/api/types'
import { RequireProfile } from '@/app/guards'
import { formatDateTime, plural, timeAgo } from '@/pages/shell/format'
import { usePageTitle } from '@/pages/shell/usePageTitle'
import { useSession } from '@/state/session'
import { EmptyState, ErrorState, PageSpinner } from '@/ui'
import { CategoryTable } from './CategoryTable'
import { ActivityHeatmap } from './ActivityHeatmap'
import { ImportPanel } from './ImportPanel'
import styles from './dashboard.module.css'

export function Component() {
  return (
    <RequireProfile>
      <Dashboard />
    </RequireProfile>
  )
}

const DIFFICULTY_COLOR: Record<Difficulty, string> = {
  Easy: 'var(--green)',
  Medium: 'var(--amber)',
  Hard: 'var(--red)',
}

const RUN_STATUS: Record<RunStatus, { label: string; tone: 'good' | 'bad' | 'warn' | 'dim' }> = {
  ok: { label: 'ran', tone: 'good' },
  passed: { label: 'passed', tone: 'good' },
  failed: { label: 'failed', tone: 'bad' },
  compile_error: { label: 'compile error', tone: 'bad' },
  runtime_error: { label: 'runtime error', tone: 'bad' },
  timeout: { label: 'timed out', tone: 'warn' },
  error: { label: 'platform error', tone: 'dim' },
}

function Dashboard() {
  usePageTitle('Dashboard')
  const { activeProfile, user } = useSession()
  const stats = useStats(activeProfile?.id ?? null)
  const catalog = useCatalog()

  const lookups = useMemo(() => {
    const problems = catalog.data?.categories.flatMap((c) => c.problems) ?? []
    return {
      titles: new Map(problems.map((p) => [p.slug, p.title])),
      slugs: new Set(problems.map((p) => p.slug)),
      langs: new Map((catalog.data?.languages ?? []).map((l) => [l.id, l.label])),
    }
  }, [catalog.data])

  if (!activeProfile || !user) {
    return (
      <div className={styles.page}>
        <EmptyState title="No profile yet">
          <Link className="mini-btn" to="/profiles">
            make one
          </Link>
        </EmptyState>
      </div>
    )
  }
  if (stats.isPending) return <PageSpinner label="Adding it all up…" />
  if (stats.isError) {
    return (
      <div className={styles.page}>
        <ErrorState error={stats.error} onRetry={() => void stats.refetch()} />
      </div>
    )
  }

  const s = stats.data
  const total = catalog.data?.total
  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}>
          <span aria-hidden>{activeProfile.avatar}</span> {activeProfile.name}’s{' '}
          <span className="grad-text">progress</span>
        </h1>
        <p className={styles.lede}>
          Counted for this profile only.{' '}
          <Link className="link" to="/profiles?next=%2Fdashboard">
            Switch profile
          </Link>
        </p>
      </header>

      <div className={styles.tiles}>
        <Tile label="solved" value={s.totals.solved} sub={total !== undefined ? `of ${total}` : undefined} tone="green" />
        <Tile label="attempted" value={s.totals.attempted} sub="started, not solved yet" tone="amber" />
        <Tile label="favourites" value={s.totals.favourites} sub="starred" />
        <Tile label="submissions" value={s.totals.submissions} sub="runs and test runs" />
        <StreakTile streak={s.streak} />
      </div>

      <section className={clsx('card', styles.panel)} aria-labelledby="activity-title">
        <h2 id="activity-title" className={styles.panelTitle}>
          Activity
        </h2>
        <ActivityHeatmap activity={s.activity} timezone={user.timezone} />
      </section>

      <div className={styles.columns}>
        <section className={clsx('card', styles.panel)} aria-labelledby="lists-title">
          <h2 id="lists-title" className={styles.panelTitle}>
            By list
          </h2>
          {s.by_tier.map((t) => (
            <Bar key={t.tier} label={t.title} solved={t.solved} total={t.total} color="var(--accent)" />
          ))}
          <h2 className={clsx(styles.panelTitle, styles.spaced)}>By difficulty</h2>
          {s.by_difficulty.map((d) => (
            <Bar
              key={d.difficulty}
              label={d.difficulty}
              solved={d.solved}
              total={d.total}
              color={DIFFICULTY_COLOR[d.difficulty]}
            />
          ))}
        </section>

        <section className={clsx('card', styles.panel)} aria-labelledby="recent-title">
          <h2 id="recent-title" className={styles.panelTitle}>
            Recent submissions
          </h2>
          <Recent items={s.recent} titles={lookups.titles} langs={lookups.langs} />
        </section>
      </div>

      <section className={clsx('card', styles.panel)} aria-labelledby="categories-title">
        <h2 id="categories-title" className={styles.panelTitle}>
          By category
        </h2>
        <CategoryTable rows={s.by_category} />
      </section>

      <ImportPanel pid={activeProfile.id} profileName={activeProfile.name} knownSlugs={lookups.slugs} />
    </div>
  )
}

function Tile({
  label,
  value,
  sub,
  tone,
}: {
  label: string
  value: number
  sub?: string
  tone?: 'green' | 'amber'
}) {
  return (
    <div className={clsx('card', styles.tile)}>
      <span className={clsx(styles.tileValue, tone && styles[tone])}>{value}</span>
      <span className={styles.tileLabel}>{label}</span>
      {sub && <span className={styles.tileSub}>{sub}</span>}
    </div>
  )
}

function StreakTile({ streak }: { streak: Stats['streak'] }) {
  return (
    <div className={clsx('card', styles.tile)}>
      <span className={styles.tileValue}>
        {streak.current}
        <span className={styles.tileUnit}> {streak.current === 1 ? 'day' : 'days'}</span>
      </span>
      <span className={styles.tileLabel}>current streak</span>
      <span className={styles.tileSub}>
        longest {plural(streak.longest, 'day')} ·{' '}
        {streak.active_today ? (
          <span className={styles.green}>✓ active today</span>
        ) : streak.current > 0 ? (
          <span className={styles.amber}>run something today to keep it</span>
        ) : (
          'not yet today'
        )}
      </span>
    </div>
  )
}

function Bar({ label, solved, total, color }: { label: string; solved: number; total: number; color: string }) {
  const pct = total ? Math.min(100, (solved / total) * 100) : 0
  return (
    <div className={styles.bar}>
      <div className={styles.barHead}>
        <span>{label}</span>
        <span className={styles.barNum}>
          {solved}/{total}
        </span>
      </div>
      <div
        className={styles.track}
        role="progressbar"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={total}
        aria-valuenow={solved}
        aria-valuetext={`${solved} of ${total} solved`}
      >
        <span style={{ width: `${pct}%`, background: color }} />
      </div>
    </div>
  )
}

function Recent({
  items,
  titles,
  langs,
}: {
  items: SubmissionSummary[]
  titles: Map<string, string>
  langs: Map<string, string>
}) {
  if (items.length === 0) {
    return <p className={styles.panelText}>Nothing yet — open a problem and press Run; every run shows up here.</p>
  }
  return (
    <ul className={styles.recent}>
      {items.map((r) => {
        const status = RUN_STATUS[r.status]
        return (
          <li key={r.id} className={styles.recentRow}>
            <span className={clsx(styles.runStatus, styles[status.tone])}>{status.label}</span>
            <Link className={styles.recentTitle} to={`/problems/${encodeURIComponent(r.slug)}`}>
              {titles.get(r.slug) ?? r.slug}
            </Link>
            <span className={styles.recentMeta}>
              {langs.get(r.lang) ?? r.lang} · {r.kind === 'test' ? `tests ${r.passed}/${r.total}` : 'run'}
            </span>
            <time className={styles.recentTime} dateTime={r.created_at} title={formatDateTime(r.created_at)}>
              {timeAgo(r.created_at)}
            </time>
          </li>
        )
      })}
    </ul>
  )
}
