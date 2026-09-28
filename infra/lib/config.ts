/**
 * Stage configuration.
 *
 * Everything that differs between `dev` and `prod` (or between two operators'
 * accounts) is resolved here, once, from CDK context (`-c key=value` or
 * `cdk.json`). Stacks never read context themselves, so the full parameter
 * surface of the platform is this file — and it is validated before a single
 * construct is created, which turns a class of deploy-time failures (bad
 * Fargate sizes, a CloudFront certificate in the wrong region, …) into
 * synth-time errors with a readable message.
 *
 * Nothing here performs an AWS lookup: synth must work offline, without
 * credentials. Values that would normally need a lookup (the CloudFront
 * origin-facing prefix list, the hosted zone, AZ names) are explicit context
 * parameters with documented defaults.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';

export type StageName = 'dev' | 'prod';
export type AiProvider = 'bedrock' | 'anthropic' | 'none';
export type CpuArch = 'X86_64' | 'ARM64';

/** Anything with a `tryGetContext` — an App, a Stage, or a test double. */
export interface ContextSource {
  node: { tryGetContext(key: string): unknown };
}

export interface DomainConfig {
  /** Apex or subdomain the SPA is served on, e.g. `dsavisualized.com`. */
  readonly domainName: string;
  /** Route 53 hosted zone; when absent the operator creates DNS records by hand. */
  readonly hostedZoneId?: string;
  readonly hostedZoneName?: string;
  /** ACM certificate in us-east-1 covering `domainName` (CloudFront). */
  readonly certificateArn: string;
  /** Regional ACM certificate covering `originDomainName` (ALB HTTPS listener). */
  readonly albCertificateArn: string;
  /** Name CloudFront uses to reach the ALB over TLS, e.g. `origin.dsavisualized.com`. */
  readonly originDomainName: string;
}

export interface ImageRef {
  /** ECR repository name (same account/region as the stage). */
  readonly repositoryName: string;
  /** Tag or `sha256:` digest. */
  readonly tagOrDigest: string;
}

export interface StageConfig {
  readonly stage: StageName;
  readonly isProd: boolean;
  readonly project: string;
  readonly costCenter: string;
  /** `dsa-dev` / `dsa-prod`: prefix of every physical resource name. */
  readonly prefix: string;
  /** `DsaDev` / `DsaProd`: prefix of every stack id. */
  readonly stackPrefix: string;
  readonly region: string;
  /** Optional account pin: `cdk deploy` then refuses to run against another account. */
  readonly account?: string;

  /** Repository root (Docker build context for both images). */
  readonly repoRoot: string;
  /** `web/dist`; deployed to S3 only when it exists at synth time. */
  readonly webDistDir: string;

  // ── network ────────────────────────────────────────────────────────────────
  readonly vpcCidr: string;
  readonly availabilityZones: string[];
  readonly natGateways: number;
  readonly interfaceEndpoints: boolean;
  readonly flowLogTrafficType: 'ALL' | 'REJECT';
  readonly cloudFrontPrefixListId: string;

  // ── data ───────────────────────────────────────────────────────────────────
  /** RDS class without the `db.` prefix, e.g. `t4g.micro`, `m7g.large`. */
  readonly dbInstanceClass: string;
  readonly dbAllocatedStorageGiB: number;
  readonly dbMaxAllocatedStorageGiB: number;
  readonly dbMultiAz: boolean;
  readonly dbBackupRetentionDays: number;
  readonly dbName: string;
  readonly rdsProxy: boolean;
  readonly cacheEngine: 'valkey' | 'redis';
  readonly cacheEngineVersion: string;
  readonly cacheNodeType: string;
  readonly cacheReplicas: number;

  // ── runner ─────────────────────────────────────────────────────────────────
  readonly runnerMemoryMiB: number;
  readonly runnerEphemeralStorageMiB: number;
  readonly runnerTimeoutSeconds: number;
  /** Reserved concurrency (cost ceiling). `undefined` = draw from the unreserved pool. */
  readonly runnerReservedConcurrency?: number;
  readonly runnerProvisionedConcurrency: number;
  readonly runnerImage?: ImageRef;

