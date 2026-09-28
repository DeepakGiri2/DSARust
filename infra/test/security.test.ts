/**
 * Security-critical properties of the synthesized platform, asserted on the
 * CloudFormation templates for dev, prod, and prod with a custom domain and
 * RDS Proxy. If one of these fails, a change weakened an isolation boundary.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { beforeAll, describe, expect, it } from 'vitest';
import { AwsSolutionsChecks } from 'cdk-nag';
import { buildApp, DOMAIN_CONTEXT, logicalId, refersTo, resourcesOfType, type Built, type TemplateJson } from './helpers';

const CACHING_DISABLED = '4135ea2d-6df8-44a3-9df3-4b5a84be39ad';
const ALL_VIEWER_EXCEPT_HOST_HEADER = 'b689b0a8-53d0-40ab-baf2-68738e2966ac';
const CLOUDFRONT_PREFIX_LIST_US_EAST_1 = 'pl-3b927c52';
/** What CDK writes for allowAllOutbound=false with no rules: matches no packet. */
const NO_TRAFFIC_SENTINEL = { CidrIp: '255.255.255.255/32', Description: 'Disallow all traffic', FromPort: 252, IpProtocol: 'icmp', ToPort: 86 };

const VARIANTS: Array<[string, Record<string, unknown>]> = [
  ['dev', { stage: 'dev' }],
  ['prod', { stage: 'prod' }],
  ['prod + domain + RDS Proxy', { stage: 'prod', ...DOMAIN_CONTEXT, rdsProxy: 'true', alarmEmail: 'ops@example.com', aiModelId: 'us.anthropic.claude-opus-5' }],
];

/** Every ingress rule (inline or standalone) whose target group is `sgId`. */
function ingressRules(t: TemplateJson, sgId: string): any[] {
  const inline = (t.Resources[sgId]?.Properties?.SecurityGroupIngress ?? []) as any[];
  const standalone = resourcesOfType(t, 'AWS::EC2::SecurityGroupIngress')
    .filter(([, r]) => refersTo(r.Properties.GroupId, sgId))
    .map(([, r]) => r.Properties);
  return [...inline, ...standalone];
}

function egressRules(t: TemplateJson, sgId: string): any[] {
  const inline = (t.Resources[sgId]?.Properties?.SecurityGroupEgress ?? []) as any[];
  const standalone = resourcesOfType(t, 'AWS::EC2::SecurityGroupEgress')
    .filter(([, r]) => refersTo(r.Properties.GroupId, sgId))
    .map(([, r]) => r.Properties);
  return [...inline, ...standalone];
}

function statementsOfRole(t: TemplateJson, roleId: string): any[] {
  const role = t.Resources[roleId].Properties;
  const inline = (role.Policies ?? []).flatMap((p: any) => p.PolicyDocument.Statement);
  const attached = resourcesOfType(t, 'AWS::IAM::Policy')
    .filter(([, p]) => (p.Properties.Roles ?? []).some((r: unknown) => refersTo(r, roleId)))
    .flatMap(([, p]) => p.Properties.PolicyDocument.Statement);
  return [...inline, ...attached];
}

const asArray = <T>(v: T | T[] | undefined): T[] => (v === undefined ? [] : Array.isArray(v) ? v : [v]);

