import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ProgressEntry } from '@/api/types'
import {
  baseRoutes,
  type Call,
  mockApi,
  profileFixture,
  renderRoutes,
  sessionFixture,
} from '@/pages/shell/test-utils'
import { Component as CatalogPage } from './CatalogPage'

afterEach(() => {
  vi.unstubAllGlobals()
  localStorage.clear()
})

const routes = [{ path: '/', Component: CatalogPage }]

const solved: ProgressEntry = {
  status: 'solved',
  favourite: false,
  attempts: 2,
  solved_at: '2026-09-01T00:00:00Z',
  updated_at: '2026-09-01T00:00:00Z',
}

function signedIn() {
  const alex = profileFixture('p1', 'Alex', 1)
  return mockApi({
    ...baseRoutes(sessionFixture([alex])),
    'GET /profiles/p1/settings': {},
    'PUT /profiles/p1/settings': {},
    'GET /profiles/p1/playlists': [],
    'GET /profiles/p1/progress': {
      entries: { 'two-sum': solved },
      stats: { solved: 1, attempted: 0, favourites: 0 },
    },
    'PUT /profiles/p1/progress/two-sum/favourite': ({ body }: Call) => ({
      ...solved,
      favourite: (body as { favourite: boolean }).favourite,
    }),
  })
}

describe('catalogue', () => {
  it('shows the hero and NeetCode 150 to a guest, with the progress filters switched off', async () => {
    mockApi(baseRoutes(null))
    renderRoutes(routes, '/')
    expect(await screen.findByRole('heading', { level: 1, name: 'DSA Visualized' })).toBeInTheDocument()
    expect(await screen.findByText('4 shown')).toBeInTheDocument()
    // Python is declared but no problem ships it, so it is not claimed.
    expect(screen.getByText('Go · C++ · Java')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Two Sum' })).toHaveAttribute('href', '/problems/two-sum')
    expect(screen.getByRole('heading', { level: 2, name: 'Two Pointers' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'solved' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'favourites' })).toBeDisabled()
  })

  it('focuses the search on "/" and filters as you type', async () => {
    const user = userEvent.setup()
    mockApi(baseRoutes(null))
    renderRoutes(routes, '/')
    await screen.findByText('4 shown')
    await user.keyboard('/')
    const box = screen.getByRole('searchbox', { name: 'Search problems' })
    expect(box).toHaveFocus()
    await user.type(box, 'RAIN')
    expect(screen.getByText('1 shown')).toBeInTheDocument()
    expect(screen.queryByRole('link', { name: 'Two Sum' })).not.toBeInTheDocument()
  })

  it('explains an empty list and offers the bigger lists', async () => {
    const user = userEvent.setup()
    mockApi(baseRoutes(null))
    renderRoutes(routes, '/?q=sort%20colors')
    expect(await screen.findByText('No problems match')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'search every list (1)' }))
    expect(await screen.findByRole('link', { name: 'Sort Colors' })).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: /Interview/ }))
    expect(screen.getByText('1 shown')).toBeInTheDocument()
  })

  it('asks a guest to sign in when they star a problem', async () => {
    const user = userEvent.setup()
    mockApi(baseRoutes(null))
    renderRoutes(routes, '/')
    await user.click(await screen.findByRole('button', { name: 'Add Two Sum to favourites' }))
    expect(screen.getByRole('dialog')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Create a free account' })).toHaveAttribute('href', '/signup?next=%2F')
  })

  it('stars a problem at once and saves it', async () => {
    const user = userEvent.setup()
    const { calls } = signedIn()
    renderRoutes(routes, '/')
    await user.click(await screen.findByRole('button', { name: 'Add Two Sum to favourites' }))
    expect(screen.getByRole('button', { name: 'Remove Two Sum from favourites' })).toHaveAttribute('aria-pressed', 'true')
    await waitFor(() =>
      expect(calls).toContainEqual({
        method: 'PUT',
        path: '/profiles/p1/progress/two-sum/favourite',
        body: { favourite: true },
      }),
    )
  })

  it('narrows to what is solved, with the counters from the profile', async () => {
    const user = userEvent.setup()
    signedIn()
    renderRoutes(routes, '/')
    expect(await screen.findByRole('link', { name: 'Two Sum (solved)' })).toBeInTheDocument()
    expect(screen.getByTitle('1 solved')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'solved' }))
    expect(screen.getByText('1 shown')).toBeInTheDocument()
    expect(screen.queryByRole('link', { name: 'Group Anagrams' })).not.toBeInTheDocument()
  })
})
