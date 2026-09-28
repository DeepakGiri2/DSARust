// The problem list — the home screen, ported from crates/dsa-app/src/list.rs.
//
// Same layout as the desktop: a hero with the three numbers worth claiming, a
// raised filter card (the catalogue's filters on one line, the profile's on
// the next), then one section per category with a 📘 guide button and a row
// per problem. The filters are the profile's `Settings`, so they follow the
// person between devices; the search lives in the URL, so "back" from a
// problem lands on the same list, as it does on the desktop.

import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { Link, useLocation, useSearchParams } from 'react-router'
import clsx from 'clsx'
import { errorMessage } from '@/api/client'
import {
  useCatalog,
  usePlaylists,
  useProfileSettings,
  useProgress,
  useSetFavourite,
} from '@/api/hooks'
import type { Catalog, CatalogProblem, Playlist, ProfileStats, ProgressEntry, Tier } from '@/api/types'
import { nextParam } from '@/app/guards'
import { HelperModal } from '@/features/helper/HelperModal'
import { plural } from '@/pages/shell/format'
import { usePageTitle } from '@/pages/shell/usePageTitle'
import { useActiveProfileId, useSession } from '@/state/session'
import { useSettings } from '@/state/settings'
import { EmptyState, ErrorState, PageSpinner, Seg, useToast } from '@/ui'
import {
  filterCatalog,
  makeProgressLookup,
  STATUS_FILTERS,
  TIER_ORDER,
  type CatalogFilters,
  type ProgressLookup,
} from './filter'
import { PlaylistPicker } from './PlaylistPicker'
import { PlaylistsModal } from './PlaylistsModal'
import { ProblemRow } from './ProblemRow'
import { SignInPrompt } from './SignInPrompt'
import styles from './CatalogPage.module.css'

export function Component() {
  usePageTitle(null)
  const catalog = useCatalog()
  if (catalog.isPending) return <PageSpinner label="Loading the problem list…" />
  if (catalog.isError) {
    return (
      <div className={styles.page}>
        <ErrorState error={catalog.error} onRetry={() => void catalog.refetch()} />
      </div>
    )
  }
  return <CatalogScreen catalog={catalog.data} />
}

/** "NeetCode 150" → "150" and "+ Interview Extra" → "+ Extra" on a phone. */
function tierLabel(title: string): ReactNode {
  return title.split(/(NeetCode |Interview )/).map((part, i) =>
    part === 'NeetCode ' || part === 'Interview ' ? (
      <span key={i} className={styles.long}>
        {part}
      </span>
    ) : (
      part
    ),
  )
}

const sectionId = (name: string) => `cat-${name.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`

