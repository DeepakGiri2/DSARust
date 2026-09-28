import { Duration, RemovalPolicy, Stack, StackProps } from 'aws-cdk-lib';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as elasticache from 'aws-cdk-lib/aws-elasticache';
import * as kms from 'aws-cdk-lib/aws-kms';
import * as logs from 'aws-cdk-lib/aws-logs';
import * as rds from 'aws-cdk-lib/aws-rds';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import { Construct } from 'constructs';
import { supportsPerformanceInsights, type StageConfig } from '../config';
import { acknowledge, acknowledgeFindings, REASONS } from '../nag';
import { CACHE_PORT, DB_PORT, type PlatformSecurityGroups } from './network-stack';

export interface DataStackProps extends StackProps {
  readonly config: StageConfig;
  readonly vpc: ec2.IVpc;
  readonly securityGroups: PlatformSecurityGroups;
}

/**
 * Stateful tier: PostgreSQL 17 (optionally behind RDS Proxy) and a
 * TLS + AUTH Valkey/Redis replication group, both in isolated subnets, both
 * encrypted with a stage-owned KMS key.
 */
export class DataStack extends Stack {
  public readonly key: kms.Key;
  public readonly db: rds.DatabaseInstance;
  public readonly dbSecret: secretsmanager.ISecret;
  /** Host the API connects to: the proxy endpoint when enabled, else the instance. */
  public readonly dbHost: string;
  public readonly dbPort: string;
  public readonly cacheSecret: secretsmanager.Secret;
  public readonly cacheHost: string;
  public readonly cachePort: string;
  public readonly cacheReplicationGroupId: string;

