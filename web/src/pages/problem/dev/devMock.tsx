// DEVELOPMENT ONLY — referenced solely behind `import.meta.env.DEV`, so it is
// tree-shaken out of production builds together with ./mockData.
//
// Lets the problem workspace be developed without a backend:
//   localStorage.dsaMock = '1'       serve /api/v1 from fixtures (as a guest)
//   localStorage.dsaMockAuth = '1'   …and be signed in, with a profile
// then reload a /problems/two-sum page.

import { useEffect } from 'react'
import { useSession } from '@/state/session'

const flag = (key: string) => {
  try {
    return localStorage.getItem(key) === '1'
  } catch {
    return false
  }
}

let installed = false
let refreshed = false

/** Patch `fetch` so /api/v1 is answered by ./mockData. No-op unless enabled. */
export function installDevMock(): void {
  if (installed || !flag('dsaMock')) return
  installed = true
  const real = window.fetch.bind(window)
  window.fetch = async (input, init) => {
    const href = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
    const url = new URL(href, window.location.origin)
    if (!url.pathname.startsWith('/api/v1/')) return real(input, init)
    const { handle } = await import('./mockData')
    const body = typeof init?.body === 'string' ? (JSON.parse(init.body) as unknown) : undefined
    return handle(init?.method ?? 'GET', url.pathname.slice('/api/v1'.length), body, flag('dsaMockAuth'))
  }
}

/**
 * The session is probed at boot, before this page's module (and the mock)
 * loads, so a mocked sign-in needs one re-probe once the mock is in place.
 */
export function DevMockSession() {
  const { status, refresh } = useSession()
  useEffect(() => {
    if (!installed || refreshed || status !== 'guest' || !flag('dsaMockAuth')) return
    refreshed = true
    void refresh()
  }, [status, refresh])
  return null
}
