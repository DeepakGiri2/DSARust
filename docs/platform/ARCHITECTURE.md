# DSA Visualized cloud platform — architecture

The desktop app (`crates/`, `content/`) is a single-user, offline program. The
cloud platform is the same product as a multi-user web service: accounts,
progress that follows you between devices, code execution without a local
toolchain, hosted AI assist, and a subscription tier. It is built **beside**
the desktop app, not on top of it, and shares exactly the parts that must never
drift.

```
                         ┌──────────────────────── CloudFront ─────────────────────────┐
   browser (React SPA) ─▶│ /*                   → S3 (SPA; index.html for client routes) │
                         │ /api/v1/content/*    → ALB (cached: catalog, problems, traces)│
                         │ /api/*               → ALB (never cached; cookies forwarded)  │
                         └───────────────────────────────┬──────────────────────────────┘
                                                         │ X-Origin-Verify
                                       WAF ─▶ ALB ─▶ dsa-api  (ECS Fargate, 2–20 tasks)
                                                         │
                ┌──────────────────────┬─────────────────┼──────────────────┬───────────────┐
                ▼                      ▼                 ▼                  ▼               ▼
        RDS PostgreSQL          ElastiCache Valkey   Lambda: dsa-runner   Bedrock/Claude   SES / Stripe
        (isolated subnets)      (rate limits)        (isolated, NO egress) (AI assist)     (email, billing)
```

## What is shared with the desktop, and what is not

| Desktop crate | Cloud use |
| --- | --- |
| `dsa-core` | **Shared as-is.** The trace model, the practice starter/assembler, the harness synthesizer, input validation. The web client mirrors its serde output in `web/src/trace/types.ts`. |
| `dsa-content` | **Shared as-is.** `Library` loads `content/` and runs the Rhai animations — the server records every trace with the desktop's own engine, so all 287 animations behave identically. |
| `dsa-harness` | `serialize_input` (the stdin contract) and `outputs_match` (the comparison rule). Its process runner is used only in the development-only `RUNNER_MODE=local`. |
| `dsa-ai` | The three modes' prompts and their assembly rules; the server fills them from the same problem brief. The Ollama client is replaced by async providers. |
| `dsa-store` | **Not used.** Its semantics (never-downgrade status, "to do" = no row, per-profile playlists) were moved to PostgreSQL in `backend/crates/api/migrations/0001_init.sql`. |
| `dsa-viz`, `dsa-app` | **Ported, not linked.** The renderers were ported to Canvas 2D (`web/src/viz`), the debugger timeline to TypeScript (`web/src/debugger`), and each screen to React. |

The backend is its own Cargo workspace (`backend/`) with its own lockfile, so
the service can upgrade dependencies without touching the desktop build and
vice versa. It depends on the desktop crates by path.

## Services

### `dsa-api` — stateless HTTP API (`backend/crates/api`)

axum on tokio. Every task is interchangeable: sessions, progress and
everything else live in PostgreSQL; fleet-wide rate limits live in Redis;
content is immutable and baked into the image. Adding tasks behind the load
balancer is the scaling model (CPU and request-count target tracking).

Request path: sensitive headers masked → request id → tracing → panic guard →
compression → body limit → security headers (`no-store` by default) →
response timeout → **origin guard** (CloudFront secret; cross-origin writes
refused) → per-route metrics → route. Authentication and CSRF are one
extractor (`Authed`): resolving the session cookie on a state-changing method
also requires the session-bound `X-CSRF-Token`, so no route can forget it.

| Concern | Where |
| --- | --- |
| Config (env only, strict) | `config.rs` |
| Errors → `{error:{code,message,details}}` | `error.rs` |
| Sessions, CSRF, client IP | `extract.rs`, `security.rs` |
| SQL (one file per aggregate) | `store/` |
| Content payloads, content version | `content.rs` |
| Trace recording + cache | `traces.rs` |
| Runner client (http, lambda, local) | `runner.rs` |
| AI providers (Claude API, Bedrock, Ollama) | `ai.rs` |
| Rate limiting (Redis / memory) | `ratelimit.rs` |
| Email (SES / SMTP / log) | `mail.rs` |
| Routes | `routes/` |

### `dsa-runner` — sandboxed code execution (`backend/crates/runner`)

Receives an `ExecuteRequest` (language id, complete program, stdin per case,
limits), compiles once, runs each case in a fresh directory, returns outputs.
In AWS it is a Lambda container function in isolated subnets whose security
group allows no egress: user code has no network route anywhere, each
concurrent execution is its own microVM, and Lambda absorbs bursts without a
queue we operate. Locally the same image runs as an HTTP service. The sandbox
layers (per-job uid, rlimits, seccomp, scrubbed environment, process sweep)
are documented in the runner's README.

