import { CfnOutput, Duration, Stack, StackProps } from 'aws-cdk-lib';
import * as iam from 'aws-cdk-lib/aws-iam';
import { Construct } from 'constructs';
import type { StageConfig } from '../config';
import { acknowledgeFindings, nagAccount, nagArn, nagRegion } from '../nag';

export interface GithubOidcStackProps extends StackProps {
  readonly config: StageConfig;
}

const GITHUB_ISSUER = 'token.actions.githubusercontent.com';

/**
 * GitHub Actions → AWS without long-lived keys.
 *
 * Deployed once per account/stage by an administrator (it is *not* part of
 * `cdk deploy --all` in CI: a pipeline must not be able to widen its own
 * trust). The deploy role can do exactly one thing: assume the CDK bootstrap
 * roles (deploy, file/image publishing, lookup) of this account and region.
 * CloudFormation then acts through the bootstrap execution role, whose
 * policies are chosen at `cdk bootstrap` time.
 */
export class GithubOidcStack extends Stack {
  constructor(scope: Construct, id: string, props: GithubOidcStackProps) {
    super(scope, id, props);
    const { config } = props;

    // The provider is account-global: create it once, import it elsewhere.
    const providerArn = config.githubOidcProviderArn
      ? config.githubOidcProviderArn
      : new iam.OidcProviderNative(this, 'GithubProvider', {
          url: `https://${GITHUB_ISSUER}`,
          clientIds: ['sts.amazonaws.com'],
        }).oidcProviderArn;

    const role = new iam.Role(this, 'DeployRole', {
      roleName: `${config.prefix}-github-deploy`,
      description: `GitHub Actions deploys of ${config.githubRepo} to ${config.stage}`,
      maxSessionDuration: Duration.hours(2),
      assumedBy: new iam.WebIdentityPrincipal(providerArn, {
        StringEquals: { [`${GITHUB_ISSUER}:aud`]: 'sts.amazonaws.com' },
        // Default: only jobs running in the GitHub environment named after the
        // stage (which is where required reviewers are enforced for prod).
        StringLike: { [`${GITHUB_ISSUER}:sub`]: config.githubSubject },
      }),
      inlinePolicies: {
        AssumeCdkBootstrapRoles: new iam.PolicyDocument({
          statements: [
            new iam.PolicyStatement({
              actions: ['sts:AssumeRole', 'sts:TagSession'],
              resources: [`arn:${this.partition}:iam::${this.account}:role/cdk-${config.cdkQualifier}-*-${this.account}-${this.region}`],
              conditions: {
                StringEquals: { 'iam:ResourceTag/aws-cdk:bootstrap-role': ['deploy', 'file-publishing', 'image-publishing', 'lookup'] },
              },
            }),
          ],
        }),
        SmokeTest: new iam.PolicyDocument({
          statements: [
            new iam.PolicyStatement({
              // Post-deploy check that every API target passes /readyz.
              actions: ['elasticloadbalancing:DescribeTargetHealth'],
              resources: ['*'],
            }),
          ],
        }),
      },
    });
    acknowledgeFindings(
      role,
      'AwsSolutions-IAM5',
      [
        `Resource::${nagArn(this, 'iam', `role/cdk-${config.cdkQualifier}-*-${nagAccount(this)}-${nagRegion(this)}`, { global: true })}`,
        'Resource::*',
      ],
      'The wildcard selects only the CDK bootstrap roles of this account/region, further restricted by their aws-cdk:bootstrap-role tag; ' +
        'elasticloadbalancing:DescribeTargetHealth (read-only) supports no resource-level permissions.',
    );

    new CfnOutput(this, 'DeployRoleArn', { value: role.roleArn, description: 'Set as the AWS_DEPLOY_ROLE_ARN variable of the GitHub environment' });
  }
}
