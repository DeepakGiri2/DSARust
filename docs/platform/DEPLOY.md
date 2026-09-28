# Deploying the cloud platform

The AWS infrastructure is a CDK v2 (TypeScript) app in [`infra/`](../../infra);
CI/CD lives in [`.github/workflows/platform-*.yml`](../../.github/workflows).
This page is the operator's runbook. The service contract the infra codes
against is [SERVICES.md](SERVICES.md).

```
                         Route 53 (optional)
                                │
 browser ── CloudFront ─────────┼───────────────────────────────────────────────────────┐
            │ edge WAF (prod)   │                                                       │
            │ /*        → S3 (private, OAC) + SPA rewrite function                      │
            │ /assets/* → S3, cached 1 year                                             │
            │ /api/v1/content/* → ALB, cached (path + query, origin Cache-Control)      │
            │ /api/*    → ALB, never cached, cookies/CSRF/Origin forwarded              │
            └── X-Origin-Verify: <secret>, x-viewer-ip: <viewer> ──┐                    │
                                                                   ▼                    │
 VPC  public    ALB (SG: CloudFront prefix list only) + regional WAF (rate limits, managed rules)
      private   dsa-api on Fargate (2–20 tasks) ──NAT──▶ Stripe, OAuth, SES, Bedrock
                  │ lambda:Invoke          │ 5432 (TLS)          │ 6379 (TLS + AUTH)
      isolated  dsa-runner Lambda       RDS PostgreSQL 17     ElastiCache Valkey
                (no ingress, no egress)  (Multi-AZ in prod)    (replica + failover in prod)
```

## Stacks

One set per stage (`dev`, `prod`), all named `Dsa<Stage>-<Name>`, deployed in
this order by `cdk deploy --all`:

| Stack | Contents |
| --- | --- |
| `Network` | VPC (2 AZs dev / 3 prod; public, private-with-egress, isolated subnets), NAT gateways (1 / one per AZ), S3 gateway endpoint, interface endpoints (prod: ECR api+dkr, Logs, Secrets Manager, Lambda), VPC flow logs (dev: rejected only, prod: all), **every security group and every rule between tiers**. |
| `Data` | KMS data key, RDS PostgreSQL 17 (gp3 + storage autoscaling, `rds.force_ssl=1`, Performance Insights where the class supports it, log exports), generated master secret, optional RDS Proxy, ElastiCache Valkey (TLS required, AUTH token, at-rest KMS, slow log). |
| `Runner` | `dsa-runner` Lambda container (x86_64, ≥ 2 GiB, 60 s, isolated subnets, no-egress SG, reserved concurrency), alias `live` (optional provisioned concurrency), log group, least-privilege role. |
| `Ingress` | Public ALB (idle 180 s, drop invalid headers, access logs in prod), API target group (`/readyz`), listener that returns 403 unless `X-Origin-Verify` matches, HTTP→HTTPS redirect with a domain, origin-verify secret, regional WAF, `origin.<domain>` alias record. |
| `Edge` | Site bucket (private, OAC), CloudFront (functions, cache/origin-request/security-header policies), optional edge WAF (prod), `A`/`AAAA` alias records, `web/dist` deployment + `/index.html` invalidation. |
| `Api` | ECS cluster (Container Insights), `dsa-api` service + migration task definitions, task role, app secrets, SES identity + configuration set, autoscaling, the migrations custom resource. |
| `Observability` | SNS alarm topic (KMS) + e-mail subscription, alarms, the `dsa-<stage>-platform` dashboard, the monthly budget. |
| `GithubOidc` | Only with `-c githubOidc=true` (never from CI): GitHub OIDC provider and the stage's deploy role. |

## Parameters

Pass with `-c key=value` (or the `CDK_EXTRA_CONTEXT` GitHub variable). All are
validated at synth time; see [`infra/lib/config.ts`](../../infra/lib/config.ts).

