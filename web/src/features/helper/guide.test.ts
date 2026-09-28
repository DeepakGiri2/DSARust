// The helper's rules; the first two cases are ported from helper.rs.
import { describe, expect, it } from 'vitest'
import { CATALOG, GUIDE } from '@/pages/shell/test-utils'
import { cheatDefaults, filterCheatsheet, guideLanguages, restFor, startLanguage, topicsFor } from './guide'

const langs = guideLanguages(GUIDE, CATALOG.languages)

describe('helper', () => {
  it('opening defaults the cheat sheet to a different language', () => {
    expect(cheatDefaults('go', langs)).toEqual({ from: 'go', to: 'cpp' })
    // Comparing a language with itself is useless.
    expect(cheatDefaults('cpp', langs)).toEqual({ from: 'cpp', to: 'go' })
    expect(cheatDefaults('java', langs)).toEqual({ from: 'java', to: 'cpp' })
  })

  it('carries the problem’s language into the syntax box, when there is code for it', () => {
    expect(startLanguage('java', langs)).toBe('java')
    expect(startLanguage('python', langs)).toBe('go') // no Python samples in this guide
  })

  it('offers the languages the guide has code for, in catalogue order', () => {
    expect(langs).toEqual([
      { id: 'go', label: 'Go' },
      { id: 'cpp', label: 'C++' },
      { id: 'java', label: 'Java' },
    ])
    expect(guideLanguages(GUIDE, [])).toEqual([
      { id: 'cpp', label: 'cpp' },
      { id: 'go', label: 'go' },
      { id: 'java', label: 'java' },
    ])
  })

  it('shows a category’s topics first, in the guide’s reading order', () => {
    expect(topicsFor(GUIDE, 'Arrays & Hashing').map((t) => t.id)).toEqual(['hashmap', 'array'])
    expect(restFor(GUIDE, 'Arrays & Hashing').map((t) => t.id)).toEqual(['twopointers'])
  })

  it('without a category, everything is “everything else”', () => {
    expect(topicsFor(GUIDE, undefined)).toEqual([])
    expect(restFor(GUIDE, undefined)).toHaveLength(GUIDE.topics.length)
    expect(topicsFor(GUIDE, 'Unknown Category')).toEqual([])
  })

  it('filters the cheat sheet by topic and by code in the two shown languages', () => {
    const rows = (q: string, shown = ['go', 'cpp']) =>
      filterCheatsheet(GUIDE.cheatsheet, q, shown).flatMap((s) => s.rows.map((r) => r.topic))
    expect(rows('')).toEqual(['declare variable', 'function'])
    expect(rows('FUNC')).toEqual(['function'])
    expect(rows(':=')).toEqual(['declare variable'])
    expect(rows(':=', ['java', 'cpp'])).toEqual([]) // Go is not on screen
    expect(rows('basics')).toEqual(['declare variable', 'function'])
  })
})
