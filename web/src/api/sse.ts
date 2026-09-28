// Server-sent events over a POST.
//
// `EventSource` only does GET and cannot send a CSRF header, and the AI chat
// needs both (the request carries the conversation). So this reads the
// `text/event-stream` body of a normal fetch and dispatches each event as it
// arrives. Aborting the signal closes the connection, which is also how the
// server learns to stop generating.

import { API_BASE, buildHeaders, networkError, notifyUnauthorized, toApiError } from './client'

export interface SseEvent {
  event: string
  data: string
}

/** Parse a chunk stream into events. Exported for tests. */
export class SseParser {
  private buf = ''
  private event = 'message'
  private data: string[] = []

  push(chunk: string, emit: (e: SseEvent) => void): void {
    this.buf += chunk
    let nl: number
    while ((nl = this.buf.search(/\r?\n/)) >= 0) {
      const line = this.buf.slice(0, nl)
      this.buf = this.buf.slice(nl + (this.buf[nl] === '\r' ? 2 : 1))
      this.line(line, emit)
    }
  }

  end(emit: (e: SseEvent) => void): void {
    if (this.buf) this.line(this.buf, emit)
    this.buf = ''
    this.line('', emit)
  }

  private line(line: string, emit: (e: SseEvent) => void): void {
    if (line === '') {
      if (this.data.length) emit({ event: this.event, data: this.data.join('\n') })
      this.event = 'message'
      this.data = []
      return
    }
    if (line.startsWith(':')) return // comment / keep-alive
    const colon = line.indexOf(':')
    const field = colon < 0 ? line : line.slice(0, colon)
    let value = colon < 0 ? '' : line.slice(colon + 1)
    if (value.startsWith(' ')) value = value.slice(1)
    if (field === 'event') this.event = value
    else if (field === 'data') this.data.push(value)
  }
}

/**
 * POST `body` to `path` and call `onEvent` for every event in the response.
 * Rejects with an `ApiError` if the server refuses before streaming starts.
 */
export async function postSse(
  path: string,
  body: unknown,
  onEvent: (e: SseEvent) => void,
  signal?: AbortSignal,
): Promise<void> {
  let res: Response
  try {
    res = await fetch(API_BASE + path, {
      method: 'POST',
      headers: { ...buildHeaders('POST', true), Accept: 'text/event-stream' },
      body: JSON.stringify(body),
      credentials: 'same-origin',
      signal,
    })
  } catch (e) {
    if (e instanceof DOMException && e.name === 'AbortError') throw e
    throw networkError()
  }
  if (!res.ok || !res.body) {
    if (res.status === 401) notifyUnauthorized()
    throw await toApiError(res)
  }

  const reader = res.body.getReader()
  const decoder = new TextDecoder()
  const parser = new SseParser()
  for (;;) {
    const { value, done } = await reader.read()
    if (done) break
    parser.push(decoder.decode(value, { stream: true }), onEvent)
  }
  parser.push(decoder.decode(), onEvent)
  parser.end(onEvent)
}
