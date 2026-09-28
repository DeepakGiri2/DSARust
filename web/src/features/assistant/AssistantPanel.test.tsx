// The panel end to end: the real session provider, TanStack Query and
// `postSse`, over a mocked `fetch` whose `/ai/chat` body is a ReadableStream
// the test writes SSE chunks into — split wherever it likes.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router'
import type { AiChatRequest, AiStatus, Problem, SessionInfo } from '@/api/types'
import { SessionProvider } from '@/state/session'
import { AssistantPanel, type AssistantPanelProps } from './AssistantPanel'
import { FIX_REQUEST } from './conversation'
import { assistantStore } from './store'

const SESSION: SessionInfo = {
  user: {
    id: 'user-1',
    email: 'ada@example.com',
    email_verified: true,
    display_name: 'Ada',
    role: 'user',
    plan: 'pro',
    plan_renews_at: null,
    timezone: 'UTC',
    has_password: true,
    oauth_providers: [],
    created_at: '2026-01-01T00:00:00Z',
  },
  csrf_token: 'csrf',
  entitlements: { plan: 'pro', premium_content: true, ai_daily_limit: 20, ai_used_today: 2, runs_per_minute: 10 },
  profiles: [],
}

/** A response body the test feeds by hand; aborting the request errors it, as fetch does. */
class SseBody {
  readonly stream: ReadableStream<Uint8Array>
  private controller!: ReadableStreamDefaultController<Uint8Array>
  constructor(signal: AbortSignal | null | undefined) {
    this.stream = new ReadableStream({ start: (c) => void (this.controller = c) })
    signal?.addEventListener('abort', () => this.controller.error(new DOMException('aborted', 'AbortError')))
  }
  push(chunk: string) {
    this.controller.enqueue(new TextEncoder().encode(chunk))
  }
}

interface ChatCall {
  body: AiChatRequest
  sse: SseBody
  signal: AbortSignal | null | undefined
}

const sse = (event: string, data: unknown) => `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`
const done = (remaining: number) => sse('done', { input_tokens: 10, output_tokens: 20, remaining_today: remaining })
const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } })

let chats: ChatCall[] = []
let signedIn = true
let aiStatus: AiStatus
let refuseChat: Response | null = null

beforeEach(() => {
  chats = []
  signedIn = true
  refuseChat = null
  aiStatus = { enabled: true, provider: 'anthropic', model: 'claude-test', daily_limit: 20, used_today: 2 }
  assistantStore.setMode('interview')
  assistantStore.setAttach(false)
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input).replace('/api/v1', '')
      if (path === '/auth/session') return signedIn ? json(SESSION) : json({ error: { code: 'unauthorized', message: 'no' } }, 401)
      if (path === '/profiles') return json([])
      if (path === '/ai/status') return json(aiStatus)
      if (path === '/ai/chat') {
        if (refuseChat) return refuseChat
        const body = new SseBody(init?.signal)
        chats.push({ body: JSON.parse(String(init?.body)) as AiChatRequest, sse: body, signal: init?.signal })
        return new Response(body.stream, { status: 200, headers: { 'Content-Type': 'text/event-stream' } })
      }
      return json({ error: { code: 'not_found', message: path } }, 404)
    }),
  )
})

afterEach(() => {
  act(() => assistantStore.stop())
  vi.unstubAllGlobals()
})

let seq = 0
function problem(): Problem {
  return {
    slug: `problem-${++seq}`, // every test talks about its own problem, so the shared store never leaks between them
    title: 'Two Sum',
    category: 'Arrays & Hashing',
    difficulty: 'Easy',
    tier: '50',
    description: '',
    approach: '',
    complexity: '',
    leetcode_url: '',
    inputs: [],
    default_input: {},
    default_fields: {},
    tests: [],
    hints: [],
    related: [],
    guide_topics: [],
    has_trace: false,
    premium: false,
    locked: false,
    sources: [],
  }
}

function renderPanel(over: Partial<AssistantPanelProps> = {}) {
  const props: AssistantPanelProps = {
    problem: problem(),
    lang: 'go',
    code: 'func f() int {\n\treturn 1\n}',
    runContext: '',
    onApplyCode: vi.fn(),
    onClose: vi.fn(),
    ...over,
  }
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={client}>
      <SessionProvider>
        <MemoryRouter>
          <AssistantPanel {...props} />
        </MemoryRouter>
      </SessionProvider>
    </QueryClientProvider>,
  )
  return { props, user: userEvent.setup() }
}

async function ask(user: ReturnType<typeof userEvent.setup>, text: string, speaker = 'interviewer') {
  const box = await screen.findByRole('textbox', { name: `Message the ${speaker}` })
  await user.type(box, `${text}{Enter}`)
  await waitFor(() => expect(chats.length).toBeGreaterThan(0))
  return chats[chats.length - 1]
}

