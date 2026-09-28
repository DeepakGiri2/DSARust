import { CfnOutput, Duration, IgnoreMode, RemovalPolicy, Size, Stack, StackProps, SecretValue } from 'aws-cdk-lib';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as ecr from 'aws-cdk-lib/aws-ecr';
import { DockerImageAsset, Platform } from 'aws-cdk-lib/aws-ecr-assets';
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as elbv2 from 'aws-cdk-lib/aws-elasticloadbalancingv2';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as logs from 'aws-cdk-lib/aws-logs';
import * as route53 from 'aws-cdk-lib/aws-route53';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import * as ses from 'aws-cdk-lib/aws-ses';
import { Construct } from 'constructs';
import { DOCKER_CONTEXT_EXCLUDES, type StageConfig } from '../config';
import { MigrationRunner } from '../constructs/migration-runner';
import { acknowledge, acknowledgeFindings, nagArn, REASONS } from '../nag';
import type { DataStack } from './data-stack';
import { API_PORT, type PlatformSecurityGroups } from './network-stack';

export interface ApiStackProps extends StackProps {
  readonly config: StageConfig;
  readonly vpc: ec2.IVpc;
  readonly securityGroups: PlatformSecurityGroups;
  readonly data: DataStack;
  readonly runnerAlias: lambda.IAlias;
  readonly targetGroup: elbv2.ApplicationTargetGroup;
  readonly originVerifySecret: secretsmanager.ISecret;
  /** The SPA origin (`https://…`): links in e-mails, OAuth redirects, allowed origins. */
  readonly publicUrl: string;
}

/**
 * Operator-filled integration secrets. Created with every key present and an
 * empty value: ECS refuses to start a task whose secret JSON key is missing,
 * and the API treats an empty value as "integration disabled".
 */
const INTEGRATION_SECRETS = {
  stripe: {
    description: 'Stripe billing (fill in to enable)',
    keys: { STRIPE_SECRET_KEY: 'secret_key', STRIPE_WEBHOOK_SECRET: 'webhook_secret', STRIPE_PRICE_MONTHLY: 'price_monthly', STRIPE_PRICE_YEARLY: 'price_yearly' },
  },
  'oauth-github': {
    description: 'GitHub OAuth app (fill in to enable GitHub sign-in)',
    keys: { OAUTH_GITHUB_CLIENT_ID: 'client_id', OAUTH_GITHUB_CLIENT_SECRET: 'client_secret' },
  },
  'oauth-google': {
    description: 'Google OAuth client (fill in to enable Google sign-in)',
    keys: { OAUTH_GOOGLE_CLIENT_ID: 'client_id', OAUTH_GOOGLE_CLIENT_SECRET: 'client_secret' },
  },
} as const;

/**
 * dsa-api on Fargate behind the ingress ALB, plus everything only the API
 * uses: its secrets, SES identity, task role, autoscaling and the
 * migrate-before-rollout custom resource.
 */
export class ApiStack extends Stack {
  public readonly cluster: ecs.Cluster;
  public readonly service: ecs.FargateService;
  public readonly emailIdentityName?: string;