function CatalogScreen({ catalog }: { catalog: Catalog }) {
  const session = useSession()
  const pid = useActiveProfileId()
  // Progress belongs to a profile; without one (a guest) there is nothing to
  // filter by, and the controls say so instead of silently doing nothing.
  const tracking = session.status === 'authenticated' && pid !== null
  const progress = useProgress(pid)
  const playlists = usePlaylists(pid)
  const savedSettings = useProfileSettings(pid)
  const { settings, update } = useSettings()
  const { mutate: setFavourite } = useSetFavourite(pid)
  const toast = useToast()
  const location = useLocation()
  const [params, setParams] = useSearchParams()
  const search = params.get('q') ?? ''

  const [helper, setHelper] = useState<{ open: boolean; category?: string }>({ open: false })
  const [prompt, setPrompt] = useState(false)
  const [managing, setManaging] = useState(false)
  const searchRef = useRef<HTMLInputElement>(null)

  const tier: Tier = TIER_ORDER.includes(settings.tier) ? settings.tier : '150'
  const lookup = useMemo<ProgressLookup | null>(
    () => (tracking && progress.data ? makeProgressLookup(progress.data, playlists.data ?? []) : null),
    [tracking, progress.data, playlists.data],
  )
  const filters = useMemo<CatalogFilters>(
    () => ({
      tier,
      vizOnly: settings.viz_only,
      search,
      status: settings.status_filter,
      favouritesOnly: settings.favourites_only,
      playlist: settings.playlist,
    }),
    [tier, settings.viz_only, search, settings.status_filter, settings.favourites_only, settings.playlist],
  )
  const result = useMemo(() => filterCatalog(catalog.categories, filters, lookup), [catalog.categories, filters, lookup])

  // Until the profile's own state arrives the list would show the defaults
  // (NeetCode 150, nothing solved) and then jump; wait for it instead.
  const waiting =
    tracking &&
    (progress.isPending || savedSettings.isPending || (settings.playlist !== null && playlists.isPending))

  const languages = useMemo(() => {
    const used = new Set(catalog.categories.flatMap((c) => c.problems.flatMap((p) => p.langs)))
    return [...catalog.languages]
      .sort((a, b) => a.order - b.order)
      .filter((l) => used.has(l.id))
      .map((l) => l.label)
      .join(' · ')
  }, [catalog])

  const setSearch = useCallback(
    (q: string) =>
      setParams(
        (prev) => {
          const next = new URLSearchParams(prev)
          if (q) next.set('q', q)
          else next.delete('q')
          return next
        },
        { replace: true, preventScrollReset: true },
      ),
    [setParams],
  )

  // `/` jumps to the search box from anywhere on the page that is not already
  // taking text.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== '/' || e.ctrlKey || e.metaKey || e.altKey || e.defaultPrevented) return
      const t = e.target
      if (t instanceof HTMLElement && (t.isContentEditable || t.closest('input, textarea, select, [role="dialog"]'))) {
        return
      }
      e.preventDefault()
      searchRef.current?.focus()
      searchRef.current?.select()
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [])

  const toggleFavourite = useCallback(
    (slug: string, on: boolean) => {
      if (!tracking) {
        setPrompt(true)
        return
      }
      setFavourite(
        { slug, value: !on },
        { onError: (e) => toast.error(`Couldn’t save that star: ${errorMessage(e)}`) },
      )
    },
    [tracking, setFavourite, toast],
  )

  // Stable, because the dialogs re-run their focus handling when it changes.
  const closeHelper = useCallback(() => setHelper({ open: false }), [])
  const closePrompt = useCallback(() => setPrompt(false), [])
  const closeManager = useCallback(() => setManaging(false), [])
  const openGuide = useCallback((category?: string) => setHelper({ open: true, category }), [])

  const clearFilters = () => {
    update({ viz_only: false, status_filter: 'all', favourites_only: false, playlist: null })
    setSearch('')
  }

  const stats = tracking ? progress.data?.stats : undefined
  const entries = tracking ? progress.data?.entries : undefined
  const entitled = session.entitlements?.premium_content ?? false
  const next = nextParam(location)

  return (
    <div className={styles.page}>
      <header className={styles.hero}>
        <div className={styles.kicker}>
          <span className={clsx(styles.pill, styles.pillCyan)}>▶ step-through debugger</span>
          <span className={clsx(styles.pill, styles.pillDim)}>
            {catalog.animated} animated · {catalog.categories.length} categories
          </span>
          {languages && <span className={clsx(styles.pill, styles.pillViolet)}>{languages}</span>}
        </div>
        <h1 className={styles.title}>
          DSA <span className="grad-text">Visualized</span>
        </h1>
        <p className={styles.subtitle}>
          The NeetCode roadmaps, plus the interview extras they leave out. Every variable, map and
          pointer, <strong>one step at a time</strong>.
        </p>
      </header>

      <section className={styles.filters} aria-label="Filters">
        <div className={styles.filterRow}>
          <div className={styles.segScroll}>
            <Seg
              aria-label="Problem list"
              value={tier}
              onChange={(t) => update({ tier: t })}
              options={catalog.tiers.map((t) => ({
                value: t.id,
                label: tierLabel(t.title),
                title: `${plural(t.count, 'problem')} · ${t.animated} animated`,
              }))}
            />
          </div>
          <div className={styles.searchBox}>
            <input
              ref={searchRef}
              type="search"
              className={clsx('input', styles.search)}
              placeholder="search problems…"
              aria-label="Search problems"
              aria-keyshortcuts="/"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Escape' && search) {
                  e.preventDefault()
                  setSearch('')
                }
              }}
            />
            {!search && (
              <kbd className={styles.slash} aria-hidden>
                /
              </kbd>
            )}
          </div>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.viz_only}
              onChange={(e) => update({ viz_only: e.target.checked })}
            />
            interactive only
          </label>
          <button
            type="button"
            className="mini-btn"
            onClick={() => openGuide()}
            title="Data structures, techniques and a cross-language syntax cheat sheet"
          >
            <span aria-hidden>📘</span> guide
          </button>
          <button
            type="button"
            className="mini-btn"
            aria-pressed={settings.backdrop}
            aria-label="Animated background"
            title="Animated background — turn it off to stop the page repainting when idle"
            onClick={() => update({ backdrop: !settings.backdrop })}
          >
            <span aria-hidden>✨</span>
          </button>
          <span className={clsx(styles.pill, styles.pillCyan, styles.shown)} role="status">
            {result.shown} shown
          </span>
        </div>

        {/* The profile's filters, on their own line: they belong to whoever is
            signed in, where the row above belongs to the catalogue. */}
        <div className={clsx(styles.filterRow, !tracking && styles.locked)}>
          <Seg
            aria-label="Progress"
            value={tracking ? settings.status_filter : 'all'}
            onChange={(s) => update({ status_filter: s })}
            options={STATUS_FILTERS.map((f) => ({ ...f, disabled: !tracking }))}
          />
          <button
            type="button"
            className="mini-btn"
            aria-pressed={tracking && settings.favourites_only}
            disabled={!tracking}
            title="Only problems you have starred"
            onClick={() => update({ favourites_only: !settings.favourites_only })}
          >
            <span aria-hidden>{tracking && settings.favourites_only ? '★' : '☆'}</span> favourites
          </button>
          <PlaylistPicker
            playlists={playlists.data ?? []}
            selected={tracking ? settings.playlist : null}
            onSelect={(id) => update({ playlist: id })}
            onManage={() => setManaging(true)}
            disabled={!tracking}
          />
          {tracking ? (
            stats && <Counters stats={stats} />
          ) : (
            <span className={styles.hint}>
              <Link className="link" to={`/login?next=${next}`}>
                Sign in
              </Link>{' '}
              to track progress
            </span>
          )}
        </div>
      </section>

      {tracking && progress.isError && (
        <div className={styles.warn} role="alert">
          Couldn’t load your progress — the list is shown without it.
          <button type="button" className="mini-btn" onClick={() => void progress.refetch()}>
            try again
          </button>
        </div>
      )}

      {waiting ? (
        <PageSpinner label="Loading your progress…" />
      ) : result.shown === 0 ? (
        <NoMatches
          catalog={catalog}
          filters={filters}
          lookup={lookup}
          playlists={playlists.data ?? []}
          stats={stats}
          onClear={clearFilters}
          onShowAll={(patch) => update(patch)}
        />
      ) : (
        result.sections.map((section) => (
          <Section
            key={section.name}
            name={section.name}
            problems={section.problems}
            entries={entries}
            entitled={entitled}
            onGuide={openGuide}
            onToggleFavourite={toggleFavourite}
          />
        ))
      )}

      <footer className={styles.foot}>
        Problem lists follow the neetcode.io roadmaps (the “250” tier is an extended superset of the
        150); “Interview Extra” adds high-frequency problems and patterns the roadmap leaves out.
      </footer>

      <HelperModal open={helper.open} onClose={closeHelper} category={helper.category} lang={settings.lang} />
      <SignInPrompt open={prompt} onClose={closePrompt} next={next} />
      {tracking && pid && (
        <PlaylistsModal
          open={managing}
          onClose={closeManager}
          pid={pid}
          playlists={playlists.data ?? []}
          selected={settings.playlist}
          onSelectedDeleted={() => update({ playlist: null })}
        />
      )}
    </div>
  )
}

