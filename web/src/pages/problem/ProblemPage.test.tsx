// Render tests of the whole workspace against the dev mock's API (real
// problem content, the engine's own two-sum trace), with the renderer — a
// separate package — replaced by a list of view labels.
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, Route, Routes, useLocation } from 'react-router'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { SessionProvider } from '@/state/session'
import { SettingsProvider } from '@/state/settings'
import { ToastProvider } from '@/ui'
import type { VizCanvasProps } from '@/viz'
import { handle } from './dev/mockData'
import { Component } from './ProblemPage'

vi.mock('@/viz', () => ({
  VizCanvas: ({ views }: VizCanvasProps) => <div data-testid="viz">{views.map((v) => v.label).join(' | ')}</div>,
}))

let authed = false

beforeAll(() => {
  // jsdom does no layout; CodeMirror measures text through ranges.
  Range.prototype.getBoundingClientRect = () => new DOMRect()
  Range.prototype.getClientRects = () => [] as unknown as DOMRectList
})

beforeEach(() => {
  localStorage.clear()
  sessionStorage.clear()
  vi.stubGlobal('fetch', async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = new URL(String(input), 'http://localhost')
    const body: unknown = typeof init?.body === 'string' ? JSON.parse(init.body) : undefined
    return handle(init?.method ?? 'GET', url.pathname.replace('/api/v1', ''), body, authed, 0)
  })
})

afterEach(() => vi.unstubAllGlobals())

/** Where the app currently is — the declarative router has no inspectable state. */
const at = { pathname: '' }
function LocationProbe() {
  at.pathname = useLocation().pathname
  return null
}

// The declarative router: the data router builds a `Request` per navigation,
// and jsdom's AbortSignal is not the one Node's fetch implementation accepts.
function renderAt(path: string) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
  render(
    <QueryClientProvider client={client}>
      <ToastProvider>
        <SessionProvider>
          <SettingsProvider>
            <MemoryRouter initialEntries={[path]}>
              <LocationProbe />
              <Routes>
                <Route path="/" element={<p>problem list</p>} />
                <Route path="/problems/:slug" element={<Component />} />
                <Route path="/problems/:slug/:tab" element={<Component />} />
              </Routes>
            </MemoryRouter>
          </SettingsProvider>
        </SessionProvider>
      </ToastProvider>
    </QueryClientProvider>,
  )
}

const editorText = () => document.querySelector('.cm-content')?.textContent ?? ''

describe('Practice', () => {
  it('shows the statement, the starter, stdin and the tests, and asks a guest to sign in to run', async () => {
    authed = false
    renderAt('/problems/two-sum')
    expect(await screen.findByRole('heading', { level: 1, name: 'Two Sum' })).toBeInTheDocument()

    const statement = screen.getByRole('article', { name: 'Problem statement' })
    expect(within(statement).getByText(/Given an array of integers/)).toBeInTheDocument()
    expect(within(statement).getByText(/Input: nums = \[2,7,11,15,3\]\s+Input: target = 14\s+Output: 2 4/)).toBeInTheDocument()
    expect(screen.getByLabelText('stdin')).toHaveValue('2 7 11 15 3\n14\n')
    expect(screen.getByText('in: 2 7 11 15 3 | 14')).toBeInTheDocument()
    await waitFor(() => expect(editorText()).toContain('panic("todo")'))
    expect(screen.getByText('fmt.Println("dbg:", x)')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: '▶ Run' }))
    expect(await screen.findByText(/Sign in to run code\. Your solution runs/)).toBeInTheDocument()
    expect(screen.getAllByRole('status').map((s) => s.textContent)).toContain('Sign in to run code.')
    expect(screen.getByRole('link', { name: 'Sign in' })).toHaveAttribute('href', '/login?next=%2Fproblems%2Ftwo-sum')
    expect(document.title).toBe('Two Sum · Practice · DSA Visualized')
  })

  it('assembles the full program from the solution', async () => {
    authed = false
    renderAt('/problems/two-sum')
    fireEvent.click(await screen.findByRole('button', { name: 'full program' }))
    await waitFor(() => expect(editorText()).toContain('package main'))
    expect(editorText()).toContain('panic("todo")')
    expect(editorText()).not.toContain('seen := map[int]int{}')
  })

  it('runs the tests for a signed-in user and ticks the problem off when every case passes', async () => {
    authed = true
    const code = 'func twoSum(nums []int, target int) []int {\n    return nil\n}\n'
    await handle('PUT', '/profiles/dev-profile/drafts/two-sum/go', { code }, true, 0)
    renderAt('/problems/two-sum')
    await waitFor(() => expect(editorText()).toContain('return nil'))

    const runTests = screen.getByRole('button', { name: '✔ run tests' })
    await waitFor(() => expect(runTests).toBeEnabled())
    fireEvent.click(runTests)
    expect(await screen.findByText(/2\/2 passed · solved/)).toBeInTheDocument()
    expect(await screen.findByRole('button', { name: '✔ solved' })).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByText('1 attempt')).toBeInTheDocument()
  })
})