  constructor(scope: Construct, id: string, props: ApiStackProps) {
    super(scope, id, props);
    const { config, vpc, securityGroups, data } = props;
    const prefix = config.prefix;
    const secretRemoval = config.isProd ? RemovalPolicy.RETAIN : RemovalPolicy.DESTROY;
    const logRemoval = config.isProd ? RemovalPolicy.RETAIN : RemovalPolicy.DESTROY;
    const privateSubnets: ec2.SubnetSelection = { subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS };

    // ── secrets ─────────────────────────────────────────────────────────────
    const sessionSecret = new secretsmanager.Secret(this, 'SessionSecret', {
      secretName: `${prefix}/api/session-secret`,
      description: `${prefix}: SESSION_SECRET (HMAC key for CSRF tokens and OAuth state; 64 hex chars = 32 bytes)`,
      generateSecretString: {
        passwordLength: 64,
        excludeUppercase: true,
        excludePunctuation: true,
        excludeCharacters: 'ghijklmnopqrstuvwxyz',
        includeSpace: false,
        requireEachIncludedType: false,
      },
      removalPolicy: secretRemoval,
    });
    acknowledge(
      sessionSecret,
      'Rotating SESSION_SECRET invalidates every CSRF token and in-flight OAuth state, so it is a deliberate runbook action (DEPLOY.md → Rotating secrets) followed by a redeploy.',
      'AwsSolutions-SMG4',
    );

    // The data-tier secrets are encrypted with the Data stack's KMS key. They are
    // referenced by ARN here (not as the Data stack's objects) so CDK grants the
    // execution roles through their own policies instead of editing the key
    // policy in the Data stack, which would make Data depend on this stack.
    const dbSecret = secretsmanager.Secret.fromSecretCompleteArn(this, 'DbSecretRef', data.dbSecret.secretArn);
    const cacheSecret = secretsmanager.Secret.fromSecretCompleteArn(this, 'CacheSecretRef', data.cacheSecret.secretArn);
    const decryptDataSecrets = new iam.PolicyStatement({
      sid: 'DecryptDataTierSecrets',
      actions: ['kms:Decrypt'],
      resources: [data.key.keyArn],
      conditions: { StringEquals: { 'kms:ViaService': `secretsmanager.${this.region}.amazonaws.com` } },
    });

    const secretEnv: Record<string, ecs.Secret> = {
      DB_USER: ecs.Secret.fromSecretsManager(dbSecret, 'username'),
      DB_PASSWORD: ecs.Secret.fromSecretsManager(dbSecret, 'password'),
      REDIS_AUTH_TOKEN: ecs.Secret.fromSecretsManager(cacheSecret),
      SESSION_SECRET: ecs.Secret.fromSecretsManager(sessionSecret),
      ORIGIN_VERIFY_SECRET: ecs.Secret.fromSecretsManager(props.originVerifySecret),
    };
    for (const [name, spec] of Object.entries(INTEGRATION_SECRETS)) {
      const secret = this.placeholderSecret(`${prefix}/api/${name}`, spec.description, Object.values(spec.keys), secretRemoval);
      for (const [envName, key] of Object.entries(spec.keys)) {
        secretEnv[envName] = ecs.Secret.fromSecretsManager(secret, key);
      }
    }
    if (config.aiProvider === 'anthropic') {
      const anthropic = this.placeholderSecret(`${prefix}/api/anthropic`, 'Anthropic API key (AI_PROVIDER=anthropic)', ['api_key'], secretRemoval);
      secretEnv.ANTHROPIC_API_KEY = ecs.Secret.fromSecretsManager(anthropic, 'api_key');
    }

    // ── e-mail (SES) ────────────────────────────────────────────────────────
    const mail = this.configureEmail(config);
    this.emailIdentityName = mail?.identityName;

    // ── image ───────────────────────────────────────────────────────────────
    const arm = config.apiCpuArchitecture === 'ARM64';
    const image = config.apiImage
      ? ecs.ContainerImage.fromEcrRepository(
          ecr.Repository.fromRepositoryName(this, 'ApiRepository', config.apiImage.repositoryName),
          config.apiImage.tagOrDigest,
        )
      : ecs.ContainerImage.fromDockerImageAsset(
          new DockerImageAsset(this, 'ApiImage', {
            directory: config.repoRoot,
            file: 'backend/docker/api.Dockerfile',
            platform: arm ? Platform.LINUX_ARM64 : Platform.LINUX_AMD64,
            exclude: DOCKER_CONTEXT_EXCLUDES,
            ignoreMode: IgnoreMode.DOCKER,
            displayName: `${prefix}-api`,
          }),
        );
    const runtimePlatform: ecs.RuntimePlatform = {
      cpuArchitecture: arm ? ecs.CpuArchitecture.ARM64 : ecs.CpuArchitecture.X86_64,
      operatingSystemFamily: ecs.OperatingSystemFamily.LINUX,
    };

    // ── cluster ─────────────────────────────────────────────────────────────
    this.cluster = new ecs.Cluster(this, 'Cluster', {
      clusterName: prefix,
      vpc,
      containerInsightsV2: ecs.ContainerInsights.ENABLED,
      enableFargateCapacityProviders: true,
    });

    // ── task role: exactly what SERVICES.md lists, nothing more ─────────────
    const metricsNamespace = `DSA/${config.stage}`;
    const taskRole = new iam.Role(this, 'TaskRole', {
      roleName: `${prefix}-api-task`,
      description: 'dsa-api task role: invoke the runner, send mail, call the AI model, publish metrics',
      assumedBy: new iam.ServicePrincipal('ecs-tasks.amazonaws.com', {
        conditions: {
          StringEquals: { 'aws:SourceAccount': this.account },
          ArnLike: { 'aws:SourceArn': this.formatArn({ service: 'ecs', resource: '*' }) },
        },
      }),
    });
    const taskPolicy = new iam.Policy(this, 'TaskPolicy', {
      policyName: 'dsa-api',
      statements: [
        new iam.PolicyStatement({
          sid: 'InvokeRunner',
          actions: ['lambda:InvokeFunction'],
          resources: [props.runnerAlias.functionArn],
        }),
        new iam.PolicyStatement({
          sid: 'PublishMetrics',
          actions: ['cloudwatch:PutMetricData'],
          // PutMetricData has no resource-level permissions; the namespace condition scopes it.
          resources: ['*'],
          conditions: { StringEquals: { 'cloudwatch:namespace': metricsNamespace } },
        }),
        ...(mail
          ? [
              new iam.PolicyStatement({
                sid: 'SendEmail',
                actions: ['ses:SendEmail', 'ses:SendRawEmail'],
                resources: [mail.identityArn, mail.configurationSetArn],
              }),
            ]
          : []),
        ...(config.aiProvider === 'bedrock' ? bedrockStatements(this, config.aiModelId) : []),
      ],
    });
    taskRole.attachInlinePolicy(taskPolicy);
    acknowledgeFindings(
      taskPolicy,
      'AwsSolutions-IAM5',
      ['Resource::*', ...(config.aiProvider === 'bedrock' && isInferenceProfile(config.aiModelId) ? [`Resource::${nagArn(this, 'bedrock', `foundation-model/${baseModelId(config.aiModelId)}`, { region: '*', noAccount: true })}`] : [])],
      'cloudwatch:PutMetricData supports no resource-level permissions and is pinned to one namespace by condition; ' +
        'a cross-region inference profile routes to the same foundation model in several regions, allowed only through that profile (bedrock:InferenceProfileArn condition).',
    );

    // ── environment ─────────────────────────────────────────────────────────
    const logLevel = config.isProd ? 'info' : 'info,dsa_api=debug';
    const dbEnv = {
      DB_HOST: data.dbHost,
      DB_PORT: data.dbPort,
      DB_NAME: config.dbName,
      DB_SSLMODE: 'require',
    };
    const environment: Record<string, string> = {
      DSA_ENV: 'production',
      DSA_BIND: `0.0.0.0:${API_PORT}`,
      // Metrics stay on loopback: nothing outside the task may scrape them.
      DSA_METRICS_BIND: '127.0.0.1:9090',
      DSA_PUBLIC_URL: props.publicUrl,
      DSA_ALLOWED_ORIGINS: props.publicUrl,
      DSA_CONTENT_DIR: '/app/content',
      ...dbEnv,
      DB_MAX_CONNECTIONS: String(config.dbMaxConnectionsPerTask),
      // No credentials in the URL: the AUTH token arrives as REDIS_AUTH_TOKEN.
      REDIS_URL: `rediss://${data.cacheHost}:${data.cachePort}`,
      SESSION_TTL_DAYS: '30',
      RUN_MIGRATIONS: 'false',
      // CloudFront + ALB.
      TRUSTED_PROXY_HOPS: '2',
      RUNNER_MODE: 'lambda',
      RUNNER_LAMBDA_FUNCTION: props.runnerAlias.functionArn,
      RUNNER_CONCURRENCY: String(config.runnerConcurrencyPerTask),
      MAIL_MODE: mail ? 'ses' : 'log',
      MAIL_FROM: mail?.from ?? 'DSA Visualized <no-reply@localhost>',
      AI_PROVIDER: config.aiProvider,
      ...(config.aiProvider === 'none' ? {} : { AI_MODEL: config.aiModelId }),
      AWS_REGION: this.region,
      CLOUDWATCH_NAMESPACE: metricsNamespace,
      LOG_FORMAT: 'json',
      RUST_LOG: logLevel,
      ...config.apiEnv,
    };

    // ── service task ────────────────────────────────────────────────────────
    const apiLogs = new logs.LogGroup(this, 'ApiLogs', {
      logGroupName: `/dsa/${config.stage}/api`,
      retention: config.logRetentionDays,
      removalPolicy: logRemoval,
    });
    const taskDefinition = new ecs.FargateTaskDefinition(this, 'ApiTask', {
      family: `${prefix}-api`,
      cpu: config.apiCpu,
      memoryLimitMiB: config.apiMemoryMiB,
      runtimePlatform,
      taskRole,
      volumes: [{ name: 'tmp' }],
    });
    const container = taskDefinition.addContainer('api', {
      containerName: 'api',
      image,
      // Explicit entrypoint + command: independent of the image's CMD/ENTRYPOINT.
      entryPoint: ['dsa-api'],
      command: ['serve'],
      essential: true,
      user: '10001',
      readonlyRootFilesystem: true,
      linuxParameters: this.hardenedLinux('ApiLinux'),
      portMappings: [{ name: 'http', containerPort: API_PORT, protocol: ecs.Protocol.TCP, appProtocol: ecs.AppProtocol.http }],
      environment,
      secrets: secretEnv,
      stopTimeout: Duration.seconds(30),
      ulimits: [{ name: ecs.UlimitName.NOFILE, softLimit: 65536, hardLimit: 65536 }],
      logging: ecs.LogDrivers.awsLogs({
        logGroup: apiLogs,
        streamPrefix: 'api',
        mode: ecs.AwsLogDriverMode.NON_BLOCKING,
        maxBufferSize: Size.mebibytes(25),
      }),
    });
    container.addMountPoints({ containerPath: '/tmp', sourceVolume: 'tmp', readOnly: false });
    taskDefinition.addToExecutionRolePolicy(decryptDataSecrets);

    // ── migration task ──────────────────────────────────────────────────────
    const migrateLogs = new logs.LogGroup(this, 'MigrateLogs', {
      logGroupName: `/dsa/${config.stage}/migrate`,
      retention: config.logRetentionDays,
      removalPolicy: logRemoval,
    });
    const migrateTask = new ecs.FargateTaskDefinition(this, 'MigrateTask', {
      family: `${prefix}-migrate`,
      cpu: 512,
      memoryLimitMiB: 1024,
      runtimePlatform,
      volumes: [{ name: 'tmp' }],
    });
    const migrateContainer = migrateTask.addContainer('migrate', {
      containerName: 'migrate',
      image,
      entryPoint: ['dsa-api'],
      command: ['migrate'],
      essential: true,
      user: '10001',
      readonlyRootFilesystem: true,
      linuxParameters: this.hardenedLinux('MigrateLinux'),
      // Migrations get the database and nothing else. They always hit the
      // instance directly (never the proxy).
      environment: {
        DSA_ENV: 'production',
        DSA_CONTENT_DIR: '/app/content',
        DB_HOST: data.db.dbInstanceEndpointAddress,
        DB_PORT: data.db.dbInstanceEndpointPort,
        DB_NAME: config.dbName,
        DB_SSLMODE: 'require',
        DB_MAX_CONNECTIONS: '2',
        LOG_FORMAT: 'json',
        RUST_LOG: logLevel,
      },
      secrets: { DB_USER: secretEnv.DB_USER, DB_PASSWORD: secretEnv.DB_PASSWORD },
      logging: ecs.LogDrivers.awsLogs({ logGroup: migrateLogs, streamPrefix: 'migrate' }),
    });
    migrateContainer.addMountPoints({ containerPath: '/tmp', sourceVolume: 'tmp', readOnly: false });
    migrateTask.addToExecutionRolePolicy(decryptDataSecrets);

    const migrations = new MigrationRunner(this, 'Migrations', {
      prefix,
      cluster: this.cluster,
      taskDefinition: migrateTask,
      containerName: 'migrate',
      taskLogGroup: migrateLogs,
      taskLogStreamPrefix: 'migrate',
      subnets: vpc.selectSubnets(privateSubnets).subnets,
      securityGroup: securityGroups.migrate,
      logRetentionDays: config.logRetentionDays,
      removalPolicy: logRemoval,
    });

    // ── service ─────────────────────────────────────────────────────────────
    this.service = new ecs.FargateService(this, 'Service', {
      serviceName: `${prefix}-api`,
      cluster: this.cluster,
      taskDefinition,
      // desiredCount is deliberately unset: autoscaling owns it, and a deploy
      // must not reset a scaled-out service back to the minimum.
      minHealthyPercent: 100,
      maxHealthyPercent: 200,
      circuitBreaker: { enable: true, rollback: true },
      healthCheckGracePeriod: Duration.seconds(60),
      vpcSubnets: privateSubnets,
      securityGroups: [securityGroups.api],
      assignPublicIp: false,
      platformVersion: ecs.FargatePlatformVersion.LATEST,
      enableExecuteCommand: false,
      propagateTags: ecs.PropagatedTagSource.SERVICE,
      enableECSManagedTags: true,
      capacityProviderStrategies: config.apiFargateSpot
        ? [
            { capacityProvider: 'FARGATE_SPOT', weight: 1 },
            { capacityProvider: 'FARGATE', weight: 0 },
          ]
        : [{ capacityProvider: 'FARGATE', weight: 1 }],
    });
    this.service.attachToApplicationTargetGroup(props.targetGroup);
    // Schema first, then code.
    this.service.node.addDependency(migrations.resource);

    const scaling = this.service.autoScaleTaskCount({ minCapacity: config.apiMinTasks, maxCapacity: config.apiMaxTasks });
    scaling.scaleOnCpuUtilization('Cpu', {
      targetUtilizationPercent: config.apiCpuTargetPercent,
      scaleOutCooldown: Duration.seconds(60),
      scaleInCooldown: Duration.minutes(5),
    });
    scaling.scaleOnRequestCount('Requests', {
      requestsPerTarget: config.apiRequestsPerTarget,
      targetGroup: props.targetGroup,
      scaleOutCooldown: Duration.seconds(60),
      scaleInCooldown: Duration.minutes(5),
    });

    // ── cdk-nag ─────────────────────────────────────────────────────────────
    for (const td of [taskDefinition, migrateTask]) {
      acknowledge(
        td,
        'Only non-secret configuration is passed as environment (URLs, ports, modes, limits); every credential arrives through `secrets` from Secrets Manager, which a unit test enforces.',
        'AwsSolutions-ECS2',
      );
      const executionRole = td.executionRole;
      if (executionRole) {
        const policy = executionRole.node.tryFindChild('DefaultPolicy');
        if (policy) acknowledgeFindings(policy, 'AwsSolutions-IAM5', ['Resource::*'], REASONS.ecrAuthToken);
      }
    }

    new CfnOutput(this, 'ClusterName', { value: this.cluster.clusterName });
    new CfnOutput(this, 'ServiceName', { value: this.service.serviceName });
    new CfnOutput(this, 'MigrationTaskDefinition', { value: migrateTask.taskDefinitionArn });
  }