| Key | dev | prod | Notes |
| --- | --- | --- | --- |
| `stage` | `dev` | `prod` | Selects the defaults below. |
| `region` / `account` | `us-east-1` / — | | `account` pins the stacks to one account (recommended for prod). |
| `domainName`, `certificateArn` | — | | Custom domain; the ACM certificate must be in **us-east-1** and cover `<domain>` and `origin.<domain>`. |
| `hostedZoneId`, `hostedZoneName` | — | | Route 53 zone for alias, DKIM and MAIL FROM records. Without it, create them by hand from the stack outputs. |
| `albCertificateArn`, `originSubdomain` | = `certificateArn`, `origin` | | Needed only when the stage region is not us-east-1. |
| `azCount`, `natGateways`, `interfaceEndpoints` | 2, 1, false | 3, 3, true | |
| `dbInstanceClass` | `t4g.micro` | `m7g.large` | Synth refuses a class whose `max_connections` cannot fit `apiMaxTasks × dbMaxConnectionsPerTask` (unless `rdsProxy=true`). |
| `dbAllocatedStorageGiB` / `dbMaxAllocatedStorageGiB` | 20 / 100 | 50 / 500 | |
| `dbMultiAz`, `dbBackupRetentionDays`, `rdsProxy` | false, 7, false | true, 14, false | |
| `cacheEngine`, `cacheNodeType`, `cacheReplicas` | `valkey`, `cache.t4g.micro`, 0 | `valkey`, `cache.t4g.medium`, 1 | `cacheEngine=redis` switches to Redis OSS 7.1. |
| `apiCpu` / `apiMemoryMiB` | 512 / 1024 | 1024 / 2048 | Valid Fargate combinations only. |
| `apiMinTasks` / `apiMaxTasks` | 1 / 2 | 2 / 20 | |
| `apiCpuArchitecture`, `apiFargateSpot` | `X86_64`, true | `X86_64`, false | |
| `apiCpuTargetPercent`, `apiRequestsPerTarget` | 60, 1000 | 60, 1000 | Target tracking on CPU and ALB requests/target/minute. |
| `dbMaxConnectionsPerTask`, `runnerConcurrencyPerTask` | 10, 32 | 20, 64 | `DB_MAX_CONNECTIONS`, `RUNNER_CONCURRENCY`. |
| `runnerMemoryMiB`, `runnerEphemeralStorageMiB`, `runnerTimeoutSeconds` | 2048, 2048, 60 | 3008, 2048, 60 | |
| `runnerReservedConcurrency`, `runnerProvisionedConcurrency` | 10, 0 | 200, 0 | Reserved = cost ceiling (`none` to disable). |
| `aiProvider`, `aiModelId` | `bedrock`, `anthropic.claude-opus-5` | | `anthropic` wires `ANTHROPIC_API_KEY`; geo-prefixed ids (`us.…`) are treated as inference profiles. |
| `mailFromAddress` | — | | Without a domain: send from one SES-verified address. |
| `apiEnv` | — | | JSON of allow-listed, non-secret API variables, e.g. `{"ADMIN_EMAILS":"me@example.com"}`. |
| `apiImageUri`, `runnerImageUri` | — | | Deploy a prebuilt ECR image (`…/repo:tag` or `@sha256:…`) instead of building. |
| `wafRateLimit`, `wafAuthRateLimit` | 2000, 100 | | Requests per 5 minutes per viewer IP (all `/api`, `/api/v1/auth/`). |
| `edgeWaf`, `priceClass`, `publishSourceMaps` | false, `PriceClass_100`, true | true, `PriceClass_All`, false | |
| `albIdleTimeoutSeconds` | 180 | 180 | ≥ 120. |
| `logRetentionDays`, `alarmEmail`, `monthlyBudgetUsd`, `latencyAlarmSeconds` | 14, —, 200, 8 | 90, —, 1500, 8 | |
| `originVerifyGeneration` | 1 | 1 | Bump to rotate the origin-verify secret. |
| `githubOidc`, `githubRepo`, `githubSubject`, `githubOidcProviderArn` | false, `DeepakGiri2/DSARust`, `repo:<repo>:environment:<stage>`, — | | |
| `cloudFrontPrefixListId` | `pl-3b927c52` (us-east-1) | | Required in any other region: `aws ec2 describe-managed-prefix-lists --filters Name=prefix-list-name,Values=com.amazonaws.global.cloudfront.origin-facing`. |

