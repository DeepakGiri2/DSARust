import type { Language, ProblemSource } from '@/api/types'

/** Used only until the catalogue (which carries the real labels) has loaded. */
const FALLBACK_LABELS: Record<string, string> = { go: 'Go', cpp: 'C++', java: 'Java', python: 'Python' }

export interface LangInfo {
  id: string
  label: string
  /** Highlighter hint for the editor and code panel. */
  syntax: string
  /** The platform can compile and run it. Optimistic until the catalogue says otherwise — the server has the final word. */
  runnable: boolean
}

export function langInfo(id: string, languages: readonly Language[] | undefined): LangInfo {
  const l = languages?.find((x) => x.id === id)
  return {
    id,
    label: l?.label ?? FALLBACK_LABELS[id] ?? id,
    syntax: l?.syntax || id,
    runnable: l?.runnable ?? true,
  }
}

/**
 * The language to show: the remembered choice when this problem has it,
 * otherwise the first source — `Problem.sources` is in manifest order, so
 * that is the leftmost tab, as on the desktop. Unlike the desktop, the
 * fallback is not written back to settings: opening a Go-only problem should
 * not silently switch a Java user's preference for every other problem.
 */
export function pickLang(preferred: string, sources: readonly ProblemSource[]): string | null {
  if (sources.some((s) => s.lang === preferred)) return preferred
  return sources[0]?.lang ?? null
}
