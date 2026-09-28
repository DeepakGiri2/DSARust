# DSA Visualized API — v1

Base path `/api/v1`. JSON in and out, snake_case keys, RFC 3339 timestamps,
UUIDv7 ids. **Every request and response shape is defined in
[`web/src/api/types.ts`](../../web/src/api/types.ts)**, which is the contract;
this page lists the endpoints that carry them.

## Conventions

| Topic | Rule |
| --- | --- |
| Auth | Server-side session. Cookie `dsa_session` (HttpOnly, Secure, SameSite=Lax, Path=/). |
| CSRF | Every non-GET request with a session must send `X-CSRF-Token: <SessionInfo.csrf_token>`. The token is an HMAC of the session, so it cannot be forged or replayed across sessions. Missing/invalid → `403 csrf`. |
| Origin | Non-GET requests carrying an `Origin` (or `Referer`) header must match an allowed origin → otherwise `403 forbidden`. |
| Errors | `{"error":{"code","message","details?"}}`. Codes: see `ErrorCode` in types.ts. `422 validation` carries `details.fields` (forms) or `details.errors` (input sets). `429 rate_limited` and `423 account_locked` carry `details.retry_after_secs`. |
| Caching | `/content/*` is public and CDN-cacheable (`Cache-Control: public`, strong `ETag` = content version). Everything else is `Cache-Control: no-store`. |
| Pagination | Cursor based: `?limit=20&cursor=…` → `Page<T>` `{ items, next_cursor }`. |
| Profiles | `:pid` must belong to the session's account, otherwise `404 not_found` (never `403`, so ids cannot be probed). |

## Health (outside `/api`)

| Method | Path | Notes |
| --- | --- | --- |
| GET | `/healthz` | Liveness. 200 while the process runs. |
| GET | `/readyz` | Readiness. 200 when Postgres answers and content is loaded; 503 otherwise. |
| GET | `:9090/metrics` | Prometheus text format, on a separate port that is never routed publicly. |

## Meta and content (public)

| Method | Path | Body → Response |
| --- | --- | --- |
| GET | `/meta` | → `Meta` |
| GET | `/content/catalog` | → `Catalog` |
| GET | `/content/guide` | → `Guide` |
| GET | `/content/problems/:slug` | → `Problem` (premium problems: `locked: true`, no sources) |
| GET | `/content/problems/:slug/trace` | → `TraceResponse` for the default input; `402` if premium |
| GET | `/problems/:slug` | → `Problem`, entitlement-aware (sources for premium when on Pro). Not CDN-cached. |
| POST | `/problems/:slug/trace` | `TraceRequest` → `TraceResponse`; `422` with `details.errors`. Auth optional, rate-limited per user/IP, `402` if premium and not entitled. |

## Auth

| Method | Path | Body → Response |
| --- | --- | --- |
| POST | `/auth/signup` | `SignupRequest` → `201 SessionInfo` + cookie. Creates the first profile (named after `display_name`) and sends a verification email. `409 conflict` if the email is taken. |
| POST | `/auth/login` | `LoginRequest` → `SessionInfo` + cookie. `401` bad credentials (generic message), `423 account_locked`, `429`. |
| POST | `/auth/logout` | → `204`, revokes the session, clears the cookie. |
| GET | `/auth/session` | → `SessionInfo` or `401`. The SPA calls this on boot. |
| POST | `/auth/verify-email` | `VerifyEmailRequest` → `204` |
| POST | `/auth/verify-email/resend` | → `202` (signed in) |
| POST | `/auth/password/forgot` | `ForgotPasswordRequest` → `202` always (no account enumeration) |
| POST | `/auth/password/reset` | `ResetPasswordRequest` → `204`; revokes every session of the account |
| POST | `/auth/password/change` | `ChangePasswordRequest` → `204`; revokes every *other* session |
| GET | `/auth/oauth/:provider/start?next=/path` | → `302` to GitHub/Google. `provider` ∈ `github`, `google`. |
| GET | `/auth/oauth/:provider/callback` | → `302` to `next` (or `/login?error=…`), session cookie set |
| GET | `/auth/sessions` | → `SessionRow[]` |
| DELETE | `/auth/sessions/:id` | → `204` |
| POST | `/auth/sessions/revoke-others` | → `204` |