## Prerequisites (once per AWS account)

1. **Tools**: Node 22+, Docker (builds both images; the runner image must be
   `linux/amd64`), AWS CLI v2, admin credentials for the bootstrap steps.
2. **Lambda concurrency quota**: new accounts can start at 10 concurrent
   executions, and a reservation must leave 100 unreserved. Request
   *Concurrent executions* ≥ 1000 in Service Quotas before the first deploy
   (or deploy with `-c runnerReservedConcurrency=none`).
3. **Certificate** (custom domain): request an ACM certificate **in us-east-1**
   for `example.com` and `*.example.com` (or `origin.example.com`), DNS-validated.
4. **SES**: new accounts are in the SES sandbox (verified recipients only,
   200 mails/day). After the first deploy has verified the domain, request
   production access (SES console → *Get set up* → *Request production
   access*). Add a DMARC record (`_dmarc.example.com TXT "v=DMARC1; p=none; rua=mailto:…"`);
   DKIM and the `bounce.example.com` MAIL FROM records are created for you
   when the hosted zone is given (otherwise copy them from the `DsaStage-Api`
   outputs).
5. **Bedrock**: enable access to the configured model in the Bedrock console
   (Model access) for the stage region.
6. **Cost allocation tags**: activate `Project`, `Stage` and `CostCenter` in
   Billing → Cost allocation tags. The budget is account-wide; use one
   account per stage (recommended) or read it as the account total.

## First-time bootstrap

```bash
cd infra
npm ci

# 1. CDK bootstrap (asset bucket, ECR repo, deploy roles). Harden prod by
#    replacing AdministratorAccess with a policy scoped to the services above.
npx cdk bootstrap aws://<ACCOUNT>/us-east-1 \
  --cloudformation-execution-policies arn:aws:iam::aws:policy/AdministratorAccess

# 2. GitHub OIDC deploy role (admin credentials, never from CI).
npx cdk deploy DsaDev-GithubOidc -c stage=dev -c githubOidc=true
#    Same account for prod? The OIDC provider is account-global — import it:
npx cdk deploy DsaProd-GithubOidc -c stage=prod -c githubOidc=true \
  -c githubOidcProviderArn=arn:aws:iam::<ACCOUNT>:oidc-provider/token.actions.githubusercontent.com
```

3. **GitHub** (repository → Settings):
   - *Environments*: create `dev` and `prod`. On `prod` add **required
     reviewers** and restrict deployment branches to `main`.
   - Per environment, set variables: `AWS_DEPLOY_ROLE_ARN` (output of the
     OIDC stack), optionally `AWS_REGION`, `AWS_ACCOUNT_ID`, `DOMAIN_NAME`,
     `CERTIFICATE_ARN`, `HOSTED_ZONE_ID`, `HOSTED_ZONE_NAME`, `ALARM_EMAIL`,
     `CDK_EXTRA_CONTEXT`.
   - *Branch protection* on `main`: require the `platform-ci` checks.
   The deploy role only trusts `repo:<owner>/<repo>:environment:<stage>`, so
   a workflow that is not running in the matching environment cannot assume it.

## Deploying

**Through CI (normal path).** A push to `main` touching `backend/`, `web/`,
`infra/`, `crates/` or `content/` runs `platform-deploy`: the CI jobs, then
`cdk deploy --all` to **dev** and a smoke test. For **prod**, run
*Actions → platform-deploy → Run workflow → stage: prod*; the job waits for
the `prod` reviewers.

Each deployment:
1. builds the SPA and both images (CDK pushes them to the bootstrap ECR repo;
   an unchanged image is not rebuilt);
