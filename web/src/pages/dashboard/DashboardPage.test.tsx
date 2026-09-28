import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { Stats } from '@/api/types'
import { baseRoutes, mockApi, profileFixture, renderRoutes, sessionFixture } from '@/pages/shell/test-utils'
import { Component as DashboardPage } from './DashboardPage'

afterEach(() => {
  vi.unstubAllGlobals()
  localStorage.clear()
})

const STATS: Stats = {
  totals: { solved: 3, attempted: 2, favourites: 1, submissions: 12 },
  by_tier: [
    { tier: '50', title: 'NeetCode 50', solved: 2, total: 2 },
    { tier: '150', title: 'NeetCode 150', solved: 3, total: 4 },
  ],
  by_category: [
    { category: 'Arrays & Hashing', solved: 1, attempted: 1, total: 4 },
    { category: 'Two Pointers', solved: 2, attempted: 1, total: 2 },
  ],
  by_difficulty: [
    { difficulty: 'Easy', solved: 2, total: 3 },
    { difficulty: 'Medium', solved: 1, total: 2 },
    { difficulty: 'Hard', solved: 0, total: 1 },
  ],
  streak: { current: 4, longest: 9, active_today: true },
  activity: [{ day: '2026-09-20', runs: 3, solved: 1 }],
  recent: [
    {
      id: 's1',
      slug: 'two-sum',
      lang: 'go',
      kind: 'test',
      mode: 'solution',
      status: 'passed',
      passed: 5,
      total: 5,
      duration_ms: 900,
      created_at: new Date().toISOString(),
    },
  ],
}

function render() {
  mockApi({
    ...baseRoutes(sessionFixture([profileFixture('p1', 'Alex')])),
    'GET /profiles/p1/settings': {},
    'GET /profiles/p1/stats': STATS,
  })
  return renderRoutes([{ path: '/dashboard', Component: DashboardPage }], '/dashboard')
}

describe('dashboard', () => {
  it('adds up the profile’s progress', async () => {
    render()
    expect(await screen.findByRole('heading', { level: 1, name: /Alex’s progress/ })).toBeInTheDocument()
    expect(screen.getByText('of 6')).toBeInTheDocument()
    expect(screen.getByText('current streak')).toBeInTheDocument()
    expect(screen.getByText('✓ active today')).toBeInTheDocument()
    expect(screen.getByRole('progressbar', { name: 'NeetCode 150' })).toHaveAttribute('aria-valuenow', '3')
    expect(screen.getByRole('group', { name: /3 runs and 1 solved/ })).toBeInTheDocument()
    const recent = screen.getByRole('link', { name: 'Two Sum' })
    expect(recent).toHaveAttribute('href', '/problems/two-sum')
    expect(screen.getByText('Go · tests 5/5')).toBeInTheDocument()
  })

  it('sorts the categories by any column, and back to roadmap order', async () => {
    const user = userEvent.setup()
    render()
    const table = await screen.findByRole('table')
    const names = () => within(table).getAllByRole('rowheader').map((c) => c.textContent)
    expect(names()).toEqual(['Arrays & Hashing', 'Two Pointers'])
    await user.click(within(table).getByRole('button', { name: /Solved/ }))
    expect(names()).toEqual(['Two Pointers', 'Arrays & Hashing'])
    expect(within(table).getByRole('columnheader', { name: /Solved/ })).toHaveAttribute('aria-sort', 'descending')
    await user.click(screen.getByRole('button', { name: 'back to roadmap order' }))
    expect(names()).toEqual(['Arrays & Hashing', 'Two Pointers'])
  })
})
