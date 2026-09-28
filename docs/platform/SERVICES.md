# Services contract

What each deployable unit is, what it listens on, and what it reads from its
environment. Infrastructure (`infra/`), Docker (`backend/docker/`,
`docker-compose.yml`) and CI all code against this page.

```
                 ┌──────────────── CloudFront ────────────────┐
  browser ──────▶│  /*        → S3 (web SPA, index.html fallback)│
                 │  /api/*    → ALB → dsa-api (Fargate, N tasks) │
                 └──────────────────────────────────────────────┘
                                   │            │           │
                         RDS Postgres   ElastiCache   Lambda: dsa-runner
                         (isolated)     Redis         (isolated subnets,
                                        (isolated)     no egress at all)
```

## dsa-api — `backend/crates/api`

Stateless HTTP API. Any task can serve any request; scale horizontally.

| | |
| --- | --- |
| Image | `backend/docker/api.Dockerfile` (build context: repo root) |
| Command | `dsa-api serve` (default). `dsa-api migrate` runs migrations and exits (one-off ECS task per deploy). |
| Ports | `8080` HTTP API, `9090` Prometheus metrics (never public) |
| Health | `GET /healthz` liveness, `GET /readyz` readiness (DB + content) |
| Content | Baked into the image at `/app/content` (the repo's `content/`) |
| Runs as | non-root user `dsa` (uid 10001) |

| Variable | Default | Meaning |
| --- | --- | --- |
| `DSA_ENV` | `development` | `production` turns on secure cookies, JSON logs, stricter checks |
| `DSA_BIND` | `0.0.0.0:8080` | API listen address |
| `DSA_METRICS_BIND` | `0.0.0.0:9090` | metrics listen address (empty = off) |
| `DSA_PUBLIC_URL` | `http://localhost:5173` | The SPA origin; used for links in emails and OAuth redirects |
| `DSA_ALLOWED_ORIGINS` | = `DSA_PUBLIC_URL` | Comma-separated origins allowed on non-GET requests |
| `DSA_CONTENT_DIR` | `./content` or `/app/content` | Content root |
| `DATABASE_URL` | — | `postgres://user:pass@host:5432/db?sslmode=require`. **Or** the parts below. |
| `DB_HOST` `DB_PORT` `DB_NAME` `DB_USER` `DB_PASSWORD` | — | Used when `DATABASE_URL` is unset (ECS injects the RDS secret's keys individually) |
| `DB_SSLMODE` | `prefer` (`require` in production) | |
| `DB_MAX_CONNECTIONS` | `20` | Pool size per task. Tasks × this must stay under the instance's `max_connections` (or put RDS Proxy in front). |
| `REDIS_URL` | — | `redis://…` or `rediss://…` (ElastiCache TLS). Unset = in-process rate limits (single task only). |
| `REDIS_AUTH_TOKEN` | — | ElastiCache AUTH token, applied as the password of `REDIS_URL` (so the URL itself carries no secret) |
| `REDIS_HOST` `REDIS_PORT` `REDIS_TLS` | — / `6379` / `true` | Alternative to `REDIS_URL` |
| `COOKIE_SECURE` | `true` in production or on https | `Secure` on the session cookie; refused as `false` in production |
| `SESSION_SECRET` | — (required in production) | ≥ 32 random bytes, base64 or hex. HMAC key for CSRF tokens and signed OAuth state. |
| `SESSION_TTL_DAYS` | `30` | Sliding session lifetime |
| `RUN_MIGRATIONS` | `false` | Migrate on startup (dev/compose). Production uses `dsa-api migrate`. |
| `TRUSTED_PROXY_HOPS` | `0` | How many `X-Forwarded-For` hops to trust for the client IP. CloudFront + ALB = `2`. |
| `ORIGIN_VERIFY_SECRET` | — | If set, requests must carry `X-Origin-Verify: <secret>` (CloudFront adds it as an origin custom header) — blocks direct-to-ALB traffic. Health checks are exempt. |
| `RUNNER_MODE` | `disabled` | `http` \| `lambda` \| `disabled` |
| `RUNNER_URL` | — | `http` mode: e.g. `http://runner:8081` |
| `RUNNER_TOKEN` | — | Bearer token sent to an `http` runner |
| `RUNNER_LAMBDA_FUNCTION` | — | `lambda` mode: function name or ARN (uses the task role) |
| `RUNNER_CONCURRENCY` | `64` | Max in-flight runner calls per API task |
| `RUNNER_LOCAL_ALLOW_REMOTE` | — | `local` mode only: `1` lets the desktop harness fall back to Compiler Explorer for languages with no local toolchain |
| `MAIL_MODE` | `log` | `log` \| `smtp` \| `ses` |
| `MAIL_FROM` | `DSA Visualized <no-reply@localhost>` | |
| `SMTP_URL` | — | `smtp://user:pass@host:587` (`smtps://` for implicit TLS); `smtp://mailpit:1025` locally |
| `AI_PROVIDER` | `none` | `none` \| `ollama` \| `anthropic` \| `bedrock` |
| `AI_MODEL` | `claude-opus-5` (anthropic), `anthropic.claude-opus-5` (bedrock), `gemma3:4b` (ollama) | Model id (Bedrock: the model or inference-profile id) |
| `AI_EFFORT` | API default | Claude `output_config.effort` (`low` … `max`) — the cost/quality dial; measure before lowering |
| `OLLAMA_URL` | `http://localhost:11434` | |
| `ANTHROPIC_API_KEY` | — | |
| `ANTHROPIC_BASE_URL` | `https://api.anthropic.com` | |
| `PRICE_PRO_MONTHLY_USD` / `PRICE_PRO_YEARLY_USD` | `15` / `120` | Display prices on `/meta` (the charged price is the Stripe price) |
| `PROFILES_PER_ACCOUNT` | `5` | |
| `AI_FREE_DAILY` / `AI_PRO_DAILY` | `20` / `300` | Assistant requests per user per day |
| `RUNS_FREE_PER_MIN` / `RUNS_PRO_PER_MIN` | `10` / `30` | Code runs per user per minute |
| `PREMIUM_TIERS` | *(empty)* | Comma-separated tiers that need Pro, e.g. `extra` |
| `REQUIRE_VERIFIED_EMAIL` | `false` | Running code needs a verified address |
| `SIGNUP_ENABLED` | `true` | |
| `ADMIN_EMAILS` | — | Comma-separated; these accounts are promoted to `admin` at sign-in |
| `OAUTH_GITHUB_CLIENT_ID` / `_SECRET` | — | GitHub sign-in (enabled when both set) |
| `OAUTH_GOOGLE_CLIENT_ID` / `_SECRET` | — | Google sign-in (enabled when both set) |
| `STRIPE_SECRET_KEY` | — | Billing (enabled when set, with the three below) |
| `STRIPE_WEBHOOK_SECRET` | — | |
| `STRIPE_PRICE_MONTHLY` / `STRIPE_PRICE_YEARLY` | — | Price ids |
| `AWS_REGION` | — | For `lambda`, `ses`, `bedrock` (task role credentials) |
| `LOG_FORMAT` | `pretty` (`json` in production) | |
| `RUST_LOG` | `info,dsa_api=debug` | |

IAM (task role): `lambda:InvokeFunction` on the runner function;
`ses:SendEmail` on the configured identity; `bedrock:InvokeModel*` on the
configured model; nothing else. Secrets (`SESSION_SECRET`, DB password, Stripe,
OAuth, Anthropic) arrive as ECS secrets from Secrets Manager.

## dsa-runner — `backend/crates/runner`

Executes untrusted code. Holds no secrets, no database access, and in AWS no
network route anywhere.

| | |
| --- | --- |
| Image | `backend/docker/runner.Dockerfile` (build context: repo root). One image, two modes. |
| Modes | `lambda` when `AWS_LAMBDA_RUNTIME_API` is set (AWS); `http` otherwise (compose, ECS, local) |
| Port | `8081` (`http` mode) |
| Endpoints | `POST /v1/execute` (`ExecuteRequest` → `ExecuteResponse`, see `backend/crates/protocol`), `GET /healthz` (`RunnerHealth`) |
| Lambda | payload = `ExecuteRequest`, response = `ExecuteResponse`. Timeout ≥ 60 s, memory ≥ 2048 MB, x86_64, VPC isolated subnets with a security group that allows **no egress**, role with CloudWatch Logs + VPC ENI permissions only. |

| Variable | Default | Meaning |
| --- | --- | --- |
| `RUNNER_BIND` | `0.0.0.0:8081` | `http` mode listen address |
| `RUNNER_TOKEN` | — | If set, `http` mode requires `Authorization: Bearer <token>` |
| `RUNNER_MAX_CONCURRENCY` | CPU count | Simultaneous jobs (`http` mode); excess get `busy` |
| `RUNNER_WORK_DIR` | `/tmp/dsa-runner` | Scratch root (per-job subdirectories, wiped after) |
| `RUNNER_MAX_RUN_TIMEOUT_MS` | `10000` | Ceiling for `Limits.run_timeout_ms` |
| `RUNNER_MAX_COMPILE_TIMEOUT_MS` | `30000` | Ceiling for `Limits.compile_timeout_ms` |
| `RUNNER_MAX_MEMORY_MB` | `512` | Ceiling for `Limits.memory_mb` |
| `LOG_FORMAT` / `RUST_LOG` | as above | |

## web — `web/`

Static SPA. `npm ci && npm run build` → `web/dist`, synced to the S3 bucket.
`index.html` must be served for every non-asset path (client-side routing) and
must not be cached; `assets/*` are content-hashed and cache forever.

## Local stack — `docker-compose.yml` (repo root)

`postgres:17`, `redis:7`, `mailpit` (SMTP on 1025, UI on 8025), `runner`
(http mode), `api` (migrations on startup) and the Vite dev server on the host
(`cd web && npm run dev`, proxying `/api` to `localhost:8080`).