2. updates the stacks in dependency order (independent ones in parallel);
3. inside the `Api` stack, **`Custom::DsaMigrations` runs `dsa-api migrate`**
   as a one-off Fargate task with the new image and waits for exit 0 — only
   then does the ECS service roll (minimum 100 % healthy, circuit breaker
   with automatic rollback). A failed migration fails the deployment and the
   old tasks keep serving; its last log lines appear in the CloudFormation
   event (`/dsa/<stage>/migrate` has the full log);
4. uploads `web/dist` (assets first, `index.html` last) and invalidates
   `/index.html`;
5. smoke-tests: all ALB targets healthy (the ALB health check *is* `/readyz`),
   `GET <url>/api/v1/meta` through CloudFront, the SPA shell and a
   client-side route, and that the ALB refuses direct requests.

**By hand** (break-glass, same result):

```bash
cd web && npm ci && npm run build && cd ../infra
npx cdk deploy --all -c stage=prod -c account=<ACCOUNT> \
  -c domainName=example.com -c certificateArn=arn:aws:acm:us-east-1:…:certificate/… \
  -c hostedZoneId=Z… -c hostedZoneName=example.com -c alarmEmail=ops@example.com
```

First prod deploy: expect ~40 minutes (Multi-AZ RDS and CloudFront dominate).
Confirm the SNS e-mail subscription, then fill in the integration secrets.

## Secrets

All secrets live in Secrets Manager and reach the containers as ECS secrets;
nothing sensitive is in a task definition or template.

| Secret | Keys → API variable | Filled by |
| --- | --- | --- |
| `dsa-<stage>/db/master` | `username` → `DB_USER`, `password` → `DB_PASSWORD` | generated (RDS) |
| `dsa-<stage>/cache/auth-token` | (string) → `REDIS_AUTH_TOKEN` | generated |
| `dsa-<stage>/api/session-secret` | (string, 64 hex) → `SESSION_SECRET` | generated |
| `dsa-<stage>/edge/origin-verify-g<N>` | (string, 64 hex) → `ORIGIN_VERIFY_SECRET` | generated |
| `dsa-<stage>/api/stripe` | `secret_key`, `webhook_secret`, `price_monthly`, `price_yearly` → `STRIPE_*` | operator |
| `dsa-<stage>/api/oauth-github` | `client_id`, `client_secret` → `OAUTH_GITHUB_CLIENT_*` | operator |
| `dsa-<stage>/api/oauth-google` | `client_id`, `client_secret` → `OAUTH_GOOGLE_CLIENT_*` | operator |
| `dsa-<stage>/api/anthropic` (only `aiProvider=anthropic`) | `api_key` → `ANTHROPIC_API_KEY` | operator |

Operator secrets are created with every key present and an **empty** value
(= integration disabled). Fill them, keeping every key:

```bash
aws secretsmanager put-secret-value --secret-id dsa-prod/api/stripe --secret-string \
  '{"secret_key":"sk_live_…","webhook_secret":"whsec_…","price_monthly":"price_…","price_yearly":"price_…"}'
aws ecs update-service --cluster dsa-prod --service dsa-prod-api --force-new-deployment
```

OAuth redirect URLs: `https://<domain>/api/v1/auth/oauth/github/callback` and
`…/google/callback`. Stripe webhook endpoint: `https://<domain>/api/v1/billing/webhook`.

## Rotating secrets

ECS injects secrets when a task starts, so every rotation ends with a forced
redeploy (`aws ecs update-service --cluster dsa-<stage> --service dsa-<stage>-api --force-new-deployment`).

- **Third-party keys** (Stripe, OAuth, Anthropic): rotate at the provider,
  `put-secret-value` as above, force a redeploy.
- **`SESSION_SECRET`**: `put-secret-value --secret-string "$(openssl rand -hex 32)"`,
  force a redeploy. Outstanding CSRF tokens and OAuth flows become invalid
  (sessions themselves survive; the SPA re-reads its token on reload).