  // ── api ────────────────────────────────────────────────────────────────────
  readonly apiCpu: number;
  readonly apiMemoryMiB: number;
  readonly apiMinTasks: number;
  readonly apiMaxTasks: number;
  readonly apiCpuArchitecture: CpuArch;
  readonly apiFargateSpot: boolean;
  readonly apiRequestsPerTarget: number;
  readonly apiCpuTargetPercent: number;
  readonly dbMaxConnectionsPerTask: number;
  readonly runnerConcurrencyPerTask: number;
  readonly aiProvider: AiProvider;
  readonly aiModelId: string;
  /** Sender when there is no domain: a single SES-verified address (sandbox testing). */
  readonly mailFromAddress?: string;
  /** Allow-listed, non-secret API environment overrides (see API_ENV_OVERRIDABLE). */
  readonly apiEnv: Record<string, string>;
  readonly apiImage?: ImageRef;

  // ── ingress / edge ─────────────────────────────────────────────────────────
  readonly albIdleTimeoutSeconds: number;
  readonly wafRateLimit: number;
  readonly wafAuthRateLimit: number;
  readonly edgeWaf: boolean;
  readonly priceClass: 'PriceClass_100' | 'PriceClass_200' | 'PriceClass_All';
  readonly publishSourceMaps: boolean;
  /** Bump to rotate the CloudFront → ALB origin-verify secret (see DEPLOY.md). */
  readonly originVerifyGeneration: number;
  readonly domain?: DomainConfig;

  // ── observability ──────────────────────────────────────────────────────────
  readonly logRetentionDays: number;
  readonly alarmEmail?: string;
  readonly monthlyBudgetUsd: number;
  readonly latencyAlarmSeconds: number;
  readonly wafBlockedAlarmThreshold: number;

  // ── CI ─────────────────────────────────────────────────────────────────────
  readonly githubOidc: boolean;
  readonly githubRepo: string;
  readonly githubSubject: string;
  readonly githubOidcProviderArn?: string;
  readonly cdkQualifier: string;
}

/**
 * Non-secret API variables an operator may set per stage with
 * `-c apiEnv='{"ADMIN_EMAILS":"me@example.com"}'`. Anything else is refused:
 * secrets must never travel as plaintext task-definition environment, and the
 * infra-owned variables (DB_*, REDIS_*, RUNNER_*, …) are derived from the
 * resources themselves.
 */
export const API_ENV_OVERRIDABLE = [
  'ADMIN_EMAILS',
  'AI_FREE_DAILY',
  'AI_PRO_DAILY',
  'DSA_ALLOWED_ORIGINS',
  'MAIL_FROM',
  'PREMIUM_TIERS',
  'REQUIRE_VERIFIED_EMAIL',
  'RUNS_FREE_PER_MIN',
  'RUNS_PRO_PER_MIN',
  'RUST_LOG',
  'SESSION_TTL_DAYS',
  'SIGNUP_ENABLED',
] as const;

/**
 * Per-region id of the AWS-managed prefix list
 * `com.amazonaws.global.cloudfront.origin-facing`. Only the default region is
 * built in; for any other region pass `-c cloudFrontPrefixListId=pl-…`, found with
 *   aws ec2 describe-managed-prefix-lists \
 *     --filters Name=prefix-list-name,Values=com.amazonaws.global.cloudfront.origin-facing
 */
const CLOUDFRONT_ORIGIN_FACING_PREFIX_LISTS: Record<string, string> = {
  'us-east-1': 'pl-3b927c52',
};

/** Paths never sent to Docker: build output, dependencies, and trees no image needs. */
export const DOCKER_CONTEXT_EXCLUDES = [
  '.git',
  '**/.git',
  'target',
  '**/target',
  // Extra Cargo target dirs a developer may keep beside the backend
  // workspace (anchored: `**/target-*` would also match content/problems/target-sum).
  'backend/target-*',
  '**/node_modules',
  'cdk.out',
  '**/cdk.out',
  // Infra, CI and docs never enter an image; excluding them also keeps the
  // asset hash (and therefore the ECS/Lambda rollout) stable when they change.
  'infra',
  '.github',
  'docs',
  // The SPA ships to S3, never inside an image.
  'web',
  '**/*.pdb',
  '**/.idea',
  '**/.vscode',
  '**/.DS_Store',
];

