import * as fs from 'node:fs';
import * as path from 'node:path';
import { App } from 'aws-cdk-lib';
import { Template } from 'aws-cdk-lib/assertions';
import { describe, expect, it } from 'vitest';
import { loadStageConfig } from '../lib/config';
import { buildPlatform } from '../lib/platform';
import { FIXTURE_CONTEXT } from './helpers';

const INFRA_DIR = path.resolve(__dirname, '..');
const FLAGS = JSON.parse(fs.readFileSync(path.join(INFRA_DIR, 'cdk.json'), 'utf8')).context;

function oidcTemplate(context: Record<string, unknown>) {
  const app = new App({ context: { ...FLAGS, ...FIXTURE_CONTEXT, githubOidc: 'true', ...context } });
  const platform = buildPlatform(app, loadStageConfig(app, INFRA_DIR));
  return Template.fromStack(platform.githubOidc!).toJSON() as { Resources: Record<string, any> };
}

describe('GitHub OIDC deploy role', () => {
  it('trusts only the repository environment of its stage and can only assume CDK bootstrap roles', () => {
    const t = oidcTemplate({ stage: 'prod' });
    const provider = Object.values(t.Resources).find((r) => r.Type === 'AWS::IAM::OIDCProvider');
    expect(provider.Properties.Url).toBe('https://token.actions.githubusercontent.com');
    const role = Object.values(t.Resources).find((r) => r.Type === 'AWS::IAM::Role').Properties;
    const trust = role.AssumeRolePolicyDocument.Statement;
    expect(trust).toHaveLength(1);
    expect(trust[0].Action).toBe('sts:AssumeRoleWithWebIdentity');
    expect(trust[0].Condition).toEqual({
      StringEquals: { 'token.actions.githubusercontent.com:aud': 'sts.amazonaws.com' },
      StringLike: { 'token.actions.githubusercontent.com:sub': 'repo:DeepakGiri2/DSARust:environment:prod' },
    });
    const statements = role.Policies.flatMap((p: any) => p.PolicyDocument.Statement);
    const actions = statements.flatMap((s: any) => [s.Action].flat());
    expect(actions.sort()).toEqual(['elasticloadbalancing:DescribeTargetHealth', 'sts:AssumeRole', 'sts:TagSession']);
    const assume = statements.find((s: any) => [s.Action].flat().includes('sts:AssumeRole'));
    expect(JSON.stringify(assume.Resource)).toContain('role/cdk-hnb659fds-*-');
    expect(assume.Condition.StringEquals['iam:ResourceTag/aws-cdk:bootstrap-role']).toEqual(['deploy', 'file-publishing', 'image-publishing', 'lookup']);
  });

  it('imports an existing provider instead of creating a second one', () => {
    const t = oidcTemplate({ stage: 'dev', githubOidcProviderArn: 'arn:aws:iam::111111111111:oidc-provider/token.actions.githubusercontent.com', githubRepo: 'acme/dsa' });
    expect(Object.values(t.Resources).some((r) => r.Type === 'AWS::IAM::OIDCProvider')).toBe(false);
    const role = Object.values(t.Resources).find((r) => r.Type === 'AWS::IAM::Role').Properties;
    expect(role.AssumeRolePolicyDocument.Statement[0].Condition.StringLike['token.actions.githubusercontent.com:sub']).toBe('repo:acme/dsa:environment:dev');
  });
});
