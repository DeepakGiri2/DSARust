// "manage playlists…" — the one place a playlist is created, renamed or
// deleted. Filling one happens on a problem page (its ♪ menu), exactly as on
// the desktop.

import { useState, type FormEvent, type KeyboardEvent } from 'react'
import {
  useCreatePlaylist,
  useDeletePlaylist,
  useRenamePlaylist,
} from '@/api/hooks'
import type { Playlist, Uuid } from '@/api/types'
import { describeError } from '@/pages/auth/errors'
import { plural } from '@/pages/shell/format'
import { Modal } from '@/ui'
import styles from './CatalogPage.module.css'

/** Names compare the way the server's unique index does: trimmed, case-insensitive. */
function sameName(a: string, b: string): boolean {
  return a.trim().toLowerCase() === b.trim().toLowerCase()
}

function nameProblem(name: string, taken: readonly Playlist[]): string | null {
  const n = name.trim()
  if (!n) return 'A playlist needs a name.'
  if (taken.some((p) => sameName(p.name, n))) return `“${n}” already exists.`
  return null
}

export function PlaylistsModal({
  open,
  onClose,
  pid,
  playlists,
  selected,
  onSelectedDeleted,
}: {
  open: boolean
  onClose: () => void
  pid: Uuid
  playlists: readonly Playlist[]
  selected: Uuid | null
  onSelectedDeleted: () => void
}) {
  return (
    <Modal open={open} onClose={onClose} title="♪ playlists" labelledBy="playlists-title" width={520}>
      {playlists.length === 0 ? (
        <p className={styles.modalNote}>
          No playlists yet. Make one for a study plan, a week of revision, or the problems that
          keep tripping you up.
        </p>
      ) : (
        <ul className={styles.playlistList}>
          {playlists.map((p) => (
            <PlaylistItem
              key={p.id}
              pid={pid}
              playlist={p}
              others={playlists.filter((o) => o.id !== p.id)}
              onDeleted={() => {
                if (selected === p.id) onSelectedDeleted()
              }}
            />
          ))}
        </ul>
      )}
      <NewPlaylist pid={pid} playlists={playlists} autoFocus={playlists.length === 0} />
      <p className={styles.modalHint}>
        Add problems from the <strong>♪ playlist</strong> menu on any problem page.
      </p>
    </Modal>
  )
}

function PlaylistItem({
  pid,
  playlist,
  others,
  onDeleted,
}: {
  pid: Uuid
  playlist: Playlist
  others: readonly Playlist[]
  onDeleted: () => void
}) {
  const rename = useRenamePlaylist(pid)
  const remove = useDeletePlaylist(pid)
  const [mode, setMode] = useState<'view' | 'rename' | 'confirm'>('view')
  const [name, setName] = useState(playlist.name)
  const [error, setError] = useState<string | null>(null)
  const inputId = `playlist-${playlist.id}-name`

  const cancel = () => {
    setMode('view')
    setName(playlist.name)
    setError(null)
  }

  const save = (e: FormEvent) => {
    e.preventDefault()
    if (name.trim() === playlist.name) return cancel()
    // Checked against the others only: re-casing its own name is a rename too.
    const problem = nameProblem(name, others)
    if (problem) return setError(problem)
    rename.mutate(
      { id: playlist.id, name: name.trim() },
      {
        onSuccess: () => {
          setMode('view')
          setError(null)
        },
        onError: (err) =>
          setError(describeError(err, { conflict: `“${name.trim()}” already exists.` }).message),
      },
    )
  }

  // Esc backs out of the rename rather than closing the whole dialog.
  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.stopPropagation()
      cancel()
    }
  }

  if (mode === 'rename') {
    return (
      <li className={styles.playlistItem}>
        <form className={styles.inlineForm} onSubmit={save} noValidate>
          <label className="visually-hidden" htmlFor={inputId}>
            New name for {playlist.name}
          </label>
          <input
            id={inputId}
            className="input"
            value={name}
            maxLength={80}
            autoFocus
            aria-invalid={error ? true : undefined}
            aria-describedby={error ? `${inputId}-error` : undefined}
            onChange={(e) => {
              setName(e.target.value)
              setError(null)
            }}
            onKeyDown={onKey}
          />
          <button type="submit" className="mini-btn" disabled={rename.isPending}>
            {rename.isPending ? 'saving…' : 'save'}
          </button>
          <button type="button" className="mini-btn" onClick={cancel}>
            cancel
          </button>
        </form>
        {error && (
          <p className="field-error" id={`${inputId}-error`}>
            {error}
          </p>
        )}
      </li>
    )
  }

  return (
    <li className={styles.playlistItem}>
      <div className={styles.playlistRow}>
        <span className={styles.playlistName}>
          <span aria-hidden>♪</span> {playlist.name}
        </span>
        <span className={styles.menuCount}>{plural(playlist.slugs.length, 'problem')}</span>
        {mode === 'confirm' ? (
          <span className={styles.confirm}>
            <button
              type="button"
              className="btn btn-danger"
              disabled={remove.isPending}
              onClick={() =>
                remove.mutate(playlist.id, {
                  onSuccess: onDeleted,
                  onError: (err) => setError(describeError(err).message),
                })
              }
            >
              {remove.isPending ? 'deleting…' : '✖ delete for good'}
            </button>
            <button type="button" className="mini-btn" onClick={() => setMode('view')}>
              keep
            </button>
          </span>
        ) : (
          <span className={styles.confirm}>
            <button type="button" className="mini-btn" onClick={() => setMode('rename')}>
              rename
            </button>
            <button
              type="button"
              className="mini-btn"
              onClick={() => setMode('confirm')}
              aria-label={`Delete ${playlist.name}`}
            >
              delete
            </button>
          </span>
        )}
      </div>
      {mode === 'confirm' && (
        <p className={styles.modalNote}>The list goes; the problems and your progress on them stay.</p>
      )}
      {error && <p className="field-error">{error}</p>}
    </li>
  )
}

function NewPlaylist({
  pid,
  playlists,
  autoFocus,
}: {
  pid: Uuid
  playlists: readonly Playlist[]
  autoFocus: boolean
}) {
  const create = useCreatePlaylist(pid)
  const [name, setName] = useState('')
  const [error, setError] = useState<string | null>(null)

  const submit = (e: FormEvent) => {
    e.preventDefault()
    const problem = nameProblem(name, playlists)
    if (problem) return setError(problem)
    create.mutate(
      { name: name.trim() },
      {
        onSuccess: () => {
          setName('')
          setError(null)
        },
        onError: (err) =>
          setError(describeError(err, { conflict: `“${name.trim()}” already exists.` }).message),
      },
    )
  }

  return (
    <form className={styles.newPlaylist} onSubmit={submit} noValidate>
      <label htmlFor="new-playlist" className="section-label">
        new playlist
      </label>
      <div className={styles.inlineForm}>
        <input
          id="new-playlist"
          className="input"
          placeholder="e.g. week 1, graphs, revision"
          value={name}
          maxLength={80}
          autoFocus={autoFocus}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? 'new-playlist-error' : undefined}
          onChange={(e) => {
            setName(e.target.value)
            setError(null)
          }}
        />
        <button type="submit" className="btn btn-primary" disabled={create.isPending}>
          {create.isPending ? 'creating…' : '+ create'}
        </button>
      </div>
      {error && (
        <p className="field-error" id="new-playlist-error">
          {error}
        </p>
      )}
    </form>
  )
}
