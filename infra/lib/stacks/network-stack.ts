import { RemovalPolicy, Stack, StackProps, Validations } from 'aws-cdk-lib';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as logs from 'aws-cdk-lib/aws-logs';
import { Construct } from 'constructs';
import type { StageConfig } from '../config';

export const API_PORT = 8080;
export const DB_PORT = 5432;
export const CACHE_PORT = 6379;

export interface NetworkStackProps extends StackProps {
  readonly config: StageConfig;
}

/**
 * The security groups of the whole platform, in one place.
 *
 * Every allowed flow between tiers is declared here and nowhere else, so the
 * network security model can be audited (and unit-tested) by reading one
 * construct. Other stacks attach resources to these groups but add no rules
 * of their own (CDK's automatic ALB → target rules land here too and are
 * de-duplicated against the explicit ones below).
 */
export interface PlatformSecurityGroups {
  /** Public ALB: CloudFront origin-facing ranges in, API tasks out. */
  readonly alb: ec2.SecurityGroup;
  /** API tasks (Fargate). */
  readonly api: ec2.SecurityGroup;
  /** One-off migration tasks. */
  readonly migrate: ec2.SecurityGroup;
  /** RDS PostgreSQL. */
  readonly db: ec2.SecurityGroup;
  /** RDS Proxy (only used when rdsProxy=true). */
  readonly dbProxy: ec2.SecurityGroup;
  /** ElastiCache (Valkey/Redis). */
  readonly cache: ec2.SecurityGroup;
  /** Runner Lambda: no ingress, no egress. */
  readonly runner: ec2.SecurityGroup;
  /** Interface VPC endpoints. */
  readonly endpoints: ec2.SecurityGroup;
}

/**
 * VPC with three tiers:
 *   public    ALB and NAT gateways only (no public IPs are auto-assigned)
 *   private   API/migration tasks — egress via NAT to Stripe, OAuth, SES, Bedrock
 *   isolated  RDS, ElastiCache and the runner Lambda — no route out of the VPC
 */
export class NetworkStack extends Stack {
  public readonly vpc: ec2.Vpc;
  public readonly securityGroups: PlatformSecurityGroups;

  constructor(scope: Construct, id: string, props: NetworkStackProps) {
    super(scope, id, props);
    const { config } = props;
    const prefix = config.prefix;

    this.vpc = new ec2.Vpc(this, 'Vpc', {
      vpcName: `${prefix}-vpc`,
      ipAddresses: ec2.IpAddresses.cidr(config.vpcCidr),
      // Explicit AZ names keep synth offline and deterministic (no AZ lookup).
      availabilityZones: config.availabilityZones,
      natGateways: config.natGateways,
      natGatewaySubnets: { subnetGroupName: 'public' },
      subnetConfiguration: [
        { name: 'public', subnetType: ec2.SubnetType.PUBLIC, cidrMask: 24, mapPublicIpOnLaunch: false },
        { name: 'private', subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS, cidrMask: 20 },
        { name: 'isolated', subnetType: ec2.SubnetType.PRIVATE_ISOLATED, cidrMask: 22 },
      ],
      enableDnsHostnames: true,
      enableDnsSupport: true,
      // Strip the default security group's rules: nothing may use it by accident.
      restrictDefaultSecurityGroup: true,
      gatewayEndpoints: {
        // Free, and carries ECR image layers (stored in S3) for tasks in private subnets.
        S3: {
          service: ec2.GatewayVpcEndpointAwsService.S3,
          subnets: [{ subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS }, { subnetType: ec2.SubnetType.PRIVATE_ISOLATED }],
        },
      },
    });

    // Flow logs: every flow in prod; only rejected flows in dev, which is cheap
    // and still surfaces anything probing the security groups.
    const flowLogGroup = new logs.LogGroup(this, 'FlowLogs', {
      logGroupName: `/dsa/${config.stage}/vpc-flow-logs`,
      retention: config.logRetentionDays,
      removalPolicy: config.isProd ? RemovalPolicy.RETAIN : RemovalPolicy.DESTROY,
    });
    this.vpc.addFlowLog('FlowLog', {
      destination: ec2.FlowLogDestination.toCloudWatchLogs(flowLogGroup),
      trafficType: config.flowLogTrafficType === 'ALL' ? ec2.FlowLogTrafficType.ALL : ec2.FlowLogTrafficType.REJECT,
      maxAggregationInterval: ec2.FlowLogMaxAggregationInterval.TEN_MINUTES,
    });

    this.securityGroups = this.createSecurityGroups(config);
    this.createInterfaceEndpoints(config);

    // cdk-nag acknowledgements for CDK-managed helpers in this stack.
    const restrictDefaultSg = this.node.tryFindChild('Custom::VpcRestrictDefaultSGCustomResourceProvider');
    if (restrictDefaultSg) {
      Validations.of(restrictDefaultSg).acknowledge(
        {
          id: 'AwsSolutions-IAM4',
          reason: 'CDK-managed custom resource that strips the default security group; its role uses AWSLambdaBasicExecutionRole for its own logs only.',
        },
        {
          id: 'AwsSolutions-L1',
          reason: 'CDK-managed provider whose runtime is pinned by aws-cdk-lib; it runs once per deployment and never handles user data.',
        },
      );
    }
  }