describe.each(VARIANTS)('%s', (_name, context) => {
  let b: Built;
  let sg: Record<string, string>;
  const withDomain = 'domainName' in context;
  const withProxy = context.rdsProxy === 'true';

  beforeAll(() => {
    b = buildApp(context);
    const groups = b.platform.network.securityGroups;
    sg = Object.fromEntries(Object.entries(groups).map(([k, g]) => [k, logicalId(g)]));
  });

  it('passes the cdk-nag AwsSolutions pack with no unacknowledged finding', () => {
    const report = new AwsSolutionsChecks(b.app, { verbose: false }).validateScope(b.app);
    const summary = report.violations.map((v) => `${v.ruleName}: ${v.violatingResources.map((r) => r.resourceLogicalId ?? r.locations?.join(',')).join(' ')}`);
    expect(summary).toEqual([]);
  });

  describe('runner', () => {
    it('has a security group with no ingress and no egress', () => {
      const net = b.json.network;
      expect(net.Resources[sg.runner].Properties.SecurityGroupEgress).toEqual([NO_TRAFFIC_SENTINEL]);
      expect(net.Resources[sg.runner].Properties.SecurityGroupIngress ?? []).toEqual([]);
      for (const type of ['AWS::EC2::SecurityGroupIngress', 'AWS::EC2::SecurityGroupEgress']) {
        for (const [id, r] of resourcesOfType(net, type)) {
          expect(refersTo(r.Properties, sg.runner), `${id} references the runner security group`).toBe(false);
        }
      }
    });

    it('runs in the isolated subnets only, as an x86_64 function with ≥ 2 GiB and 60 s', () => {
      const [, fn] = resourcesOfType(b.json.runner, 'AWS::Lambda::Function').find(([, r]) => r.Properties.PackageType === 'Image')!;
      const vpc = fn.Properties.VpcConfig;
      expect(vpc.SubnetIds).toHaveLength(b.platform.network.vpc.isolatedSubnets.length);
      for (const subnet of b.platform.network.vpc.isolatedSubnets) {
        expect(vpc.SubnetIds.some((s: unknown) => refersTo(s, logicalId(subnet)))).toBe(true);
      }
      expect(JSON.stringify(vpc.SubnetIds)).not.toMatch(/privateSubnet|publicSubnet/);
      expect(vpc.SecurityGroupIds).toHaveLength(1);
      expect(refersTo(vpc.SecurityGroupIds[0], sg.runner)).toBe(true);
      expect(fn.Properties.Architectures).toEqual(['x86_64']);
      expect(fn.Properties.MemorySize).toBeGreaterThanOrEqual(2048);
      expect(fn.Properties.Timeout).toBeGreaterThanOrEqual(60);
    });

    it('has a role limited to its own logs and VPC ENIs', () => {
      const [, fn] = resourcesOfType(b.json.runner, 'AWS::Lambda::Function').find(([, r]) => r.Properties.PackageType === 'Image')!;
      const roleId = fn.Properties.Role['Fn::GetAtt'][0];
      const role = b.json.runner.Resources[roleId].Properties;
      expect(role.ManagedPolicyArns ?? []).toEqual([]);
      const statements = statementsOfRole(b.json.runner, roleId);
      const actions = statements.filter((s) => s.Effect === 'Allow').flatMap((s) => asArray(s.Action));
      const allowed = [
        'logs:CreateLogStream', 'logs:PutLogEvents',
        'ec2:CreateNetworkInterface', 'ec2:DeleteNetworkInterface', 'ec2:DescribeNetworkInterfaces', 'ec2:DescribeSubnets',
        'ec2:AssignPrivateIpAddresses', 'ec2:UnassignPrivateIpAddresses',
      ];
      expect(actions.length).toBeGreaterThan(0);
      for (const a of actions) expect(allowed).toContain(a);
      expect(statements.every((s) => s.Effect === 'Allow' || s.Effect === 'Deny')).toBe(true);
    });

    it('builds the runtime stage of the runner image, never the sandbox-test stage', () => {
      const assembly = b.app.synth();
      const file = path.join(assembly.directory, `${b.platform.runner.artifactId}.assets.json`);
      const manifest = JSON.parse(fs.readFileSync(file, 'utf8'));
      const images = Object.values<any>(manifest.dockerImages ?? {}).filter(
        (d) => d.source.dockerFile === 'backend/docker/runner.Dockerfile',
      );
      expect(images).toHaveLength(1);
      expect(images[0].source.dockerBuildTarget).toBe('runtime');
    });

    it("denies every ENI action to calls made with the function's own credentials", () => {
      const [, fn] = resourcesOfType(b.json.runner, 'AWS::Lambda::Function').find(([, r]) => r.Properties.PackageType === 'Image')!;
      const roleId = fn.Properties.Role['Fn::GetAtt'][0];
      const statements = statementsOfRole(b.json.runner, roleId);
      const denies = statements.filter((s) => s.Effect === 'Deny');
      expect(denies).toHaveLength(1);
      const [deny] = denies;
      const eniActions = statements
        .filter((s) => s.Effect === 'Allow')
        .flatMap((s) => asArray<string>(s.Action))
        .filter((a) => a.startsWith('ec2:'));
      for (const a of [...eniActions, 'ec2:DetachNetworkInterface']) expect(asArray(deny.Action)).toContain(a);
      expect(asArray(deny.Resource)).toEqual(['*']);
      // Bound to this function's (unqualified) ARN, built from its name.
      const source = JSON.stringify(deny.Condition.ArnEquals['lambda:SourceFunctionArn']);
      expect(source).toContain(`:function:${fn.Properties.FunctionName}`);
      expect(Object.keys(deny.Condition)).toEqual(['ArnEquals']);
    });
  });

  describe('data tier', () => {
    it('RDS is private, encrypted with the data key, forces TLS, and admits only the API/migration tiers', () => {
      const [, db] = resourcesOfType(b.json.data, 'AWS::RDS::DBInstance')[0];
      expect(db.Properties.PubliclyAccessible).toBe(false);
      expect(db.Properties.StorageEncrypted).toBe(true);
      expect(refersTo(db.Properties.KmsKeyId, logicalId(b.platform.data.key))).toBe(true);
      expect(db.Properties.EngineVersion).toBe('17');
      const [, params] = resourcesOfType(b.json.data, 'AWS::RDS::DBParameterGroup')[0];
      expect(params.Properties.Parameters['rds.force_ssl']).toBe('1');
      if (context.stage === 'prod') {
        expect(db.Properties.MultiAZ).toBe(true);
        expect(db.Properties.DeletionProtection).toBe(true);
        expect(resourcesOfType(b.json.data, 'AWS::RDS::DBInstance')[0][1].DeletionPolicy).toBe('Snapshot');
      }

      const rules = ingressRules(b.json.network, sg.db);
      const allowedSources = withProxy ? [sg.migrate, sg.dbProxy] : [sg.api, sg.migrate];
      expect(rules.length).toBe(2);
      for (const r of rules) {
        expect(r.CidrIp ?? r.CidrIpv6 ?? r.SourcePrefixListId).toBeUndefined();
        expect(allowedSources.some((s) => refersTo(r.SourceSecurityGroupId, s))).toBe(true);
        expect([r.FromPort, r.ToPort]).toEqual([5432, 5432]);
      }
      if (withProxy) {
        const proxyRules = ingressRules(b.json.network, sg.dbProxy);
        expect(proxyRules).toHaveLength(1);
        expect(refersTo(proxyRules[0].SourceSecurityGroupId, sg.api)).toBe(true);
      }
    });

    it('the cache requires TLS and AUTH, is encrypted at rest, and admits only the API', () => {
      const [, cache] = resourcesOfType(b.json.data, 'AWS::ElastiCache::ReplicationGroup')[0];
      expect(cache.Properties.TransitEncryptionEnabled).toBe(true);
      expect(cache.Properties.TransitEncryptionMode).toBe('required');
      expect(cache.Properties.AtRestEncryptionEnabled).toBe(true);
      expect(cache.Properties.KmsKeyId).toBeDefined();
      expect(JSON.stringify(cache.Properties.AuthToken)).toContain('{{resolve:secretsmanager:');
      expect(refersTo(cache.Properties.SecurityGroupIds, sg.cache)).toBe(true);
      const rules = ingressRules(b.json.network, sg.cache);
      expect(rules).toHaveLength(1);
      expect(refersTo(rules[0].SourceSecurityGroupId, sg.api)).toBe(true);
      expect([rules[0].FromPort, rules[0].ToPort]).toEqual([6379, 6379]);
      const [, subnets] = resourcesOfType(b.json.data, 'AWS::ElastiCache::SubnetGroup')[0];
      expect(JSON.stringify(subnets.Properties.SubnetIds)).toMatch(/isolatedSubnet/);
      expect(JSON.stringify(subnets.Properties.SubnetIds)).not.toMatch(/privateSubnet|publicSubnet/);
    });
  });

  describe('ingress', () => {
    it('the ALB security group admits only the CloudFront origin-facing prefix list, on one port', () => {
      const rules = ingressRules(b.json.network, sg.alb);
      expect(rules).toHaveLength(1);
      expect(rules[0].SourcePrefixListId).toBe(CLOUDFRONT_PREFIX_LIST_US_EAST_1);
      expect(rules[0].CidrIp ?? rules[0].SourceSecurityGroupId).toBeUndefined();
      const port = withDomain ? 443 : 80;
      expect([rules[0].FromPort, rules[0].ToPort]).toEqual([port, port]);
      for (const r of egressRules(b.json.network, sg.alb)) {
        expect(refersTo(r.DestinationSecurityGroupId, sg.api)).toBe(true);
      }
    });

    it('the listener forwards only requests carrying the X-Origin-Verify secret; everything else is 403', () => {
      const listeners = resourcesOfType(b.json.ingress, 'AWS::ElasticLoadBalancingV2::Listener');
      const serving = listeners.filter(([, l]) => l.Properties.DefaultActions[0].Type !== 'redirect');
      expect(serving).toHaveLength(1);
      const [listenerId, listener] = serving[0];
      expect(listener.Properties.Port).toBe(withDomain ? 443 : 80);
      expect(listener.Properties.DefaultActions).toEqual([
        { Type: 'fixed-response', FixedResponseConfig: { StatusCode: '403', ContentType: 'text/plain', MessageBody: 'Forbidden' } },
      ]);
      const rules = resourcesOfType(b.json.ingress, 'AWS::ElasticLoadBalancingV2::ListenerRule').filter(([, r]) => refersTo(r.Properties.ListenerArn, listenerId));
      expect(rules).toHaveLength(1);
      const [, rule] = rules[0];
      expect(rule.Properties.Actions[0].Type).toBe('forward');
      expect(rule.Properties.Conditions).toHaveLength(1);
      const cond = rule.Properties.Conditions[0];
      expect(cond.Field).toBe('http-header');
      expect(cond.HttpHeaderConfig.HttpHeaderName).toBe('X-Origin-Verify');
      expect(JSON.stringify(cond.HttpHeaderConfig.Values)).toContain('{{resolve:secretsmanager:');
      for (const [, l] of listeners.filter(([, l]) => l.Properties.DefaultActions[0].Type === 'redirect')) {
        expect(l.Properties.DefaultActions[0].RedirectConfig.Protocol).toBe('HTTPS');
      }
    });

    it('the ALB idles for at least 120 s and health-checks /readyz on 8080', () => {
      const [, alb] = resourcesOfType(b.json.ingress, 'AWS::ElasticLoadBalancingV2::LoadBalancer')[0];
      const idle = alb.Properties.LoadBalancerAttributes.find((a: any) => a.Key === 'idle_timeout.timeout_seconds');
      expect(Number(idle.Value)).toBeGreaterThanOrEqual(120);
      const [, tg] = resourcesOfType(b.json.ingress, 'AWS::ElasticLoadBalancingV2::TargetGroup')[0];
      expect(tg.Properties.HealthCheckPath).toBe('/readyz');
      expect(tg.Properties.Port).toBe(8080);
    });

    it('a regional WAF with managed rule groups and viewer-IP rate limits is associated with the ALB', () => {
      const [aclId, acl] = resourcesOfType(b.json.ingress, 'AWS::WAFv2::WebACL')[0];
      expect(acl.Properties.Scope).toBe('REGIONAL');
      const [, assoc] = resourcesOfType(b.json.ingress, 'AWS::WAFv2::WebACLAssociation')[0];
      expect(assoc.Properties.ResourceArn).toEqual({ Ref: logicalId(b.platform.ingress.loadBalancer) });
      expect(assoc.Properties.WebACLArn).toEqual({ 'Fn::GetAtt': [aclId, 'Arn'] });
      const groups = acl.Properties.Rules.map((r: any) => r.Statement.ManagedRuleGroupStatement?.Name).filter(Boolean);
      expect(groups).toEqual(expect.arrayContaining(['AWSManagedRulesCommonRuleSet', 'AWSManagedRulesKnownBadInputsRuleSet', 'AWSManagedRulesSQLiRuleSet']));
      const rate = acl.Properties.Rules.filter((r: any) => r.Statement.RateBasedStatement).map((r: any) => r.Statement.RateBasedStatement);
      expect(rate).toHaveLength(2);
      for (const r of rate) expect(r.CustomKeys).toEqual([{ Header: { Name: 'x-viewer-ip', TextTransformations: [{ Priority: 0, Type: 'NONE' }] } }]);
      expect(JSON.stringify(rate)).toContain('/api/v1/auth/');
    });
  });

  describe('edge', () => {
    it('caches /api/v1/content/* on path + query only, never caches the rest of /api/*', () => {
      const [, dist] = resourcesOfType(b.json.edge, 'AWS::CloudFront::Distribution')[0];
      const behaviors = dist.Properties.DistributionConfig.CacheBehaviors as any[];
      const patterns = behaviors.map((x) => x.PathPattern);
      expect(patterns.indexOf('/api/v1/content/*')).toBeGreaterThanOrEqual(0);
      expect(patterns.indexOf('/api/v1/content/*')).toBeLessThan(patterns.indexOf('/api/*'));

      const api = behaviors.find((x) => x.PathPattern === '/api/*');
      expect(api.CachePolicyId).toBe(CACHING_DISABLED);
      expect(api.OriginRequestPolicyId).toBe(ALL_VIEWER_EXCEPT_HOST_HEADER);
      expect(api.AllowedMethods).toEqual(expect.arrayContaining(['POST', 'PUT', 'PATCH', 'DELETE']));

      const content = behaviors.find((x) => x.PathPattern === '/api/v1/content/*');
      const policy = b.json.edge.Resources[content.CachePolicyId.Ref].Properties.CachePolicyConfig;
      expect(policy.MaxTTL).toBeGreaterThan(0);
      expect(policy.ParametersInCacheKeyAndForwardedToOrigin.CookiesConfig.CookieBehavior).toBe('none');
      expect(policy.ParametersInCacheKeyAndForwardedToOrigin.HeadersConfig.HeaderBehavior).toBe('none');
      expect(policy.ParametersInCacheKeyAndForwardedToOrigin.QueryStringsConfig.QueryStringBehavior).toBe('all');
      const orp = b.json.edge.Resources[content.OriginRequestPolicyId.Ref].Properties.OriginRequestPolicyConfig;
      expect(orp.CookiesConfig.CookieBehavior).toBe('none');

      const assets = behaviors.find((x) => x.PathPattern === '/assets/*');
      expect(b.json.edge.Resources[assets.CachePolicyId.Ref].Properties.CachePolicyConfig.DefaultTTL).toBe(365 * 24 * 3600);
    });

    it('sends X-Origin-Verify from a secret to the API origin and serves the SPA from a private bucket via OAC', () => {
      const [, dist] = resourcesOfType(b.json.edge, 'AWS::CloudFront::Distribution')[0];
      const cfg = dist.Properties.DistributionConfig;
      const apiOrigin = cfg.Origins.find((o: any) => o.CustomOriginConfig);
      const header = apiOrigin.OriginCustomHeaders.find((h: any) => h.HeaderName === 'X-Origin-Verify');
      expect(JSON.stringify(header.HeaderValue)).toContain('{{resolve:secretsmanager:');
      expect(apiOrigin.CustomOriginConfig.OriginProtocolPolicy).toBe(withDomain ? 'https-only' : 'http-only');
      const s3Origin = cfg.Origins.find((o: any) => o.S3OriginConfig);
      expect(s3Origin.OriginAccessControlId).toBeDefined();
      expect(cfg.DefaultCacheBehavior.FunctionAssociations[0].EventType).toBe('viewer-request');

      const [, bucket] = resourcesOfType(b.json.edge, 'AWS::S3::Bucket').find(([id]) => id.startsWith('SiteBucket'))!;
      expect(bucket.Properties.PublicAccessBlockConfiguration).toEqual({
        BlockPublicAcls: true, BlockPublicPolicy: true, IgnorePublicAcls: true, RestrictPublicBuckets: true,
      });
      expect(bucket.Properties.BucketEncryption).toBeDefined();
    });

    it('adds security headers (CSP, frame denial, nosniff, referrer policy; HSTS in prod)', () => {
      const [, policy] = resourcesOfType(b.json.edge, 'AWS::CloudFront::ResponseHeadersPolicy')[0];
      const sec = policy.Properties.ResponseHeadersPolicyConfig.SecurityHeadersConfig;
      expect(sec.ContentSecurityPolicy.ContentSecurityPolicy).toMatch(/script-src 'self';/);
      expect(sec.ContentSecurityPolicy.ContentSecurityPolicy).toMatch(/frame-ancestors 'none'/);
      expect(sec.ContentSecurityPolicy.ContentSecurityPolicy).toMatch(/connect-src 'self'/);
      expect(sec.FrameOptions.FrameOption).toBe('DENY');
      expect(sec.ContentTypeOptions.Override).toBe(true);
      expect(sec.ReferrerPolicy.ReferrerPolicy).toBe('strict-origin-when-cross-origin');
      expect(sec.StrictTransportSecurity !== undefined).toBe(context.stage === 'prod');
    });
  });

  describe('api', () => {
    const SECRET_NAME = /(PASSWORD|SECRET|TOKEN|API_KEY|PRIVATE|CREDENTIAL)/i;
    const CREDENTIAL_IN_VALUE = /resolve:secretsmanager|:\/\/[^/\s"]*:[^/\s"]*@/;

    it('no task definition carries a secret in plaintext environment', () => {
      for (const [id, td] of resourcesOfType(b.json.api, 'AWS::ECS::TaskDefinition')) {
        for (const c of td.Properties.ContainerDefinitions) {
          for (const e of c.Environment ?? []) {
            expect(e.Name, `${id}/${c.Name}: ${e.Name}`).not.toMatch(SECRET_NAME);
            // No dynamic secret references and no credentials embedded in URLs (scheme://user:pass@host).
            expect(JSON.stringify(e.Value), `${id}/${c.Name}: ${e.Name}`).not.toMatch(CREDENTIAL_IN_VALUE);
          }
          // Secrets are references to Secrets Manager resources, never literal values.
          for (const s of c.Secrets ?? []) expect(typeof s.ValueFrom, `${id}/${c.Name}: ${s.Name}`).toBe('object');
        }
      }
      const [, api] = resourcesOfType(b.json.api, 'AWS::ECS::TaskDefinition').find(([id]) => id.startsWith('ApiTask'))!;
      const container = api.Properties.ContainerDefinitions[0];
      const secretNames = container.Secrets.map((s: any) => s.Name);
      expect(secretNames).toEqual(
        expect.arrayContaining(['DB_USER', 'DB_PASSWORD', 'REDIS_AUTH_TOKEN', 'SESSION_SECRET', 'ORIGIN_VERIFY_SECRET', 'STRIPE_SECRET_KEY', 'OAUTH_GITHUB_CLIENT_SECRET', 'OAUTH_GOOGLE_CLIENT_SECRET']),
      );
      const env = Object.fromEntries(container.Environment.map((e: any) => [e.Name, e.Value]));
      expect(env).toMatchObject({ DSA_ENV: 'production', LOG_FORMAT: 'json', RUNNER_MODE: 'lambda', TRUSTED_PROXY_HOPS: '2', DB_SSLMODE: 'require' });
      expect(env.RUNNER_LAMBDA_FUNCTION).toBeDefined();
      expect(JSON.stringify(env.REDIS_URL)).toContain('rediss://');
      expect(container.ReadonlyRootFilesystem).toBe(true);
      expect(container.User).toBe('10001');
    });

    it('the task role can invoke the runner alias and nothing broader', () => {
      const [, api] = resourcesOfType(b.json.api, 'AWS::ECS::TaskDefinition').find(([id]) => id.startsWith('ApiTask'))!;
      const roleId = api.Properties.TaskRoleArn['Fn::GetAtt'][0];
      const role = b.json.api.Resources[roleId].Properties;
      expect(role.ManagedPolicyArns ?? []).toEqual([]);
      const statements = statementsOfRole(b.json.api, roleId);
      const allowed = new Set(['lambda:InvokeFunction', 'ses:SendEmail', 'ses:SendRawEmail', 'bedrock:InvokeModel', 'bedrock:InvokeModelWithResponseStream', 'cloudwatch:PutMetricData']);
      for (const s of statements) {
        expect(s.Effect).toBe('Allow');
        for (const a of asArray(s.Action)) expect(allowed.has(a as string), `unexpected action ${String(a)}`).toBe(true);
        const resources = asArray(s.Resource);
        if (resources.includes('*')) {
          expect(asArray(s.Action)).toEqual(['cloudwatch:PutMetricData']);
          expect(s.Condition.StringEquals['cloudwatch:namespace']).toBeDefined();
        }
      }
      const invoke = statements.filter((s) => asArray(s.Action).includes('lambda:InvokeFunction'));
      expect(invoke).toHaveLength(1);
      expect(asArray(invoke[0].Resource)).toHaveLength(1);
      expect(refersTo(invoke[0].Resource, logicalId(b.platform.runner.alias))).toBe(true);
    });

    it('runs migrations before the service rolls out', () => {
      const [, service] = resourcesOfType(b.json.api, 'AWS::ECS::Service')[0];
      const [migrationsId] = resourcesOfType(b.json.api, 'Custom::DsaMigrations')[0];
      expect(asArray(service.DependsOn)).toContain(migrationsId);
      expect(service.Properties.DeploymentConfiguration.DeploymentCircuitBreaker).toEqual({ Enable: true, Rollback: true });
      const [, migrate] = resourcesOfType(b.json.api, 'AWS::ECS::TaskDefinition').find(([id]) => id.startsWith('MigrateTask'))!;
      expect(migrate.Properties.ContainerDefinitions[0].Command).toEqual(['migrate']);
      expect(migrate.Properties.ContainerDefinitions[0].Secrets.map((s: any) => s.Name).sort()).toEqual(['DB_PASSWORD', 'DB_USER']);
    });
  });
});