function Counters({ stats }: { stats: ProfileStats }) {
  // Nothing to celebrate yet is not worth two empty pills.
  if (stats.solved === 0 && stats.attempted === 0) return null
  return (
    <>
      <span className={clsx(styles.pill, styles.pillGreen)} title={`${stats.solved} solved`}>
        <span aria-hidden>✓</span> {stats.solved}
        <span className="visually-hidden"> solved</span>
      </span>
      {stats.attempted > 0 && (
        <span className={clsx(styles.pill, styles.pillAmber)} title={`${stats.attempted} attempted`}>
          <span aria-hidden>◐</span> {stats.attempted}
          <span className="visually-hidden"> attempted</span>
        </span>
      )}
    </>
  )
}

function Section({
  name,
  problems,
  entries,
  entitled,
  onGuide,
  onToggleFavourite,
}: {
  name: string
  problems: CatalogProblem[]
  entries: Record<string, ProgressEntry> | undefined
  entitled: boolean
  onGuide: (category: string) => void
  onToggleFavourite: (slug: string, favourite: boolean) => void
}) {
  const id = sectionId(name)
  return (
    <section className={styles.section} aria-labelledby={id}>
      <div className={styles.sectionHead}>
        <span className={styles.bar} aria-hidden />
        <h2 id={id} className={styles.sectionTitle}>
          {name}
        </h2>
        <span className={clsx(styles.pill, styles.pillDim)} aria-label={`${plural(problems.length, 'problem')} shown`}>
          {problems.length}
        </span>
        <button
          type="button"
          className="mini-btn"
          onClick={() => onGuide(name)}
          aria-label={`Guide to ${name}`}
          title={`How the structures and techniques behind “${name}” work — syntax, complexity, pitfalls`}
        >
          <span aria-hidden>📘</span> guide
        </button>
        <span className={styles.rule} aria-hidden />
      </div>
      <ul className={styles.rows}>
        {problems.map((p) => {
          const entry = entries?.[p.slug]
          return (
            <li key={p.slug}>
              <ProblemRow
                problem={p}
                status={entry?.status ?? 'todo'}
                favourite={entry?.favourite ?? false}
                locked={p.premium && !entitled}
                onToggleFavourite={onToggleFavourite}
              />
            </li>
          )
        })}
      </ul>
    </section>
  )
}

