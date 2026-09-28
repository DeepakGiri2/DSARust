import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { estimateMaxConnections, loadStageConfig, parseEcrImageUri } from '../lib/config';
import { DOMAIN_CONTEXT, FIXTURE_CONTEXT } from './helpers';

const INFRA_DIR = path.resolve(__dirname, '..');
const load = (context: Record<string, unknown>) =>
  loadStageConfig({ node: { tryGetContext: (k: string) => ({ ...FIXTURE_CONTEXT, ...context })[k] } }, INFRA_DIR);

describe('stage configuration', () => {
  it('derives stage-specific defaults', () => {
    const dev = load({ stage: 'dev' });
    const prod = load({ stage: 'prod' });
    expect(dev.prefix).toBe('dsa-dev');
    expect(dev.availabilityZones).toEqual(['us-east-1a', 'us-east-1b']);
    expect([dev.natGateways, dev.apiMinTasks, dev.apiMaxTasks, dev.dbMultiAz]).toEqual([1, 1, 2, false]);
    expect(prod.availabilityZones).toHaveLength(3);
    expect([prod.natGateways, prod.apiMinTasks, prod.apiMaxTasks, prod.dbMultiAz]).toEqual([3, 2, 20, true]);
    expect(prod.githubSubject).toBe('repo:DeepakGiri2/DSARust:environment:prod');
    expect(dev.domain).toBeUndefined();
  });

  it('reuses the us-east-1 certificate for the ALB origin name', () => {
    const cfg = load({ stage: 'prod', ...DOMAIN_CONTEXT });
    expect(cfg.domain).toMatchObject({ originDomainName: 'origin.dsa.example.com', albCertificateArn: DOMAIN_CONTEXT.certificateArn });
  });

  it.each([
    [{ runnerMemoryMiB: '1024' }, /runnerMemoryMiB/],
    [{ domainName: 'dsa.example.com' }, /certificateArn/],
    [{ ...DOMAIN_CONTEXT, certificateArn: 'arn:aws:acm:eu-west-1:111111111111:certificate/abc' }, /us-east-1/],
    [{ ...DOMAIN_CONTEXT, hostedZoneName: 'other.example.org' }, /not inside hosted zone/],
    [{ apiCpu: '512', apiMemoryMiB: '512' }, /apiMemoryMiB/],
    [{ apiEnv: '{"STRIPE_SECRET_KEY":"sk_live"}' }, /not overridable/],
    [{ region: 'eu-west-1' }, /cloudFrontPrefixListId/],
    [{ stage: 'prod', dbInstanceClass: 't4g.micro' }, /max_connections/],
    [{ runnerReservedConcurrency: '0' }, /runnerReservedConcurrency/],
    [{ stage: 'staging' }, /stage must be one of/],
  ])('rejects %j', (context, message) => {
    expect(() => load({ stage: 'dev', ...context })).toThrow(message);
  });

  it('accepts allow-listed API overrides and prebuilt image URIs', () => {
    const cfg = load({
      apiEnv: '{"ADMIN_EMAILS":"me@example.com","SIGNUP_ENABLED":false}',
      apiImageUri: '111111111111.dkr.ecr.us-east-1.amazonaws.com/dsa-api:1.2.3',
    });
    expect(cfg.apiEnv).toEqual({ ADMIN_EMAILS: 'me@example.com', SIGNUP_ENABLED: 'false' });
    expect(cfg.apiImage).toEqual({ repositoryName: 'dsa-api', tagOrDigest: '1.2.3' });
    expect(parseEcrImageUri(`111111111111.dkr.ecr.us-east-1.amazonaws.com/team/runner@sha256:${'a'.repeat(64)}`, 'x')).toEqual({
      repositoryName: 'team/runner',
      tagOrDigest: `sha256:${'a'.repeat(64)}`,
    });
  });

  it('requires the Dockerfiles unless prebuilt images are given', () => {
    expect(() => load({ repoRoot: 'test' })).toThrow(/api\.Dockerfile not found/);
  });

  it('estimates max_connections from the instance memory', () => {
    expect(estimateMaxConnections('t4g.micro')).toBeLessThan(120);
    expect(estimateMaxConnections('m7g.large')).toBeGreaterThan(600);
    expect(estimateMaxConnections('r7g.large')).toBeGreaterThan(estimateMaxConnections('m7g.large'));
  });
});