interface StageDefaults {
  vpcCidr: string;
  azCount: number;
  natGateways: number;
  interfaceEndpoints: boolean;
  flowLogTrafficType: 'ALL' | 'REJECT';
  dbInstanceClass: string;
  dbAllocatedStorageGiB: number;
  dbMaxAllocatedStorageGiB: number;
  dbMultiAz: boolean;
  dbBackupRetentionDays: number;
  cacheNodeType: string;
  cacheReplicas: number;
  runnerMemoryMiB: number;
  runnerReservedConcurrency: number | undefined;
  apiCpu: number;
  apiMemoryMiB: number;
  apiMinTasks: number;
  apiMaxTasks: number;
  apiFargateSpot: boolean;
  dbMaxConnectionsPerTask: number;
  runnerConcurrencyPerTask: number;
  edgeWaf: boolean;
  priceClass: StageConfig['priceClass'];
  publishSourceMaps: boolean;
  logRetentionDays: number;
  monthlyBudgetUsd: number;
  wafBlockedAlarmThreshold: number;
}

const DEFAULTS: Record<StageName, StageDefaults> = {
  // dev: the smallest footprint that still exercises every production code
  // path (TLS Redis with AUTH, RDS with force_ssl, the Lambda runner, WAF).
  dev: {
    vpcCidr: '10.40.0.0/16',
    azCount: 2,
    natGateways: 1,
    interfaceEndpoints: false,
    flowLogTrafficType: 'REJECT',
    dbInstanceClass: 't4g.micro',
    dbAllocatedStorageGiB: 20,
    dbMaxAllocatedStorageGiB: 100,
    dbMultiAz: false,
    dbBackupRetentionDays: 7,
    cacheNodeType: 'cache.t4g.micro',
    cacheReplicas: 0,
    runnerMemoryMiB: 2048,
    runnerReservedConcurrency: 10,
    apiCpu: 512,
    apiMemoryMiB: 1024,
    apiMinTasks: 1,
    apiMaxTasks: 2,
    apiFargateSpot: true,
    dbMaxConnectionsPerTask: 10,
    runnerConcurrencyPerTask: 32,
    edgeWaf: false,
    priceClass: 'PriceClass_100',
    publishSourceMaps: true,
    logRetentionDays: 14,
    monthlyBudgetUsd: 200,
    wafBlockedAlarmThreshold: 500,
  },
  // prod: Multi-AZ everything, one NAT per AZ, private AWS API endpoints.
  prod: {
    vpcCidr: '10.41.0.0/16',
    azCount: 3,
    natGateways: 3,
    interfaceEndpoints: true,
    flowLogTrafficType: 'ALL',
    dbInstanceClass: 'm7g.large',
    dbAllocatedStorageGiB: 50,
    dbMaxAllocatedStorageGiB: 500,
    dbMultiAz: true,
    dbBackupRetentionDays: 14,
    cacheNodeType: 'cache.t4g.medium',
    cacheReplicas: 1,
    runnerMemoryMiB: 3008,
    runnerReservedConcurrency: 200,
    apiCpu: 1024,
    apiMemoryMiB: 2048,
    apiMinTasks: 2,
    apiMaxTasks: 20,
    apiFargateSpot: false,
    dbMaxConnectionsPerTask: 20,
    runnerConcurrencyPerTask: 64,
    edgeWaf: true,
    priceClass: 'PriceClass_All',
    publishSourceMaps: false,
    logRetentionDays: 90,
    monthlyBudgetUsd: 1500,
    wafBlockedAlarmThreshold: 2000,
  },
};

/** Valid Fargate CPU → memory (MiB) ranges. */
const FARGATE_MEMORY: Record<number, [number, number, number]> = {
  256: [512, 2048, 512],
  512: [1024, 4096, 1024],
  1024: [2048, 8192, 1024],
  2048: [4096, 16384, 1024],
  4096: [8192, 30720, 1024],
  8192: [16384, 61440, 4096],
  16384: [32768, 122880, 8192],
};

/** Log retention values CloudWatch accepts. */
const LOG_RETENTION_DAYS = [1, 3, 5, 7, 14, 30, 60, 90, 120, 150, 180, 365, 400, 545, 731, 1096, 1827, 2192, 2557, 2922, 3288, 3653];

class ConfigError extends Error {
  constructor(message: string) {
    super(`Invalid stage configuration: ${message}`);
    this.name = 'ConfigError';
  }
}

function raw(src: ContextSource, key: string): unknown {
  const v = src.node.tryGetContext(key);
  return v === undefined || v === null || v === '' ? undefined : v;
}

function str(src: ContextSource, key: string): string | undefined {
  const v = raw(src, key);
  return v === undefined ? undefined : String(v).trim() || undefined;
}

function num(src: ContextSource, key: string, fallback: number): number {
  const v = raw(src, key);
  if (v === undefined) return fallback;
  const n = typeof v === 'number' ? v : Number(String(v).trim());
  if (!Number.isFinite(n)) throw new ConfigError(`${key} must be a number, got "${String(v)}"`);
  return n;
}

