import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import {
  apiError,
  baseRoutes,
  mockApi,
  profileFixture,
  renderRoutes,
  sessionFixture,
} from '@/pages/shell/test-utils'
import { Component as ProfilesPage } from './ProfilesPage'

afterEach(() => {
  vi.unstubAllGlobals()
  localStorage.clear()
})

const routes = [{ path: '/profiles', Component: ProfilesPage }]

function twoProfiles(extra: Record<string, unknown> = {}) {
  const alex = profileFixture('p1', 'Alex', 5)
  const sam = profileFixture('p2', 'Sam')
  return mockApi({
    ...baseRoutes(sessionFixture([alex, sam])),
    'GET /profiles/p1/settings': {},
    'GET /profiles/p2/settings': {},
    ...extra,
  })
}

describe('who’s practising', () => {
  it('lists the profiles and enters one, then goes on to ?next', async () => {
    const user = userEvent.setup()
    twoProfiles()
    const { router } = renderRoutes(routes, '/profiles?next=%2Fdashboard')
    expect(await screen.findByRole('heading', { name: 'Who’s practising?' })).toBeInTheDocument()
    expect(screen.getByText('5 solved')).toBeInTheDocument()
    expect(screen.getByText('not started')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: /Sam/ }))
    await waitFor(() => expect(router.state.location.pathname).toBe('/dashboard'))
    expect(localStorage.getItem('dsa.activeProfile.u1')).toBe('p2')
  })

  it('shows the server’s duplicate-name refusal under the name field', async () => {
    const user = userEvent.setup()
    twoProfiles({ 'POST /profiles': () => apiError(409, 'conflict', 'duplicate') })
    renderRoutes(routes, '/profiles')
    await user.click(await screen.findByRole('button', { name: 'add profile' }))
    expect(screen.getByRole('heading', { name: 'New profile' })).toBeInTheDocument()
    const name = screen.getByLabelText('name')
    await user.type(name, 'Robin')
    await user.click(screen.getByRole('radio', { name: '⚡' }))
    await user.click(screen.getByRole('radio', { name: 'cyan' }))
    await user.click(screen.getByRole('button', { name: '✓ create' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('“Robin” already exists.')
    expect(name).toHaveAttribute('aria-invalid', 'true')
  })

  it('catches a duplicate before the round-trip, ignoring case', async () => {
    const user = userEvent.setup()
    const { calls } = twoProfiles()
    renderRoutes(routes, '/profiles')
    await user.click(await screen.findByRole('button', { name: 'add profile' }))
    await user.type(screen.getByLabelText('name'), 'alex{Enter}')
    expect(screen.getByRole('alert')).toHaveTextContent('“alex” already exists.')
    expect(calls.some((c) => c.method === 'POST')).toBe(false)
  })

  it('edits under “manage”, and deleting asks first', async () => {
    const user = userEvent.setup()
    const { calls } = twoProfiles({ 'DELETE /profiles/p2': undefined })
    renderRoutes(routes, '/profiles')
    await user.click(await screen.findByRole('button', { name: '⚙ manage profiles' }))
    await user.click(screen.getByRole('button', { name: 'Edit Sam' }))
    await user.click(screen.getByRole('button', { name: '✖ delete profile' }))
    expect(screen.getByText('progress, favourites and playlists too')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: '✖ delete for good' }))
    await waitFor(() => expect(calls).toContainEqual({ method: 'DELETE', path: '/profiles/p2', body: undefined }))
  })

  it('never offers to delete the only profile', async () => {
    const user = userEvent.setup()
    mockApi({ ...baseRoutes(sessionFixture([profileFixture('p1', 'Alex')])), 'GET /profiles/p1/settings': {} })
    renderRoutes(routes, '/profiles')
    await user.click(await screen.findByRole('button', { name: '⚙ manage profiles' }))
    await user.click(screen.getByRole('button', { name: 'Edit Alex' }))
    expect(screen.getByText('Your only profile can’t be deleted.')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /delete/ })).not.toBeInTheDocument()
  })
})
