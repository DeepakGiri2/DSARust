// The editor's chrome, matching the desktop practice editor: code on the
// panel surface, a quiet line-number gutter, the accent for the caret and the
// selection. Everything is a CSS variable, so the one theme serves both the
// dark and the light palette without being rebuilt when the theme flips.

import { EditorView } from '@codemirror/view'

export const editorChrome = EditorView.theme({
  '&': {
    height: '100%',
    color: 'var(--code-text)',
    backgroundColor: 'transparent',
    fontSize: '13px',
  },
  '&.cm-focused': { outline: 'none' },
  '.cm-scroller': {
    fontFamily: 'var(--mono)',
    lineHeight: '1.6',
    overflow: 'auto',
  },
  '.cm-content': { caretColor: 'var(--accent)', padding: '4px 0 24px' },
  '.cm-cursor, .cm-dropCursor': { borderLeft: '2px solid var(--accent)' },
  '&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection':
    { backgroundColor: 'color-mix(in srgb, var(--accent) 34%, transparent)' },
  '.cm-activeLine': { backgroundColor: 'color-mix(in srgb, var(--accent) 7%, transparent)' },
  '.cm-gutters': {
    backgroundColor: 'transparent',
    border: 'none',
    color: 'var(--text-dim)',
  },
  '.cm-lineNumbers .cm-gutterElement': {
    minWidth: '30px',
    padding: '0 12px 0 4px',
    fontSize: '11px',
  },
  '.cm-activeLineGutter': { backgroundColor: 'transparent', color: 'var(--text)' },
  '&.cm-focused .cm-matchingBracket': {
    backgroundColor: 'transparent',
    outline: '1px solid var(--accent-2)',
    borderRadius: '2px',
  },
  '&.cm-focused .cm-nonmatchingBracket': {
    backgroundColor: 'transparent',
    outline: '1px solid var(--red)',
  },
})
