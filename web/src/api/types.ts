// The REST contract between the web client and `dsa-api`.
//
// This file is the source of truth for JSON shapes on the wire. The Rust DTOs
// in backend/crates/api/src/dto.rs serialize to exactly these; when one side
// changes, the other changes in the same commit. docs/platform/API.md lists
// the endpoints that carry them.
//
// Conventions:
//  * snake_case keys, RFC 3339 timestamps, UUIDv7 string ids.
//  * `T | null` means the key is always present; `key?: T` means it may be
//    omitted.
//  * Errors are `{ "error": { code, message, details? } }` with a 4xx/5xx
//    status; see `ApiErrorBody` and `ErrorCode`.

import type { Trace } from '@/trace/types'

export type Uuid = string
/** RFC 3339, UTC. */
export type Timestamp = string

export type Difficulty = 'Easy' | 'Medium' | 'Hard'
/** Smallest problem list containing a problem. `extra` sits outside the roadmap. */
export type Tier = '50' | '150' | '250' | 'extra'
/** Ordered worst-to-best; a status never downgrades except by an explicit set. */
export type ProgressStatus = 'todo' | 'attempted' | 'solved'
export type Plan = 'free' | 'pro'
export type Role = 'user' | 'admin'

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

export type ErrorCode =
  | 'bad_request'
  | 'validation' // 422 — details.fields: Record<field, message>, or details.errors: string[]
  | 'unauthorized' // 401 — no or expired session
  | 'forbidden' // 403 — signed in, not allowed
  | 'csrf' // 403 — missing/invalid X-CSRF-Token
  | 'not_found'
  | 'conflict' // 409 — e.g. duplicate profile or playlist name
  | 'payment_required' // 402 — premium content or plan limit
  | 'rate_limited' // 429 — details.retry_after_secs
  | 'email_unverified' // 403 — action requires a verified email
  | 'account_locked' // 423 — too many failed logins; details.retry_after_secs
  | 'unavailable' // 503 — a dependency (runner, AI provider) is down or busy
  | 'internal'

