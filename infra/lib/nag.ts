/**
 * cdk-nag (v3) wiring and the helpers every stack uses to acknowledge a rule.
 *
 * cdk-nag v3 runs as a CDK policy-validation plugin: *any* unacknowledged
 * finding fails `cdk synth`. Acknowledgements match rule ids exactly, and
 * rules that report granular findings (IAM4, IAM5) need the finding id, e.g.
 * `AwsSolutions-IAM5[Resource::*]`. Every acknowledgement in this code base
 * carries the reason it is safe — reviewers should read them as the list of
 * deliberate exceptions to the AWS Solutions baseline.
 */
import { Stack, Token, Validations } from 'aws-cdk-lib';
import { AwsSolutionsChecks } from 'cdk-nag';
import type { IConstruct } from 'constructs';

export function enableNag(app: IConstruct): void {
  Validations.of(app).addPlugins(new AwsSolutionsChecks(app, { verbose: true, writeSuppressionsToCloudFormation: true }));
}

/** Acknowledge one or more rule ids (plain or granular) on a construct subtree. */
export function acknowledge(scope: IConstruct, reason: string, ...ids: string[]): void {
  Validations.of(scope).acknowledge(...ids.map((id) => ({ id, reason })));
}

/** Acknowledge granular findings of one rule, e.g. finding('AwsSolutions-IAM5', 'Resource::*'). */
export function acknowledgeFindings(scope: IConstruct, rule: string, findings: string[], reason: string): void {
  acknowledge(scope, reason, ...findings.map((f) => `${rule}[${f}]`));
}

/**
 * An ARN rendered the way cdk-nag prints it in finding ids: concrete partition,
 * region and account where the stack knows them, `<AWS::…>` placeholders where
 * they are deploy-time tokens.
 */
export function nagArn(
  scope: IConstruct,
  service: string,
  resource: string,
  opts: { region?: string; global?: boolean; noAccount?: boolean } = {},
): string {
  const partition = shown(Stack.of(scope).partition, '<AWS::Partition>');
  const region = opts.global ? '' : (opts.region ?? nagRegion(scope));
  const account = opts.noAccount ? '' : nagAccount(scope);
  return `arn:${partition}:${service}:${region}:${account}:${resource}`;
}

/** The stack account as cdk-nag prints it. */
export function nagAccount(scope: IConstruct): string {
  return shown(Stack.of(scope).account, '<AWS::AccountId>');
}

/** The stack region as cdk-nag prints it. */
export function nagRegion(scope: IConstruct): string {
  return shown(Stack.of(scope).region, '<AWS::Region>');
}

function shown(value: string, placeholder: string): string {
  return Token.isUnresolved(value) ? placeholder : value;
}

/** Common reason strings, so the same exception reads the same everywhere. */
export const REASONS = {
  lambdaBasicExecution:
    'CDK-managed helper Lambda; AWSLambdaBasicExecutionRole only lets it write its own CloudWatch Logs.',
  cdkManagedRuntime:
    'Runtime of a CDK-managed helper Lambda is pinned by aws-cdk-lib and upgraded with it; it runs only during deployments and never handles user traffic.',
  defaultPort:
    'Changing the port is security through obscurity: the endpoint lives in isolated subnets and its security group admits only the named application groups.',
  ecrAuthToken:
    'ecr:GetAuthorizationToken does not support resource-level permissions (AWS requires "*"); image pulls themselves are scoped to the repository.',
} as const;
