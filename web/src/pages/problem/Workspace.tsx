// One problem, open: the header over whichever tab the URL names.
//
// Both tabs stay mounted once visited and the inactive one is only hidden, so
// switching tabs keeps the editor's buffers and undo history, the run
// results, the debugger position and the reveal — the desktop's `Practice`
// and `Visualize` likewise live across mode switches.

import { lazy, Suspense, useCallback, useMemo, useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router'
import clsx from 'clsx'
import { nextParam } from '@/app/guards'
import { seg } from '@/api/client'
import { entryOf, useCatalog, useProgress } from '@/api/hooks'
import type { Problem, Uuid } from '@/api/types'
import { HelperModal } from '@/features/helper/HelperModal'
import { useSettings } from '@/state/settings'
import { PageSpinner } from '@/ui'
import { ProblemHeader, type Tab } from './header/ProblemHeader'
import { PracticeTab } from './practice/PracticeTab'
import { QuestionPanel } from './practice/QuestionPanel'
import { langInfo, pickLang } from './shared/langs'
import { useDocumentTitle } from './shared/useDocumentTitle'
import ui from './shared/ui.module.css'
import styles from './ProblemPage.module.css'

// The walkthrough (and the renderer behind it) loads on first use: most
// visits start on Practice, and many never reveal the solution.
const VisualizeTab = lazy(() => import('./visualize/VisualizeTab').then((m) => ({ default: m.VisualizeTab })))

export const practicePath = (slug: string) => `/problems/${seg(slug)}`
export const visualizePath = (slug: string) => `/problems/${seg(slug)}/visualize`

export function Workspace({ problem, tab, guest, pid }: { problem: Problem; tab: Tab; guest: boolean; pid: Uuid | null }) {
  const navigate = useNavigate()
  const location = useLocation()
  const { settings, update } = useSettings()
  const catalog = useCatalog()
  const progress = useProgress(pid)
  const entry = entryOf(progress.data, problem.slug)
  const solved = entry.status === 'solved'

  const [helperOpen, setHelperOpen] = useState(false)
  const closeHelper = useCallback(() => setHelperOpen(false), [])
  const [celebrate, setCelebrate] = useState(0)
  const onSolved = useCallback(() => setCelebrate((c) => c + 1), [])

  const [visited, setVisited] = useState<ReadonlySet<Tab>>(() => new Set([tab]))
  if (!visited.has(tab)) setVisited(new Set([...visited, tab]))

  const langs = useMemo(
    () => problem.sources.map((s) => langInfo(s.lang, catalog.data?.languages)),
    [problem.sources, catalog.data?.languages],
  )
  const langId = pickLang(settings.lang, problem.sources)
  const lang = langs.find((l) => l.id === langId) ?? null
  const source = problem.sources.find((s) => s.lang === langId) ?? null
  const workable = !problem.locked && source !== null && lang !== null

  const practiceHref = practicePath(problem.slug)
  const visualizeHref = visualizePath(problem.slug)
  const toTab = useCallback(
    (t: Tab) => navigate(t === 'visualize' ? visualizeHref : practiceHref, { replace: true }),
    [navigate, practiceHref, visualizeHref],
  )
  const toPractice = useCallback(() => toTab('practice'), [toTab])

  useDocumentTitle(`${problem.title} · ${tab === 'visualize' ? 'Visualize' : 'Practice'} · DSA Visualized`)

  return (
    <div className={styles.workspace}>
      <ProblemHeader
        problem={problem}
        entry={entry}
        pid={pid}
        guest={guest}
        langs={langs}
        langId={langId}
        onLang={(id) => update({ lang: id })}
        tab={workable ? tab : null}
        onTab={toTab}
        onHelper={() => setHelperOpen(true)}
        celebrate={celebrate}
      />

      {problem.locked ? (
        <div className={styles.locked}>
          <div className={styles.lockedStatement}>
            <QuestionPanel problem={problem} visualizeHref={null} />
          </div>
          <aside className={clsx(ui.card, styles.upgrade)}>
            <h2>🔒 Part of Pro</h2>
            <p>
              The statement is free to read. Writing and running your own solution to {problem.title} — and stepping
              through its animated walkthrough — comes with DSA Visualized Pro.
            </p>
            <div className={ui.noticeActions}>
              <Link className={ui.apply} to="/pricing">
                See plans →
              </Link>
              {guest && (
                <Link className="mini-btn" to={`/login?next=${nextParam(location)}`}>
                  Already on Pro? Sign in
                </Link>
              )}
            </div>
          </aside>
        </div>
      ) : !workable ? (
        <div className={styles.locked}>
          <div className={styles.lockedStatement}>
            <QuestionPanel problem={problem} visualizeHref={null} />
            <p className={ui.empty}>No solution code has been published for this problem yet.</p>
          </div>
        </div>
      ) : (
        <>
          {visited.has('practice') && (
            <PracticeTab
              problem={problem}
              source={source}
              lang={lang}
              pid={pid}
              guest={guest}
              solved={solved}
              hidden={tab !== 'practice'}
              visualizeHref={problem.has_trace ? visualizeHref : null}
              onSolved={onSolved}
            />
          )}
          {visited.has('visualize') && (
            <Suspense
              fallback={
                <div className={styles.tabFallback} hidden={tab !== 'visualize'}>
                  <PageSpinner label="Loading the walkthrough…" />
                </div>
              }
            >
              <VisualizeTab
                problem={problem}
                source={source}
                lang={lang}
                authed={!guest}
                solved={solved}
                hidden={tab !== 'visualize'}
                practiceHref={practiceHref}
                onPractice={toPractice}
              />
            </Suspense>
          )}
        </>
      )}

      <HelperModal open={helperOpen} onClose={closeHelper} category={problem.category} lang={langId ?? undefined} />
    </div>
  )
}
