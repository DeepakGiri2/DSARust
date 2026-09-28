// Language modes, keyed by `Language.syntax` from the catalogue.
//
// Languages are data on this platform (content/languages.toml), so nothing
// here switches on a closed set of ids: an unknown syntax hint is plain text,
// exactly as the desktop's highlighter treats it — the code still shows, it
// just is not coloured.

import type { LanguageSupport } from '@codemirror/language'
import { cpp } from '@codemirror/lang-cpp'
import { go } from '@codemirror/lang-go'
import { java } from '@codemirror/lang-java'
import { python } from '@codemirror/lang-python'

/** The spellings `languages.toml` and the desktop's `highlight::keywords` accept. */
const FACTORIES: Record<string, () => LanguageSupport> = {
  go,
  cpp,
  'c++': cpp,
  c: cpp,
  java,
  python,
  py: python,
}

const cache = new Map<string, LanguageSupport>()

/**
 * The CodeMirror language for a syntax hint, or null for plain text.
 *
 * Cached so the practice editor and the read-only code panel share one parser
 * instance per language — building a `LanguageSupport` is cheap, but handing
 * CodeMirror a fresh one on every language switch would re-parse for nothing.
 */
export function languageSupport(syntax: string): LanguageSupport | null {
  const key = syntax.trim().toLowerCase()
  const hit = cache.get(key)
  if (hit) return hit
  const make = FACTORIES[key]
  if (!make) return null
  const support = make()
  cache.set(key, support)
  return support
}