  /** Init process for signal handling and zombie reaping; no Linux capabilities. */
  private hardenedLinux(id: string): ecs.LinuxParameters {
    const params = new ecs.LinuxParameters(this, id, { initProcessEnabled: true });
    params.dropCapabilities(ecs.Capability.ALL);
    return params;
  }

  private placeholderSecret(name: string, description: string, keys: readonly string[], removalPolicy: RemovalPolicy): secretsmanager.Secret {
    const secret = new secretsmanager.Secret(this, `Secret-${name.split('/').pop()}`, {
      secretName: name,
      description: `${description}. Every key must stay present; an empty value disables the integration.`,
      // Placeholder only: CloudFormation writes this once at creation and never
      // again (the template value does not change), so operator edits survive deploys.
      secretObjectValue: Object.fromEntries(keys.map((k) => [k, SecretValue.unsafePlainText('')])),
      removalPolicy,
    });
    acknowledge(
      secret,
      'Third-party credential issued by an external provider (Stripe, GitHub, Google, Anthropic); it is rotated at the provider and then updated here by an operator (DEPLOY.md → Rotating secrets).',
      'AwsSolutions-SMG4',
    );
    return secret;
  }

  private configureEmail(config: StageConfig): { identityArn: string; configurationSetArn: string; identityName: string; from: string } | undefined {
    const domain = config.domain;
    const address = config.mailFromAddress;
    if (!domain && !address) return undefined;

    const configurationSet = new ses.ConfigurationSet(this, 'MailConfigurationSet', {
      configurationSetName: config.prefix,
      reputationMetrics: true,
      sendingEnabled: true,
      suppressionReasons: ses.SuppressionReasons.BOUNCES_AND_COMPLAINTS,
      tlsPolicy: ses.ConfigurationSetTlsPolicy.REQUIRE,
    });

    let identity: ses.Identity;
    let identityName: string;
    if (domain) {
      identityName = domain.domainName;
      identity =
        domain.hostedZoneId && domain.hostedZoneName
          ? ses.Identity.publicHostedZone(
              route53.PublicHostedZone.fromPublicHostedZoneAttributes(this, 'MailZone', {
                hostedZoneId: domain.hostedZoneId,
                zoneName: domain.hostedZoneName,
              }),
            )
          : ses.Identity.domain(domain.domainName);
    } else {
      identityName = address!;
      identity = ses.Identity.email(address!);
    }
    const emailIdentity = new ses.EmailIdentity(this, 'MailIdentity', {
      identity,
      configurationSet,
      // Custom MAIL FROM aligns SPF with the domain (DMARC); DKIM is Easy DKIM.
      mailFromDomain: domain ? `bounce.${domain.domainName}` : undefined,
    });
    if (domain && !domain.hostedZoneId) {
      emailIdentity.dkimRecords.forEach((record, i) => {
        new CfnOutput(this, `DkimRecord${i + 1}`, { value: `${record.name} CNAME ${record.value}`, description: 'Add to DNS for Easy DKIM' });
      });
    }
    return {
      identityName,
      identityArn: this.formatArn({ service: 'ses', resource: 'identity', resourceName: identityName }),
      configurationSetArn: this.formatArn({ service: 'ses', resource: 'configuration-set', resourceName: configurationSet.configurationSetName }),
      from: config.apiEnv.MAIL_FROM ?? `DSA Visualized <no-reply@${domain ? domain.domainName : address!.split('@')[1]}>`,
    };
  }
}

