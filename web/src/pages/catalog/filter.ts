// The catalogue's filter: `list.rs::visible` and `progress.rs::accepts` as one
// pure function. The "N shown" pill and every category section read the same
// result, so the count can never disagree with the rows — and the rules can be
// tested without a DOM.

import type {
  CatalogCategory,
  CatalogProblem,
  Playlist,
  ProgressSnapshot,
  ProgressStatus,
  StatusFilter,
  Tier,
  Uuid,
} from '@/api/types'

/** Smallest to largest; each tier contains every tier before it. */
export const TIER_ORDER: readonly Tier[] = ['50', '150', '250', 'extra']

export const STATUS_FILTERS: readonly { value: StatusFilter; label: string }[] = [
  { value: 'all', label: 'all' },
  { value: 'todo', label: 'to do' },
  { value: 'attempted', label: 'attempted' },
  { value: 'solved', label: 'solved' },
]

/** A problem shows in a tier if it belongs to that tier or a smaller one (`Tier::contains`). */
export function tierContains(tier: Tier, item: Tier): boolean {
  const t = TIER_ORDER.indexOf(tier)
  const i = TIER_ORDER.indexOf(item)
  return i >= 0 && i <= t
}

export interface CatalogFilters {
  tier: Tier
  vizOnly: boolean
  search: string
  status: StatusFilter
  favouritesOnly: boolean
  playlist: Uuid | null
}

/**
 * The profile's side of the filter. A guest passes `null` instead, and the
 * progress filters then filter nothing away: the catalogue has to stay usable
 * before anyone has signed in, and a control that is shown disabled must not
 * be quietly narrowing the list.
 */
export interface ProgressLookup {
  status(slug: string): ProgressStatus
  favourite(slug: string): boolean
  /** Member slugs, or undefined for a playlist that does not exist (any more). */
  playlist(id: Uuid): ReadonlySet<string> | undefined
}

export function makeProgressLookup(snapshot: ProgressSnapshot, playlists: readonly Playlist[]): ProgressLookup {
  const members = new Map(playlists.map((p) => [p.id, new Set(p.slugs)]))
  return {
    // Untouched problems have no entry; absence means "to do".
    status: (slug) => snapshot.entries[slug]?.status ?? 'todo',
    favourite: (slug) => snapshot.entries[slug]?.favourite ?? false,
    playlist: (id) => members.get(id),
  }
}

export function statusAccepts(filter: StatusFilter, status: ProgressStatus): boolean {
  return filter === 'all' || filter === status
}

/** `Progress::accepts` — does this problem pass the progress-side filters? */
export function progressAccepts(
  progress: ProgressLookup,
  slug: string,
  f: Pick<CatalogFilters, 'status' | 'favouritesOnly' | 'playlist'>,
): boolean {
  if (!statusAccepts(f.status, progress.status(slug))) return false
  if (f.favouritesOnly && !progress.favourite(slug)) return false
  if (f.playlist !== null) {
    const members = progress.playlist(f.playlist)
    // A playlist that has been deleted filters nothing, rather than hiding the
    // whole catalogue behind a filter you cannot see.
    if (members && !members.has(slug)) return false
  }
  return true
}

/** The whole test for one row: the catalogue's own filters, then the profile's. */
export function problemVisible(
  problem: CatalogProblem,
  f: CatalogFilters,
  progress: ProgressLookup | null,
  needle = f.search.trim().toLowerCase(),
): boolean {
  return (
    tierContains(f.tier, problem.tier) &&
    (!f.vizOnly || problem.viz) &&
    (needle === '' || problem.title.toLowerCase().includes(needle)) &&
    (progress === null || progressAccepts(progress, problem.slug, f))
  )
}

export interface CatalogSection {
  name: string
  problems: CatalogProblem[]
}

export interface FilterResult {
  /** Categories in catalogue order, empty ones dropped. */
  sections: CatalogSection[]
  /** Rows shown across every section — the "N shown" pill. */
  shown: number
}

export function filterCatalog(
  categories: readonly CatalogCategory[],
  f: CatalogFilters,
  progress: ProgressLookup | null,
): FilterResult {
  const needle = f.search.trim().toLowerCase()
  const sections: CatalogSection[] = []
  let shown = 0
  for (const category of categories) {
    const problems = category.problems.filter((p) => problemVisible(p, f, progress, needle))
    if (problems.length > 0) {
      sections.push({ name: category.name, problems })
      shown += problems.length
    }
  }
  return { sections, shown }
}