describe('Visualize', () => {
  it('gates the walkthrough, then steps through it like a debugger', async () => {
    authed = false
    renderAt('/problems/two-sum')
    fireEvent.click(await screen.findByRole('button', { name: '⏵ Visualize' }))
    expect(at.pathname).toBe('/problems/two-sum/visualize')

    fireEvent.click(await screen.findByRole('button', { name: /show the walkthrough/ }))
    expect(await screen.findByText(/Start with an empty hash map/)).toBeInTheDocument()
    expect(screen.getByText('1 / 21')).toBeInTheDocument()
    expect(screen.getByTestId('viz')).toHaveTextContent('nums')
    expect(document.querySelector('[aria-current="step"]')).toHaveTextContent('seen := map[int]int{}')
    // Nothing to compare the first step with, so every value counts as changed.
    expect(screen.getAllByText('(changed)')).toHaveLength(3)

    fireEvent.keyDown(document.body, { key: 'ArrowRight' })
    expect(await screen.findByText('2 / 21')).toBeInTheDocument()
    expect(screen.getByText(/i = 0: look at nums\[0\] = 2/)).toBeInTheDocument()
    expect(document.querySelector('[aria-current="step"]')).toHaveTextContent('for i, x := range nums')

    // A breakpoint on `seen[x] = i`; continue stops at its first hit.
    fireEvent.click(screen.getByRole('button', { name: 'Breakpoint on line 8' }))
    fireEvent.click(screen.getByRole('button', { name: 'Continue to the next breakpoint' }))
    expect(await screen.findByText('5 / 21')).toBeInTheDocument()
    expect(screen.getByText('● breakpoint')).toBeInTheDocument()

    // Pinning a variable moves it to the top of the list.
    fireEvent.click(screen.getByRole('button', { name: 'Watch target' }))
    expect(screen.getAllByRole('button', { name: /^Watch / })[0]).toHaveAccessibleName('Watch target')
  })

  it('remembers the reveal for the session', async () => {
    authed = false
    sessionStorage.setItem('dsa.revealed.two-sum', '1')
    renderAt('/problems/two-sum/visualize')
    expect(await screen.findByText(/Start with an empty hash map/)).toBeInTheDocument()
  })
})

describe('Routing and access', () => {
  it('shows a friendly page for an unknown problem', async () => {
    renderAt('/problems/no-such-problem')
    expect(await screen.findByRole('heading', { name: 'No such problem' })).toBeInTheDocument()
  })

  it('sends an unknown tab to Practice', async () => {
    renderAt('/problems/two-sum/bogus')
    await waitFor(() => expect(at.pathname).toBe('/problems/two-sum'))
  })

  it('shows a locked premium problem’s statement with an upgrade link instead of code', async () => {
    authed = false
    renderAt('/problems/lru-cache')
    expect(await screen.findByRole('heading', { name: '🔒 Part of Pro' })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'See plans →' })).toHaveAttribute('href', '/pricing')
    expect(screen.getByText(/Design a data structure/)).toBeInTheDocument()
    expect(document.querySelector('.cm-editor')).toBeNull()
  })
})
