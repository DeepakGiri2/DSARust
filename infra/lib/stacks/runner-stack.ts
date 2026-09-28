import { ArnFormat, CfnElement, Duration, IgnoreMode, RemovalPolicy, Size, Stack, StackProps } from 'aws-cdk-lib';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as ecr from 'aws-cdk-lib/aws-ecr';
import { Platform } from 'aws-cdk-lib/aws-ecr-assets';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as logs from 'aws-cdk-lib/aws-logs';
import { Construct } from 'constructs';
import { DOCKER_CONTEXT_EXCLUDES, type StageConfig } from '../config';
import { acknowledgeFindings, nagArn } from '../nag';

export interface RunnerStackProps extends StackProps {
  readonly config: StageConfig;
  readonly vpc: ec2.IVpc;
  readonly securityGroup: ec2.ISecurityGroup;
}

/**
 * dsa-runner: executes untrusted user code as a Lambda container function.
 *
 * Isolation is layered so that no single mistake opens a path out:
 *  - isolated subnets (no route to a NAT or internet gateway),
 *  - a security group with no ingress and no egress rules,
 *  - an execution role that can write its own log group and manage its own
 *    ENIs in those subnets — nothing else. Lambda exposes the role's
 *    credentials to the process environment, so they must be worthless: the
 *    ENI permissions are for Lambda's own VPC attachment and are explicitly
 *    denied to anything signed with the function's credentials.
 * The API reaches it only through the Lambda Invoke API (payload = job).
 */
export class RunnerStack extends Stack {
  public readonly function: lambda.DockerImageFunction;
  /** The API invokes this alias (provisioned concurrency, when enabled, lives here). */
  public readonly alias: lambda.Alias;

