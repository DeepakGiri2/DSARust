// The routing and lifecycle cases are ported from the tests in
// crates/dsa-app/src/assistant.rs; the request and persistence cases are the
// web's own.
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '@/api/client'
import type { AiChatRequest } from '@/api/types'
import { FIX_REQUEST } from './conversation'
import { AssistantStore, chatKey, fixKey, type ChatRequest, type Transport } from './store'

interface Call {
  body: AiChatRequest
  signal: AbortSignal
  emit: (event: string, data: unknown) => void
  end: () => void
  fail: (e: unknown) => void
}

function fakeServer() {
  const calls: Call[] = []
  const transport: Transport = (_path, body, onEvent, signal) =>
    new Promise<void>((resolve, reject) => {
      signal?.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')))
      calls.push({
        body: body as AiChatRequest,
        signal: signal!,
        emit: (event, data) => onEvent({ event, data: JSON.stringify(data) }),
        end: resolve,
        fail: reject,
      })
    })
  return { calls, transport }
}

const stores: AssistantStore[] = []
function setup(storage: Storage | null = null) {
  const server = fakeServer()
  const store = new AssistantStore({ transport: server.transport, storage, now: () => 1_000 })
  store.adopt('user-1')
  stores.push(store)
  return { store, ...server }
}
afterEach(() => stores.splice(0).forEach((s) => s.stop()))

const req = (over: Partial<ChatRequest> = {}): ChatRequest => ({
  slug: 'two-sum',
  lang: 'go',
  mode: 'guide',
  code: 'func f() {}',
  runContext: 'test 1 FAILED',
  ...over,
})

const lastTurn = (store: AssistantStore, key: string) => store.getState().chats[key]?.at(-1)

