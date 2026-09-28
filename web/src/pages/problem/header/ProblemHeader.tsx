// The problem header — `App::problem_header` on the desktop: back, title,
// difficulty, category, complexity, LeetCode, this problem's progress, and on
// the right the language, the Practice/Visualize switch and the helper.

import { Link, useLocation } from 'react-router'
import clsx from 'clsx'
import { nextParam } from '@/app/guards'
import { errorMessage } from '@/api/client'
import {
  useCreatePlaylist,
  usePlaylists,
  useSetFavourite,
  useSetStatus,
  useTogglePlaylistItem,
} from '@/api/hooks'
import type { Problem, ProgressEntry, Uuid } from '@/api/types'
import { DifficultyPill, Menu, MenuItem, MenuSeparator, Seg, useToast, type SegOption } from '@/ui'
import type { LangInfo } from '../shared/langs'
import ui from '../shared/ui.module.css'
import styles from '../ProblemPage.module.css'

export type Tab = 'practice' | 'visualize'

const TAB_OPTIONS: SegOption<Tab>[] = [
  { value: 'practice', label: '✏ Practice', title: 'Write and run your own solution' },
  { value: 'visualize', label: '⏵ Visualize', title: 'Step through the reference solution like a debugger' },
]

export interface ProblemHeaderProps {
  problem: Problem
  entry: ProgressEntry
  pid: Uuid | null
  guest: boolean
  langs: LangInfo[]
  langId: string | null
  onLang: (id: string) => void
  /** Null hides the switch (a locked problem has nothing to switch between). */
  tab: Tab | null
  onTab: (tab: Tab) => void
  onHelper: () => void
  /** Bumped when a test sweep solves the problem, to play the tick's animation. */
  celebrate: number
}

export function ProblemHeader(p: ProblemHeaderProps) {
  const { problem } = p
  return (
    <header className={clsx(ui.toolbar, styles.header)}>
      <Link to="/" className="mini-btn" title="Back to the problem list">
        ← list
      </Link>
      <h1 className={styles.title}>{problem.title}</h1>
      <DifficultyPill difficulty={problem.difficulty} />
      {problem.premium && (
        <span className="chip chip-violet" title="Part of the Pro plan">
          Pro
        </span>
      )}
      <span className={styles.category}>{problem.category}</span>
      {problem.complexity && <span className={styles.complexity}>{problem.complexity}</span>}
      <a
        className={styles.leetcode}
        href={problem.leetcode_url}
        target="_blank"
        rel="noopener noreferrer"
        title="Open this problem on LeetCode"
      >
        leetcode ↗
      </a>
      <ProgressControls slug={problem.slug} entry={p.entry} pid={p.pid} guest={p.guest} celebrate={p.celebrate} />

      <div className={styles.headerRight}>
        {p.langId && p.langs.length > 0 && (
          <Seg
            aria-label="Language"
            value={p.langId}
            options={p.langs.map((l) => ({ value: l.id, label: l.label }))}
            onChange={p.onLang}
          />
        )}
        {p.tab && <Seg aria-label="Mode" value={p.tab} options={TAB_OPTIONS} onChange={p.onTab} />}
        <button
          type="button"
          className="mini-btn"
          onClick={p.onHelper}
          title="Data structures & techniques for this category, complexity tables and a cross-language syntax cheat sheet"
        >
          📘 helper
        </button>
      </div>
    </header>
  )
}

/** The desktop's auto-name for "+ new playlist with this": the next free `playlist N`. */
export function nextPlaylistName(existing: readonly string[]): string {
  let n = existing.length + 1
  while (existing.includes(`playlist ${n}`)) n++
  return `playlist ${n}`
}