  constructor(scope: Construct, id: string, props: RunnerStackProps) {
    super(scope, id, props);
    const { config, vpc, securityGroup } = props;
    const functionName = `${config.prefix}-runner`;

    const code = config.runnerImage
      ? lambda.DockerImageCode.fromEcr(
          ecr.Repository.fromRepositoryName(this, 'RunnerRepository', config.runnerImage.repositoryName),
          { tagOrDigest: config.runnerImage.tagOrDigest },
        )
      : lambda.DockerImageCode.fromImageAsset(config.repoRoot, {
          file: 'backend/docker/runner.Dockerfile',
          // The shipped stage, never the sandbox-test image built beside it.
          target: 'runtime',
          // Lambda runs this image on x86_64 (the toolchains in it are x86_64).
          platform: Platform.LINUX_AMD64,
          exclude: DOCKER_CONTEXT_EXCLUDES,
          ignoreMode: IgnoreMode.DOCKER,
          displayName: `${config.prefix}-runner`,
        });

    const logGroup = new logs.LogGroup(this, 'Logs', {
      logGroupName: `/aws/lambda/${functionName}`,
      retention: config.logRetentionDays,
      removalPolicy: config.isProd ? RemovalPolicy.RETAIN : RemovalPolicy.DESTROY,
    });

    const isolatedSubnets = vpc.selectSubnets({ subnetType: ec2.SubnetType.PRIVATE_ISOLATED }).subnets;
    const subnetArns = isolatedSubnets.map((s) => this.formatArn({ service: 'ec2', resource: 'subnet', resourceName: s.subnetId }));
    const networkInterfaces = this.formatArn({ service: 'ec2', resource: 'network-interface', resourceName: '*' });
    // Built from the name, not taken from the function, so the role does not
    // depend on the function that depends on it.
    const functionArn = this.formatArn({
      service: 'lambda',
      resource: 'function',
      resourceName: functionName,
      arnFormat: ArnFormat.COLON_RESOURCE_NAME,
    });

    const role = new iam.Role(this, 'Role', {
      roleName: `${config.prefix}-runner`,
      description: 'dsa-runner execution role: own log group + own VPC ENIs only',
      assumedBy: new iam.ServicePrincipal('lambda.amazonaws.com'),
      inlinePolicies: {
        Logs: new iam.PolicyDocument({
          statements: [
            new iam.PolicyStatement({
              actions: ['logs:CreateLogStream', 'logs:PutLogEvents'],
              resources: [`${logGroup.logGroupArn}:log-stream:*`, logGroup.logGroupArn],
            }),
          ],
        }),
        // The permissions Lambda needs to attach the function to the VPC
        // (what AWSLambdaVPCAccessExecutionRole grants), scoped as tightly as
        // EC2 allows: creation only in the isolated subnets with the runner
        // group, and mutation only of ENIs inside this VPC.
        VpcAccess: new iam.PolicyDocument({
          statements: [
            new iam.PolicyStatement({
              actions: ['ec2:CreateNetworkInterface'],
              resources: [networkInterfaces, ...subnetArns, this.formatArn({ service: 'ec2', resource: 'security-group', resourceName: securityGroup.securityGroupId })],
            }),
            new iam.PolicyStatement({
              actions: ['ec2:DeleteNetworkInterface', 'ec2:AssignPrivateIpAddresses', 'ec2:UnassignPrivateIpAddresses'],
              resources: [networkInterfaces],
              conditions: {
                ArnEquals: { 'ec2:Vpc': this.formatArn({ service: 'ec2', resource: 'vpc', resourceName: vpc.vpcId }) },
              },
            }),
            new iam.PolicyStatement({
              // Describe* calls have no resource-level permissions in EC2.
              actions: ['ec2:DescribeNetworkInterfaces', 'ec2:DescribeSubnets'],
              resources: ['*'],
            }),
            new iam.PolicyStatement({
              // The pattern AWS documents for keeping function code from using
              // the VPC permissions: requests signed with the function's
              // credentials carry lambda:SourceFunctionArn, while Lambda's own
              // ENI management does not, so the attachment keeps working.
              sid: 'DenyEniActionsToFunctionCode',
              effect: iam.Effect.DENY,
              actions: [
                'ec2:CreateNetworkInterface',
                'ec2:DeleteNetworkInterface',
                'ec2:DescribeNetworkInterfaces',
                'ec2:DescribeSubnets',
                'ec2:DetachNetworkInterface',
                'ec2:AssignPrivateIpAddresses',
                'ec2:UnassignPrivateIpAddresses',
              ],
              resources: ['*'],
              conditions: { ArnEquals: { 'lambda:SourceFunctionArn': functionArn } },
            }),
          ],
        }),
      },
    });
    acknowledgeFindings(
      role,
      'AwsSolutions-IAM5',
      [
        'Resource::*',
        `Resource::<${this.getLogicalId(logGroup.node.defaultChild as CfnElement)}.Arn>:log-stream:*`,
        `Resource::${nagArn(this, 'ec2', 'network-interface/*')}`,
      ],
      'Log-stream names and Lambda-managed ENI ids are generated at runtime, so those ARNs end in "*" (still bound to this log group / this VPC); ' +
        'ec2:Describe* does not support resource-level permissions.',
    );

    this.function = new lambda.DockerImageFunction(this, 'Function', {
      functionName,
      description: 'dsa-runner: sandboxed execution of user programs (network-less)',
      code,
      architecture: lambda.Architecture.X86_64,
      memorySize: config.runnerMemoryMiB,
      ephemeralStorageSize: Size.mebibytes(config.runnerEphemeralStorageMiB),
      timeout: Duration.seconds(config.runnerTimeoutSeconds),
      reservedConcurrentExecutions: config.runnerReservedConcurrency,
      role,
      vpc,
      vpcSubnets: { subnetType: ec2.SubnetType.PRIVATE_ISOLATED },
      securityGroups: [securityGroup],
      logGroup,
      loggingFormat: lambda.LoggingFormat.JSON,
      tracing: lambda.Tracing.DISABLED,
      environment: {
        LOG_FORMAT: 'json',
        RUST_LOG: 'info',
        // /tmp is the only writable path in Lambda.
        RUNNER_WORK_DIR: '/tmp/dsa-runner',
        RUNNER_MAX_RUN_TIMEOUT_MS: '10000',
        RUNNER_MAX_COMPILE_TIMEOUT_MS: '30000',
        RUNNER_MAX_MEMORY_MB: '512',
      },
    });

    this.alias = new lambda.Alias(this, 'Live', {
      aliasName: 'live',
      version: this.function.currentVersion,
      description: 'Invoked by dsa-api (RUNNER_LAMBDA_FUNCTION)',
      provisionedConcurrentExecutions: config.runnerProvisionedConcurrency > 0 ? config.runnerProvisionedConcurrency : undefined,
    });
  }
}
