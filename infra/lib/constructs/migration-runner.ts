import * as path from 'node:path';
import { CfnElement, CustomResource, Duration, RemovalPolicy, Stack } from 'aws-cdk-lib';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as logs from 'aws-cdk-lib/aws-logs';
import { Construct } from 'constructs';
import { acknowledgeFindings, nagArn } from '../nag';

export interface MigrationRunnerProps {
  readonly prefix: string;
  readonly cluster: ecs.ICluster;
  readonly taskDefinition: ecs.FargateTaskDefinition;
  readonly containerName: string;
  /** Log group and awslogs stream prefix of the migration container. */
  readonly taskLogGroup: logs.ILogGroup;
  readonly taskLogStreamPrefix: string;
  readonly subnets: ec2.ISubnet[];
  readonly securityGroup: ec2.ISecurityGroup;
  readonly logRetentionDays: number;
  readonly removalPolicy: RemovalPolicy;
}

/**
 * Runs `dsa-api migrate` (the migration task definition) during every
 * CloudFormation deployment that changes it, and waits for it to exit 0.
 *
 * Why a custom resource rather than a CI step: the ordering "new image
 * registered → schema migrated → service rolls" is then enforced by
 * CloudFormation for *every* deployer (the pipeline or an operator running
 * `cdk deploy`), a failed migration fails the stack update before any task
 * runs the new code, and there is no second set of CI credentials with
 * ecs:RunTask/iam:PassRole to protect.
 */
export class MigrationRunner extends Construct {
  public readonly resource: CustomResource;

  constructor(scope: Construct, id: string, props: MigrationRunnerProps) {
    super(scope, id);
    const stack = Stack.of(this);
    const functionName = `${props.prefix}-migrations`;
    const taskDef = props.taskDefinition;

    const handlerLogs = new logs.LogGroup(this, 'HandlerLogs', {
      logGroupName: `/aws/lambda/${functionName}`,
      retention: props.logRetentionDays,
      removalPolicy: props.removalPolicy,
    });

    const taskDefinitionFamily = stack.formatArn({ service: 'ecs', resource: 'task-definition', resourceName: `${taskDef.family}:*` });
    const clusterTasks = stack.formatArn({ service: 'ecs', resource: 'task', resourceName: `${props.cluster.clusterName}/*` });
    const clusterCondition = { ArnEquals: { 'ecs:cluster': props.cluster.clusterArn } };
    const passRoles = [taskDef.taskRole.roleArn, taskDef.obtainExecutionRole().roleArn];

    const role = new iam.Role(this, 'HandlerRole', {
      roleName: functionName,
      description: 'Runs the dsa-api migration task during deployments',
      assumedBy: new iam.ServicePrincipal('lambda.amazonaws.com'),
      inlinePolicies: {
        RunMigrations: new iam.PolicyDocument({
          statements: [
            new iam.PolicyStatement({ actions: ['ecs:RunTask'], resources: [taskDefinitionFamily], conditions: clusterCondition }),
            new iam.PolicyStatement({ actions: ['ecs:DescribeTasks', 'ecs:StopTask'], resources: [clusterTasks], conditions: clusterCondition }),
            new iam.PolicyStatement({
              actions: ['iam:PassRole'],
              resources: passRoles,
              conditions: { StringEquals: { 'iam:PassedToService': 'ecs-tasks.amazonaws.com' } },
            }),
            // Rollback detection (see the handler).
            new iam.PolicyStatement({ actions: ['cloudformation:DescribeStacks'], resources: [stack.stackId] }),
            // The migration's last log lines go into the failure reason.
            new iam.PolicyStatement({ actions: ['logs:GetLogEvents'], resources: [`${props.taskLogGroup.logGroupArn}:log-stream:*`] }),
            new iam.PolicyStatement({ actions: ['logs:CreateLogStream', 'logs:PutLogEvents'], resources: [`${handlerLogs.logGroupArn}:log-stream:*`] }),
          ],
        }),
      },
    });

    const handler = new lambda.Function(this, 'Handler', {
      functionName,
      description: 'CloudFormation custom resource: run dsa-api migrate and wait for exit 0',
      runtime: lambda.Runtime.NODEJS_24_X,
      architecture: lambda.Architecture.ARM_64,
      handler: 'index.handler',
      code: lambda.Code.fromAsset(path.join(__dirname, '..', '..', 'lambda', 'migrate')),
      memorySize: 256,
      // Lambda's ceiling; the handler stops the task and reports a minute before it.
      timeout: Duration.minutes(15),
      role,
      logGroup: handlerLogs,
      loggingFormat: lambda.LoggingFormat.JSON,
    });

    this.resource = new CustomResource(this, 'Resource', {
      serviceToken: handler.functionArn,
      resourceType: 'Custom::DsaMigrations',
      // Backstop in case the handler itself dies without answering.
      serviceTimeout: Duration.minutes(20),
      properties: {
        Cluster: props.cluster.clusterArn,
        // A new revision (new image or environment) means "run migrations again".
        TaskDefinition: taskDef.taskDefinitionArn,
        ContainerName: props.containerName,
        Subnets: props.subnets.map((s) => s.subnetId),
        SecurityGroups: [props.securityGroup.securityGroupId],
        LogGroup: props.taskLogGroup.logGroupName,
        LogStreamPrefix: props.taskLogStreamPrefix,
      },
    });
    this.resource.node.addDependency(role);

    const logicalId = (c: Construct) => stack.getLogicalId(c.node.defaultChild as CfnElement);
    acknowledgeFindings(
      role,
      'AwsSolutions-IAM5',
      [
        `Resource::${nagArn(this, 'ecs', `task-definition/${taskDef.family}:*`)}`,
        `Resource::${nagArn(this, 'ecs', `task/<${logicalId(props.cluster as unknown as Construct)}>/*`)}`,
        `Resource::<${logicalId(props.taskLogGroup as unknown as Construct)}.Arn>:log-stream:*`,
        `Resource::<${logicalId(handlerLogs)}.Arn>:log-stream:*`,
      ],
      'Task-definition revisions, task ids and log-stream names are generated at runtime; each wildcard is confined to ' +
        'this migration family, this cluster (plus an ecs:cluster condition) or one log group.',
    );
  }
}