describe('AssistantStore', () => {
  it('tokens land on the chat the stream was started for', () => {
    const { store, calls } = setup()
    store.sendChat('q', req({ mode: 'guide' }))
    calls[0].emit('token', { channel: 'content', text: 'hel' })
    calls[0].emit('token', { channel: 'content', text: 'lo' })
    calls[0].emit('token', { channel: 'thinking', text: 'hmm' })
    expect(lastTurn(store, chatKey('two-sum', 'guide'))).toMatchObject({ content: 'hello', thinking: 'hmm' })
    expect(store.getState().chats[chatKey('two-sum', 'interview')], "the other mode's transcript is untouched").toBe(
      undefined,
    )
  })

  it('switching mode mid-stream does not misroute tokens', () => {
    const { store, calls } = setup()
    store.sendChat('q', req({ mode: 'interview' }))
    store.setMode('guide') // the user clicked away while it streamed
    calls[0].emit('token', { channel: 'content', text: 'hint' })
    expect(lastTurn(store, chatKey('two-sum', 'interview'))).toMatchObject({ content: 'hint' })
    expect(store.getState().chats[chatKey('two-sum', 'guide')]).toBe(undefined)
  })

  it('stopping clears the stream, cancels the request and keeps what arrived', () => {
    const { store, calls } = setup()
    store.sendChat('q', req())
    calls[0].emit('token', { channel: 'content', text: 'partial' })
    store.stop()
    expect(store.getState().active).toBeNull()
    expect(calls[0].signal.aborted, 'aborting is what stops generation server-side').toBe(true)
    expect(lastTurn(store, chatKey('two-sum', 'guide'))).toMatchObject({ content: 'partial', status: 'stopped' })
  })

  it('one reply at a time', () => {
    const { store, calls } = setup()
    expect(store.sendChat('one', req())).toBe(true)
    expect(store.sendChat('two', req({ mode: 'interview' }))).toBe(false)
    expect(store.analyze({ ...req(), note: '' })).toBe(false)
    expect(calls).toHaveLength(1)
  })

  it('sends the code, and the run output with it, only when attached', () => {
    const { store, calls } = setup()
    store.sendChat('plain', req())
    calls[0].emit('done', { input_tokens: 1, output_tokens: 1, remaining_today: 5 })
    expect(calls[0].body).toEqual({
      slug: 'two-sum',
      lang: 'go',
      mode: 'guide',
      messages: [{ role: 'user', content: 'plain' }],
    })

    store.setAttach(true)
    store.sendChat('with code', req())
    expect(calls[1].body).toMatchObject({ code: 'func f() {}', run_context: 'test 1 FAILED' })
    expect(lastTurn(store, chatKey('two-sum', 'guide'))?.role).toBe('assistant')
    expect(store.getState().chats[chatKey('two-sum', 'guide')]?.at(-2)).toMatchObject({ codeAttached: true })
  })

  it('reports the quota after a reply', () => {
    const { store, calls } = setup()
    const onQuota = vi.fn()
    store.sendChat('q', req(), { onQuota })
    calls[0].emit('done', { input_tokens: 3, output_tokens: 4, remaining_today: 7 })
    expect(onQuota).toHaveBeenCalledWith(7)
    expect(lastTurn(store, chatKey('two-sum', 'guide'))).toMatchObject({ status: 'done' })
    expect(calls[0].signal.aborted, 'the connection is released as soon as the reply is done').toBe(true)
  })

  it('an error event ends the reply as an error; retry asks again in its place', () => {
    const { store, calls } = setup()
    store.sendChat('q', req())
    calls[0].emit('error', { code: 'rate_limited', message: 'slow down', details: { retry_after_secs: 30 } })
    const failed = lastTurn(store, chatKey('two-sum', 'guide'))
    expect(failed).toMatchObject({ status: 'error', error: { code: 'rate_limited', retryAt: 31_000 } })
    expect(store.getState().active).toBeNull()

    expect(store.retryChat(req())).toBe(true)
    expect(calls[1].body.messages).toEqual([{ role: 'user', content: 'q' }])
    const turns = store.getState().chats[chatKey('two-sum', 'guide')]!
    expect(turns).toHaveLength(2)
    expect(turns[1]).toMatchObject({ status: 'streaming', error: null })
  })

  it('a refusal before streaming, a dropped body and a lost session are each reported', async () => {
    const { store, calls } = setup()
    const onAuthIssue = vi.fn()

    store.sendChat('q', req(), { onAuthIssue })
    calls[0].fail(new ApiError(401, 'unauthorized', 'session expired'))
    await vi.waitFor(() => expect(lastTurn(store, chatKey('two-sum', 'guide'))).toMatchObject({ status: 'error' }))
    expect(onAuthIssue).toHaveBeenCalled()

    store.sendChat('again', req())
    calls[1].emit('token', { channel: 'content', text: 'half' })
    calls[1].end() // the body ended with neither `done` nor `error`
    await vi.waitFor(() =>
      expect(lastTurn(store, chatKey('two-sum', 'guide'))).toMatchObject({ content: 'half', error: { code: 'dropped' } }),
    )
  })

  it('fix always sends the code and the last run, and parses the finished reply once', () => {
    const { store, calls } = setup()
    const code = 'func f() {\n\treturn 1\n}'
    store.analyze({ slug: 'two-sum', lang: 'go', note: '', code, runContext: '' })
    expect(calls[0].body).toEqual({
      slug: 'two-sum',
      lang: 'go',
      mode: 'fix',
      messages: [{ role: 'user', content: FIX_REQUEST }],
      code,
      run_context: '',
    })
    calls[0].emit('token', { channel: 'thinking', text: 'the loop…' })
    calls[0].emit('token', { channel: 'content', text: 'ISSUES:\n- off by one\n\n```go\nfunc f() {\n    return 2\n}\n```' })
    const key = fixKey('two-sum', 'go')
    expect(store.getState().fixes[key]?.outcome, 'nothing is parsed while it streams').toBeNull()
    calls[0].emit('done', { input_tokens: 1, output_tokens: 1, remaining_today: 1 })
    expect(store.getState().fixes[key]?.outcome).toEqual({
      kind: 'proposal',
      analysis: '- off by one',
      code: 'func f() {\n\treturn 2\n}',
    })
  })

  it('a fix is kept per language, so a Go proposal never meets the C++ buffer', () => {
    const { store, calls } = setup()
    store.analyze({ slug: 'two-sum', lang: 'go', note: 'look at the loop', code: 'x', runContext: '' })
    calls[0].emit('done', { input_tokens: 1, output_tokens: 1, remaining_today: 1 })
    expect(calls[0].body.messages).toEqual([{ role: 'user', content: 'look at the loop' }])
    expect(store.getState().fixes[fixKey('two-sum', 'go')]).toBeDefined()
    expect(store.getState().fixes[fixKey('two-sum', 'cpp')]).toBeUndefined()
  })

  it('another account on this tab starts from nothing, and its reply in flight is dropped', () => {
    const { store, calls } = setup()
    store.sendChat('mine', req())
    store.adopt('user-2')
    expect(calls[0].signal.aborted).toBe(true)
    expect(store.getState()).toMatchObject({ owner: 'user-2', chats: {}, fixes: {}, active: null })
  })

  it('survives a reload, and a reply that was streaming comes back stopped', () => {
    const data = new Map<string, string>()
    const storage = {
      getItem: (k: string) => data.get(k) ?? null,
      setItem: (k: string, v: string) => void data.set(k, v),
    } as Storage
    const { store, calls } = setup(storage)
    store.sendChat('q', req())
    calls[0].emit('token', { channel: 'content', text: 'partial' })
    window.dispatchEvent(new Event('pagehide'))

    const reloaded = new AssistantStore({ transport: fakeServer().transport, storage })
    stores.push(reloaded)
    expect(reloaded.getState().owner).toBe('user-1')
    expect(reloaded.getState().active).toBeNull()
    expect(lastTurn(reloaded, chatKey('two-sum', 'guide'))).toMatchObject({ content: 'partial', status: 'stopped' })

    data.set('dsa.assistant.v1', '{"v":1,"owner":7}')
    expect(new AssistantStore({ transport: fakeServer().transport, storage }).getState().chats, 'unreadable data starts afresh').toEqual({})
  })
})
