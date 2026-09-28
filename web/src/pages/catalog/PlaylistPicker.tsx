// The playlist filter (`list.rs::playlist_picker`): "♪ all problems", each
// playlist with its size, and the way into managing them.

import clsx from 'clsx'
import type { Playlist, Uuid } from '@/api/types'
import { Menu, MenuItem, MenuSeparator } from '@/ui'
import styles from './CatalogPage.module.css'

export function PlaylistPicker({
  playlists,
  selected,
  onSelect,
  onManage,
  disabled,
}: {
  playlists: readonly Playlist[]
  selected: Uuid | null
  onSelect: (id: Uuid | null) => void
  onManage: () => void
  disabled?: boolean
}) {
  // A selected id that no longer exists reads as "all problems", which is also
  // what the filter does with it.
  const current = playlists.find((p) => p.id === selected)
  const label = (
    <>
      <span className="visually-hidden">Playlist: </span>
      <span aria-hidden>♪</span>
      <span className={styles.pickerText}>
        {current ? `${current.name} (${current.slugs.length})` : 'all problems'}
      </span>
      <span className={styles.caret} aria-hidden>
        ▾
      </span>
    </>
  )

  if (disabled) {
    return (
      <button type="button" className={clsx('mini-btn', styles.picker)} disabled>
        {label}
      </button>
    )
  }

  return (
    <Menu label={label} buttonClassName={clsx('mini-btn', styles.picker)} title="Show only one playlist">
      {(close) => (
        <>
          <MenuItem
            onSelect={() => {
              close()
              onSelect(null)
            }}
          >
            <Tick on={!current} /> all problems
          </MenuItem>
          {playlists.map((p) => (
            <MenuItem
              key={p.id}
              onSelect={() => {
                close()
                onSelect(p.id)
              }}
            >
              <Tick on={p.id === current?.id} />
              <span className={styles.menuName}>{p.name}</span>
              <span className={styles.menuCount}>{p.slugs.length}</span>
            </MenuItem>
          ))}
          <MenuSeparator />
          <MenuItem
            onSelect={() => {
              close()
              onManage()
            }}
          >
            <span className={styles.tick} aria-hidden>
              ✎
            </span>
            manage playlists…
          </MenuItem>
        </>
      )}
    </Menu>
  )
}

function Tick({ on }: { on: boolean }) {
  return (
    <span className={styles.tick}>
      <span aria-hidden>{on ? '✓' : ''}</span>
      {on && <span className="visually-hidden">(showing) </span>}
    </span>
  )
}