- **Cache AUTH token** (zero downtime):
  ```bash
  NEW=$(openssl rand -hex 32)
  aws elasticache modify-replication-group --replication-group-id dsa-prod-cache \
    --auth-token "$NEW" --auth-token-update-strategy ROTATE --apply-immediately   # old + new valid
  aws secretsmanager put-secret-value --secret-id dsa-prod/cache/auth-token --secret-string "$NEW"
  # force a redeploy, wait for it to finish, then drop the old token:
  aws elasticache modify-replication-group --replication-group-id dsa-prod-cache \
    --auth-token "$NEW" --auth-token-update-strategy SET --apply-immediately
  ```
- **Database password** (brief risk window — do it when quiet): generate a
  password without `"@/\ '%`, `aws rds modify-db-instance --db-instance-identifier dsa-prod-postgres --master-user-password … --apply-immediately`,
  write the same value into the `password` key of `dsa-prod/db/master`
  (`get-secret-value | jq '.password=$p' | put-secret-value`), force a
  redeploy. Existing pooled connections keep working; new connections from
  old tasks fail until they are replaced (1–3 minutes).
- **Origin-verify secret**: deploy with `-c originVerifyGeneration=<N+1>`. A
  fresh secret is created and CloudFront, the listener rule and the API are
  re-pointed in one deployment; while CloudFront propagates (a few minutes)
  some API requests are refused with 403, so pick a quiet window.

## Rollback

- **Automatic**: a task set that fails its `/readyz` checks is rolled back by
  the ECS circuit breaker; a failed stack update is rolled back by
  CloudFormation. The migrations resource does nothing during a rollback.
- **Roll back a release**: redeploy the previous commit (dispatch
  `platform-deploy` on a branch or tag at that commit). To pin an exact
  earlier image without rebuilding, pass `-c apiImageUri=<acct>.dkr.ecr.<region>.amazonaws.com/cdk-hnb659fds-container-assets-<acct>-<region>@sha256:…`
  (and/or `runnerImageUri`).
- **Schema**: migrations are forward-only. Write them expand/contract (add,
  backfill, switch, then drop in a later release) so the previous release
  runs on the newer schema; the old image is never asked to migrate down.
- **SPA**: redeploy the previous commit. Old hashed assets are never pruned,
  so open tabs keep working through either direction.

## Scaling knobs

| Pressure | Knob |
| --- | --- |
| API CPU or request rate | `apiMaxTasks`, `apiCpuTargetPercent`, `apiRequestsPerTarget`, `apiCpu`/`apiMemoryMiB` |
| DB connections (`rds-connections-high`) | `rdsProxy=true`, then more tasks; or `dbInstanceClass`; `dbMaxConnectionsPerTask` |
| DB CPU / latency | `dbInstanceClass` (modify in place, Multi-AZ fails over in ~1–2 min) |
| Storage | automatic up to `dbMaxAllocatedStorageGiB` |
| Runner throttles (`runner-throttles`, users see 503) | `runnerReservedConcurrency` (cost ceiling), `runnerConcurrencyPerTask` |
| Runner cold starts | `runnerProvisionedConcurrency` (billed while idle) |
| Slow compiles | `runnerMemoryMiB` (CPU scales with memory; 1 769 MB = 1 vCPU) |
| Cache memory / evictions | `cacheNodeType`, `cacheReplicas` |
| Legit users hitting 429 | `wafRateLimit`, `wafAuthRateLimit` (per viewer IP per 5 min) |

## Cost estimate (us-east-1, on-demand, excluding traffic)

