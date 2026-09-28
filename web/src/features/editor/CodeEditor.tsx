// The practice editor: one CodeMirror view for the lifetime of the component.
//
// Deliberately not `@uiw/react-codemirror`: that wrapper reconfigures every
// extension whenever its `onChange` prop changes identity, and it holds back
// external value updates while the user is typing. This editor swaps whole
// documents (language and view switches), receives code from outside (reset,
// the AI's fix, a restored draft) and must never echo those back as edits —
// so it owns the view directly:
//
//  * callbacks live in a ref, so the keymap and the update listener are built
//    once and a parent re-render costs nothing;
//  * `docKey` names the document. A new key parks the current EditorState and
//    restores (or creates) the new one, so each buffer keeps its own undo
//    history and undo can never pull another language's code into this one;
//  * a new `value` under the same key is applied as a minimal, undoable change
//    tagged `External`, which the update listener does not report.

import { useLayoutEffect, useId, useRef } from 'react'
import { Annotation, Compartment, EditorState, Prec, type Extension } from '@codemirror/state'
import {
  EditorView,
  drawSelection,
  dropCursor,
  highlightActiveLine,
  highlightActiveLineGutter,
  highlightSpecialChars,
  keymap,
  lineNumbers,
  rectangularSelection,
} from '@codemirror/view'
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands'
import { bracketMatching, indentOnInput, indentUnit, syntaxHighlighting } from '@codemirror/language'
import { codeHighlighter } from './highlight'
import { languageSupport } from './languages'
import { editorChrome } from './theme'

/** Tags a transaction as the parent pushing a value in, not the user typing. */
const External = Annotation.define<boolean>()

export interface CodeEditorProps {
  /** Identity of the document shown — e.g. `go:solution`. */
  docKey: string
  value: string
  /** `Language.syntax` hint (`go`, `cpp`, `java`, `python`). */
  syntax: string
  /** Called with the full text after every user edit (never for `value` updates). */
  onChange: (value: string) => void
  /** Ctrl/Cmd+Enter. */
  onRun?: () => void
  /** Ctrl/Cmd+'. */
  onTest?: () => void
  /** Accessible name of the editing surface. */
  label: string
  className?: string
}

interface Parked {
  state: EditorState
  scroll: number
}

/** The smallest single replacement turning `a` into `b`, so the cursor and undo history stay local. */
export function minimalChange(a: string, b: string): { from: number; to: number; insert: string } {
  const max = Math.min(a.length, b.length)
  let start = 0
  while (start < max && a.charCodeAt(start) === b.charCodeAt(start)) start++
  let endA = a.length
  let endB = b.length
  while (endA > start && endB > start && a.charCodeAt(endA - 1) === b.charCodeAt(endB - 1)) {
    endA--
    endB--
  }
  return { from: start, to: endA, insert: b.slice(start, endB) }
}

const run = (fn: (() => void) | undefined) => {
  if (!fn) return false
  fn()
  return true
}

export function CodeEditor({
  docKey,
  value,
  syntax,
  onChange,
  onRun,
  onTest,
  label,
  className,
}: CodeEditorProps) {
  const host = useRef<HTMLDivElement>(null)
  const view = useRef<EditorView | null>(null)
  const hintId = useId()

  const latest = useRef({ onChange, onRun, onTest, syntax, label, value, hintId })
  const keyRef = useRef(docKey)
  const parked = useRef(new Map<string, Parked>())
  const configured = useRef({ syntax, label })
  const compartments = useRef<{ lang: Compartment; attrs: Compartment } | null>(null)

  // Declared first so every effect below reads this render's props.
  useLayoutEffect(() => {
    latest.current = { onChange, onRun, onTest, syntax, label, value, hintId }
  })

  const parts = () => (compartments.current ??= { lang: new Compartment(), attrs: new Compartment() })
  const languageExt = (s: string): Extension => languageSupport(s) ?? []
  const attrsExt = (l: string, describedBy: string): Extension =>
    EditorView.contentAttributes.of({ 'aria-label': l, 'aria-describedby': describedBy })

  const makeState = (doc: string): EditorState => {
    const { lang, attrs } = parts()
    const cur = latest.current
    return EditorState.create({
      doc,
      extensions: [
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightSpecialChars(),
        history(),
        drawSelection(),
        dropCursor(),
        EditorState.allowMultipleSelections.of(true),
        indentOnInput(),
        bracketMatching(),
        rectangularSelection(),
        highlightActiveLine(),
        syntaxHighlighting(codeHighlighter),
        // The content packs indent with four spaces; so does the starter.
        indentUnit.of('    '),
        EditorState.tabSize.of(4),
        // Above the default keymap, which binds Mod-Enter to insertBlankLine.
        Prec.highest(
          keymap.of([
            { key: 'Mod-Enter', run: () => run(latest.current.onRun) },
            { key: "Mod-'", run: () => run(latest.current.onTest) },
          ]),
        ),
        keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
        editorChrome,
        lang.of(languageExt(cur.syntax)),
        attrs.of(attrsExt(cur.label, cur.hintId)),
        EditorView.updateListener.of((u) => {
          if (u.docChanged && !u.transactions.some((tr) => tr.annotation(External))) {
            latest.current.onChange(u.state.doc.toString())
          }
        }),
      ],
    })
  }

  // One view per mount. (StrictMode's mount → unmount → mount builds a second
  // one from the same props; the first is destroyed.)
  useLayoutEffect(() => {
    const v = new EditorView({ parent: host.current!, state: makeState(latest.current.value) })
    view.current = v
    configured.current = { syntax: latest.current.syntax, label: latest.current.label }
    return () => {
      v.destroy()
      view.current = null
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- created once; props arrive through `latest`
  }, [])

  // Document identity and content.
  useLayoutEffect(() => {
    const v = view.current
    if (!v) return
    if (keyRef.current !== docKey) {
      parked.current.set(keyRef.current, { state: v.state, scroll: v.scrollDOM.scrollTop })
      keyRef.current = docKey
      const kept = parked.current.get(docKey)
      const reuse = kept !== undefined && kept.state.doc.toString() === value
      v.setState(reuse ? kept.state : makeState(value))
      const { lang, attrs } = parts()
      const { syntax: s, label: l, hintId: h } = latest.current
      v.dispatch({ effects: [lang.reconfigure(languageExt(s)), attrs.reconfigure(attrsExt(l, h))] })
      configured.current = { syntax: s, label: l }
      v.scrollDOM.scrollTop = reuse ? kept.scroll : 0
      return
    }
    const current = v.state.doc.toString()
    if (current !== value) {
      v.dispatch({ changes: minimalChange(current, value), annotations: External.of(true) })
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- keyed on the document only
  }, [docKey, value])

  // Language and accessible name, for the document already on screen.
  useLayoutEffect(() => {
    const v = view.current
    if (!v || (configured.current.syntax === syntax && configured.current.label === label)) return
    const { lang, attrs } = parts()
    v.dispatch({ effects: [lang.reconfigure(languageExt(syntax)), attrs.reconfigure(attrsExt(label, hintId))] })
    configured.current = { syntax, label }
  }, [syntax, label, hintId])

  return (
    <>
      <div ref={host} className={className} />
      <span id={hintId} className="visually-hidden">
        Press Escape, then Tab, to move focus out of the editor.
        {onRun && ' Control or Command plus Enter runs the code.'}
        {onTest && ' Control or Command plus apostrophe runs the tests.'}
      </span>
    </>
  )
}
