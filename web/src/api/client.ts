// Thin fetch wrapper for /api/v1.
//
// * Same-origin: CloudFront serves the SPA and routes /api/* to the API (Vite's
//   dev proxy does the same locally), so the session cookie just travels.
// * CSRF: every non-GET request carries `X-CSRF-Token`, a value bound to the
//   session that the server hands out in `SessionInfo`. It lives in memory
//   only — never in storage — and is set by the session provider.
// * Errors: every non-2xx becomes an `ApiError` with the server's code.

import type { ApiErrorBody, ErrorCode } from './types'

export const API_BASE = '/api/v1'

export class ApiError extends Error {
  readonly status: number
  readonly code: ErrorCode
  readonly details?: Record<string, unknown>

  constructor(status: number, code: ErrorCode, message: string, details?: Record<string, unknown>) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.code = code
    this.details = details
  }

  /** `details.errors` — every validation message, for inputs checked as a set. */
  get errors(): string[] {
    const e = this.details?.errors
    return Array.isArray(e) ? e.map(String) : []
  }

  /** `details.fields` — per-field messages for forms. */
  get fieldErrors(): Record<string, string> {
    const f = this.details?.fields
    return f && typeof f === 'object' ? (f as Record<string, string>) : {}
  }

  /** `details.retry_after_secs` on 429 / 423. */
  get retryAfterSecs(): number | undefined {
    const r = this.details?.retry_after_secs
    return typeof r === 'number' ? r : undefined
  }
}

export function isApiError(e: unknown, code?: ErrorCode): e is ApiError {
  return e instanceof ApiError && (code === undefined || e.code === code)
}

/** Human-readable message for any thrown value. */
export function errorMessage(e: unknown): string {
  if (e instanceof ApiError) return e.message
  if (e instanceof Error) return e.message
  return String(e)
}

let csrfToken: string | null = null

/** Set by the session provider whenever a session starts, refreshes or ends. */
export function setCsrfToken(token: string | null): void {
  csrfToken = token
}

export function getCsrfToken(): string | null {
  return csrfToken
}

type Listener = () => void
const unauthorizedListeners = new Set<Listener>()

/** Called whenever the server answers 401 — the session has ended. */
export function onUnauthorized(fn: Listener): () => void {
  unauthorizedListeners.add(fn)
  return () => unauthorizedListeners.delete(fn)
}

/** Tell the app the session has ended (every transport reports 401 this way). */
export function notifyUnauthorized(): void {
  unauthorizedListeners.forEach((f) => f())
}

/** A fetch that failed before any response: offline, DNS, CORS, reset. */
export function networkError(): ApiError {
  return new ApiError(0, 'unavailable', 'Could not reach the server. Check your connection and try again.')
}

export type Method = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE'

export interface RequestOptions {
  signal?: AbortSignal
  headers?: Record<string, string>
  /** Do not broadcast a 401 (the session probe itself expects one). */
  quiet401?: boolean
}

export function buildHeaders(method: Method, hasBody: boolean, extra?: Record<string, string>): Record<string, string> {
  const headers: Record<string, string> = { Accept: 'application/json', ...extra }
  if (hasBody) headers['Content-Type'] = 'application/json'
  if (method !== 'GET' && csrfToken) headers['X-CSRF-Token'] = csrfToken
  return headers
}

export async function toApiError(res: Response): Promise<ApiError> {
  let body: ApiErrorBody | undefined
  try {
    body = (await res.json()) as ApiErrorBody
  } catch {
    body = undefined
  }
  const err = body?.error
  return new ApiError(
    res.status,
    err?.code ?? (res.status >= 500 ? 'internal' : 'bad_request'),
    err?.message ?? (res.statusText || `HTTP ${res.status}`),
    err?.details,
  )
}

export async function request<T>(method: Method, path: string, body?: unknown, opts: RequestOptions = {}): Promise<T> {
  let res: Response
  try {
    res = await fetch(API_BASE + path, {
      method,
      headers: buildHeaders(method, body !== undefined, opts.headers),
      body: body === undefined ? undefined : JSON.stringify(body),
      credentials: 'same-origin',
      signal: opts.signal,
    })
  } catch (e) {
    if (e instanceof DOMException && e.name === 'AbortError') throw e
    throw networkError()
  }

  if (!res.ok) {
    const err = await toApiError(res)
    if (res.status === 401 && !opts.quiet401) notifyUnauthorized()
    throw err
  }
  if (res.status === 204) return undefined as T
  const text = await res.text()
  return (text ? JSON.parse(text) : undefined) as T
}

export const api = {
  get: <T>(path: string, opts?: RequestOptions) => request<T>('GET', path, undefined, opts),
  post: <T>(path: string, body?: unknown, opts?: RequestOptions) => request<T>('POST', path, body ?? {}, opts),
  put: <T>(path: string, body?: unknown, opts?: RequestOptions) => request<T>('PUT', path, body ?? {}, opts),
  patch: <T>(path: string, body?: unknown, opts?: RequestOptions) => request<T>('PATCH', path, body ?? {}, opts),
  del: <T = void>(path: string, body?: unknown, opts?: RequestOptions) => request<T>('DELETE', path, body, opts),
}

/** Path segment encoder — slugs and ids are safe, but user text never is. */
export const seg = (s: string) => encodeURIComponent(s)