  private createSecurityGroups(config: StageConfig): PlatformSecurityGroups {
    const vpc = this.vpc;
    const prefix = config.prefix;
    const sg = (id: string, name: string, description: string) =>
      new ec2.SecurityGroup(this, id, {
        vpc,
        securityGroupName: `${prefix}-${name}`,
        description,
        // Egress is always explicit. With allowAllOutbound=false and no rule,
        // CDK installs a single sentinel rule (ICMP 252/86 to 255.255.255.255/32)
        // that replaces EC2's default allow-all egress and can never match traffic.
        allowAllOutbound: false,
      });

    const alb = sg('AlbSg', 'alb', 'Public ALB - CloudFront origin-facing ranges only');
    const api = sg('ApiSg', 'api', 'dsa-api Fargate tasks');
    const migrate = sg('MigrateSg', 'migrate', 'dsa-api migration tasks');
    const db = sg('DbSg', 'db', 'RDS PostgreSQL');
    const dbProxy = sg('DbProxySg', 'db-proxy', 'RDS Proxy');
    const cache = sg('CacheSg', 'cache', 'ElastiCache (Valkey/Redis)');
    const runner = sg('RunnerSg', 'runner', 'dsa-runner Lambda - no ingress, no egress');
    const endpoints = sg('EndpointsSg', 'endpoints', 'Interface VPC endpoints');

    // Edge → ALB. The origin-facing prefix list counts as ~55 rules against the
    // 60-rules-per-group quota, so exactly one reference is possible: the port
    // CloudFront actually uses for this origin (HTTPS with a certificate).
    const originPort = config.domain ? 443 : 80;
    alb.addIngressRule(
      ec2.Peer.prefixList(config.cloudFrontPrefixListId),
      ec2.Port.tcp(originPort),
      'CloudFront origin-facing (com.amazonaws.global.cloudfront.origin-facing)',
    );
    alb.addEgressRule(api, ec2.Port.tcp(API_PORT), 'ALB to API tasks');

    // API tasks.
    api.addIngressRule(alb, ec2.Port.tcp(API_PORT), 'ALB to API tasks');
    api.addEgressRule(
      ec2.Peer.anyIpv4(),
      ec2.Port.tcp(443),
      'HTTPS: AWS APIs (Lambda, SES, Bedrock, ECR, Logs, Secrets Manager), Stripe, OAuth providers',
    );
    api.addEgressRule(config.rdsProxy ? dbProxy : db, ec2.Port.tcp(DB_PORT), 'API to PostgreSQL');
    api.addEgressRule(cache, ec2.Port.tcp(CACHE_PORT), 'API to Valkey/Redis (TLS)');

    // Migrations always talk to the instance directly (DDL through a proxy pins
    // sessions and gains nothing).
    migrate.addEgressRule(ec2.Peer.anyIpv4(), ec2.Port.tcp(443), 'HTTPS: ECR image pull, CloudWatch Logs, Secrets Manager');
    migrate.addEgressRule(db, ec2.Port.tcp(DB_PORT), 'Migrations to PostgreSQL');

    // Database: only the API (directly or via the proxy) and the migration task.
    db.addIngressRule(migrate, ec2.Port.tcp(DB_PORT), 'Migration task');
    if (config.rdsProxy) {
      db.addIngressRule(dbProxy, ec2.Port.tcp(DB_PORT), 'RDS Proxy');
      dbProxy.addIngressRule(api, ec2.Port.tcp(DB_PORT), 'API tasks');
      dbProxy.addEgressRule(db, ec2.Port.tcp(DB_PORT), 'Proxy to PostgreSQL');
      dbProxy.addEgressRule(endpoints, ec2.Port.tcp(443), 'Proxy to Secrets Manager endpoint');
      endpoints.addIngressRule(dbProxy, ec2.Port.tcp(443), 'RDS Proxy');
    } else {
      db.addIngressRule(api, ec2.Port.tcp(DB_PORT), 'API tasks');
    }

    // Cache: API only.
    cache.addIngressRule(api, ec2.Port.tcp(CACHE_PORT), 'API tasks');

    // Interface endpoints: HTTPS from the tiers that call AWS APIs.
    endpoints.addIngressRule(api, ec2.Port.tcp(443), 'API tasks');
    endpoints.addIngressRule(migrate, ec2.Port.tcp(443), 'Migration tasks');

    // The runner group gets no rules at all: untrusted code has nowhere to go.
    return { alb, api, migrate, db, dbProxy, cache, runner, endpoints };
  }

  private createInterfaceEndpoints(config: StageConfig): void {
    // Interface endpoints cost ~$7.30/AZ/month each, so dev reaches AWS APIs
    // through its NAT gateway. In prod they keep image pulls, logs, secrets and
    // runner invocations off the NAT path (cost, and no dependency on NAT/IGW
    // for the control plane of the service).
    const services: Array<[string, ec2.InterfaceVpcEndpointAwsService]> = config.interfaceEndpoints
      ? [
          ['EcrApi', ec2.InterfaceVpcEndpointAwsService.ECR],
          ['EcrDocker', ec2.InterfaceVpcEndpointAwsService.ECR_DOCKER],
          ['Logs', ec2.InterfaceVpcEndpointAwsService.CLOUDWATCH_LOGS],
          ['SecretsManager', ec2.InterfaceVpcEndpointAwsService.SECRETS_MANAGER],
          ['Lambda', ec2.InterfaceVpcEndpointAwsService.LAMBDA],
        ]
      : config.rdsProxy
        ? // RDS Proxy sits in the isolated subnets and must reach Secrets Manager.
          [['SecretsManager', ec2.InterfaceVpcEndpointAwsService.SECRETS_MANAGER]]
        : [];
    for (const [id, service] of services) {
      this.vpc.addInterfaceEndpoint(id, {
        service,
        subnets: { subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS },
        securityGroups: [this.securityGroups.endpoints],
        privateDnsEnabled: true,
        // Ingress is managed explicitly above instead of "whole VPC CIDR".
        open: false,
      });
    }
  }
}