/** Geo-prefixed ids (`us.`, `eu.`, `apac.`, `global.` …) are cross-region inference profiles. */
function isInferenceProfile(modelId: string): boolean {
  return /^(us|eu|apac|jp|au|ca|us-gov|global)\./.test(modelId);
}

function baseModelId(modelId: string): string {
  return isInferenceProfile(modelId) ? modelId.slice(modelId.indexOf('.') + 1) : modelId;
}

/**
 * bedrock:InvokeModel* on exactly the configured model. For a cross-region
 * inference profile that is the profile itself plus the underlying foundation
 * model in whichever region the profile routes to — but only via that profile.
 */
function bedrockStatements(stack: Stack, modelId: string): iam.PolicyStatement[] {
  const actions = ['bedrock:InvokeModel', 'bedrock:InvokeModelWithResponseStream'];
  if (!isInferenceProfile(modelId)) {
    return [
      new iam.PolicyStatement({
        sid: 'InvokeModel',
        actions,
        resources: [stack.formatArn({ service: 'bedrock', account: '', resource: 'foundation-model', resourceName: modelId })],
      }),
    ];
  }
  const profileArn = stack.formatArn({ service: 'bedrock', resource: 'inference-profile', resourceName: modelId });
  return [
    new iam.PolicyStatement({ sid: 'InvokeInferenceProfile', actions, resources: [profileArn] }),
    new iam.PolicyStatement({
      sid: 'InvokeModelViaProfile',
      actions,
      resources: [stack.formatArn({ service: 'bedrock', region: '*', account: '', resource: 'foundation-model', resourceName: baseModelId(modelId) })],
      conditions: { StringLike: { 'bedrock:InferenceProfileArn': profileArn } },
    }),
  ];
}