  constructor(scope: Construct, id: string, props: DataStackProps) {
    super(scope, id, props);
    const { config, vpc, securityGroups } = props;
    const prefix = config.prefix;
    const removal = config.isProd ? RemovalPolicy.RETAIN : RemovalPolicy.DESTROY;
    const isolated: ec2.SubnetSelection = { subnetType: ec2.SubnetType.PRIVATE_ISOLATED };

    // One customer-managed key for the data tier: storage, snapshots,
    // Performance Insights, the cache and the credentials that open them.
    // Destroying the key crypto-shreds every snapshot, so prod retains it.
    this.key = new kms.Key(this, 'DataKey', {
      alias: `alias/${prefix}-data`,
      description: `${prefix}: RDS, ElastiCache and their credentials`,
      enableKeyRotation: true,
      pendingWindow: Duration.days(config.isProd ? 30 : 7),
      removalPolicy: removal,
    });

    // ── PostgreSQL ──────────────────────────────────────────────────────────
    const engine = rds.DatabaseInstanceEngine.postgres({
      // Major version only: RDS applies minor upgrades (autoMinorVersionUpgrade)
      // without the template drifting behind a pinned minor.
      version: rds.PostgresEngineVersion.of('17', '17'),
    });
    const parameterGroup = new rds.ParameterGroup(this, 'PostgresParams', {
      engine,
      description: `${prefix} PostgreSQL 17`,
      parameters: {
        // Refuse any unencrypted client connection at the server.
        'rds.force_ssl': '1',
        // Slow queries (≥ 1 s) and lock waits land in the exported postgresql log.
        log_min_duration_statement: '1000',
        log_lock_waits: '1',
      },
    });

    const instanceId = `${prefix}-postgres`;
    // Pre-create the log groups RDS exports to, so they get a retention period
    // (RDS would otherwise create them with "never expire").
    const dbLogGroups = ['postgresql', 'upgrade'].map(
      (kind) =>
        new logs.LogGroup(this, `PostgresLogs-${kind}`, {
          logGroupName: `/aws/rds/instance/${instanceId}/${kind}`,
          retention: config.logRetentionDays,
          removalPolicy: removal,
        }),
    );

    const pi = supportsPerformanceInsights(config.dbInstanceClass);
    this.db = new rds.DatabaseInstance(this, 'Postgres', {
      instanceIdentifier: instanceId,
      engine,
      instanceType: new ec2.InstanceType(config.dbInstanceClass),
      vpc,
      vpcSubnets: isolated,
      securityGroups: [securityGroups.db],
      publiclyAccessible: false,
      port: DB_PORT,
      databaseName: config.dbName,
      credentials: rds.Credentials.fromGeneratedSecret('dsa_admin', {
        secretName: `${prefix}/db/master`,
        encryptionKey: this.key,
      }),
      parameterGroup,
      multiAz: config.dbMultiAz,
      storageType: rds.StorageType.GP3,
      allocatedStorage: config.dbAllocatedStorageGiB,
      maxAllocatedStorage: config.dbMaxAllocatedStorageGiB,
      storageEncrypted: true,
      storageEncryptionKey: this.key,
      backupRetention: Duration.days(config.dbBackupRetentionDays),
      preferredBackupWindow: '03:00-03:30',
      preferredMaintenanceWindow: 'sun:04:00-sun:04:30',
      copyTagsToSnapshot: true,
      deleteAutomatedBackups: !config.isProd,
      deletionProtection: config.isProd,
      removalPolicy: config.isProd ? RemovalPolicy.SNAPSHOT : RemovalPolicy.DESTROY,
      autoMinorVersionUpgrade: true,
      allowMajorVersionUpgrade: false,
      // IAM auth is enabled for break-glass operator access (rds-db:connect is
      // granted to no one by default); the application uses the password.
      iamAuthentication: true,
      caCertificate: rds.CaCertificate.RDS_CA_RSA2048_G1,
      enablePerformanceInsights: pi,
      performanceInsightEncryptionKey: pi ? this.key : undefined,
      performanceInsightRetention: pi ? rds.PerformanceInsightRetention.DEFAULT : undefined,
      monitoringInterval: config.isProd ? Duration.seconds(60) : undefined,
      cloudwatchLogsExports: ['postgresql', 'upgrade'],
    });
    dbLogGroups.forEach((g) => this.db.node.addDependency(g));
    this.dbSecret = this.db.secret!;
    this.dbHost = this.db.dbInstanceEndpointAddress;
    this.dbPort = this.db.dbInstanceEndpointPort;

    if (config.rdsProxy) {
      // Connection multiplexing for large task counts: tasks × pool size may
      // then exceed the instance's max_connections.
      const proxy = new rds.DatabaseProxy(this, 'Proxy', {
        dbProxyName: `${prefix}-postgres`,
        // Target the instance through an import with a literal port: the proxy
        // otherwise adds its own DB ingress rule keyed on the endpoint-port token,
        // which would land in the Network stack (both groups live there) and
        // make Network depend on Data. The equivalent rule is declared in Network.
        proxyTarget: rds.ProxyTarget.fromInstance(
          rds.DatabaseInstance.fromDatabaseInstanceAttributes(this, 'PostgresForProxy', {
            instanceIdentifier: this.db.instanceIdentifier,
            instanceEndpointAddress: this.db.dbInstanceEndpointAddress,
            instanceResourceId: this.db.instanceResourceId,
            port: DB_PORT,
            securityGroups: [securityGroups.db],
            engine,
          }),
        ),
        secrets: [this.dbSecret],
        vpc,
        vpcSubnets: isolated,
        securityGroups: [securityGroups.dbProxy],
        requireTLS: true,
        iamAuth: false,
        idleClientTimeout: Duration.minutes(30),
        borrowTimeout: Duration.seconds(30),
        maxConnectionsPercent: 90,
        maxIdleConnectionsPercent: 50,
        debugLogging: false,
      });
      this.dbHost = proxy.endpoint;
    }

    acknowledge(this.db, 'The master secret is injected into ECS tasks at start-up, so an unattended rotation would break every new connection until the service is redeployed; rotation is a runbook procedure (DEPLOY.md → Rotating secrets) that ends in a forced redeploy.', 'AwsSolutions-SMG4');
    acknowledge(this.db, REASONS.defaultPort, 'AwsSolutions-RDS11');
    if (!config.dbMultiAz) {
      acknowledge(this.db, 'dev runs single-AZ to halve the database cost; prod is Multi-AZ.', 'AwsSolutions-RDS3');
    }
    if (!config.isProd) {
      acknowledge(this.db, 'dev databases are disposable (destroyed with the stack); prod enables deletion protection.', 'AwsSolutions-RDS10');
    }
    const monitoringRole = this.db.node.tryFindChild('MonitoringRole');
    if (monitoringRole) {
      acknowledgeFindings(
        monitoringRole,
        'AwsSolutions-IAM4',
        ['Policy::arn:<AWS::Partition>:iam::aws:policy/service-role/AmazonRDSEnhancedMonitoringRole'],
        'Enhanced Monitoring requires the AWS-managed AmazonRDSEnhancedMonitoringRole policy; the role is assumable only by monitoring.rds.amazonaws.com.',
      );
    }

    // ── Valkey / Redis ──────────────────────────────────────────────────────
    // AUTH token: alphanumeric only (ElastiCache forbids most punctuation, and
    // it must be URL-safe for clients that compose rediss://:token@host).
    this.cacheSecret = new secretsmanager.Secret(this, 'CacheAuthToken', {
      secretName: `${prefix}/cache/auth-token`,
      description: `${prefix} ElastiCache AUTH token`,
      encryptionKey: this.key,
      generateSecretString: { passwordLength: 64, excludePunctuation: true, includeSpace: false },
      removalPolicy: removal,
    });
    acknowledge(
      this.cacheSecret,
      'ElastiCache AUTH rotation is a two-phase operation (ROTATE then SET) coordinated with an API redeploy; it is a runbook procedure (DEPLOY.md → Rotating secrets).',
      'AwsSolutions-SMG4',
    );

    const subnetGroup = new elasticache.CfnSubnetGroup(this, 'CacheSubnets', {
      cacheSubnetGroupName: `${prefix}-cache`,
      description: `${prefix} cache (isolated subnets)`,
      subnetIds: vpc.selectSubnets(isolated).subnetIds,
    });
    const slowLog = new logs.LogGroup(this, 'CacheSlowLog', {
      logGroupName: `/dsa/${config.stage}/cache/slow-log`,
      retention: config.logRetentionDays,
      removalPolicy: removal,
    });

    const replicas = config.cacheReplicas;
    const cache = new elasticache.CfnReplicationGroup(this, 'Cache', {
      replicationGroupId: `${prefix}-cache`,
      replicationGroupDescription: `${prefix} rate limits and ephemeral state`,
      engine: config.cacheEngine,
      engineVersion: config.cacheEngineVersion,
      cacheNodeType: config.cacheNodeType,
      // Cluster mode disabled: one primary plus `replicas` read replicas.
      clusterMode: 'disabled',
      numCacheClusters: 1 + replicas,
      automaticFailoverEnabled: replicas > 0,
      multiAzEnabled: replicas > 0,
      cacheSubnetGroupName: subnetGroup.ref,
      securityGroupIds: [securityGroups.cache.securityGroupId],
      port: CACHE_PORT,
      transitEncryptionEnabled: true,
      transitEncryptionMode: 'required',
      atRestEncryptionEnabled: true,
      kmsKeyId: this.key.keyArn,
      // Resolved by CloudFormation at deploy time; never present in the template.
      authToken: this.cacheSecret.secretValue.unsafeUnwrap(),
      autoMinorVersionUpgrade: true,
      preferredMaintenanceWindow: 'sun:05:00-sun:06:00',
      snapshotRetentionLimit: config.isProd ? 1 : 0,
      logDeliveryConfigurations: [
        {
          destinationType: 'cloudwatch-logs',
          logFormat: 'json',
          logType: 'slow-log',
          destinationDetails: { cloudWatchLogsDetails: { logGroup: slowLog.logGroupName } },
        },
      ],
    });
    cache.node.addDependency(slowLog);
    cache.applyRemovalPolicy(config.isProd ? RemovalPolicy.SNAPSHOT : RemovalPolicy.DESTROY);
    this.cacheHost = cache.attrPrimaryEndPointAddress;
    this.cachePort = cache.attrPrimaryEndPointPort;
    this.cacheReplicationGroupId = cache.ref;

    acknowledge(cache, REASONS.defaultPort, 'AwsSolutions-AEC5');
    if (replicas === 0) {
      acknowledge(cache, 'dev runs a single cache node (it only holds rate-limit counters); prod runs a Multi-AZ replica with automatic failover.', 'AwsSolutions-AEC4');
    }
  }
}
