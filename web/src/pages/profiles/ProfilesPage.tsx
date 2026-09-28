// "Who's practising?" — the profile picker (crates/dsa-app/src/profiles.rs).
//
// Netflix's model, and for the same reason: one account, several people (or
// one person keeping a "first pass" and a "revision" run side by side). A
// profile is a name, a face and a colour; what it holds is its own copy of the
// progress, favourites, playlists and settings.
//
// Two modes, as on the desktop. Normally the cards are doors — click one and
// you are in. Under "manage" they open the editor, which is where renaming,
// re-facing and deleting live, so a mis-click on the way in can never delete
// months of progress.

import { useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router'
import clsx from 'clsx'
import { useMeta } from '@/api/hooks'
import type { Profile } from '@/api/types'
import { RequireAuth, safeNext } from '@/app/guards'
import { usePageTitle } from '@/pages/shell/usePageTitle'
import { useSession } from '@/state/session'
import { ErrorState, PageSpinner } from '@/ui'
import { faceVars } from './faces'
import { ProfileEditor } from './ProfileEditor'
import styles from './profiles.module.css'

export function Component() {
  return (
    <RequireAuth>
      <ProfilesScreen />
    </RequireAuth>
  )
}

function ProfilesScreen() {
  const session = useSession()
  const meta = useMeta()
  const navigate = useNavigate()
  const [params] = useSearchParams()
  const [editing, setEditing] = useState<Profile | 'new' | null>(null)
  const [manage, setManage] = useState(false)
  usePageTitle(editing === 'new' ? 'New profile' : editing ? 'Edit profile' : 'Who’s practising?')

  const raw = safeNext(params.get('next'))
  const next = raw.startsWith('/profiles') ? '/' : raw
  const profiles = session.profiles
  const limit = meta.data?.limits.profiles_per_account

  if (editing) {
    if (meta.isPending) return <PageSpinner />
    if (meta.isError) return <ErrorState error={meta.error} onRetry={() => void meta.refetch()} />
    return (
      <ProfileEditor
        profile={editing === 'new' ? null : editing}
        profiles={profiles}
        avatars={meta.data.avatars}
        colors={meta.data.colors}
        onClose={() => setEditing(null)}
      />
    )
  }

  const enter = (p: Profile) => {
    session.setActiveProfile(p.id)
    navigate(next, { replace: true })
  }
  const canAdd = limit === undefined || profiles.length < limit

  return (
    <div className={styles.page}>
      <h1 className={styles.heading}>
        Who’s <span className="grad-text">practising?</span>
      </h1>
      <p className={styles.lede}>Each profile keeps its own progress, favourites and playlists.</p>

      <ul className={styles.grid}>
        {profiles.map((p) => {
          const active = p.id === session.activeProfile?.id
          return (
            <li key={p.id}>
              <button
                type="button"
                className={clsx(styles.card, manage && styles.managing)}
                style={faceVars(p.color)}
                onClick={() => (manage ? setEditing(p) : enter(p))}
                aria-label={manage ? `Edit ${p.name}` : undefined}
              >
                <span className={styles.face}>
                  <span className={styles.glyph} aria-hidden>
                    {p.avatar}
                  </span>
                  {manage && (
                    <span className={styles.pencil} aria-hidden>
                      ✏
                    </span>
                  )}
                </span>
                <span className={styles.name}>{p.name}</span>
                <span className={styles.stat}>
                  {p.stats.solved === 0 ? 'not started' : `${p.stats.solved} solved`}
                  {active && <span className={styles.current}> · current</span>}
                </span>
              </button>
            </li>
          )
        })}
        {canAdd && (
          <li>
            <button type="button" className={clsx(styles.card, styles.add)} onClick={() => setEditing('new')}>
              <span className={styles.face}>
                <span className={styles.plus} aria-hidden>
                  +
                </span>
              </span>
              <span className={styles.name}>add profile</span>
            </button>
          </li>
        )}
      </ul>

      {profiles.length > 0 && (
        <div className={styles.manageRow}>
          <button
            type="button"
            className="mini-btn"
            aria-pressed={manage}
            title="Rename, re-face or delete a profile"
            onClick={() => setManage((m) => !m)}
          >
            {manage ? '✓ done' : '⚙ manage profiles'}
          </button>
          {manage && limit !== undefined && (
            <span className={styles.limit}>
              {profiles.length} of {limit} profiles
            </span>
          )}
        </div>
      )}
    </div>
  )
}
