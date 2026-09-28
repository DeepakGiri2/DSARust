import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import {
  apiError,
  baseRoutes,
  mockApi,
  profileFixture,
  renderRoutes,
  sessionFixture,
} from '@/pages/shell/test-utils'
import { Component as LoginPage } from './LoginPage'
import { Component as SignupPage } from './SignupPage'

afterEach(() => {
  vi.unstubAllGlobals()
  localStorage.clear()
})

const routes = [
  { path: '/login', Component: LoginPage },
  { path: '/signup', Component: SignupPage },
  { path: '/dashboard', element: <p>dashboard</p> },
]

async function fill(user: ReturnType<typeof userEvent.setup>, email: string, password: string) {
  await user.type(await screen.findByLabelText('Email'), email)
  await user.type(screen.getByLabelText('Password'), password)
}

describe('sign in', () => {
  it('flags empty fields, and focuses the first', async () => {
    const user = userEvent.setup()
    mockApi(baseRoutes(null))
    renderRoutes(routes, '/login')
    await user.click(await screen.findByRole('button', { name: 'Sign in' }))
    const email = screen.getByLabelText('Email')
    expect(email).toHaveAttribute('aria-invalid', 'true')
    expect(email).toHaveFocus()
    expect(screen.getByText('Enter your email address.')).toBeInTheDocument()
    expect(screen.getByLabelText('Password')).toHaveAttribute('aria-invalid', 'true')
  })

  it('works with password managers: real autocomplete hints and a show/hide switch', async () => {
    const user = userEvent.setup()
    mockApi(baseRoutes(null))
    renderRoutes(routes, '/login')
    expect(await screen.findByLabelText('Email')).toHaveAttribute('autocomplete', 'username')
    const password = screen.getByLabelText('Password')
    expect(password).toHaveAttribute('autocomplete', 'current-password')
    await user.click(screen.getByRole('button', { name: 'Show password' }))
    expect(password).toHaveAttribute('type', 'text')
  })

  it('says how long to wait when throttled', async () => {
    const user = userEvent.setup()
    mockApi({
      ...baseRoutes(null),
      'POST /auth/login': () => apiError(429, 'rate_limited', 'slow down', { retry_after_secs: 30 }),
    })
    renderRoutes(routes, '/login')
    await fill(user, 'alex@example.com', 'hunter2hunter2')
    await user.click(screen.getByRole('button', { name: 'Sign in' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Try again in 30 seconds.')
  })

  it('signs in and goes on to ?next', async () => {
    const user = userEvent.setup()
    const session = sessionFixture([profileFixture('p1', 'Alex')])
    mockApi({
      ...baseRoutes(null),
      'POST /auth/login': session,
      'GET /profiles': session.profiles,
      'GET /profiles/p1/settings': {},
    })
    renderRoutes(routes, '/login?next=%2Fdashboard')
    await fill(user, 'alex@example.com', 'correct horse battery')
    await user.click(screen.getByRole('button', { name: 'Sign in' }))
    expect(await screen.findByText('dashboard')).toBeInTheDocument()
  })

  it('offers the OAuth providers the server has, keeping the destination', async () => {
    mockApi(baseRoutes(null))
    renderRoutes(routes, '/login?next=%2Fdashboard')
    expect(await screen.findByRole('link', { name: 'Continue with GitHub' })).toHaveAttribute(
      'href',
      '/api/v1/auth/oauth/github/start?next=%2Fdashboard',
    )
    expect(screen.queryByRole('link', { name: 'Continue with Google' })).not.toBeInTheDocument()
  })

  it('explains a failed OAuth round-trip', async () => {
    mockApi(baseRoutes(null))
    renderRoutes(routes, '/login?error=oauth_denied')
    expect(await screen.findByRole('alert')).toHaveTextContent('cancelled')
  })
})

describe('sign up', () => {
  it('puts a taken email next to the email field', async () => {
    const user = userEvent.setup()
    mockApi({
      ...baseRoutes(null),
      'POST /auth/signup': () => apiError(409, 'conflict', 'exists'),
    })
    renderRoutes(routes, '/signup')
    await user.type(await screen.findByLabelText('Your name'), 'Alex')
    await fill(user, 'alex@example.com', 'correct horse battery')
    await user.click(screen.getByRole('button', { name: 'Create account' }))
    const email = screen.getByLabelText('Email')
    expect(await screen.findByText(/already exists/)).toBeInTheDocument()
    expect(email).toHaveAttribute('aria-invalid', 'true')
    expect(email).toHaveFocus()
  })

  it('refuses a short password before asking the server', async () => {
    const user = userEvent.setup()
    const { calls } = mockApi(baseRoutes(null))
    renderRoutes(routes, '/signup')
    await user.type(await screen.findByLabelText('Your name'), 'Alex')
    await fill(user, 'alex@example.com', 'short')
    expect(screen.getByText('5 more characters to go')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Create account' }))
    expect(screen.getByText('Use at least 10 characters.')).toBeInTheDocument()
    expect(calls.some((c) => c.path === '/auth/signup')).toBe(false)
  })
})
