// Ported from the tests in crates/dsa-app/src/list.rs and progress.rs — the
// same cases, against the one function the count and the rows share.
import { describe, expect, it } from 'vitest'
import type { CatalogCategory, CatalogProblem, Playlist, ProgressEntry, Tier } from '@/api/types'
import {
  filterCatalog,
  makeProgressLookup,
  progressAccepts,
  tierContains,
  TIER_ORDER,
  type CatalogFilters,
  type ProgressLookup,
} from './filter'

const item = (title: string, tier: Tier, viz = true, category = 'Arrays & Hashing'): CatalogProblem => ({
  slug: title.toLowerCase().replace(/ /g, '-'),
  title,
  category,
  difficulty: 'Easy',
  tier,
  leetcode_url: '',
  viz,
  langs: ['go', 'cpp', 'java'],
  premium: false,
})

const BASE: CatalogFilters = {
  tier: '250',
  vizOnly: false,
  search: '',
  status: 'all',
  favouritesOnly: false,
  playlist: null,
}

const one = (problems: CatalogProblem[]): CatalogCategory[] => [{ name: 'Arrays & Hashing', problems }]

function titles(problems: CatalogProblem[], f: Partial<CatalogFilters>, progress: ProgressLookup | null = null) {
  return filterCatalog(one(problems), { ...BASE, ...f }, progress).sections.flatMap((s) => s.problems.map((p) => p.title))
}

const entry = (e: Partial<ProgressEntry>): ProgressEntry => ({
  status: 'todo',
  favourite: false,
  attempts: 0,
  solved_at: null,
  updated_at: '2026-01-01T00:00:00Z',
  ...e,
})

/** A profile with Two Sum solved and Group Anagrams starred (list.rs `progress_with_history`). */
function withHistory(playlists: Playlist[] = []): ProgressLookup {
  return makeProgressLookup(
    {
      entries: { 'two-sum': entry({ status: 'solved' }), 'group-anagrams': entry({ favourite: true }) },
      stats: { solved: 1, attempted: 0, favourites: 1 },
    },
    playlists,
  )
}

const three = () => [item('Two Sum', '50'), item('Group Anagrams', '50'), item('Valid Sudoku', '50')]

const playlist = (id: string, slugs: string[]): Playlist => ({ id, name: id, slugs, created_at: '2026-01-01T00:00:00Z' })

describe('catalogue filter (list.rs)', () => {
  it('the tier filter nests', () => {
    const items = [item('Two Sum', '50'), item('Valid Sudoku', '150', false), item('Sort Colors', '250', false)]
    expect(titles(items, { tier: '50' })).toEqual(['Two Sum'])
    expect(titles(items, { tier: '150' })).toHaveLength(2)
    expect(titles(items, { tier: '250' })).toHaveLength(3)
  })

  it('the interview extra tier shows everything', () => {
    for (const t of TIER_ORDER) expect(tierContains('extra', t)).toBe(true)
    expect(tierContains('250', 'extra')).toBe(false)
    expect(titles([item('Rotate Array', 'extra'), item('Two Sum', '50')], { tier: 'extra' })).toHaveLength(2)
  })

  it('interactive only hides unbuilt problems', () => {
    expect(titles([item('Two Sum', '50'), item('Sort Colors', '50', false)], { vizOnly: true })).toEqual(['Two Sum'])
  })

  it('search is case-insensitive and matches anywhere', () => {
    const items = [item('Two Sum', '50'), item('Group Anagrams', '50')]
    expect(titles(items, { search: 'SUM' })).toEqual(['Two Sum'])
    expect(titles(items, { search: ' anagram ' })).toEqual(['Group Anagrams'])
    expect(titles(items, { search: 'zzz' })).toHaveLength(0)
  })

  it('the progress filter narrows the same list the count reports', () => {
    const p = withHistory()
    const solved = filterCatalog(one(three()), { ...BASE, status: 'solved' }, p)
    expect(solved.sections.flatMap((s) => s.problems.map((x) => x.title))).toEqual(['Two Sum'])
    expect(solved.shown).toBe(1)
    expect(titles(three(), { status: 'todo' }, p)).toEqual(['Group Anagrams', 'Valid Sudoku'])
    expect(titles(three(), { status: 'all' }, p)).toHaveLength(3)
  })

  it('favourites stack with search and tier', () => {
    const p = withHistory()
    expect(titles(three(), { favouritesOnly: true }, p)).toEqual(['Group Anagrams'])
    expect(titles(three(), { favouritesOnly: true, search: 'sudoku' }, p)).toHaveLength(0)
  })

  it('without a profile nothing is filtered away', () => {
    // The catalogue has to stay usable before anyone has signed in — and a
    // guest's disabled controls must not narrow it either.
    expect(titles(three(), { status: 'solved', favouritesOnly: true, playlist: 'p1' }, null)).toHaveLength(3)
  })

  it('keeps categories in catalogue order and drops the empty ones', () => {
    const categories: CatalogCategory[] = [
      { name: 'Arrays & Hashing', problems: [item('Two Sum', '50')] },
      { name: 'Stack', problems: [item('Min Stack', '150', true, 'Stack')] },
      { name: 'Trees', problems: [item('Invert Binary Tree', '50', true, 'Trees')] },
    ]
    const r = filterCatalog(categories, { ...BASE, tier: '50' }, null)
    expect(r.sections.map((s) => s.name)).toEqual(['Arrays & Hashing', 'Trees'])
    expect(r.shown).toBe(2)
  })
})

