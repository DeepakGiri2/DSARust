// Test harness for the account-side screens: the real providers and router,
// with `fetch` replaced by a table of canned `/api/v1` answers. Going through
// the real client means CSRF headers, error bodies and cache keys are exercised
// exactly as in the browser.

import { render } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider, type RouteObject } from 'react-router'
import type {
  Catalog,
  CatalogProblem,
  Difficulty,
  Guide,
  Meta,
  Profile,
  ProgressSnapshot,
  SessionInfo,
  Tier,
  User,
} from '@/api/types'
import { SessionProvider } from '@/state/session'
import { SettingsProvider } from '@/state/settings'
import { ToastProvider } from '@/ui'

export interface Call {
  method: string
  path: string
  body: unknown
}

type Answer = unknown | ((call: Call) => unknown)

export const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } })

export const apiError = (status: number, code: string, message: string, details?: Record<string, unknown>) =>
  json({ error: { code, message, details } }, status)

/**
 * Replace `fetch` with a route table keyed `"METHOD /path"` (no `/api/v1`, no
 * query). A value is the JSON answer; a function receives the call and may
 * return a `Response` (build it per call — a body reads once) or `undefined`
 * for 204.
 */
export function mockApi(routes: Record<string, Answer>) {
  const calls: Call[] = []
  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
    const method = init?.method ?? 'GET'
    const path = url.replace(/^\/api\/v1/, '').split('?')[0]
    const call = { method, path, body: init?.body ? JSON.parse(String(init.body)) : undefined }
    calls.push(call)
    const key = `${method} ${path}`
    if (!(key in routes)) return apiError(404, 'not_found', `no mock for ${key}`)
    const route = routes[key]
    const out = typeof route === 'function' ? await (route as (c: Call) => unknown)(call) : route
    if (out instanceof Response) return out
    return out === undefined ? new Response(null, { status: 204 }) : json(out)
  })
  vi.stubGlobal('fetch', fetchMock)
  return { fetchMock, calls }
}

/**
 * React Router builds a `Request` per navigation around jsdom's AbortSignal,
 * which Node's own `Request` rejects — so under test every navigation would
 * throw. Nothing here aborts navigations, so the signal can simply go.
 */
class NavigationRequest extends Request {
  constructor(input: RequestInfo | URL, init?: RequestInit) {
    if (!init) {
      super(input)
      return
    }
    const { signal: _signal, ...rest } = init
    super(input, rest)
  }
}

