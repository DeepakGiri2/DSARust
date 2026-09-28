import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Component as AppShell } from './AppShell'
import { baseRoutes, mockApi, profileFixture, renderRoutes, sessionFixture } from './test-utils'

afterEach(() => {
  vi.unstubAllGlobals()
  localStorage.clear()
  sessionStorage.clear()
})

const routes = [
  {
    Component: AppShell,
    children: [
      { index: true, element: <p>home</p> },
      { path: 'dashboard', element: <p>dashboard</p> },
      { path: 'profiles', element: <p>picker</p> },
    ],
  },
]

const progressFor = (pid: string, solved: number) => ({
  [`GET /profiles/${pid}/settings`]: {},
  [`GET /profiles/${pid}/progress`]: { entries: {}, stats: { solved, attempted: 0, favourites: 0 } },
})

describe('app shell', () => {
  it('gives a guest the sign-in actions and the public nav', async () => {
    mockApi(baseRoutes(null))
    renderRoutes(routes, '/')
    expect(await screen.findByRole('link', { name: 'Get started' })).toHaveAttribute('href', '/signup?next=%2F')
    expect(screen.getByRole('link', { name: 'Sign in' })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Pricing' })).toBeInTheDocument()
    expect(screen.queryByRole('link', { name: 'Dashboard' })).not.toBeInTheDocument()
  })

  it('shows the profile chip with the score, and signs out from its menu', async () => {
    const user = userEvent.setup()
    const { calls } = mockApi({
      ...baseRoutes(sessionFixture([profileFixture('p1', 'Alex')])),
      ...progressFor('p1', 2),
      'POST /auth/logout': undefined,
    })
    renderRoutes(routes, '/')
    const chip = await screen.findByRole('button', { name: /Alex · 2\/6 solved/ })
    expect(screen.getByRole('link', { name: 'Dashboard' })).toBeInTheDocument()
    await user.click(chip)
    await user.click(screen.getByRole('menuitem', { name: /sign out/ }))
    expect(await screen.findByRole('link', { name: 'Sign in' })).toBeInTheDocument()
    expect(calls.some((c) => c.method === 'POST' && c.path === '/auth/logout')).toBe(true)
  })

  it('sends an account with several profiles to the picker first, then back', async () => {
    mockApi(baseRoutes(sessionFixture([profileFixture('p1', 'Alex'), profileFixture('p2', 'Sam')])))
    const { router } = renderRoutes(routes, '/dashboard')
    expect(await screen.findByText('picker')).toBeInTheDocument()
    expect(router.state.location.search).toBe('?next=%2Fdashboard')
  })

  it('asks an unverified account to verify, and resends the link', async () => {
    const user = userEvent.setup()
    const { calls } = mockApi({
      ...baseRoutes(sessionFixture([profileFixture('p1', 'Alex')], { email_verified: false })),
      ...progressFor('p1', 0),
      'POST /auth/verify-email/resend': undefined,
    })
    renderRoutes(routes, '/')
    expect(await screen.findByText(/Verify your email to run code/)).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'resend link' }))
    await waitFor(() => expect(calls.some((c) => c.path === '/auth/verify-email/resend')).toBe(true))
    await user.click(screen.getByRole('button', { name: 'Dismiss for now' }))
    expect(screen.queryByText(/Verify your email to run code/)).not.toBeInTheDocument()
  })
})
