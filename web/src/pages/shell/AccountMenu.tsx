// The desktop's profile chip ("🎓 Alex · 5/287 solved"), moved from the
// catalogue hero into the top bar so it is on every screen — still the one
// place the profile is always visible, and still the way back to the picker.
// On the web it also opens the account menu.

import { useLocation, useNavigate } from 'react-router'
import { useCatalog, useProgress } from '@/api/hooks'
import { nextParam } from '@/app/guards'
import { useSession } from '@/state/session'
import { useSettings } from '@/state/settings'
import { Menu, MenuItem, MenuSeparator, useToast } from '@/ui'
import { profileColor, withAlpha } from './profileColor'
import styles from './AppShell.module.css'

export function AccountMenu() {
  const { user, activeProfile, logout } = useSession()
  const progress = useProgress(activeProfile?.id ?? null)
  const catalog = useCatalog()
  const { settings, update } = useSettings()
  const navigate = useNavigate()
  const location = useLocation()
  const toast = useToast()

  if (!user) return null

  // The snapshot is what stars and ticks update optimistically, so it is the
  // fresher number; the profile row is only a fallback while it loads.
  const solved = progress.data?.stats.solved ?? activeProfile?.stats.solved ?? 0
  const total = catalog.data?.total
  const color = profileColor(activeProfile?.color)
  const dark = settings.theme === 'dark'
  const onPicker = location.pathname.startsWith('/profiles')

  const go = (to: string, close: () => void) => {
    close()
    navigate(to)
  }

  const signOut = async (close: () => void) => {
    close()
    // Leave first: a signed-in-only page would otherwise bounce to /login
    // for a moment as the session ends underneath it.
    navigate('/', { replace: true })
    try {
      await logout()
    } catch {
      // The local session is dropped either way; a failed request only means
      // the server forgets it when the cookie expires.
    }
    toast.show('Signed out.')
  }

  const label = (
    <span
      className={styles.chip}
      style={
        activeProfile
          ? { background: withAlpha(color, 0x2e), borderColor: withAlpha(color, 0xaa) }
          : undefined
      }
    >
      {activeProfile ? (
        <>
          <span className={styles.chipFace} aria-hidden>
            {activeProfile.avatar}
          </span>
          <span className={styles.chipName}>{activeProfile.name}</span>
          {total !== undefined && (
            <span className={styles.chipScore}>
              {' · '}
              {solved}/{total}
              <span className={styles.chipLong}> solved</span>
            </span>
          )}
        </>
      ) : (
        <span className={styles.chipName}>{user.display_name}</span>
      )}
      <span className={styles.caret} aria-hidden>
        ▾
      </span>
    </span>
  )

  return (
    <Menu
      align="right"
      label={label}
      buttonClassName={styles.chipButton}
      title="Switch profile, account and theme"
    >
      {(close) => (
        <>
          <div className={styles.menuHead} role="none">
            <span className={styles.menuName}>{user.display_name}</span>
            <span className={styles.menuEmail}>{user.email}</span>
          </div>
          <MenuSeparator />
          <MenuItem
            onSelect={() => go(onPicker ? '/profiles' : `/profiles?next=${nextParam(location)}`, close)}
          >
            <span aria-hidden>⇄</span> switch profile
          </MenuItem>
          <MenuItem onSelect={() => go('/dashboard', close)}>
            <span aria-hidden>▦</span> dashboard
          </MenuItem>
          <MenuItem onSelect={() => go('/account', close)}>
            <span aria-hidden>⚙</span> account
          </MenuItem>
          {user.role === 'admin' && (
            <MenuItem onSelect={() => go('/admin', close)}>
              <span aria-hidden>⛨</span> admin
            </MenuItem>
          )}
          <MenuSeparator />
          <MenuItem
            onSelect={() => {
              close()
              update({ theme: dark ? 'light' : 'dark' })
            }}
          >
            <span aria-hidden>{dark ? '☀' : '☾'}</span> {dark ? 'light theme' : 'dark theme'}
          </MenuItem>
          <MenuSeparator />
          <MenuItem onSelect={() => void signOut(close)}>
            <span aria-hidden>⎋</span> sign out
          </MenuItem>
        </>
      )}
    </Menu>
  )
}