describe('progress filter (progress.rs)', () => {
  const lookup = (entries: Record<string, ProgressEntry>, playlists: Playlist[] = []) =>
    makeProgressLookup({ entries, stats: { solved: 0, attempted: 0, favourites: 0 } }, playlists)

  it('the status filter partitions the catalogue', () => {
    const p = lookup({ 'solved-one': entry({ status: 'solved' }), 'tried-one': entry({ status: 'attempted' }) })
    const slugs = ['solved-one', 'tried-one', 'fresh-one']
    const count = (status: CatalogFilters['status']) =>
      slugs.filter((s) => progressAccepts(p, s, { status, favouritesOnly: false, playlist: null })).length
    expect(count('all')).toBe(3)
    expect(count('solved')).toBe(1)
    expect(count('attempted')).toBe(1)
    expect(count('todo')).toBe(1)
  })

  it('favourites and the status filter together', () => {
    const p = lookup({
      'two-sum': entry({ status: 'solved', favourite: true }),
      '3sum': entry({ favourite: true }),
    })
    const accepts = (slug: string, status: CatalogFilters['status']) =>
      progressAccepts(p, slug, { status, favouritesOnly: true, playlist: null })
    expect(accepts('two-sum', 'solved')).toBe(true)
    expect(accepts('3sum', 'solved')).toBe(false) // starred, not solved
    expect(accepts('3sum', 'all')).toBe(true)
    expect(accepts('valid-sudoku', 'all')).toBe(false)
  })

  it('a playlist filter shows only its members', () => {
    const on = lookup({}, [playlist('week', ['two-sum'])])
    const f = { status: 'all' as const, favouritesOnly: false, playlist: 'week' }
    expect(progressAccepts(on, 'two-sum', f)).toBe(true)
    expect(progressAccepts(on, '3sum', f)).toBe(false)
    const off = lookup({}, [playlist('week', [])])
    expect(progressAccepts(off, 'two-sum', f)).toBe(false)
  })

  it('a deleted playlist stops filtering instead of hiding everything', () => {
    const p = lookup({}, [])
    const f = { status: 'all' as const, favouritesOnly: false, playlist: 'gone' }
    expect(progressAccepts(p, 'two-sum', f)).toBe(true)
    expect(progressAccepts(p, '3sum', f)).toBe(true)
  })

  it('an untouched problem is to do', () => {
    const p = lookup({})
    expect(p.status('anything')).toBe('todo')
    expect(p.favourite('anything')).toBe(false)
  })
})
