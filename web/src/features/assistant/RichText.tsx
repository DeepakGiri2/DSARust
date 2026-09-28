// A reply, rendered: the desktop's `rich_text` plus the little markdown a
// hosted model uses anyway — paragraphs, `inline code` and **bold** — over the
// ported `segments()`, so fences are found exactly as the desktop finds them.

import { Fragment, memo, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import clsx from 'clsx'
import { segments } from './parse'
import styles from './AssistantPanel.module.css'

/**
 * Prose as paragraphs, fenced blocks as code — and with `lockCode` (Interview
 * mode), code collapsed behind a lock so a leaked solution is at least a
 * deliberate click.
 */
export const RichText = memo(function RichText({ text, lockCode = false }: { text: string; lockCode?: boolean }) {
  const segs = useMemo(() => segments(text), [text])
  return (
    <div className={styles.rich}>
      {segs.map((seg, i) =>
        !seg.code ? (
          <Fragment key={i}>
            {paragraphs(seg.text).map((p, j) => (
              <p key={j}>{inline(p)}</p>
            ))}
          </Fragment>
        ) : lockCode ? (
          <LockedCode key={i} code={seg.text} lang={seg.lang} />
        ) : (
          <CodeBlock key={i} code={seg.text} lang={seg.lang} />
        ),
      )}
    </div>
  )
})

/** Blank lines separate paragraphs; single newlines (a model's bullet lists) are kept. */
function paragraphs(text: string): string[] {
  const trimmed = text.trim()
  return trimmed === '' ? [] : trimmed.split(/\n\s*\n/)
}

// Code spans first, so `a**b` inside backticks stays code.
const INLINE = /`([^`\n]+)`|\*\*([^*\n]+)\*\*/g

function inline(text: string): ReactNode[] {
  const out: ReactNode[] = []
  let last = 0
  for (const m of text.matchAll(INLINE)) {
    if (m.index > last) out.push(text.slice(last, m.index))
    out.push(m[1] !== undefined ? <code key={m.index}>{m[1]}</code> : <strong key={m.index}>{m[2]}</strong>)
    last = m.index + m[0].length
  }
  if (last < text.length) out.push(text.slice(last))
  return out
}

function CodeBlock({ code, lang }: { code: string; lang: string }) {
  return (
    <figure className={styles.code}>
      <figcaption className={styles.codeHead}>
        <span>{lang || 'code'}</span>
        <CopyButton text={code} />
      </figcaption>
      <pre>
        <code>{code}</code>
      </pre>
    </figure>
  )
}

/**
 * The shape of the code shows through the blur, the text does not: it is
 * inert and hidden from assistive tech until the user chooses to look.
 */
function LockedCode({ code, lang }: { code: string; lang: string }) {
  const [shown, setShown] = useState(false)
  if (shown) return <CodeBlock code={code} lang={lang} />
  return (
    <div className={styles.locked}>
      <pre className={styles.blurred} aria-hidden inert>
        {code}
      </pre>
      <button type="button" className={styles.reveal} onClick={() => setShown(true)}>
        🔒 code hidden — interview mode (click to peek anyway)
      </button>
    </div>
  )
}

type CopyState = 'idle' | 'copied' | 'failed'

function CopyButton({ text }: { text: string }) {
  const [state, setState] = useState<CopyState>('idle')
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  useEffect(() => () => clearTimeout(timer.current), [])

  const settle = (next: CopyState) => {
    setState(next)
    clearTimeout(timer.current)
    timer.current = setTimeout(() => setState('idle'), 1600)
  }
  // The clipboard needs a secure context and can be refused; say so rather
  // than pretend it worked.
  const copy = () => {
    if (!('clipboard' in navigator)) return settle('failed')
    navigator.clipboard.writeText(text).then(
      () => settle('copied'),
      () => settle('failed'),
    )
  }

  return (
    <>
      <button type="button" className={clsx('mini-btn', styles.copy)} onClick={copy}>
        {state === 'copied' ? 'copied ✓' : state === 'failed' ? 'copy failed' : 'copy'}
      </button>
      <span className="visually-hidden" role="status">
        {state === 'copied' ? 'Code copied' : state === 'failed' ? 'Could not copy the code' : ''}
      </span>
    </>
  )
}
