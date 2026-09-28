// The helper's rules, as pure functions: which topics a category reads first
// (dsa-content's `Guide::for_category` / `rest_for`), which languages there is
// code for, and how the cheat sheet opens and filters (helper.rs).

import type { CheatSection, Guide, GuideTopic, Language } from '@/api/types'

/** Topics relevant to a problem category, in the order the guide lists them. */
export function topicsFor(guide: Guide, category: string | undefined): GuideTopic[] {
  if (!category) return []
  const byId = new Map(guide.topics.map((t) => [t.id, t]))
  return (guide.by_category[category] ?? []).flatMap((id) => byId.get(id) ?? [])
}

/** Everything not relevant to `category`, for the "everything else" group. */
export function restFor(guide: Guide, category: string | undefined): GuideTopic[] {
  const relevant = new Set(category ? (guide.by_category[category] ?? []) : [])
  return guide.topics.filter((t) => !relevant.has(t.id))
}

export interface LangOption {
  id: string
  label: string
}

/** Languages the guide has code for, in the catalogue's order and with its labels. */
export function guideLanguages(guide: Guide, languages: readonly Language[]): LangOption[] {
  const ids = new Set<string>()
  for (const t of guide.topics) Object.keys(t.syntax).forEach((id) => ids.add(id))
  for (const s of guide.cheatsheet) for (const r of s.rows) Object.keys(r.code).forEach((id) => ids.add(id))
  const known = [...languages]
    .sort((a, b) => a.order - b.order)
    .filter((l) => ids.has(l.id))
    .map((l) => ({ id: l.id, label: l.label }))
  const unknown = [...ids]
    .filter((id) => !languages.some((l) => l.id === id))
    .sort()
    .map((id) => ({ id, label: id }))
  return [...known, ...unknown]
}

/** The language to start the syntax box on: the one being worked in, when there is code for it. */
export function startLanguage(lang: string, available: readonly LangOption[]): string {
  return available.some((l) => l.id === lang) ? lang : (available[0]?.id ?? lang)
}

/**
 * The cheat sheet opens on "the language you are in → another one" —
 * comparing a language with itself is useless, and C++ is the usual other
 * side except for someone already in C++ (`Helper::open`).
 */
export function cheatDefaults(lang: string, available: readonly LangOption[]): { from: string; to: string } {
  const from = startLanguage(lang, available)
  const preferred = from === 'cpp' ? 'go' : 'cpp'
  const ids = available.map((l) => l.id)
  const to = ids.includes(preferred) ? preferred : (ids.find((id) => id !== from) ?? from)
  return { from, to }
}

/**
 * Rows whose topic, or code in either shown language, mentions the query. A
 * section whose own name matches keeps all its rows; empty sections go.
 */
export function filterCheatsheet(
  sections: readonly CheatSection[],
  query: string,
  langs: readonly string[],
): CheatSection[] {
  const q = query.trim().toLowerCase()
  if (!q) return [...sections]
  return sections
    .map((s) =>
      s.name.toLowerCase().includes(q)
        ? s
        : {
            ...s,
            rows: s.rows.filter(
              (r) =>
                r.topic.toLowerCase().includes(q) ||
                langs.some((l) => (r.code[l] ?? '').toLowerCase().includes(q)),
            ),
          },
    )
    .filter((s) => s.rows.length > 0)
}