function int(src: ContextSource, key: string, fallback: number): number {
  const n = num(src, key, fallback);
  if (!Number.isInteger(n)) throw new ConfigError(`${key} must be an integer, got ${n}`);
  return n;
}

function bool(src: ContextSource, key: string, fallback: boolean): boolean {
  const v = raw(src, key);
  if (v === undefined) return fallback;
  if (typeof v === 'boolean') return v;
  const s = String(v).trim().toLowerCase();
  if (['true', '1', 'yes', 'on'].includes(s)) return true;
  if (['false', '0', 'no', 'off'].includes(s)) return false;
  throw new ConfigError(`${key} must be true or false, got "${String(v)}"`);
}

function oneOf<T extends string>(src: ContextSource, key: string, allowed: readonly T[], fallback: T): T {
  const v = str(src, key) ?? fallback;
  if (!(allowed as readonly string[]).includes(v)) {
    throw new ConfigError(`${key} must be one of ${allowed.join(', ')}, got "${v}"`);
  }
  return v as T;
}

function requireThat(condition: boolean, message: string): void {
  if (!condition) throw new ConfigError(message);
}

/** Parses `<account>.dkr.ecr.<region>.amazonaws.com/<repo>(:tag|@sha256:…)`. */
export function parseEcrImageUri(uri: string, key: string): ImageRef {
  const m = /^\d{12}\.dkr\.ecr\.[a-z0-9-]+\.amazonaws\.com(?:\.cn)?\/([a-z0-9][a-z0-9._\/-]*)(?::([\w][\w.-]{0,127})|@(sha256:[a-f0-9]{64}))$/.exec(uri);
  if (!m) {
    throw new ConfigError(`${key} must be an ECR image URI with a tag or digest (…dkr.ecr.<region>.amazonaws.com/<repo>:<tag>), got "${uri}"`);
  }
  return { repositoryName: m[1], tagOrDigest: m[2] ?? m[3] };
}

function parseApiEnv(src: ContextSource): Record<string, string> {
  const v = raw(src, 'apiEnv');
  if (v === undefined) return {};
  let parsed: unknown = v;
  if (typeof v === 'string') {
    try {
      parsed = JSON.parse(v);
    } catch {
      throw new ConfigError('apiEnv must be a JSON object, e.g. {"ADMIN_EMAILS":"me@example.com"}');
    }
  }
  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
    throw new ConfigError('apiEnv must be a JSON object of string values');
  }
  const out: Record<string, string> = {};
  for (const [k, value] of Object.entries(parsed as Record<string, unknown>)) {
    if (!(API_ENV_OVERRIDABLE as readonly string[]).includes(k)) {
      throw new ConfigError(
        `apiEnv.${k} is not overridable. Allowed: ${API_ENV_OVERRIDABLE.join(', ')}. ` +
          'Secrets belong in Secrets Manager (see DEPLOY.md), infra-derived variables are set by the stacks.',
      );
    }
    if (typeof value !== 'string' && typeof value !== 'number' && typeof value !== 'boolean') {
      throw new ConfigError(`apiEnv.${k} must be a string, number or boolean`);
    }
    out[k] = String(value);
  }
  return out;
}