export function renderRoutes(routes: RouteObject[], initial: string) {
  vi.stubGlobal('Request', NavigationRequest)
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
  const router = createMemoryRouter([...routes, { path: '*', element: <p>elsewhere</p> }], {
    initialEntries: [initial],
  })
  const utils = render(
    <QueryClientProvider client={qc}>
      <ToastProvider>
        <SessionProvider>
          <SettingsProvider>
            <RouterProvider router={router} />
          </SettingsProvider>
        </SessionProvider>
      </ToastProvider>
    </QueryClientProvider>,
  )
  return { ...utils, router, qc }
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

export const META: Meta = {
  version: '1.0.0',
  content_version: 'test',
  features: {
    signup: true,
    email_verification_required: true,
    oauth: { github: true, google: false },
    billing: true,
    ai: { enabled: false, provider: '', model: '' },
    runner: true,
  },
  plans: [
    { id: 'free', name: 'Free', price_monthly: 0, price_yearly: 0, features: ['Every free problem'] },
    { id: 'pro', name: 'Pro', price_monthly: 10, price_yearly: 96, features: ['Premium problems'] },
  ],
  limits: { profiles_per_account: 5, draft_bytes: 65536 },
  avatars: ['🎓', '🎯', '⚡'],
  colors: ['#7c6cff', '#22d3ee', '#34d399'],
}

const problem = (title: string, category: string, tier: Tier, difficulty: Difficulty = 'Easy', viz = true): CatalogProblem => ({
  slug: title.toLowerCase().replace(/[^a-z0-9]+/g, '-'),
  title,
  category,
  difficulty,
  tier,
  leetcode_url: '',
  viz,
  langs: ['go', 'cpp', 'java'],
  premium: tier === 'extra',
})

export const CATALOG: Catalog = {
  content_version: 'test',
  tiers: [
    { id: '50', title: 'NeetCode 50', count: 2, animated: 2 },
    { id: '150', title: 'NeetCode 150', count: 4, animated: 3 },
    { id: '250', title: 'NeetCode 250', count: 5, animated: 4 },
    { id: 'extra', title: '+ Interview Extra', count: 6, animated: 5 },
  ],
  categories: [
    {
      name: 'Arrays & Hashing',
      problems: [
        problem('Two Sum', 'Arrays & Hashing', '50'),
        problem('Group Anagrams', 'Arrays & Hashing', '150', 'Medium'),
        problem('Sort Colors', 'Arrays & Hashing', '250', 'Medium', false),
        problem('Rotate Array', 'Arrays & Hashing', 'extra', 'Medium'),
      ],
    },
    {
      name: 'Two Pointers',
      problems: [
        problem('Valid Palindrome', 'Two Pointers', '50'),
        problem('Trapping Rain Water', 'Two Pointers', '150', 'Hard'),
      ],
    },
  ],
  languages: [
    { id: 'go', label: 'Go', ext: 'go', syntax: 'go', comment: '//', order: 0, runnable: true },
    { id: 'cpp', label: 'C++', ext: 'cpp', syntax: 'cpp', comment: '//', order: 1, runnable: true },
    { id: 'java', label: 'Java', ext: 'java', syntax: 'java', comment: '//', order: 2, runnable: true },
    { id: 'python', label: 'Python', ext: 'py', syntax: 'python', comment: '#', order: 3, runnable: false },
  ],
  total: 6,
  animated: 5,
}

export const GUIDE: Guide = {
  topics: [
    {
      id: 'array',
      title: 'Array',
      kind: 'structure',
      emoji: '🔢',
      what: ['Contiguous memory; `arr[i]` is O(1).'],
      complexity: [{ op: 'read by index', big: 'O(1)', note: '' }],
      syntax: { go: 'nums := []int{1}', cpp: 'vector<int> nums;' },
      notes: ['0-based everywhere.'],
    },
    {
      id: 'hashmap',
      title: 'Hash Map',
      kind: 'structure',
      emoji: '🗂',
      what: ['Buckets.'],
      complexity: [],
      syntax: { go: 'm := map[string]int{}' },
      notes: [],
    },
    {
      id: 'twopointers',
      title: 'Two Pointers',
      kind: 'technique',
      emoji: '👉',
      what: ['Walk from both ends.'],
      complexity: [],
      syntax: {},
      notes: [],
    },
  ],
  cheatsheet: [
    {
      name: 'Basics',
      rows: [
        { topic: 'declare variable', code: { go: 'x := 5', cpp: 'int x = 5;', java: 'int x = 5;' } },
        { topic: 'function', code: { go: 'func f() {}', cpp: 'void f() {}', java: 'void f() {}' } },
      ],
    },
  ],
  by_category: { 'Arrays & Hashing': ['hashmap', 'array'], 'Two Pointers': ['twopointers'] },
}

export const USER: User = {
  id: 'u1',
  email: 'alex@example.com',
  email_verified: true,
  display_name: 'Alex',
  role: 'user',
  plan: 'free',
  plan_renews_at: null,
  timezone: 'UTC',
  has_password: true,
  oauth_providers: [],
  created_at: '2026-01-01T00:00:00Z',
}

export const profileFixture = (id: string, name: string, solved = 0): Profile => ({
  id,
  name,
  avatar: '🎓',
  color: '#7c6cff',
  created_at: '2026-01-01T00:00:00Z',
  last_seen_at: '2026-01-01T00:00:00Z',
  stats: { solved, attempted: 0, favourites: 0 },
})

export function sessionFixture(profiles: Profile[], user: Partial<User> = {}): SessionInfo {
  return {
    user: { ...USER, ...user },
    csrf_token: 'csrf-test',
    entitlements: {
      plan: 'free',
      premium_content: false,
      ai_daily_limit: 10,
      ai_used_today: 0,
      runs_per_minute: 10,
    },
    profiles,
  }
}

export const EMPTY_PROGRESS: ProgressSnapshot = {
  entries: {},
  stats: { solved: 0, attempted: 0, favourites: 0 },
}

/** The answers every screen needs: meta, content, and the session probe. */
export function baseRoutes(session: SessionInfo | null): Record<string, Answer> {
  return {
    'GET /meta': META,
    'GET /content/catalog': CATALOG,
    'GET /content/guide': GUIDE,
    // A function, because a Response body can only be read once.
    'GET /auth/session': () => session ?? apiError(401, 'unauthorized', 'Not signed in.'),
    ...(session ? { 'GET /profiles': session.profiles } : {}),
  }
}
