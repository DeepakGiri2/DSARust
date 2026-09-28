// Reading a desktop progress export (`ImportRequest`) before it is sent.
//
// Checked here so a wrong file gets a sentence rather than a 422, and so the
// preview can say what will happen. Problems the catalogue does not have are
// dropped and counted: the server refuses unknown slugs, and one renamed
// problem should not sink the whole import.

import type { ImportRequest, ProgressStatus } from '@/api/types'

export const MAX_IMPORT_BYTES = 5 * 1024 * 1024

export interface ImportPreview {
  solved: number
  attempted: number
  favourites: number
  playlists: number
  playlistItems: number
  /** Progress rows for problems the catalogue does not know. */
  skippedEntries: number
  /** Playlist members the catalogue does not know. */
  skippedItems: number
}

export type ParsedImport =
  | { ok: true; request: ImportRequest; preview: ImportPreview }
  | { ok: false; error: string }

const STATUSES: readonly ProgressStatus[] = ['todo', 'attempted', 'solved']

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v)
}

function fail(error: string): ParsedImport {
  return { ok: false, error }
}

export function parseImport(text: string, known: ReadonlySet<string>): ParsedImport {
  let data: unknown
  try {
    data = JSON.parse(text)
  } catch {
    return fail('That file isn’t valid JSON.')
  }
  if (!isRecord(data) || (data.entries === undefined && data.playlists === undefined)) {
    return fail('That doesn’t look like a desktop progress export — it has no "entries" or "playlists".')
  }
  const rawEntries = data.entries ?? {}
  const rawPlaylists = data.playlists ?? []
  if (!isRecord(rawEntries)) return fail('"entries" should map each problem’s slug to its progress.')
  if (!Array.isArray(rawPlaylists)) return fail('"playlists" should be a list.')

  const entries: ImportRequest['entries'] = {}
  const preview: ImportPreview = {
    solved: 0,
    attempted: 0,
    favourites: 0,
    playlists: 0,
    playlistItems: 0,
    skippedEntries: 0,
    skippedItems: 0,
  }

  for (const [slug, value] of Object.entries(rawEntries)) {
    if (!isRecord(value)) return fail(`entries["${slug}"] should be an object.`)
    const { status, favourite = false, attempts = 0 } = value
    if (typeof status !== 'string' || !STATUSES.includes(status as ProgressStatus)) {
      return fail(`entries["${slug}"].status should be "todo", "attempted" or "solved".`)
    }
    if (typeof favourite !== 'boolean') return fail(`entries["${slug}"].favourite should be true or false.`)
    if (typeof attempts !== 'number' || !Number.isInteger(attempts) || attempts < 0) {
      return fail(`entries["${slug}"].attempts should be a whole number.`)
    }
    if (!known.has(slug)) {
      preview.skippedEntries++
      continue
    }
    entries[slug] = { status: status as ProgressStatus, favourite, attempts }
    if (status === 'solved') preview.solved++
    else if (status === 'attempted') preview.attempted++
    if (favourite) preview.favourites++
  }

  const playlists: ImportRequest['playlists'] = []
  for (const [i, p] of rawPlaylists.entries()) {
    if (!isRecord(p) || typeof p.name !== 'string' || !p.name.trim()) {
      return fail(`playlists[${i}] needs a "name".`)
    }
    if (!Array.isArray(p.slugs) || !p.slugs.every((s): s is string => typeof s === 'string')) {
      return fail(`playlists[${i}].slugs should be a list of problem slugs.`)
    }
    const slugs = p.slugs.filter((s) => known.has(s))
    preview.skippedItems += p.slugs.length - slugs.length
    preview.playlistItems += slugs.length
    playlists.push({ name: p.name.trim(), slugs })
  }
  preview.playlists = playlists.length

  if (Object.keys(entries).length === 0 && playlists.length === 0) {
    return fail('Nothing in that file matches a problem in the catalogue.')
  }
  return { ok: true, request: { entries, playlists }, preview }
}