The contract (`backend/crates/protocol`) is designed so the runner is told as
little as possible: it gets a language **id** (never a command line) and the
**stdin** of each case (never the expected output). The API compares outputs
itself, so code under test cannot read the answers.

### `web` — React SPA (`web/`)

React 19, TypeScript (strict), Vite, TanStack Query, CodeMirror 6. Every
page is a lazily loaded route. The API contract is `web/src/api/types.ts`;
every call goes through one hook per endpoint (`api/hooks.ts`) with
optimistic updates for ticks, stars and playlist membership.

## Data model

```
users ─┬─ oauth_identities
       ├─ sessions            (token stored as SHA-256; revocable, sliding, 90-day cap)
       ├─ auth_tokens         (verify email / reset password; single-use)
       ├─ subscriptions       (Stripe; users.plan is derived from these)
       ├─ ai_usage            (daily quota, atomic reservation)
       └─ profiles ─┬─ progress        (one row per touched problem)
                    ├─ playlists ── playlist_items
                    ├─ drafts          (editor contents per problem × language)
                    ├─ submissions     (every Run and test sweep)
                    └─ activity_days   (local-day roll-up: streaks, heatmap)
```

Ids are UUIDv7 (time-ordered: inserts append to the index, and submission
history paginates by id). Problem slugs are text, not foreign keys — content is
files in the image, and a renamed slug must never fail a write — but every
write is checked against the loaded catalogue. `submissions` is the table that
grows; its two indexes serve the only two ways it is read, and it is the one to
partition by month at scale.

## Flows

**A first visit.** `GET /api/v1/content/catalog`, `/content/problems/{slug}`
and `/content/problems/{slug}/trace` are public, identical for everyone, and
carry `Cache-Control: public` with an ETag of the content version: CloudFront
answers nearly all of them, and a browser revalidates with a 304. The default
trace of each problem is recorded once per task and cached in memory.

**Editing the input.** `POST /problems/{slug}/trace` with the editor's raw
text per field. The server parses and validates with the manifest's rules
(every problem reported at once, in the desktop's wording), then records on
the blocking pool behind a CPU-sized semaphore, with a wall-clock backstop on
top of Rhai's operation budget. Results are cached by (content version, slug,
input).

**Run / test.** `POST /profiles/{pid}/runs`: rate limit per plan →
assemble the program exactly as the desktop does → one runner call (compile
once, all cases) → compare → one transaction writing the submission, the
attempt (and a solve on an all-green sweep, never a downgrade) and the day's
activity. Limits keep the whole call under CloudFront's 60 s origin timeout.

**AI assist.** `POST /ai/chat` reserves one unit of the daily quota
atomically, builds the prompt from the desktop's templates, and streams
server-sent events (`token` / `done` / `error`) with keep-alives. A provider
failure before any output refunds the unit.

**Billing.** Checkout and the customer portal are Stripe-hosted. Only the
signature-verified, idempotent webhook changes a plan.

## Security model (summary)

* Passwords: Argon2id (19 MiB, t=2), per-IP and per-address rate limits, a
  per-account lockout, identical timing and wording for unknown accounts,
  reset links that are single-use and revoke every session.
* Sessions: 256-bit random cookie (`HttpOnly`, `Secure`, `SameSite=Lax`),
  stored hashed; CSRF token = HMAC(session) required on every write; the
  Origin/Referer of every write checked against the allow-list.
* Network: only CloudFront can reach the ALB (prefix list + secret header);
  database and cache in isolated subnets reachable only from the API; the
  runner has no egress at all.
* Least privilege: the API's task role can invoke the runner alias, send from
  its SES identity and call its Bedrock model — nothing else.
* Untrusted code: never runs in the API; runs in the runner's sandbox, inside a
  microVM with no network, with none of the answers.

## Local development

```bash
docker compose up --build                 # postgres, redis, mailpit, runner, api
cd web && npm install && npm run dev      # http://localhost:5173
```

Or run the API from source (`backend/.env.example`) with `RUNNER_MODE=local`
to execute with this machine's toolchains while iterating (no sandbox —
development only; refused in production).

Deployment is `docs/platform/DEPLOY.md`; the endpoint list is
`docs/platform/API.md`; every environment variable is in
`docs/platform/SERVICES.md`.
