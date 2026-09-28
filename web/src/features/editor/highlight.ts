// Syntax colouring shared by the practice editor and the Visualize code panel.
//
// The desktop's highlighter (crates/dsa-app/src/highlight.rs) sorts every token
// into seven roles — text, keyword, type, string, number, comment, punctuation
// — and the palette assigns one colour per role. The web uses the Lezer
// grammars behind the CodeMirror language modes instead of a hand tokenizer
// (they survive block comments and multi-line strings in user code, which the
// line-oriented desktop scanner never had to), and folds their much finer tag
// set back onto those same seven roles. One highlighter serves both surfaces,
// so the reference solution and the user's own code are coloured alike.
//
// Roles are plain global class names (see highlight.css) rather than a
// CSS-module map: CodeMirror's highlighter and `highlightLines` both need the
// literal strings, and the colours come from the shared `--code-*` tokens.

import { highlightTree, tagHighlighter, tags as t } from '@lezer/highlight'
import { languageSupport } from './languages'
import './highlight.css'

export const HL = {
  keyword: 'syn-kw',
  type: 'syn-type',
  string: 'syn-str',
  number: 'syn-num',
  comment: 'syn-com',
  punct: 'syn-punct',
} as const

/**
 * Lezer tag → desktop role. `tagHighlighter` resolves a node through its tag's
 * parents, so `t.keyword` also covers control/definition/module keywords,
 * modifiers, `self` and `null`; `t.standard(t.typeName)` is how the C++ and
 * Java grammars mark `int`/`bool`/`void`, which the desktop lists as keywords.
 */
export const codeHighlighter = tagHighlighter([
  { tag: [t.keyword, t.bool, t.processingInstruction, t.standard(t.typeName)], class: HL.keyword },
  { tag: [t.typeName, t.className, t.namespace], class: HL.type },
  { tag: [t.string, t.escape], class: HL.string },
  { tag: t.number, class: HL.number },
  { tag: t.comment, class: HL.comment },
  { tag: [t.punctuation, t.operator], class: HL.punct },
])

/** One run of same-coloured text within a line. `cls` is null for plain text. */
export interface HlSpan {
  text: string
  cls: string | null
}

/**
 * Highlight a whole source file into lines of spans, for renderers that draw
 * row by row (the Visualize code panel puts a breakpoint gutter beside each
 * line, which a CodeMirror instance would make needlessly awkward).
 *
 * Line splitting follows Rust's `str::lines`, which is how the desktop and the
 * server count lines: a trailing newline does not start an extra empty line.
 * That matters because `ProblemSource.tag_lines` is 1-based over those lines.
 */
export function highlightLines(code: string, syntax: string): HlSpan[][] {
  const text = code.replace(/\r\n?/g, '\n')
  const lines: HlSpan[][] = [[]]
  const push = (chunk: string, cls: string | null) => {
    const parts = chunk.split('\n')
    parts.forEach((part, i) => {
      if (i > 0) lines.push([])
      if (part) lines[lines.length - 1].push({ text: part, cls })
    })
  }

  const support = languageSupport(syntax)
  if (support) {
    const tree = support.language.parser.parse(text)
    let pos = 0
    highlightTree(tree, codeHighlighter, (from, to, classes) => {
      if (from > pos) push(text.slice(pos, from), null)
      push(text.slice(from, to), classes)
      pos = to
    })
    if (pos < text.length) push(text.slice(pos), null)
  } else {
    push(text, null)
  }

  if (text.endsWith('\n')) lines.pop()
  return text === '' ? [] : lines
}