/** The empty list, explained by whichever filter emptied it. */
function NoMatches({
  catalog,
  filters,
  lookup,
  playlists,
  stats,
  onClear,
  onShowAll,
}: {
  catalog: Catalog
  filters: CatalogFilters
  lookup: ProgressLookup | null
  playlists: readonly Playlist[]
  stats: ProfileStats | undefined
  onClear: () => void
  onShowAll: (patch: { playlist?: null; favourites_only?: false; tier?: Tier }) => void
}) {
  const playlist = lookup ? playlists.find((p) => p.id === filters.playlist) : undefined
  if (playlist && playlist.slugs.length === 0) {
    return (
      <EmptyState title={`“${playlist.name}” is empty`}>
        <p className={styles.emptyText}>Add problems to it from the ♪ playlist menu on any problem page.</p>
        <button type="button" className="mini-btn" onClick={() => onShowAll({ playlist: null })}>
          show all problems
        </button>
      </EmptyState>
    )
  }
  if (lookup && filters.favouritesOnly && stats?.favourites === 0) {
    return (
      <EmptyState title="No favourites yet">
        <p className={styles.emptyText}>Star a problem with ☆ and it shows up here.</p>
        <button type="button" className="mini-btn" onClick={() => onShowAll({ favourites_only: false })}>
          show all problems
        </button>
      </EmptyState>
    )
  }

  const tierTitle = catalog.tiers.find((t) => t.id === filters.tier)?.title ?? filters.tier
  // A search that misses in this tier may still hit in a bigger one.
  const elsewhere =
    filters.search.trim() && filters.tier !== 'extra'
      ? filterCatalog(catalog.categories, { ...filters, tier: 'extra' }, lookup).shown
      : 0
  return (
    <EmptyState title="No problems match">
      <p className={styles.emptyText}>
        {filters.search.trim()
          ? `Nothing in ${tierTitle} matches “${filters.search.trim()}” with these filters.`
          : `Nothing in ${tierTitle} passes these filters.`}
      </p>
      <div className={styles.emptyActions}>
        <button type="button" className="mini-btn" onClick={onClear}>
          clear filters
        </button>
        {elsewhere > 0 && (
          <button type="button" className="mini-btn" onClick={() => onShowAll({ tier: 'extra' })}>
            search every list ({elsewhere})
          </button>
        )}
      </div>
    </EmptyState>
  )
}
