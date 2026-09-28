// Creating and editing a profile — the desktop's editor sheet: a big preview
// tile, NAME, FACE and COLOUR, then save / cancel, and (when editing) a delete
// that asks once, in place.

import { useRef, useState, type FormEvent } from 'react'
import clsx from 'clsx'
import { useCreateProfile, useDeleteProfile, useUpdateProfile } from '@/api/hooks'
import type { Profile } from '@/api/types'
import { describeError } from '@/pages/auth/errors'
import { colorName } from '@/pages/shell/profileColor'
import { useSession } from '@/state/session'
import { faceVars } from './faces'
import styles from './profiles.module.css'

/** A stored face or colour that has since left the offered set still shows, first. */
function withCurrent(options: readonly string[], current: string | undefined): string[] {
  return current && !options.includes(current) ? [current, ...options] : [...options]
}

export function ProfileEditor({
  profile,
  profiles,
  avatars,
  colors,
  onClose,
}: {
  /** Null while creating. */
  profile: Profile | null
  profiles: readonly Profile[]
  avatars: readonly string[]
  colors: readonly string[]
  onClose: () => void
}) {
  const session = useSession()
  const create = useCreateProfile()
  const update = useUpdateProfile()
  const remove = useDeleteProfile()
  const [name, setName] = useState(profile?.name ?? '')
  const [avatar, setAvatar] = useState(profile?.avatar ?? avatars[0] ?? '🎓')
  const [color, setColor] = useState(profile?.color ?? colors[0] ?? '#7c6cff')
  const [error, setError] = useState<string | null>(null)
  const [confirmDelete, setConfirmDelete] = useState(false)
  const nameRef = useRef<HTMLInputElement>(null)
  const editing = profile !== null
  const saving = create.isPending || update.isPending

  const fail = (message: string) => {
    setError(message)
    nameRef.current?.focus()
  }

  const save = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    const n = name.trim()
    // The picker shows nothing but the name, so two profiles called "Sam"
    // could not be told apart; the server enforces the same rule.
    if (!n) return fail('A profile needs a name.')
    if (profiles.some((p) => p.id !== profile?.id && p.name.toLowerCase() === n.toLowerCase())) {
      return fail(`“${n}” already exists.`)
    }
    const onError = (err: unknown) => {
      const d = describeError(err, { conflict: `“${n}” already exists.` })
      fail(d.fields.name ?? d.message ?? 'Could not save the profile.')
    }
    if (profile) update.mutate({ id: profile.id, name: n, avatar, color }, { onSuccess: onClose, onError })
    else create.mutate({ name: n, avatar, color }, { onSuccess: onClose, onError })
  }

  const deleteProfile = () => {
    if (!profile) return
    remove.mutate(profile.id, {
      onSuccess: () => {
        // Forget it as this device's choice, so the picker asks again.
        if (session.activeProfile?.id === profile.id) session.setActiveProfile(null)
        onClose()
      },
      onError: (err) =>
        setError(describeError(err, { conflict: 'The last profile on an account can’t be deleted.' }).message),
    })
  }

  return (
    <div className={styles.editorPage}>
      <h1 className={styles.editorHeading}>
        {editing ? 'Edit' : 'New'} <span className="grad-text">profile</span>
      </h1>

      <form className={clsx('card', styles.editor)} onSubmit={save} noValidate>
        <div className={styles.editorTop}>
          <span className={styles.preview} style={faceVars(color)} aria-hidden>
            {avatar}
          </span>
          <div className={styles.nameCol}>
            <label htmlFor="profile-name" className="section-label">
              name
            </label>
            <input
              ref={nameRef}
              id="profile-name"
              className={clsx('input', styles.nameInput)}
              placeholder="who is this?"
              autoComplete="off"
              autoFocus
              maxLength={40}
              value={name}
              onChange={(e) => {
                setName(e.target.value)
                setError(null)
              }}
              aria-invalid={error ? true : undefined}
              aria-describedby={error ? 'profile-name-error' : undefined}
            />
            {error && (
              <p id="profile-name-error" className="field-error" role="alert">
                {error}
              </p>
            )}
          </div>
        </div>

        <fieldset className={styles.fieldset}>
          <legend className={clsx('section-label', styles.legend)}>face</legend>
          <div className={styles.faces}>
            {withCurrent(avatars, profile?.avatar).map((a) => (
              <label key={a} className={styles.faceOption}>
                <input
                  type="radio"
                  name="avatar"
                  value={a}
                  checked={avatar === a}
                  onChange={() => setAvatar(a)}
                  className="visually-hidden"
                />
                <span className={styles.faceGlyph}>{a}</span>
              </label>
            ))}
          </div>
        </fieldset>

        <fieldset className={styles.fieldset}>
          <legend className={clsx('section-label', styles.legend)}>colour</legend>
          <div className={styles.swatches}>
            {withCurrent(colors, profile?.color).map((c) => (
              <label key={c} className={styles.swatch} style={faceVars(c)}>
                <input
                  type="radio"
                  name="color"
                  value={c}
                  checked={color === c}
                  onChange={() => setColor(c)}
                  className="visually-hidden"
                />
                <span className="visually-hidden">{colorName(c)}</span>
              </label>
            ))}
          </div>
        </fieldset>

        <div className={styles.actions}>
          <button type="submit" className="btn btn-primary" disabled={saving}>
            {saving ? 'saving…' : editing ? '✓ save' : '✓ create'}
          </button>
          <button type="button" className="mini-btn" onClick={onClose}>
            cancel
          </button>
          {editing && (
            <div className={styles.danger}>
              {profiles.length <= 1 ? (
                <span className={styles.dangerNote}>Your only profile can’t be deleted.</span>
              ) : confirmDelete ? (
                <>
                  <span className={styles.dangerNote}>progress, favourites and playlists too</span>
                  <button type="button" className="btn btn-danger" onClick={deleteProfile} disabled={remove.isPending}>
                    {remove.isPending ? 'deleting…' : '✖ delete for good'}
                  </button>
                  <button type="button" className="mini-btn" onClick={() => setConfirmDelete(false)}>
                    keep
                  </button>
                </>
              ) : (
                <button type="button" className="mini-btn" onClick={() => setConfirmDelete(true)}>
                  ✖ delete profile
                </button>
              )}
            </div>
          )}
        </div>
      </form>
    </div>
  )
}