describe('AssistantPanel', () => {
  it('streams a reply, keeps thinking collapsed and apart, and shows the quota', async () => {
    const { props, user } = renderPanel()
    expect(await screen.findByText('18 of 20 left today')).toBeInTheDocument()
    expect(screen.getByText('anthropic · claude-test')).toBeInTheDocument()

    const call = await ask(user, 'am I on the right track?')
    // "attach my code" starts unticked: no code, no run output.
    expect(call.body).toEqual({
      slug: props.problem.slug,
      lang: 'go',
      mode: 'interview',
      messages: [{ role: 'user', content: 'am I on the right track?' }],
    })

    // Chunks split mid-line and mid-JSON, with a thinking token first.
    const wire =
      sse('token', { channel: 'thinking', text: 'They compare every pair.' }) +
      sse('token', { channel: 'content', text: 'What could you ' }) +
      sse('token', { channel: 'content', text: 'remember as you scan?' })
    call.sse.push(wire.slice(0, 23))
    call.sse.push(wire.slice(23, 101))
    call.sse.push(wire.slice(101))

    const answer = await screen.findByText('What could you remember as you scan?')
    expect(answer.textContent).not.toContain('pair')
    const thinking = screen.getByText(/💭 thinking/).closest('details')
    expect(thinking).not.toBeNull()
    expect(thinking).not.toHaveAttribute('open')
    expect(thinking).toHaveTextContent('They compare every pair.')
    expect(screen.getByRole('button', { name: '■ stop' })).toBeInTheDocument()

    call.sse.push(done(17))
    expect(await screen.findByText('17 of 20 left today')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Send' })).toBeInTheDocument()
  })

  it('renders a mid-stream error inline, keeps what arrived, and retries in its place', async () => {
    const { user } = renderPanel()
    const first = await ask(user, 'hint please')
    first.sse.push(sse('token', { channel: 'content', text: 'Think about' }) + sse('error', { code: 'unavailable', message: 'down' }))

    expect(await screen.findByText(/busy or unavailable/)).toBeInTheDocument()
    expect(screen.getByText('Think about')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: '↻ retry' }))
    await waitFor(() => expect(chats).toHaveLength(2))
    expect(chats[1].body.messages).toEqual([{ role: 'user', content: 'hint please' }])
    chats[1].sse.push(sse('token', { channel: 'content', text: 'What repeats?' }) + done(16))
    expect(await screen.findByText('What repeats?')).toBeInTheDocument()
    expect(screen.queryByText(/busy or unavailable/)).not.toBeInTheDocument()
  })

  it('maps a refusal to its next step: wait out a rate limit, or see plans', async () => {
    refuseChat = json({ error: { code: 'rate_limited', message: 'slow down', details: { retry_after_secs: 30 } } }, 429)
    const { user } = renderPanel()
    const box = await screen.findByRole('textbox', { name: 'Message the interviewer' })
    await user.type(box, 'q{Enter}')
    expect(await screen.findByText(/Try again in 30s/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '↻ retry' })).toBeDisabled()

    refuseChat = json({ error: { code: 'payment_required', message: "You've used today's 20 requests." } }, 402)
    await user.click(screen.getByRole('button', { name: 'Clear this conversation' }))
    await user.type(box, 'q{Enter}')
    expect(await screen.findByText(/used today's 20 requests/)).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'see plans →' })).toHaveAttribute('href', '/pricing')
  })

  it('turns Guide OPTION lines into choices that send themselves', async () => {
    const { user } = renderPanel()
    await user.click(await screen.findByRole('button', { name: /Guide$/ }))
    const call = await ask(user, 'help', 'mentor')
    const reply = 'What would you like help with?\nOPTION: Explain the approach\nOPTION: Review my code'
    call.sse.push(sse('token', { channel: 'content', text: reply }) + done(17))

    const choice = await screen.findByRole('button', { name: 'Explain the approach' })
    expect(screen.getByRole('button', { name: 'Review my code' })).toBeInTheDocument()
    expect(screen.queryByText(/OPTION:/)).not.toBeInTheDocument()

    await user.click(choice)
    await waitFor(() => expect(chats).toHaveLength(2))
    expect(chats[1].body.mode).toBe('guide')
    expect(chats[1].body.messages).toEqual([
      { role: 'user', content: 'help' },
      { role: 'assistant', content: reply },
      { role: 'user', content: 'Explain the approach' },
    ])
  })

  it('locks code in Interview replies until asked, and attaches code only when ticked', async () => {
    const { user } = renderPanel({ runContext: 'test 1 FAILED' })
    await user.click(await screen.findByRole('checkbox', { name: '📎 attach my code' }))
    const call = await ask(user, 'why does it fail?')
    expect(call.body).toMatchObject({ code: 'func f() int {\n\treturn 1\n}', run_context: 'test 1 FAILED' })
    expect(screen.getByText('📎 code attached')).toBeInTheDocument()

    call.sse.push(sse('token', { channel: 'content', text: 'Look:\n```go\nreturn nums\n```' }) + done(17))
    const reveal = await screen.findByRole('button', { name: /code hidden — interview mode/ })
    expect(screen.queryByRole('button', { name: 'copy' })).not.toBeInTheDocument()
    await user.click(reveal)
    expect(screen.getByRole('button', { name: 'copy' })).toBeInTheDocument()
    expect(screen.getByText('go')).toBeInTheDocument()
  })

  it('stop cancels the request and keeps the partial reply', async () => {
    const { user } = renderPanel()
    const call = await ask(user, 'q')
    call.sse.push(sse('token', { channel: 'content', text: 'partial' }))
    await screen.findByText('partial')
    await user.click(screen.getByRole('button', { name: '■ stop' }))
    expect(call.signal?.aborted).toBe(true)
    expect(await screen.findByText('■ stopped')).toBeInTheDocument()
  })

  it('shows a Fix as a diff against the code and applies only the ticked changes', async () => {
    const code = 'a\nWRONG1\nb\nc\nd\ne\nf\nWRONG2\ng'
    const { props, user } = renderPanel({ code, runContext: 'test 2 FAILED' })
    await user.click(await screen.findByRole('button', { name: /Fix$/ }))
    const attach = screen.getByRole('checkbox', { name: '📎 attach my code' })
    expect(attach).toBeChecked()
    expect(attach).toBeDisabled()

    await user.click(screen.getByRole('button', { name: 'Analyze my code' }))
    await waitFor(() => expect(chats).toHaveLength(1))
    expect(chats[0].body).toEqual({
      slug: props.problem.slug,
      lang: 'go',
      mode: 'fix',
      messages: [{ role: 'user', content: FIX_REQUEST }],
      code,
      run_context: 'test 2 FAILED',
    })

    chats[0].sse.push(
      sse('token', { channel: 'thinking', text: 'two lines look off' }) +
        sse('token', { channel: 'content', text: 'ISSUES:\n- two wrong lines\n\nFIXED CODE:\n```go\n' }) +
        sse('token', { channel: 'content', text: 'a\nRIGHT1\nb\nc\nd\ne\nf\nRIGHT2\ng\n```' }) +
        done(15),
    )

    expect(await screen.findByText('- two wrong lines')).toBeInTheDocument()
    expect(screen.getByText('WRONG1')).toBeInTheDocument()
    expect(screen.getByText('RIGHT2')).toBeInTheDocument()
    const first = screen.getByRole('checkbox', { name: 'change 1 of 2' })
    const second = screen.getByRole('checkbox', { name: 'change 2 of 2' })
    expect(first).toBeChecked()
    expect(second).toBeChecked()
    expect(props.onApplyCode, 'the editor is untouched until apply').not.toHaveBeenCalled()

    await user.click(second)
    expect(screen.getByText(/1 of 2 changes kept/)).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: '✔ apply selected' }))
    expect(props.onApplyCode).toHaveBeenCalledWith('a\nRIGHT1\nb\nc\nd\ne\nf\nWRONG2\ng\n')
    expect(await screen.findByText(/applied 1 of 2 changes/)).toBeInTheDocument()
  })

  it('discarding a Fix leaves the editor alone', async () => {
    const { props, user } = renderPanel({ code: 'x := 1' })
    await user.click(await screen.findByRole('button', { name: /Fix$/ }))
    await user.click(screen.getByRole('button', { name: 'Analyze my code' }))
    await waitFor(() => expect(chats).toHaveLength(1))
    chats[0].sse.push(sse('token', { channel: 'content', text: 'ISSUES:\n- typo\n\n```go\nx := 2\n```' }) + done(15))
    await user.click(await screen.findByRole('button', { name: '✖ discard' }))
    expect(screen.getByText(/discarded — your code is unchanged/)).toBeInTheDocument()
    expect(props.onApplyCode).not.toHaveBeenCalled()
  })

  it('asks a guest to sign in, without calling the AI', async () => {
    signedIn = false
    renderPanel()
    expect(await screen.findByText('Sign in to use AI assist')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Sign in' })).toHaveAttribute('href', expect.stringContaining('/login?next='))
    expect(vi.mocked(fetch).mock.calls.some(([url]) => String(url).includes('/ai/'))).toBe(false)
  })

  it('says so when the server has AI assist switched off', async () => {
    aiStatus = { ...aiStatus, enabled: false }
    renderPanel()
    expect(await screen.findByText("AI assist isn't available on this server")).toBeInTheDocument()
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
  })

  it('closes', async () => {
    const { props, user } = renderPanel()
    await user.click(await screen.findByRole('button', { name: 'Close AI assist' }))
    expect(props.onClose).toHaveBeenCalled()
  })
})