| Item | dev / month | prod / month |
| --- | ---: | ---: |
| NAT gateways (+ public IPv4) | $33 (1) | $99 (3) |
| Interface endpoints (5 × AZs) | — | $110 |
| RDS PostgreSQL | $14 (t4g.micro, single-AZ, 20 GB) | $260 (m7g.large Multi-AZ, 50 GB) |
| ElastiCache Valkey | $10 (1 × t4g.micro) | $76 (2 × t4g.medium) |
| Fargate API | $6 (1 × 0.5 vCPU/1 GB, Spot) | $72 (2 × 1 vCPU/2 GB; up to ~$720 at 20 tasks) |
| ALB (+ LCUs) | $22 | $40 |
| WAF (regional; + edge in prod) | $10 | $22 + $0.60/M requests |
| Other public IPv4 (ALB) | $7 | $11 |
| CloudWatch (logs, Container Insights, alarms, dashboard, flow logs) | $10 | $45 |
| Secrets Manager, KMS, ECR, S3, Route 53 | $8 | $15 |
| **Baseline** | **≈ $120** | **≈ $750** |
| Runner Lambda | ≈ $1 per 10 k runs (2 GB × 3 s) | ≈ $15 per 100 k runs (3 GB × 3 s) |
| CloudFront | free tier (1 TB, 10 M requests) | $0.085/GB beyond the free tier (NA/EU) |
| Bedrock | per token for the configured model; bounded by `AI_FREE_DAILY`/`AI_PRO_DAILY` | |

Biggest dev savings if needed: `-c natGateways=1` is already the minimum;
stop the stage with `cdk destroy` when idle (dev data is disposable).

## Disaster recovery

- **Protections (prod)**: deletion protection, a final snapshot on stack
  deletion (`DeletionPolicy: Snapshot`), 14 days of point-in-time recovery,
  retained KMS key, stack termination protection. RPO ≈ 5 minutes (PITR),
  RTO ≈ 30–60 minutes.
- **Restore to a point in time / from a snapshot** without touching the
  templates — the replacement takes over the original identifier, so its
  endpoint (and `DB_HOST`) stays the same:
  ```bash
  SRC=dsa-prod-postgres
  aws rds restore-db-instance-to-point-in-time --source-db-instance-identifier $SRC \
    --target-db-instance-identifier $SRC-restore --restore-time 2026-01-01T12:00:00Z \
    --db-subnet-group-name <from the Data stack> --vpc-security-group-ids <dsa-prod-db SG> \
    --db-parameter-group-name <from the Data stack> --multi-az --no-publicly-accessible
  #  (or: restore-db-instance-from-db-snapshot --db-snapshot-identifier <snapshot> …)
  aws rds wait db-instance-available --db-instance-identifier $SRC-restore
  # verify the data (one-off task / psql through a migration-style task), then swap:
  aws rds modify-db-instance --db-instance-identifier $SRC --deletion-protection false --apply-immediately   # re-enabled below
  aws rds modify-db-instance --db-instance-identifier $SRC --new-db-instance-identifier $SRC-old --apply-immediately
  aws rds modify-db-instance --db-instance-identifier $SRC-restore --new-db-instance-identifier $SRC --deletion-protection --apply-immediately
  aws ecs update-service --cluster dsa-prod --service dsa-prod-api --force-new-deployment
  ```
  Keep `$SRC-old` until the next deploy is clean, then delete it with a final
  snapshot. The restored instance keeps the same master password (secret
  unchanged).
- **Cache**: holds only rate-limit counters; a new replication group starts empty.
- **SPA**: rebuilt from git on every deploy.
- **Region loss**: not automated. For a warm standby, copy snapshots to a
  second region (`aws rds copy-db-snapshot --source-region …` with a KMS key
  there) and deploy the stacks with `-c region=<other> -c cloudFrontPrefixListId=…`.

## Operations notes

- Logs: `/dsa/<stage>/api`, `/dsa/<stage>/migrate`, `/aws/lambda/dsa-<stage>-runner`,
  `/aws/lambda/dsa-<stage>-migrations`, `/dsa/<stage>/vpc-flow-logs`,
  `/dsa/<stage>/cache/slow-log`, `/aws/rds/instance/dsa-<stage>-postgres/postgresql`.
- Dashboard `dsa-<stage>-platform`; alarms publish to `dsa-<stage>-alarms`.
- ECS Exec is disabled (it would need `ssmmessages:*` on the task role).
  Database break-glass: run a one-off task from the migration task definition
  with a command override, or connect with IAM auth after granting
  `rds-db:connect` to a named operator role.
- The ALB answers only CloudFront (security group) and only with the secret
  header (listener rule); probing its DNS name returns nothing / 403 by design.
