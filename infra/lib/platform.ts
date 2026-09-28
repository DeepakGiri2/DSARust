import { App, Stack, Tags, type Environment } from 'aws-cdk-lib';
import type { StageConfig } from './config';
import { enableNag } from './nag';
import { ApiStack } from './stacks/api-stack';
import { DataStack } from './stacks/data-stack';
import { EdgeStack } from './stacks/edge-stack';
import { GithubOidcStack } from './stacks/github-oidc-stack';
import { IngressStack } from './stacks/ingress-stack';
import { NetworkStack } from './stacks/network-stack';
import { ObservabilityStack } from './stacks/observability-stack';
import { RunnerStack } from './stacks/runner-stack';

export interface Platform {
  readonly network: NetworkStack;
  readonly data: DataStack;
  readonly runner: RunnerStack;
  readonly ingress: IngressStack;
  readonly edge: EdgeStack;
  readonly api: ApiStack;
  readonly observability: ObservabilityStack;
  readonly githubOidc?: GithubOidcStack;
  readonly all: Stack[];
}

/**
 * Builds one stage of the platform.
 *
 *   Network ─┬─ Data ──────────────────────────┐
 *            ├─ Runner ────────────────────────┤
 *            └─ Ingress (ALB, WAF) ─ Edge ─────┴─ Api ─ Observability
 *
 * Edge sits between Ingress and Api because CloudFront needs the ALB's name
 * while the API needs CloudFront's URL (DSA_PUBLIC_URL) — splitting the ALB
 * from the service is what keeps that from being a cycle.
 */
export function buildPlatform(app: App, config: StageConfig): Platform {
  const env: Environment = { account: config.account, region: config.region };
  const name = (s: string) => `${config.stackPrefix}-${s}`;
  const common = (s: string, description: string) => ({
    env,
    stackName: name(s),
    description: `DSA Visualized (${config.stage}) — ${description}`,
    terminationProtection: config.isProd,
    config,
  });

  const network = new NetworkStack(app, name('Network'), common('Network', 'VPC, endpoints, flow logs, security groups'));
  const data = new DataStack(app, name('Data'), {
    ...common('Data', 'PostgreSQL, Valkey/Redis, KMS'),
    vpc: network.vpc,
    securityGroups: network.securityGroups,
  });
  const runner = new RunnerStack(app, name('Runner'), {
    ...common('Runner', 'sandboxed code runner (Lambda, no network)'),
    vpc: network.vpc,
    securityGroup: network.securityGroups.runner,
  });
  const ingress = new IngressStack(app, name('Ingress'), {
    ...common('Ingress', 'CloudFront-only ALB and regional WAF'),
    vpc: network.vpc,
    securityGroups: network.securityGroups,
  });
  const edge = new EdgeStack(app, name('Edge'), {
    ...common('Edge', 'CloudFront, SPA bucket, DNS'),
    apiOriginDomainName: ingress.originDomainName,
    apiOriginUsesHttps: ingress.originUsesHttps,
    originVerifySecretName: ingress.originVerifySecretName,
  });
  // With a custom domain Edge holds no token from Ingress, but it resolves the
  // origin-verify secret by name at deploy time, so Ingress must exist first.
  edge.addStackDependency(ingress);
  const api = new ApiStack(app, name('Api'), {
    ...common('Api', 'dsa-api on Fargate, migrations, SES'),
    vpc: network.vpc,
    securityGroups: network.securityGroups,
    data,
    runnerAlias: runner.alias,
    targetGroup: ingress.targetGroup,
    originVerifySecret: ingress.originVerifySecret,
    publicUrl: edge.publicUrl,
  });
  const observability = new ObservabilityStack(app, name('Observability'), {
    ...common('Observability', 'dashboard, alarms, budget'),
    loadBalancer: ingress.loadBalancer,
    targetGroup: ingress.targetGroup,
    webAclName: ingress.webAcl.name!,
    cluster: api.cluster,
    service: api.service,
    db: data.db,
    cacheReplicationGroupId: data.cacheReplicationGroupId,
    runner: runner.function,
    distribution: edge.distribution,
    sesEnabled: api.emailIdentityName !== undefined,
  });
  const githubOidc = config.githubOidc
    ? new GithubOidcStack(app, name('GithubOidc'), common('GithubOidc', 'GitHub Actions OIDC deploy role'))
    : undefined;

  const all: Stack[] = [network, data, runner, ingress, edge, api, observability, ...(githubOidc ? [githubOidc] : [])];
  for (const stack of all) {
    Tags.of(stack).add('Project', config.project);
    Tags.of(stack).add('Stage', config.stage);
    Tags.of(stack).add('CostCenter', config.costCenter);
  }
  enableNag(app);
  return { network, data, runner, ingress, edge, api, observability, githubOidc, all };
}