## Account

| Method | Path | Body → Response |
| --- | --- | --- |
| GET | `/me` | → `User` |
| PATCH | `/me` | `UpdateMeRequest` → `User` |
| DELETE | `/me` | `DeleteAccountRequest` → `204`. Cascades everything; cancels billing. |
| GET | `/me/export` | → JSON download of every row the account owns (GDPR) |

## Profiles

| Method | Path | Body → Response |
| --- | --- | --- |
| GET | `/profiles` | → `Profile[]` (most recently used first) |
| POST | `/profiles` | `ProfileInput` → `201 Profile`. `409` duplicate name, `422` over the per-account limit. |
| PATCH | `/profiles/:pid` | `Partial<ProfileInput>` → `Profile` |
| DELETE | `/profiles/:pid` | → `204`. Refuses (`409`) to delete the last profile. |
| GET | `/profiles/:pid/settings` | → `ProfileSettings` |
| PUT | `/profiles/:pid/settings` | `ProfileSettings` (merge-patch) → `ProfileSettings` |

## Progress, favourites, playlists

| Method | Path | Body → Response |
| --- | --- | --- |
| GET | `/profiles/:pid/progress` | → `ProgressSnapshot` |
| PUT | `/profiles/:pid/progress/:slug/status` | `SetStatusRequest` → `ProgressEntry` (forces, including back to `todo`) |
| PUT | `/profiles/:pid/progress/:slug/favourite` | `SetFavouriteRequest` → `ProgressEntry` |
| GET | `/profiles/:pid/stats` | → `Stats` |
| POST | `/profiles/:pid/import` | `ImportRequest` → `ImportResult` (merges desktop progress; never downgrades) |
| GET | `/profiles/:pid/playlists` | → `Playlist[]` (by name) |
| POST | `/profiles/:pid/playlists` | `CreatePlaylistRequest` → `201 Playlist` |
| PATCH | `/profiles/:pid/playlists/:id` | `RenamePlaylistRequest` → `Playlist` |
| DELETE | `/profiles/:pid/playlists/:id` | → `204` |
| PUT | `/profiles/:pid/playlists/:id/items/:slug` | → `204` (idempotent) |
| DELETE | `/profiles/:pid/playlists/:id/items/:slug` | → `204` (idempotent) |

Unknown slugs are rejected with `404` on writes, so rows can only ever name
problems that exist.

## Drafts

| Method | Path | Body → Response |
| --- | --- | --- |
| GET | `/profiles/:pid/drafts/:slug` | → `DraftMap` |
| PUT | `/profiles/:pid/drafts/:slug/:lang` | `SaveDraftRequest` → `204` (≤ `Meta.limits.draft_bytes`) |
| DELETE | `/profiles/:pid/drafts/:slug/:lang` | → `204` ("reset code") |

## Runs

| Method | Path | Body → Response |
| --- | --- | --- |
| POST | `/profiles/:pid/runs` | `RunRequest` → `RunResult`. Synchronous (typically 1–4 s). Records an attempt; a `test` run where every case passes marks the problem solved. `429` over the plan's per-minute limit; `503 unavailable` when the runner fleet is saturated. |
| GET | `/profiles/:pid/submissions?slug=&limit=&cursor=` | → `Page<SubmissionSummary>` (newest first) |
| GET | `/profiles/:pid/submissions/:id` | → `Submission` |

## AI assist

| Method | Path | Body → Response |
| --- | --- | --- |
| GET | `/ai/status` | → `AiStatus` |
| POST | `/ai/chat` | `AiChatRequest` → `text/event-stream`: `token` × n, then `done` or `error`. Counts against the daily quota. |

## Billing (enabled when Stripe is configured)

| Method | Path | Body → Response |
| --- | --- | --- |
| POST | `/billing/checkout` | `CheckoutRequest` → `RedirectUrl` (Stripe Checkout) |
| POST | `/billing/portal` | → `RedirectUrl` (Stripe customer portal) |
| POST | `/billing/webhook` | Stripe events. Signature-verified; exempt from CSRF/Origin. |

## Admin (role `admin`)

| Method | Path | Body → Response |
| --- | --- | --- |
| GET | `/admin/overview` | → `AdminOverview` |
