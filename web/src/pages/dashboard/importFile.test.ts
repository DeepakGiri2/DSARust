import { describe, expect, it } from 'vitest'
import { parseImport } from './importFile'

const known = new Set(['two-sum', '3sum', 'valid-sudoku'])

describe('desktop import', () => {
  it('previews what will be merged and skips problems the catalogue lacks', () => {
    const r = parseImport(
      JSON.stringify({
        entries: {
          'two-sum': { status: 'solved', favourite: true, attempts: 3 },
          '3sum': { status: 'attempted', favourite: false, attempts: 1 },
          'renamed-problem': { status: 'solved', favourite: false, attempts: 1 },
        },
        playlists: [{ name: ' week 1 ', slugs: ['two-sum', 'gone'] }],
      }),
      known,
    )
    expect(r.ok).toBe(true)
    if (!r.ok) return
    expect(r.preview).toEqual({
      solved: 1,
      attempted: 1,
      favourites: 1,
      playlists: 1,
      playlistItems: 1,
      skippedEntries: 1,
      skippedItems: 1,
    })
    expect(Object.keys(r.request.entries)).toEqual(['two-sum', '3sum'])
    expect(r.request.playlists).toEqual([{ name: 'week 1', slugs: ['two-sum'] }])
  })

  it('fills in a missing star or attempt count', () => {
    const r = parseImport(JSON.stringify({ entries: { 'two-sum': { status: 'solved' } } }), known)
    expect(r.ok && r.request.entries['two-sum']).toEqual({ status: 'solved', favourite: false, attempts: 0 })
    expect(r.ok && r.request.playlists).toEqual([])
  })

  it('explains a file that is not an export', () => {
    expect(parseImport('not json', known)).toEqual({ ok: false, error: 'That file isn’t valid JSON.' })
    expect(parseImport('[]', known).ok).toBe(false)
    expect(parseImport('{"hello": 1}', known).ok).toBe(false)
    expect(parseImport('{"entries": []}', known).ok).toBe(false)
  })

  it('names the entry that is wrong', () => {
    const r = parseImport(JSON.stringify({ entries: { 'two-sum': { status: 'done' } } }), known)
    expect(r).toEqual({ ok: false, error: 'entries["two-sum"].status should be "todo", "attempted" or "solved".' })
    const bad = parseImport(JSON.stringify({ entries: { 'two-sum': { status: 'solved', attempts: -1 } } }), known)
    expect(bad.ok).toBe(false)
    const list = parseImport(JSON.stringify({ playlists: [{ name: 'x', slugs: [1] }] }), known)
    expect(list.ok).toBe(false)
  })

  it('refuses a file with nothing this catalogue knows', () => {
    const r = parseImport(JSON.stringify({ entries: { 'no-such': { status: 'solved' } }, playlists: [] }), known)
    expect(r).toEqual({ ok: false, error: 'Nothing in that file matches a problem in the catalogue.' })
  })
})