export interface ApiErrorBody {
  error: {
    code: ErrorCode
    message: string
    details?: Record<string, unknown>
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Meta — GET /api/v1/meta
// ─────────────────────────────────────────────────────────────────────────────

export interface PlanInfo {
  id: Plan
  name: string
  /** Display prices in USD; absent when billing is disabled. */
  price_monthly?: number
  price_yearly?: number
  features: string[]
}

export interface Meta {
  version: string
  content_version: string
  features: {
    signup: boolean
    /** Running code needs a verified email address. */
    email_verification_required: boolean
    oauth: { github: boolean; google: boolean }
    billing: boolean
    ai: { enabled: boolean; provider: string; model: string }
    /** Code execution is configured. */
    runner: boolean
  }
  plans: PlanInfo[]
  limits: {
    profiles_per_account: number
    /** Max characters per editor draft. */
    draft_bytes: number
  }
  /** Profile faces and card colours (same sets as the desktop app). */
  avatars: string[]
  colors: string[]
}

// ─────────────────────────────────────────────────────────────────────────────
// Auth & account
// ─────────────────────────────────────────────────────────────────────────────

export interface User {
  id: Uuid
  email: string
  email_verified: boolean
  display_name: string
  role: Role
  plan: Plan
  plan_renews_at: Timestamp | null
  /** IANA zone; streaks count local days. */
  timezone: string
  has_password: boolean
  oauth_providers: string[]
  created_at: Timestamp
}

export interface Entitlements {
  plan: Plan
  /** May open problems in premium tiers. */
  premium_content: boolean
  ai_daily_limit: number
  ai_used_today: number
  runs_per_minute: number
}

/** GET /auth/session, and the body of successful login/signup/oauth. */
export interface SessionInfo {
  user: User
  /** Send as `X-CSRF-Token` on every non-GET request. Bound to this session. */
  csrf_token: string
  entitlements: Entitlements
  profiles: Profile[]
}

export interface SignupRequest {
  email: string
  password: string
  display_name: string
  /** IANA zone from `Intl.DateTimeFormat().resolvedOptions().timeZone`. */
  timezone?: string
}

export interface LoginRequest {
  email: string
  password: string
}

export interface VerifyEmailRequest {
  token: string
}

export interface ForgotPasswordRequest {
  email: string
}

export interface ResetPasswordRequest {
  token: string
  password: string
}

export interface ChangePasswordRequest {
  current_password: string
  new_password: string
}

export interface UpdateMeRequest {
  display_name?: string
  timezone?: string
}

export interface DeleteAccountRequest {
  /** Required when the account has a password. */
  password?: string
  /** Must equal the account email — the "type it to confirm" guard. */
  confirm_email: string
}

export interface SessionRow {
  id: Uuid
  created_at: Timestamp
  last_seen_at: Timestamp
  expires_at: Timestamp
  user_agent: string | null
  ip: string | null
  current: boolean
}

// ─────────────────────────────────────────────────────────────────────────────
// Profiles — the desktop's "Who's practising?", per account
// ─────────────────────────────────────────────────────────────────────────────

export interface ProfileStats {
  solved: number
  attempted: number
  favourites: number
}

export interface Profile {
  id: Uuid
  name: string
  /** One glyph from `Meta.avatars`. */
  avatar: string
  /** `#rrggbb` from `Meta.colors`. */
  color: string
  created_at: Timestamp
  last_seen_at: Timestamp
  stats: ProfileStats
}

export interface ProfileInput {
  name: string
  avatar: string
  color: string
}

export type StatusFilter = 'all' | 'todo' | 'attempted' | 'solved'

/**
 * The desktop's `Settings`, per profile, synced across devices. Every field is
 * optional: the server stores whatever subset the client has written and the
 * client fills gaps with `DEFAULT_SETTINGS`.
 */
export interface ProfileSettings {
  lang?: string
  tier?: Tier
  speed?: number
  animate?: boolean
  show_logs?: boolean
  viz_only?: boolean
  status_filter?: StatusFilter
  favourites_only?: boolean
  playlist?: Uuid | null
  show_question?: boolean
  ai_open?: boolean
  /** Watch pins, namespaced `slug::var` exactly like the desktop. */
  watched?: string[]
  backdrop?: boolean
  theme?: 'dark' | 'light'
}

// ─────────────────────────────────────────────────────────────────────────────
// Progress, favourites, playlists
// ─────────────────────────────────────────────────────────────────────────────

export interface ProgressEntry {
  status: ProgressStatus
  favourite: boolean
  /** How many times Run or the tests were pressed. */
  attempts: number
  solved_at: Timestamp | null
  updated_at: Timestamp
}

/** GET /profiles/:pid/progress — everything in one query, like the desktop. */
export interface ProgressSnapshot {
  /** Only touched problems appear; absence means `todo`. */
  entries: Record<string, ProgressEntry>
  stats: ProfileStats
}

export interface SetStatusRequest {
  status: ProgressStatus
}

export interface SetFavouriteRequest {
  favourite: boolean
}

export interface Playlist {
  id: Uuid
  name: string
  slugs: string[]
  created_at: Timestamp
}

export interface CreatePlaylistRequest {
  name: string
  slugs?: string[]
}

export interface RenamePlaylistRequest {
  name: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Content — public and CDN-cacheable under /api/v1/content/*
// ─────────────────────────────────────────────────────────────────────────────

export interface Language {
  id: string
  label: string
  ext: string
  /** Highlighter hint: `go`, `cpp`, `java`, `python`. */
  syntax: string
  /** Line-comment prefix that introduces a `@tag` marker. */
  comment: string
  order: number
  /** The platform can compile and run it. */
  runnable: boolean
}

export interface CatalogProblem {
  slug: string
  title: string
  category: string
  difficulty: Difficulty
  tier: Tier
  leetcode_url: string
  /** Has an animation (trace.rhai). */
  viz: boolean
  /** Language ids with a solution source, in manifest order. */
  langs: string[]
  /** Needs the Pro plan. */
  premium: boolean
}

export interface CatalogCategory {
  name: string
  problems: CatalogProblem[]
}

export interface TierInfo {
  id: Tier
  title: string
  /** Problems visible in this tier (it contains every smaller tier). */
  count: number
  animated: number
}

export interface Catalog {
  content_version: string
  tiers: TierInfo[]
  categories: CatalogCategory[]
  languages: Language[]
  total: number
  animated: number
}

export type InputType =
  | 'int'
  | 'float'
  | 'bool'
  | 'int-array'
  | 'string-array'
  | 'string'
  | 'tree'
  | 'grid'

/** `dsa_core::problem::InputField`, serialized as-is. */
export interface InputField {
  name: string
  label: string
  type: InputType
  min: number | null
  max: number | null
  min_len: number | null
  max_len: number | null
  charset: string | null
  sorted: boolean
  unique: boolean
  help: string | null
}

/** `dsa_core::problem::InputValue`, untagged. */
export type InputValue = boolean | number | string | InputValue[] | null
export type InputMap = Record<string, InputValue>

export interface TestCaseView {
  name: string
  input: InputMap
  /** Exactly what the program receives on stdin for this case. */
  stdin: string
  expected: string
  /** A deliberately tricky case. */
  edge: boolean
}

export interface ProblemSource {
  lang: string
  /** Marker-stripped reference solution, ready to display. */
  code: string
  /** `//@tag` → 1-based line. */
  tag_lines: Record<string, number>
  /** 1-based line (as a string key) → tag, for gutter breakpoints. */
  line_tags: Record<string, string>
  /** The editor's starting point: every function body blanked. */
  starter: string
  /**
   * The complete runnable program the solution is spliced into for "full
   * program" view and for runs in `solution` mode. Null when the pack ships no
   * harness and none could be synthesized — then `solution` mode runs the
   * user's text as the whole program.
   */
  harness: string | null
  /** True when `harness` was derived from the input schema, not hand-written. */
  synthesized: boolean
}

export interface RelatedProblem {
  slug: string
  title: string
  difficulty: Difficulty
}

export interface Problem {
  slug: string
  title: string
  category: string
  difficulty: Difficulty
  tier: Tier
  description: string
  approach: string
  complexity: string
  leetcode_url: string
  inputs: InputField[]
  default_input: InputMap
  /** Default input as editable text per field (`InputValue::to_editable`). */
  default_fields: Record<string, string>
  tests: TestCaseView[]
  hints: string[]
  related: RelatedProblem[]
  /** Guide topic ids worth reading first for this category. */
  guide_topics: string[]
  has_trace: boolean
  premium: boolean
  /**
   * Content withheld for lack of entitlement: `sources` is empty and the trace
   * endpoints answer 402. The statement is always present (and indexable).
   */
  locked: boolean
  /** In `languages.toml` order. Empty when `locked`. */
  sources: ProblemSource[]
}

/**
 * POST /problems/:slug/trace. Send raw editor text per field (`fields`) and
 * the server parses and validates it with the same rules as the desktop; or
 * send an already-typed `input`.
 */
export interface TraceRequest {
  fields?: Record<string, string>
  input?: InputMap
}

export interface TraceResponse {
  /** The parsed, validated input the trace was recorded with. */
  input: InputMap
  trace: Trace
}
// 422 with `details.errors: string[]` — every violation, phrased for a human.

// ─────────────────────────────────────────────────────────────────────────────
// Guide — the 📘 helper
// ─────────────────────────────────────────────────────────────────────────────

export interface GuideOpRow {
  op: string
  big: string
  note: string
}

export interface GuideTopic {
  id: string
  title: string
  kind: 'structure' | 'technique'
  emoji: string
  what: string[]
  complexity: GuideOpRow[]
  /** lang id → code sample. */
  syntax: Record<string, string>
  notes: string[]
}

export interface CheatRow {
  topic: string
  /** lang id → snippet. */
  code: Record<string, string>
}

export interface CheatSection {
  name: string
  rows: CheatRow[]
}

export interface Guide {
  topics: GuideTopic[]
  cheatsheet: CheatSection[]
  /** Problem category → topic ids, in reading order. */
  by_category: Record<string, string[]>
}

// ─────────────────────────────────────────────────────────────────────────────
// Code execution — POST /profiles/:pid/runs
// ─────────────────────────────────────────────────────────────────────────────

/** `run`: once with `stdin`. `test`: every test case of the problem. */
export type RunKind = 'run' | 'test'
/** `solution`: the editor holds the solution, spliced into the harness. `program`: it is the whole program. */
export type RunMode = 'solution' | 'program'

export interface RunRequest {
  slug: string
  lang: string
  code: string
  mode: RunMode
  kind: RunKind
  /** For `run`; defaults to the problem's default input. */
  stdin?: string
}

/** Mirrors `dsa_harness::TestStatus::label()`. */
export type TestStatus = 'pass' | 'fail' | 'build' | 'crash' | 'timeout' | 'error'

export interface TestResult {
  name: string
  status: TestStatus
  stdin: string
  expected: string
  actual: string
  /** Compiler output, stderr, or the harness error — whatever explains it. */
  detail: string
  duration_ms: number
  edge: boolean
}

export type RunStatus =
  | 'ok' // run: exited 0
  | 'passed' // test: every case passed
  | 'failed' // test: some case failed
  | 'compile_error'
  | 'runtime_error'
  | 'timeout'
  | 'error' // the platform failed, not the program

export interface CompileInfo {
  ok: boolean
  output: string
  duration_ms: number
}

export interface RunResult {
  id: Uuid
  slug: string
  lang: string
  kind: RunKind
  mode: RunMode
  status: RunStatus
  /** Null for languages with no compile step. */
  compile: CompileInfo | null
  /** kind = run */
  stdout?: string
  stderr?: string
  exit_code?: number | null
  timed_out?: boolean
  /** kind = test */
  tests?: TestResult[]
  passed: number
  total: number
  duration_ms: number
  /** This problem's progress after the run (attempts++, maybe solved). */
  progress: ProgressEntry
  created_at: Timestamp
}

export interface SubmissionSummary {
  id: Uuid
  slug: string
  lang: string
  kind: RunKind
  mode: RunMode
  status: RunStatus
  passed: number
  total: number
  duration_ms: number
  created_at: Timestamp
}

export interface Submission extends SubmissionSummary {
  code: string
  result: RunResult
}

export interface Page<T> {
  items: T[]
  next_cursor: string | null
}

// ─────────────────────────────────────────────────────────────────────────────
// Drafts — the editor, autosaved per problem and language
// ─────────────────────────────────────────────────────────────────────────────

export interface Draft {
  lang: string
  code: string
  updated_at: Timestamp
}

/** lang id → draft */
export type DraftMap = Record<string, Draft>

export interface SaveDraftRequest {
  code: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Stats — GET /profiles/:pid/stats
// ─────────────────────────────────────────────────────────────────────────────

export interface Stats {
  totals: ProfileStats & { submissions: number }
  /** Cumulative: a tier counts every problem in it and in smaller tiers. */
  by_tier: { tier: Tier; title: string; solved: number; total: number }[]
  by_category: { category: string; solved: number; attempted: number; total: number }[]
  by_difficulty: { difficulty: Difficulty; solved: number; total: number }[]
  streak: {
    current: number
    longest: number
    /** Something was done today (local time). */
    active_today: boolean
  }
  /** Sparse, oldest first, last 365 local days. */
  activity: { day: string; runs: number; solved: number }[]
  recent: SubmissionSummary[]
}

// ─────────────────────────────────────────────────────────────────────────────
// AI assist — POST /ai/chat streams text/event-stream
// ─────────────────────────────────────────────────────────────────────────────

export type AiMode = 'interview' | 'guide' | 'fix'

export interface AiMessage {
  role: 'user' | 'assistant'
  content: string
}

export interface AiChatRequest {
  slug: string
  lang: string
  mode: AiMode
  /** The conversation so far, oldest first, ending with the new user turn. */
  messages: AiMessage[]
  /** The editor's contents — sent only when the user ticked "attach code". */
  code?: string
  /** Latest run/test output, so Fix mode can see the errors. */
  run_context?: string
}

/**
 * Server-sent events, in order:
 *   event: token  data: AiTokenEvent     (many)
 *   event: done   data: AiDoneEvent      (once, on success)
 *   event: error  data: ApiErrorBody['error']  (once, instead of done)
 */
export interface AiTokenEvent {
  channel: 'content' | 'thinking'
  text: string
}

export interface AiDoneEvent {
  input_tokens: number
  output_tokens: number
  remaining_today: number
}

export interface AiStatus {
  enabled: boolean
  provider: string
  model: string
  daily_limit: number
  used_today: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Billing
// ─────────────────────────────────────────────────────────────────────────────

export interface CheckoutRequest {
  interval: 'month' | 'year'
}

export interface RedirectUrl {
  url: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Desktop import — POST /profiles/:pid/import
// ─────────────────────────────────────────────────────────────────────────────

/** A desktop profile's progress, as exported by the desktop app. */
export interface ImportRequest {
  entries: Record<string, { status: ProgressStatus; favourite: boolean; attempts: number }>
  playlists: { name: string; slugs: string[] }[]
}

export interface ImportResult {
  progress_rows: number
  playlists: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin
// ─────────────────────────────────────────────────────────────────────────────

export interface AdminOverview {
  users: number
  users_pro: number
  signups_7d: number
  runs_24h: number
  ai_requests_24h: number
  content_errors: string[]
}
