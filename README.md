# DSA Visualized

Step through every NeetCode problem the way you step through code in a
debugger — call stack, variables, breakpoints, step in / over / out — while the
data structure it works on animates beside the source. 287 problems across 22
categories, each with Go, C++ and Java solutions, test cases and a
hand-written animation.

This repository holds two clients of one product:

| | What | Where |
| --- | --- | --- |
| **Desktop** | A native Rust (egui) app: offline, single binary, local profiles in SQLite | `crates/`, `content/` |
| **Cloud platform** | The web version: accounts, synced progress, sandboxed code execution, hosted AI assist, subscriptions — deployable on AWS | `backend/`, `web/`, `infra/` |

Both read the same `content/` and record animations with the same engine
(`crates/dsa-core`, `crates/dsa-content`), so a problem looks and steps the
same everywhere.

## Desktop

```bash
cargo run --release -p dsa-app
cargo xtask lint        # run every animation, check every tag
```

## Cloud platform

```bash
docker compose up --build                 # postgres, redis, mailpit, runner, api
cd web && npm install && npm run dev      # http://localhost:5173
```

| Part | Stack |
| --- | --- |
| `backend/crates/api` | Rust, axum, PostgreSQL (sqlx), Redis — stateless, horizontally scaled |
| `backend/crates/runner` | Rust sandbox for untrusted code; AWS Lambda container in production |
| `backend/crates/protocol` | The API ↔ runner wire contract |
| `web/` | React 19, TypeScript, Vite, TanStack Query, CodeMirror 6, Canvas 2D renderer |
| `infra/` | AWS CDK (TypeScript): VPC, RDS, ElastiCache, ECS Fargate, Lambda, CloudFront, WAF |

Docs: [architecture](docs/platform/ARCHITECTURE.md) ·
[API](docs/platform/API.md) · [services & configuration](docs/platform/SERVICES.md) ·
[deployment runbook](docs/platform/DEPLOY.md)

Tests:

```bash
cd backend && DATABASE_URL=postgres://dsa:dsa@127.0.0.1:55432/dsa cargo test --workspace
cd web && npm test && npm run typecheck
cd infra && npm test && npx cdk synth -c stage=dev
```

## License

MIT
