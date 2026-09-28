// The site chrome around every screen except the problem workspace: a slim
// top bar (brand, nav, account) and the "verify your email" banner.
//
// It also enforces the invariant every profile-scoped screen relies on — the
// desktop's "no profile, back to the picker" (app.rs): a signed-in account
// with several profiles and none chosen on this device goes to "Who's
// practising?" first, and comes back afterwards.

import { useState } from 'react'
import { Link, Navigate, NavLink, Outlet, useLocation, useSearchParams } from 'react-router'
import clsx from 'clsx'
import { useMeta } from '@/api/hooks'
import { nextParam } from '@/app/guards'
import { useSession } from '@/state/session'
import { useSettings } from '@/state/settings'
import { Spinner } from '@/ui'
import { AccountMenu } from './AccountMenu'
import { VerifyBanner } from './VerifyBanner'
import styles from './AppShell.module.css'

const AUTH_PAGES = ['/login', '/signup', '/forgot-password', '/reset-password', '/verify-email']

/** Screens that work without a chosen profile — the picker itself, the account, and signing in. */
const PICK_EXEMPT = ['/profiles', '/account', ...AUTH_PAGES]

function normalize(pathname: string): string {
  return pathname.replace(/\/+$/, '') || '/'
}

export function Component() {
  const { needsProfilePick } = useSession()
  const location = useLocation()
  if (needsProfilePick && !PICK_EXEMPT.includes(normalize(location.pathname))) {
    return <Navigate to={`/profiles?next=${nextParam(location)}`} replace />
  }
  return (
    <div className={styles.shell}>
      <a className={styles.skip} href="#main">
        Skip to content
      </a>
      <TopBar />
      <VerifyBanner />
      <main id="main" className={styles.main} tabIndex={-1}>
        <Outlet />
      </main>
    </div>
  )
}

interface NavItem {
  to: string
  label: string
  end?: boolean
}

function TopBar() {
  const session = useSession()
  const meta = useMeta()
  const location = useLocation()
  const [params] = useSearchParams()
  const [menuOpen, setMenuOpen] = useState(false)

  const signedIn = session.status === 'authenticated'
  const path = normalize(location.pathname)
  const onAuthPage = AUTH_PAGES.includes(path)
  // From an auth page, "sign in" and "get started" keep the destination the
  // visitor was already heading to rather than pointing back at the form.
  const next = onAuthPage ? encodeURIComponent(params.get('next') ?? '/') : nextParam(location)

  const links: NavItem[] = [
    { to: '/', label: 'Problems', end: true },
    ...(signedIn ? [{ to: '/dashboard', label: 'Dashboard' }] : []),
    ...(meta.data?.features.billing ? [{ to: '/pricing', label: 'Pricing' }] : []),
  ]

  const navLinks = (onPick?: () => void) =>
    links.map((l) => (
      <NavLink
        key={l.to}
        to={l.to}
        end={l.end}
        onClick={onPick}
        className={({ isActive }) => clsx(styles.link, isActive && styles.active)}
      >
        {l.label}
      </NavLink>
    ))

  const guestActions = (onPick?: () => void) => (
    <>
      {path !== '/login' && (
        <Link to={`/login?next=${next}`} className={clsx('btn btn-ghost', styles.cta)} onClick={onPick}>
          Sign in
        </Link>
      )}
      {path !== '/signup' && meta.data?.features.signup !== false && (
        <Link to={`/signup?next=${next}`} className={clsx('btn btn-primary', styles.cta)} onClick={onPick}>
          Get started
        </Link>
      )}
    </>
  )

  return (
    <header className={styles.bar}>
      <div className={styles.inner}>
        <Link to="/" className={styles.brand} aria-label="DSA Visualized — all problems">
          DSA <span className="grad-text">Visualized</span>
        </Link>
        <nav className={styles.nav} aria-label="Main">
          {navLinks()}
        </nav>
        <div className={styles.right}>
          {session.status === 'loading' ? (
            <Spinner label="Checking your session" />
          ) : signedIn ? (
            <AccountMenu />
          ) : (
            <>
              <ThemeButton />
              <span className={styles.guestWide}>{guestActions()}</span>
            </>
          )}
          <button
            type="button"
            className={clsx('mini-btn', styles.burger)}
            aria-expanded={menuOpen}
            aria-controls="shell-mobile-nav"
            aria-label={menuOpen ? 'Close menu' : 'Open menu'}
            onClick={() => setMenuOpen((o) => !o)}
          >
            {menuOpen ? '✕' : '☰'}
          </button>
        </div>
      </div>
      {menuOpen && (
        <nav id="shell-mobile-nav" className={styles.mobileNav} aria-label="Main">
          {navLinks(() => setMenuOpen(false))}
          {!signedIn && session.status !== 'loading' && (
            <div className={styles.mobileActions}>{guestActions(() => setMenuOpen(false))}</div>
          )}
        </nav>
      )}
    </header>
  )
}

/** Guests get the theme switch in the bar; signed-in users find it in the account menu. */
function ThemeButton() {
  const { settings, update } = useSettings()
  const dark = settings.theme === 'dark'
  return (
    <button
      type="button"
      className={clsx('mini-btn', styles.iconBtn)}
      aria-label={dark ? 'Switch to the light theme' : 'Switch to the dark theme'}
      title={dark ? 'Light theme' : 'Dark theme'}
      onClick={() => update({ theme: dark ? 'light' : 'dark' })}
    >
      {dark ? '☀' : '☾'}
    </button>
  )
}
