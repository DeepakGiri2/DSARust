// The 📘 helper (crates/dsa-app/src/helper.rs): a dialog with two tabs —
// data-structure and technique deep-dives, the current category's first, and
// an "I know X, show me Y" syntax cheat sheet.
//
// Content is `content/guide/` served as JSON, so every paragraph, complexity
// row and sample is editable without a deploy of this code. The props are a
// contract: the catalogue opens it with a category, the problem page with the
// problem's category and the language in use.

import { useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from 'react'
import clsx from 'clsx'
import { useCatalog, useGuide } from '@/api/hooks'
import type { Guide, GuideTopic } from '@/api/types'
import { useSettings } from '@/state/settings'
import { EmptyState, ErrorState, Modal, PageSpinner, Seg } from '@/ui'
import {
  cheatDefaults,
  filterCheatsheet,
  guideLanguages,
  restFor,
  startLanguage,
  topicsFor,
  type LangOption,
} from './guide'
import styles from './HelperModal.module.css'

export interface HelperModalProps {
  open: boolean
  onClose: () => void
  /** Problem category whose topics are shown first (e.g. "Arrays & Hashing"). */
  category?: string
  /** Language id whose syntax samples are shown first (e.g. "go"). */
  lang?: string
}

type Tab = 'topics' | 'cheat'

export function HelperModal({ open, onClose, category, lang }: HelperModalProps) {
  // The body mounts on every open, so each opening starts on the category's
  // first topic — the desktop's `Helper::open`.
  return (
    <Modal open={open} onClose={onClose} width="min(1180px, 100%)" labelledBy="helper-title">
      <HelperBody category={category} lang={lang} onClose={onClose} />
    </Modal>
  )
}

function HelperBody({ category, lang, onClose }: Omit<HelperModalProps, 'open'>) {
  const guide = useGuide()
  const catalog = useCatalog()
  const { settings } = useSettings()
  const [tab, setTab] = useState<Tab>('topics')

  return (
    <div className={styles.helper}>
      <div className={styles.head}>
        <h2 id="helper-title" className={styles.heading}>
          <span aria-hidden>📘</span> helper
        </h2>
        <Seg
          aria-label="Helper section"
          value={tab}
          onChange={setTab}
          options={[
            { value: 'topics', label: 'data types & techniques' },
            { value: 'cheat', label: '⇄ syntax cheat sheet' },
          ]}
        />
        <button type="button" className={clsx('mini-btn', styles.close)} onClick={onClose} title="Close (Esc)">
          ✕ close
        </button>
      </div>
      {guide.isPending ? (
        <PageSpinner label="Loading the guide…" />
      ) : guide.isError ? (
        <ErrorState error={guide.error} onRetry={() => void guide.refetch()} />
      ) : guide.data.topics.length === 0 ? (
        <EmptyState title="No guide is installed">The helper’s content has not been published yet.</EmptyState>
      ) : (
        <LoadedHelper
          guide={guide.data}
          options={guideLanguages(guide.data, catalog.data?.languages ?? [])}
          category={category}
          lang={lang ?? settings.lang}
          tab={tab}
        />
      )}
    </div>
  )
}

function LoadedHelper({
  guide,
  options,
  category,
  lang,
  tab,
}: {
  guide: Guide
  options: LangOption[]
  category: string | undefined
  lang: string
  tab: Tab
}) {
  // Both start from the problem's language but move independently, so you can
  // compare without leaving the page; they survive switching tabs.
  const [syntaxLang, setSyntaxLang] = useState(() => startLanguage(lang, options))
  const [cheat, setCheat] = useState(() => cheatDefaults(lang, options))
  return tab === 'topics' ? (
    <TopicsTab guide={guide} category={category} options={options} syntaxLang={syntaxLang} onSyntaxLang={setSyntaxLang} />
  ) : (
    <CheatTab guide={guide} options={options} from={cheat.from} to={cheat.to} onChange={setCheat} />
  )
}

// ─────────────────────────────────────────────────────────────────────────────
// Topics
// ─────────────────────────────────────────────────────────────────────────────

function TopicsTab({
  guide,
  category,
  options,
  syntaxLang,
  onSyntaxLang,
}: {
  guide: Guide
  category: string | undefined
  options: LangOption[]
  syntaxLang: string
  onSyntaxLang: (id: string) => void
}) {
  const relevant = useMemo(() => topicsFor(guide, category), [guide, category])
  const rest = useMemo(() => restFor(guide, category), [guide, category])
  const order = useMemo(() => [...relevant, ...rest], [relevant, rest])
  // Land on the first topic that relates to this category, else the first one.
  const [selectedId, setSelectedId] = useState(() => (relevant[0] ?? guide.topics[0]).id)
  const selected = order.find((t) => t.id === selectedId) ?? order[0]
  const buttons = useRef(new Map<string, HTMLButtonElement>())
  const body = useRef<HTMLDivElement>(null)

  const select = (id: string, focus = false) => {
    setSelectedId(id)
    if (body.current) body.current.scrollTop = 0
    if (focus) buttons.current.get(id)?.focus()
  }

  // One tab stop for the whole list; the arrows walk it, across both groups.
  const onKeyDown = (e: KeyboardEvent<HTMLElement>) => {
    const i = order.findIndex((t) => t.id === selected.id)
    const last = order.length - 1
    const to =
      e.key === 'ArrowDown' ? Math.min(last, i + 1)
      : e.key === 'ArrowUp' ? Math.max(0, i - 1)
      : e.key === 'Home' ? 0
      : e.key === 'End' ? last
      : null
    if (to === null) return
    e.preventDefault()
    select(order[to].id, true)
  }

  const chip = (t: GuideTopic) => {
    const on = t.id === selected.id
    return (
      <li key={t.id}>
        <button
          type="button"
          ref={(el) => {
            if (el) buttons.current.set(t.id, el)
            else buttons.current.delete(t.id)
          }}
          className={clsx(styles.chip, on && styles.chipOn)}
          aria-current={on ? 'true' : undefined}
          tabIndex={on ? 0 : -1}
          onClick={() => select(t.id)}
        >
          <span aria-hidden>{t.emoji}</span>
          <span className={styles.chipText}>{t.title}</span>
          {t.kind === 'technique' && <span className={styles.kind}>technique</span>}
        </button>
      </li>
    )
  }

  const grouped = category !== undefined && relevant.length > 0
  return (
    <div className={styles.topics}>
      <nav className={styles.chips} aria-label="Topics — use the arrow keys to move" onKeyDown={onKeyDown}>
        {grouped && (
          <>
            <p className={styles.group} id="helper-group-for">
              for “{category}”
            </p>
            <ul aria-labelledby="helper-group-for">{relevant.map(chip)}</ul>
          </>
        )}
        {rest.length > 0 && (
          <>
            <p className={styles.group} id="helper-group-rest">
              {grouped ? 'everything else' : 'all topics'}
            </p>
            <ul aria-labelledby="helper-group-rest">{rest.map(chip)}</ul>
          </>
        )}
      </nav>
      <div className={styles.topicBody} ref={body}>
        <TopicView topic={selected} options={options} syntaxLang={syntaxLang} onSyntaxLang={onSyntaxLang} />
      </div>
    </div>
  )
}

function TopicView({
  topic,
  options,
  syntaxLang,
  onSyntaxLang,
}: {
  topic: GuideTopic
  options: LangOption[]
  syntaxLang: string
  onSyntaxLang: (id: string) => void
}) {
  const sample = topic.syntax[syntaxLang]
  return (
    <article aria-labelledby="helper-topic-title">
      <div className={styles.topicHead}>
        <h3 id="helper-topic-title" className={styles.topicTitle}>
          <span aria-hidden>{topic.emoji}</span> {topic.title}
        </h3>
        <span className={clsx(styles.kindBadge, topic.kind === 'technique' && styles.kindTechnique)}>
          {topic.kind === 'structure' ? 'data structure' : 'technique'}
        </span>
      </div>

      {topic.what.map((p, i) => (
        <p key={i} className={styles.para}>
          {inline(p)}
        </p>
      ))}

      {topic.complexity.length > 0 && (
        <>
          <h4 className={styles.sub}>⏱ time complexity</h4>
          <div className={styles.tableScroll}>
            <table className={styles.cx}>
              <thead>
                <tr>
                  <th scope="col">operation</th>
                  <th scope="col">time</th>
                  <th scope="col">
                    <span className="visually-hidden">note</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {topic.complexity.map((row, i) => (
                  <tr key={i}>
                    <td>{inline(row.op)}</td>
                    <td>
                      <code className={styles.big}>{row.big}</code>
                    </td>
                    <td className={styles.note}>{inline(row.note)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}

      {options.length > 0 && (
        <>
          <div className={styles.subRow}>
            <h4 className={styles.sub}>✏ syntax</h4>
            <Seg
              small
              aria-label="Syntax language"
              value={syntaxLang}
              onChange={onSyntaxLang}
              options={options.map((o) => ({ value: o.id, label: o.label }))}
            />
          </div>
          <pre className={styles.code}>
            <code>{sample ? sample.trimEnd() : '(no sample for this language)'}</code>
          </pre>
        </>
      )}

      {topic.notes.length > 0 && (
        <>
          <h4 className={clsx(styles.sub, styles.subWarn)}>⚠ indexing, ranges &amp; pitfalls</h4>
          <ul className={styles.notes}>
            {topic.notes.map((n, i) => (
              <li key={i}>{inline(n)}</li>
            ))}
          </ul>
        </>
      )}
    </article>
  )
}

/**
 * The guide's prose marks code with backticks and emphasis with asterisks
 * ("O(1) *amortized*"). Render those two — as elements, never as HTML.
 */
function inline(text: string): ReactNode {
  return text.split(/(`[^`]+`|\*[^*\s][^*]*\*)/).map((part, i) => {
    if (part.length > 2 && part.startsWith('`') && part.endsWith('`')) return <code key={i}>{part.slice(1, -1)}</code>
    if (part.length > 2 && part.startsWith('*') && part.endsWith('*')) return <em key={i}>{part.slice(1, -1)}</em>
    return part
  })
}

// ─────────────────────────────────────────────────────────────────────────────
// Cheat sheet
// ─────────────────────────────────────────────────────────────────────────────

function CheatTab({
  guide,
  options,
  from,
  to,
  onChange,
}: {
  guide: Guide
  options: LangOption[]
  from: string
  to: string
  onChange: (next: { from: string; to: string }) => void
}) {
  const [query, setQuery] = useState('')
  const sections = useMemo(() => filterCheatsheet(guide.cheatsheet, query, [from, to]), [guide, query, from, to])
  const label = (id: string) => options.find((o) => o.id === id)?.label ?? id
  const langOptions = options.map((o) => ({ value: o.id, label: o.label }))

  return (
    <div className={styles.cheat}>
      <div className={styles.cheatPick}>
        <span>I know</span>
        <Seg small aria-label="The language you know" value={from} onChange={(v) => onChange({ from: v, to })} options={langOptions} />
        <span>→ show me</span>
        <Seg small aria-label="The language to show" value={to} onChange={(v) => onChange({ from, to: v })} options={langOptions} />
        <button type="button" className="mini-btn" onClick={() => onChange({ from: to, to: from })} title="Swap the two languages">
          ⇄ swap
        </button>
        <input
          type="search"
          className={clsx('input', styles.cheatSearch)}
          placeholder="filter rows…"
          aria-label="Filter the cheat sheet"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>

      {guide.cheatsheet.length === 0 ? (
        <EmptyState title="No cheat sheet is installed" />
      ) : sections.length === 0 ? (
        <EmptyState title="Nothing matches">
          <button type="button" className="mini-btn" onClick={() => setQuery('')}>
            clear the filter
          </button>
        </EmptyState>
      ) : (
        sections.map((s, si) => (
          <section key={s.name} className={styles.cheatSec} aria-labelledby={`cheat-sec-${si}`}>
            <h3 id={`cheat-sec-${si}`} className={styles.cheatTitle}>
              {s.name}
            </h3>
            <table className={styles.cheatTable}>
              <thead>
                <tr>
                  <th scope="col">
                    <span className="visually-hidden">what</span>
                  </th>
                  <th scope="col" className={styles.fromHead}>
                    {label(from)}
                  </th>
                  <th scope="col" className={styles.toHead}>
                    {label(to)}
                  </th>
                </tr>
              </thead>
              <tbody>
                {s.rows.map((r) => (
                  <tr key={r.topic}>
                    <th scope="row" className={styles.cheatTopic}>
                      {r.topic}
                    </th>
                    <td data-label={label(from)}>
                      <pre>{r.code[from]?.trimEnd() ?? '—'}</pre>
                    </td>
                    <td data-label={label(to)}>
                      <pre>{r.code[to]?.trimEnd() ?? '—'}</pre>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>
        ))
      )}
    </div>
  )
}
