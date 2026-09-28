import { Outlet, ScrollRestoration } from 'react-router'
import { useSettings } from '@/state/settings'

/**
 * Behind every screen: the desktop's backdrop — a wash, a dissolving hairline
 * lattice and three slow lights. Frozen when the user turns it off.
 */
export function RootLayout() {
  const { settings } = useSettings()
  return (
    <>
      <div className={settings.backdrop ? 'backdrop' : 'backdrop frozen'} aria-hidden>
        <i />
        <i />
        <i />
      </div>
      <Outlet />
      <ScrollRestoration />
    </>
  )
}