function parseDomain(src: ContextSource, region: string): DomainConfig | undefined {
  const domainName = str(src, 'domainName')?.toLowerCase().replace(/\.$/, '');
  const hostedZoneId = str(src, 'hostedZoneId');
  const hostedZoneName = str(src, 'hostedZoneName')?.toLowerCase().replace(/\.$/, '');
  const certificateArn = str(src, 'certificateArn');
  const albCertificateArnRaw = str(src, 'albCertificateArn');
  const originSubdomain = str(src, 'originSubdomain') ?? 'origin';

  if (!domainName) {
    requireThat(
      !hostedZoneId && !hostedZoneName && !certificateArn && !albCertificateArnRaw,
      'hostedZoneId, hostedZoneName, certificateArn and albCertificateArn only make sense with domainName',
    );
    return undefined;
  }
  requireThat(/^(?=.{1,253}$)([a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$/.test(domainName), `domainName "${domainName}" is not a valid DNS name`);
  requireThat(!!certificateArn, 'domainName requires certificateArn (an ACM certificate in us-east-1 covering the domain)');
  const certRegion = /^arn:aws[a-z-]*:acm:([a-z0-9-]+):\d{12}:certificate\/[\w-]+$/.exec(certificateArn!)?.[1];
  requireThat(certRegion !== undefined, `certificateArn "${certificateArn}" is not an ACM certificate ARN`);
  requireThat(certRegion === 'us-east-1', `certificateArn must be in us-east-1 (CloudFront requirement), got ${certRegion}`);
  requireThat(!!hostedZoneId === !!hostedZoneName, 'hostedZoneId and hostedZoneName must be given together');
  if (hostedZoneName) {
    requireThat(
      domainName === hostedZoneName || domainName.endsWith(`.${hostedZoneName}`),
      `domainName ${domainName} is not inside hosted zone ${hostedZoneName}`,
    );
  }
  // The ALB lives in the stage region; the CloudFront certificate can double as
  // the listener certificate only when that region is us-east-1.
  const albCertificateArn = albCertificateArnRaw ?? (region === 'us-east-1' ? certificateArn! : undefined);
  requireThat(
    albCertificateArn !== undefined,
    `the stage region is ${region}: pass albCertificateArn (an ACM certificate in ${region} covering ${originSubdomain}.${domainName})`,
  );
  const albCertRegion = /^arn:aws[a-z-]*:acm:([a-z0-9-]+):/.exec(albCertificateArn!)?.[1];
  requireThat(albCertRegion === region, `albCertificateArn must be in the stage region ${region}, got ${albCertRegion}`);
  requireThat(/^[a-z0-9-]+$/.test(originSubdomain), 'originSubdomain must be a single DNS label');

  return {
    domainName,
    hostedZoneId,
    hostedZoneName,
    certificateArn: certificateArn!,
    albCertificateArn: albCertificateArn!,
    originDomainName: `${originSubdomain}.${domainName}`,
  };
}

/** Estimated `max_connections` of an RDS PostgreSQL class: LEAST(DBInstanceClassMemory/9531392, 5000). */
export function estimateMaxConnections(instanceClass: string): number {
  const [family, size] = instanceClass.split('.');
  const sizeGiB: Record<string, number> = {
    micro: 1, small: 2, medium: 4, large: 8, xlarge: 16, '2xlarge': 32, '4xlarge': 64, '8xlarge': 128, '12xlarge': 192, '16xlarge': 256,
  };
  let gib = sizeGiB[size] ?? 8;
  if (/^r/.test(family)) gib *= 2; // memory-optimised classes carry twice the memory per vCPU
  if (/^x/.test(family)) gib *= 4;
  // DBInstanceClassMemory is what remains after the OS and RDS processes (~85 %).
  return Math.min(Math.floor((gib * 1024 ** 3 * 0.85) / 9531392), 5000);
}

/** Performance Insights is not offered on the micro/small burstable classes. */
export function supportsPerformanceInsights(instanceClass: string): boolean {
  return !/^t\d+g?\.(micro|small)$/.test(instanceClass);
}

export function loadStageConfig(src: ContextSource, infraDir: string = path.resolve(__dirname, '..')): StageConfig {
  const stage = oneOf<StageName>(src, 'stage', ['dev', 'prod'], 'dev');
  const d = DEFAULTS[stage];
  const isProd = stage === 'prod';
  const region = str(src, 'region') ?? process.env.CDK_DEPLOY_REGION ?? 'us-east-1';
  requireThat(/^[a-z]{2}(-gov)?-[a-z]+-\d$/.test(region), `region "${region}" is not an AWS region`);
  const account = str(src, 'account');
  requireThat(account === undefined || /^\d{12}$/.test(account), `account must be a 12-digit id, got "${account}"`);

  const repoRoot = path.resolve(infraDir, str(src, 'repoRoot') ?? '..');
  const azOverride = str(src, 'availabilityZones');
  const azCount = int(src, 'azCount', d.azCount);
  requireThat(azCount >= 2 && azCount <= 3, 'azCount must be 2 or 3');
  const availabilityZones = azOverride
    ? azOverride.split(',').map((s) => s.trim()).filter(Boolean)
    : ['a', 'b', 'c'].slice(0, azCount).map((l) => `${region}${l}`);
  requireThat(availabilityZones.length >= 2, 'at least two availability zones are required');
  requireThat(availabilityZones.every((az) => az.startsWith(region)), `availabilityZones must belong to ${region}`);

  const natGateways = int(src, 'natGateways', isProd ? availabilityZones.length : d.natGateways);
  requireThat(natGateways >= 1 && natGateways <= availabilityZones.length, `natGateways must be between 1 and ${availabilityZones.length}`);

  const cloudFrontPrefixListId = str(src, 'cloudFrontPrefixListId') ?? CLOUDFRONT_ORIGIN_FACING_PREFIX_LISTS[region];
  requireThat(
    cloudFrontPrefixListId !== undefined,
    `no built-in CloudFront origin-facing prefix list id for ${region}; pass -c cloudFrontPrefixListId=pl-… ` +
      '(aws ec2 describe-managed-prefix-lists --filters Name=prefix-list-name,Values=com.amazonaws.global.cloudfront.origin-facing)',
  );
  requireThat(/^pl-[0-9a-f]{8,17}$/.test(cloudFrontPrefixListId!), `cloudFrontPrefixListId "${cloudFrontPrefixListId}" is not a prefix list id`);

  const dbInstanceClass = (str(src, 'dbInstanceClass') ?? d.dbInstanceClass).replace(/^db\./, '');
  requireThat(/^[a-z0-9]+\.[a-z0-9]+$/.test(dbInstanceClass), `dbInstanceClass "${dbInstanceClass}" is not an instance class`);
  const dbAllocatedStorageGiB = int(src, 'dbAllocatedStorageGiB', d.dbAllocatedStorageGiB);
  const dbMaxAllocatedStorageGiB = int(src, 'dbMaxAllocatedStorageGiB', d.dbMaxAllocatedStorageGiB);
  requireThat(dbAllocatedStorageGiB >= 20, 'dbAllocatedStorageGiB must be ≥ 20 (gp3 minimum)');
  requireThat(dbMaxAllocatedStorageGiB > dbAllocatedStorageGiB, 'dbMaxAllocatedStorageGiB must exceed dbAllocatedStorageGiB (storage autoscaling)');
  const dbBackupRetentionDays = int(src, 'dbBackupRetentionDays', d.dbBackupRetentionDays);
  requireThat(dbBackupRetentionDays >= 1 && dbBackupRetentionDays <= 35, 'dbBackupRetentionDays must be 1–35');

  const cacheEngine = oneOf(src, 'cacheEngine', ['valkey', 'redis'] as const, 'valkey');
  const cacheReplicas = int(src, 'cacheReplicas', d.cacheReplicas);
  requireThat(cacheReplicas >= 0 && cacheReplicas <= 5, 'cacheReplicas must be 0–5');

  const runnerMemoryMiB = int(src, 'runnerMemoryMiB', d.runnerMemoryMiB);
  requireThat(runnerMemoryMiB >= 2048 && runnerMemoryMiB <= 10240, 'runnerMemoryMiB must be 2048–10240 (compilers need ≥ 2 GiB)');
  const runnerTimeoutSeconds = int(src, 'runnerTimeoutSeconds', 60);
  requireThat(runnerTimeoutSeconds >= 60 && runnerTimeoutSeconds <= 900, 'runnerTimeoutSeconds must be 60–900');
  const runnerEphemeralStorageMiB = int(src, 'runnerEphemeralStorageMiB', 2048);
  requireThat(runnerEphemeralStorageMiB >= 512 && runnerEphemeralStorageMiB <= 10240, 'runnerEphemeralStorageMiB must be 512–10240');
  const reservedRaw = str(src, 'runnerReservedConcurrency');
  const runnerReservedConcurrency =
    reservedRaw === 'none' ? undefined : reservedRaw === undefined ? d.runnerReservedConcurrency : int(src, 'runnerReservedConcurrency', 0);
  requireThat(
    runnerReservedConcurrency === undefined || runnerReservedConcurrency >= 1,
    'runnerReservedConcurrency must be ≥ 1 (0 would disable the runner) or "none"',
  );
  const runnerProvisionedConcurrency = int(src, 'runnerProvisionedConcurrency', 0);
  requireThat(runnerProvisionedConcurrency >= 0, 'runnerProvisionedConcurrency must be ≥ 0');
  requireThat(
    runnerReservedConcurrency === undefined || runnerProvisionedConcurrency <= runnerReservedConcurrency,
    'runnerProvisionedConcurrency cannot exceed runnerReservedConcurrency',
  );

  const apiCpu = int(src, 'apiCpu', d.apiCpu);
  const apiMemoryMiB = int(src, 'apiMemoryMiB', d.apiMemoryMiB);
  const mem = FARGATE_MEMORY[apiCpu];
  requireThat(mem !== undefined, `apiCpu must be one of ${Object.keys(FARGATE_MEMORY).join(', ')}`);
  requireThat(
    apiMemoryMiB >= mem[0] && apiMemoryMiB <= mem[1] && apiMemoryMiB % mem[2] === 0,
    `apiMemoryMiB ${apiMemoryMiB} is not valid for apiCpu ${apiCpu} (${mem[0]}–${mem[1]} in steps of ${mem[2]})`,
  );
  const apiMinTasks = int(src, 'apiMinTasks', d.apiMinTasks);
  const apiMaxTasks = int(src, 'apiMaxTasks', d.apiMaxTasks);
  requireThat(apiMinTasks >= 1 && apiMaxTasks >= apiMinTasks, 'apiMinTasks must be ≥ 1 and ≤ apiMaxTasks');
  const apiCpuArchitecture = oneOf<CpuArch>(src, 'apiCpuArchitecture', ['X86_64', 'ARM64'], 'X86_64');
  const apiFargateSpot = bool(src, 'apiFargateSpot', d.apiFargateSpot);

  const aiProvider = oneOf<AiProvider>(src, 'aiProvider', ['bedrock', 'anthropic', 'none'], 'bedrock');
  const aiModelId = str(src, 'aiModelId') ?? (aiProvider === 'anthropic' ? 'claude-opus-5' : 'anthropic.claude-opus-5');
  requireThat(/^[\w.:-]+$/.test(aiModelId), `aiModelId "${aiModelId}" contains unexpected characters`);

  const mailFromAddress = str(src, 'mailFromAddress');
  requireThat(mailFromAddress === undefined || /^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(mailFromAddress), 'mailFromAddress must be an e-mail address');
  const alarmEmail = str(src, 'alarmEmail');
  requireThat(alarmEmail === undefined || /^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(alarmEmail), 'alarmEmail must be an e-mail address');

  const albIdleTimeoutSeconds = int(src, 'albIdleTimeoutSeconds', 180);
  requireThat(albIdleTimeoutSeconds >= 120 && albIdleTimeoutSeconds <= 4000, 'albIdleTimeoutSeconds must be 120–4000 (runs and AI streams are long)');
  const wafRateLimit = int(src, 'wafRateLimit', 2000);
  const wafAuthRateLimit = int(src, 'wafAuthRateLimit', 100);
  requireThat(wafRateLimit >= 10 && wafAuthRateLimit >= 10, 'WAF rate limits must be ≥ 10 requests / 5 minutes (AWS minimum)');
  requireThat(wafAuthRateLimit <= wafRateLimit, 'wafAuthRateLimit should be tighter than wafRateLimit');

  const logRetentionDays = int(src, 'logRetentionDays', d.logRetentionDays);
  requireThat(LOG_RETENTION_DAYS.includes(logRetentionDays), `logRetentionDays must be one of ${LOG_RETENTION_DAYS.join(', ')}`);

  const dbMaxConnectionsPerTask = int(src, 'dbMaxConnectionsPerTask', d.dbMaxConnectionsPerTask);
  const rdsProxy = bool(src, 'rdsProxy', false);
  if (!rdsProxy) {
    // Pools of every task at full scale, plus the migration task and headroom
    // for operators, must fit in the instance. Refuse a config that would fail
    // under load rather than discover it during a traffic spike.
    const needed = apiMaxTasks * dbMaxConnectionsPerTask + 5;
    const available = Math.floor(estimateMaxConnections(dbInstanceClass) * 0.9);
    requireThat(
      needed <= available,
      `apiMaxTasks × dbMaxConnectionsPerTask (${needed}) exceeds ~90 % of max_connections on ${dbInstanceClass} (${available}); ` +
        'use a larger dbInstanceClass, a smaller pool, or -c rdsProxy=true',
    );
  }

  const githubRepo = str(src, 'githubRepo') ?? 'DeepakGiri2/DSARust';
  requireThat(/^[\w.-]+\/[\w.-]+$/.test(githubRepo), 'githubRepo must be owner/name');
  const githubOidcProviderArn = str(src, 'githubOidcProviderArn');

  const apiImageUri = str(src, 'apiImageUri');
  const runnerImageUri = str(src, 'runnerImageUri');

  const config: StageConfig = {
    stage,
    isProd,
    project: 'dsa-visualized',
    costCenter: str(src, 'costCenter') ?? 'dsa-platform',
    prefix: `dsa-${stage}`,
    stackPrefix: `Dsa${stage[0].toUpperCase()}${stage.slice(1)}`,
    region,
    account,
    repoRoot,
    webDistDir: path.join(repoRoot, 'web', 'dist'),

    vpcCidr: str(src, 'vpcCidr') ?? d.vpcCidr,
    availabilityZones,
    natGateways,
    interfaceEndpoints: bool(src, 'interfaceEndpoints', d.interfaceEndpoints),
    flowLogTrafficType: oneOf(src, 'flowLogTrafficType', ['ALL', 'REJECT'] as const, d.flowLogTrafficType),
    cloudFrontPrefixListId: cloudFrontPrefixListId!,

    dbInstanceClass,
    dbAllocatedStorageGiB,
    dbMaxAllocatedStorageGiB,
    dbMultiAz: bool(src, 'dbMultiAz', d.dbMultiAz),
    dbBackupRetentionDays,
    dbName: 'dsa',
    rdsProxy,
    cacheEngine,
    cacheEngineVersion: str(src, 'cacheEngineVersion') ?? (cacheEngine === 'valkey' ? '8.0' : '7.1'),
    cacheNodeType: str(src, 'cacheNodeType') ?? d.cacheNodeType,
    cacheReplicas,

    runnerMemoryMiB,
    runnerEphemeralStorageMiB,
    runnerTimeoutSeconds,
    runnerReservedConcurrency,
    runnerProvisionedConcurrency,
    runnerImage: runnerImageUri ? parseEcrImageUri(runnerImageUri, 'runnerImageUri') : undefined,

    apiCpu,
    apiMemoryMiB,
    apiMinTasks,
    apiMaxTasks,
    apiCpuArchitecture,
    apiFargateSpot,
    apiRequestsPerTarget: int(src, 'apiRequestsPerTarget', 1000),
    apiCpuTargetPercent: int(src, 'apiCpuTargetPercent', 60),
    dbMaxConnectionsPerTask,
    runnerConcurrencyPerTask: int(src, 'runnerConcurrencyPerTask', d.runnerConcurrencyPerTask),
    aiProvider,
    aiModelId,
    mailFromAddress,
    apiEnv: parseApiEnv(src),
    apiImage: apiImageUri ? parseEcrImageUri(apiImageUri, 'apiImageUri') : undefined,

    albIdleTimeoutSeconds,
    wafRateLimit,
    wafAuthRateLimit,
    edgeWaf: bool(src, 'edgeWaf', d.edgeWaf),
    priceClass: oneOf(src, 'priceClass', ['PriceClass_100', 'PriceClass_200', 'PriceClass_All'] as const, d.priceClass),
    publishSourceMaps: bool(src, 'publishSourceMaps', d.publishSourceMaps),
    originVerifyGeneration: int(src, 'originVerifyGeneration', 1),
    domain: parseDomain(src, region),

    logRetentionDays,
    alarmEmail,
    monthlyBudgetUsd: num(src, 'monthlyBudgetUsd', d.monthlyBudgetUsd),
    latencyAlarmSeconds: num(src, 'latencyAlarmSeconds', 8),
    wafBlockedAlarmThreshold: int(src, 'wafBlockedAlarmThreshold', d.wafBlockedAlarmThreshold),

    githubOidc: bool(src, 'githubOidc', false),
    githubRepo,
    githubSubject: str(src, 'githubSubject') ?? `repo:${githubRepo}:environment:${stage}`,
    githubOidcProviderArn,
    cdkQualifier: str(src, 'cdkQualifier') ?? 'hnb659fds',
  };

  requireThat(config.apiMinTasks <= config.apiMaxTasks, 'apiMinTasks must be ≤ apiMaxTasks');
  requireThat(config.originVerifyGeneration >= 1, 'originVerifyGeneration must be ≥ 1');
  requireThat(config.monthlyBudgetUsd >= 0, 'monthlyBudgetUsd must be ≥ 0 (0 disables the budget)');
  requireThat(/^[a-z0-9]{1,10}$/.test(config.cdkQualifier), 'cdkQualifier must be 1–10 lowercase alphanumerics');

  for (const [name, image, file] of [
    ['api', config.apiImage, 'backend/docker/api.Dockerfile'],
    ['runner', config.runnerImage, 'backend/docker/runner.Dockerfile'],
  ] as const) {
    requireThat(
      image !== undefined || fs.existsSync(path.join(repoRoot, file)),
      `${file} not found under ${repoRoot}; the ${name} image is built from it (or pass -c ${name}ImageUri=<ecr uri> to deploy a prebuilt image)`,
    );
  }
  return config;
}