function ProgressControls({
  slug,
  entry,
  pid,
  guest,
  celebrate,
}: {
  slug: string
  entry: ProgressEntry
  pid: Uuid | null
  guest: boolean
  celebrate: number
}) {
  const location = useLocation()
  const toast = useToast()
  const setStatus = useSetStatus(pid)
  const setFavourite = useSetFavourite(pid)

  // Progress belongs to a profile. Without one the controls still show where
  // they will be, and say what it takes to use them.
  if (guest || !pid) {
    const prompt = <AuthPrompt guest={guest} next={nextParam(location)} />
    return (
      <>
        <Menu label="✔ mark solved" title="Tick it off by hand. Passing every test does this for you.">
          {prompt}
        </Menu>
        <Menu label="☆" title="Favourite">
          {prompt}
        </Menu>
        <Menu label="♪ playlist" title="Add to a playlist">
          {prompt}
        </Menu>
      </>
    )
  }

  const solved = entry.status === 'solved'
  const onError = (e: unknown) => toast.error(errorMessage(e))
  return (
    <>
      {entry.attempts > 0 && (
        <span className={styles.attempts} title="How many times you have pressed Run or the tests on this">
          {entry.attempts} attempt{entry.attempts === 1 ? '' : 's'}
        </span>
      )}
      <button
        // Re-mounted on each celebration so the animation plays again.
        key={celebrate}
        type="button"
        className={clsx('mini-btn', celebrate > 0 && solved && styles.celebrate)}
        aria-pressed={solved}
        title={solved ? 'Solved — click to put it back on the list' : 'Tick it off by hand. Passing every test does this for you.'}
        onClick={() => setStatus.mutate({ slug, value: solved ? 'todo' : 'solved' }, { onError })}
      >
        {solved ? '✔ solved' : '✔ mark solved'}
      </button>
      <button
        type="button"
        className="mini-btn"
        aria-pressed={entry.favourite}
        aria-label="Favourite"
        title="Favourite"
        onClick={() => setFavourite.mutate({ slug, value: !entry.favourite }, { onError })}
      >
        {entry.favourite ? '⭐' : '☆'}
      </button>
      <PlaylistMenu pid={pid} slug={slug} />
    </>
  )
}

/** "add to playlist" as a menu of tick-boxes — one click either way, for each list. */
function PlaylistMenu({ pid, slug }: { pid: Uuid; slug: string }) {
  const toast = useToast()
  const playlists = usePlaylists(pid)
  const toggle = useTogglePlaylistItem(pid)
  const create = useCreatePlaylist(pid)
  const lists = playlists.data ?? []
  const inAny = lists.some((l) => l.slugs.includes(slug))
  const onError = (e: unknown) => toast.error(errorMessage(e))

  return (
    <Menu
      label={inAny ? '♪ in a playlist' : '♪ playlist'}
      title="Add to a playlist"
      buttonClassName={clsx('mini-btn', inAny && styles.inPlaylist)}
    >
      {(close) => (
        <>
          {playlists.isPending ? (
            <div className={styles.menuNote}>loading…</div>
          ) : (
            lists.length === 0 && <div className={styles.menuNote}>no playlists yet</div>
          )}
          {lists.map((l) => {
            const member = l.slugs.includes(slug)
            return (
              <button
                key={l.id}
                type="button"
                role="menuitemcheckbox"
                aria-checked={member}
                className={styles.menuCheck}
                onClick={() => toggle.mutate({ id: l.id, slug, member: !member }, { onError })}
              >
                <span className={styles.box} aria-hidden>
                  {member ? '✔' : ''}
                </span>
                {l.name} ({l.slugs.length})
              </button>
            )
          })}
          <MenuSeparator />
          <MenuItem
            onSelect={() => {
              close()
              create.mutate({ name: nextPlaylistName(lists.map((l) => l.name)), slugs: [slug] }, { onError })
            }}
          >
            + new playlist with this
          </MenuItem>
        </>
      )}
    </Menu>
  )
}

function AuthPrompt({ guest, next }: { guest: boolean; next: string }) {
  return (
    <div className={styles.prompt}>
      <p>
        {guest
          ? 'Sign in to tick problems off, keep favourites and build playlists — your progress follows you to every device.'
          : 'Choose who is practising to track progress on this problem.'}
      </p>
      <div className={styles.promptActions}>
        {guest ? (
          <>
            <Link role="menuitem" className={ui.apply} to={`/login?next=${next}`}>
              Sign in
            </Link>
            <Link role="menuitem" className="mini-btn" to={`/signup?next=${next}`}>
              Create an account
            </Link>
          </>
        ) : (
          <Link role="menuitem" className={ui.apply} to={`/profiles?next=${next}`}>
            Pick a profile
          </Link>
        )}
      </div>
    </div>
  )
}
