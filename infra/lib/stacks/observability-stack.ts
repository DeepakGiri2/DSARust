import { Duration, Stack, StackProps } from 'aws-cdk-lib';
import * as budgets from 'aws-cdk-lib/aws-budgets';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as cw from 'aws-cdk-lib/aws-cloudwatch';
import * as cwActions from 'aws-cdk-lib/aws-cloudwatch-actions';
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as elbv2 from 'aws-cdk-lib/aws-elasticloadbalancingv2';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as kms from 'aws-cdk-lib/aws-kms';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as rds from 'aws-cdk-lib/aws-rds';
import * as sns from 'aws-cdk-lib/aws-sns';
import * as subs from 'aws-cdk-lib/aws-sns-subscriptions';
import { Construct } from 'constructs';
import { estimateMaxConnections, type StageConfig } from '../config';

export interface ObservabilityStackProps extends StackProps {
  readonly config: StageConfig;
  readonly loadBalancer: elbv2.ApplicationLoadBalancer;
  readonly targetGroup: elbv2.ApplicationTargetGroup;
  readonly webAclName: string;
  readonly cluster: ecs.ICluster;
  readonly service: ecs.FargateService;
  readonly db: rds.DatabaseInstance;
  readonly cacheReplicationGroupId: string;
  readonly runner: lambda.IFunction;
  readonly distribution: cloudfront.IDistribution;
  readonly sesEnabled: boolean;
}

const FIVE_MINUTES = Duration.minutes(5);

/** Dashboard, alarms (→ SNS, optional e-mail) and the monthly budget. */
export class ObservabilityStack extends Stack {
  public readonly alarmTopic: sns.Topic;

  constructor(scope: Construct, id: string, props: ObservabilityStackProps) {
    super(scope, id, props);
    const { config } = props;
    const prefix = config.prefix;

    // CloudWatch can only publish to an encrypted topic whose key lets it.
    const topicKey = new kms.Key(this, 'AlarmTopicKey', {
      alias: `alias/${prefix}-alarms`,
      description: `${prefix}: alarm topic encryption`,
      enableKeyRotation: true,
    });
    topicKey.addToResourcePolicy(
      new iam.PolicyStatement({
        sid: 'AllowCloudWatchAlarms',
        principals: [new iam.ServicePrincipal('cloudwatch.amazonaws.com')],
        actions: ['kms:Decrypt', 'kms:GenerateDataKey*'],
        resources: ['*'],
        conditions: { StringEquals: { 'aws:SourceAccount': this.account } },
      }),
    );
    this.alarmTopic = new sns.Topic(this, 'AlarmTopic', {
      topicName: `${prefix}-alarms`,
      displayName: `DSA Visualized ${config.stage} alarms`,
      masterKey: topicKey,
      enforceSSL: true,
    });
    this.alarmTopic.addToResourcePolicy(
      new iam.PolicyStatement({
        sid: 'AllowCloudWatchAlarmsPublish',
        principals: [new iam.ServicePrincipal('cloudwatch.amazonaws.com')],
        actions: ['sns:Publish'],
        resources: [this.alarmTopic.topicArn],
        conditions: { StringEquals: { 'aws:SourceAccount': this.account } },
      }),
    );
    if (config.alarmEmail) {
      this.alarmTopic.addSubscription(new subs.EmailSubscription(config.alarmEmail));
    }
    const action = new cwActions.SnsAction(this.alarmTopic);
    const alarm = (id: string, props: cw.CreateAlarmOptions & { metric: cw.IMetric }) => {
      const a = new cw.Alarm(this, id, { alarmName: `${prefix}-${id}`, treatMissingData: cw.TreatMissingData.NOT_BREACHING, ...props });
      a.addAlarmAction(action);
      a.addOkAction(action);
      return a;
    };

    // ── metrics ─────────────────────────────────────────────────────────────
    const alb = props.loadBalancer.metrics;
    const tg = props.targetGroup.metrics;
    const requests = alb.requestCount({ period: FIVE_MINUTES, statistic: 'Sum' });
    const target5xx = alb.httpCodeTarget(elbv2.HttpCodeTarget.TARGET_5XX_COUNT, { period: FIVE_MINUTES, statistic: 'Sum' });
    const elb5xx = alb.httpCodeElb(elbv2.HttpCodeElb.ELB_5XX_COUNT, { period: FIVE_MINUTES, statistic: 'Sum' });
    const latency = (statistic: string) => tg.targetResponseTime({ period: FIVE_MINUTES, statistic });

    const ciDims = { ClusterName: props.cluster.clusterName, ServiceName: props.service.serviceName };
    const ci = (metricName: string) =>
      new cw.Metric({ namespace: 'ECS/ContainerInsights', metricName, dimensionsMap: ciDims, period: Duration.minutes(1), statistic: 'Average' });
    const running = ci('RunningTaskCount');
    const desired = ci('DesiredTaskCount');

    const db = props.db;
    const dbMetric = (metricName: string, statistic = 'Average') => db.metric(metricName, { period: FIVE_MINUTES, statistic });

    const cacheNodes = Array.from({ length: 1 + config.cacheReplicas }, (_, i) => `${props.cacheReplicationGroupId}-00${i + 1}`);
    const cacheMetric = (metricName: string, node: string, statistic = 'Average') =>
      new cw.Metric({ namespace: 'AWS/ElastiCache', metricName, dimensionsMap: { CacheClusterId: node }, period: FIVE_MINUTES, statistic, label: `${metricName} ${node}` });

    const fn = props.runner;
    const fnMetric = (metricName: string, statistic: string) => fn.metric(metricName, { period: FIVE_MINUTES, statistic });

    // CloudFront publishes to us-east-1 regardless of where the stack lives.
    const cf = (metricName: string) =>
      new cw.Metric({
        namespace: 'AWS/CloudFront',
        metricName,
        dimensionsMap: { DistributionId: props.distribution.distributionId, Region: 'Global' },
        region: 'us-east-1',
        period: FIVE_MINUTES,
        statistic: 'Average',
      });
    const wafBlocked = new cw.Metric({
      namespace: 'AWS/WAFV2',
      metricName: 'BlockedRequests',
      dimensionsMap: { WebACL: props.webAclName, Region: this.region, Rule: 'ALL' },
      period: FIVE_MINUTES,
      statistic: 'Sum',
    });

    // ── alarms ──────────────────────────────────────────────────────────────
    alarm('alb-5xx-rate', {
      alarmDescription: 'More than 5 % of API requests failed with a 5xx (ALB or target) over 10 of the last 15 minutes.',
      metric: new cw.MathExpression({
        // Ignore quiet periods: 1 failure out of 3 requests at 4am is not an incident.
        expression: 'IF(req >= 20, 100 * (t5xx + e5xx) / req, 0)',
        usingMetrics: { req: requests, t5xx: target5xx, e5xx: elb5xx },
        period: FIVE_MINUTES,
        label: '5xx %',
      }),
      threshold: 5,
      evaluationPeriods: 3,
      datapointsToAlarm: 2,
      comparisonOperator: cw.ComparisonOperator.GREATER_THAN_THRESHOLD,
    });
    alarm('api-latency-p99', {
      alarmDescription: `Target p99 response time above ${config.latencyAlarmSeconds}s for 15 minutes (runs are synchronous; expect 1–4 s).`,
      metric: latency('p99'),
      threshold: config.latencyAlarmSeconds,
      evaluationPeriods: 3,
      comparisonOperator: cw.ComparisonOperator.GREATER_THAN_THRESHOLD,
    });
    alarm('ecs-running-below-desired', {
      alarmDescription: 'Fewer API tasks running than desired for 5 minutes (crash loop, capacity, or failing health checks).',
      metric: new cw.MathExpression({ expression: 'desired - running', usingMetrics: { desired, running }, period: Duration.minutes(1), label: 'missing tasks' }),
      threshold: 1,
      evaluationPeriods: 5,
      comparisonOperator: cw.ComparisonOperator.GREATER_THAN_OR_EQUAL_TO_THRESHOLD,
    });
    alarm('rds-cpu-high', {
      alarmDescription: 'PostgreSQL CPU above 80 % for 15 minutes.',
      metric: dbMetric('CPUUtilization'),
      threshold: 80,
      evaluationPeriods: 3,
      comparisonOperator: cw.ComparisonOperator.GREATER_THAN_THRESHOLD,
    });
    const lowStorageGiB = config.isProd ? 10 : 2;
    alarm('rds-free-storage-low', {
      alarmDescription: `Less than ${lowStorageGiB} GiB free. Storage autoscaling normally grows the volume first, so this means it hit dbMaxAllocatedStorageGiB or is throttled.`,
      metric: dbMetric('FreeStorageSpace', 'Minimum'),
      threshold: lowStorageGiB * 1024 ** 3,
      evaluationPeriods: 1,
      comparisonOperator: cw.ComparisonOperator.LESS_THAN_THRESHOLD,
    });
    const maxConnections = estimateMaxConnections(config.dbInstanceClass);
    alarm('rds-connections-high', {
      alarmDescription: `More than 80 % of ~${maxConnections} max_connections in use (tasks × DB_MAX_CONNECTIONS approaching the instance limit).`,
      metric: dbMetric('DatabaseConnections', 'Maximum'),
      threshold: Math.floor(maxConnections * 0.8),
      evaluationPeriods: 2,
      comparisonOperator: cw.ComparisonOperator.GREATER_THAN_THRESHOLD,
    });
    alarm('runner-errors', {
      alarmDescription: 'The runner function itself failed (not user programs: those are normal responses).',
      metric: fnMetric('Errors', 'Sum'),
      threshold: 5,
      evaluationPeriods: 1,
      comparisonOperator: cw.ComparisonOperator.GREATER_THAN_THRESHOLD,
    });
    alarm('runner-throttles', {
      alarmDescription: 'Runner invocations throttled: the reserved-concurrency cost ceiling is being hit and users see 503s.',
      metric: fnMetric('Throttles', 'Sum'),
      threshold: 10,
      evaluationPeriods: 1,
      comparisonOperator: cw.ComparisonOperator.GREATER_THAN_THRESHOLD,
    });
    alarm('waf-blocked-spike', {
      alarmDescription: `WAF blocked more than ${config.wafBlockedAlarmThreshold} requests in 5 minutes (attack, or a rule false-positive on real users).`,
      metric: wafBlocked,
      threshold: config.wafBlockedAlarmThreshold,
      evaluationPeriods: 1,
      comparisonOperator: cw.ComparisonOperator.GREATER_THAN_THRESHOLD,
    });
    if (props.sesEnabled) {
      alarm('ses-bounce-rate', {
        alarmDescription: 'SES account bounce rate above 5 % (AWS reviews accounts at 5 % and may pause sending at 10 %).',
        metric: new cw.Metric({ namespace: 'AWS/SES', metricName: 'Reputation.BounceRate', period: Duration.hours(1), statistic: 'Maximum' }),
        threshold: 0.05,
        evaluationPeriods: 1,
        comparisonOperator: cw.ComparisonOperator.GREATER_THAN_THRESHOLD,
      });
    }

    // ── dashboard ───────────────────────────────────────────────────────────
    const graph = (title: string, left: cw.IMetric[], right: cw.IMetric[] = [], width = 8) =>
      new cw.GraphWidget({ title, left, right, width, height: 6 });
    new cw.Dashboard(this, 'Dashboard', {
      dashboardName: `${prefix}-platform`,
      defaultInterval: Duration.hours(6),
      widgets: [
        [
          graph('ALB requests & 5xx', [requests], [target5xx, elb5xx]),
          graph('Target latency (s)', [latency('p50'), latency('p99')]),
          graph('CloudFront error rate (%)', [cf('4xxErrorRate'), cf('5xxErrorRate')], [cf('Requests')]),
        ],
        [
          graph('ECS CPU / memory (%)', [props.service.metricCpuUtilization({ period: FIVE_MINUTES }), props.service.metricMemoryUtilization({ period: FIVE_MINUTES })]),
          graph('ECS tasks', [running, desired]),
          graph('WAF blocked requests', [wafBlocked]),
        ],
        [
          graph('RDS CPU (%) / connections', [dbMetric('CPUUtilization')], [dbMetric('DatabaseConnections', 'Maximum')]),
          graph('RDS free storage (bytes)', [dbMetric('FreeStorageSpace', 'Minimum')]),
          graph('RDS read / write latency (s)', [dbMetric('ReadLatency'), dbMetric('WriteLatency')]),
        ],
        [
          graph('Cache engine CPU (%)', cacheNodes.map((n) => cacheMetric('EngineCPUUtilization', n))),
          graph('Cache memory used (%)', cacheNodes.map((n) => cacheMetric('DatabaseMemoryUsagePercentage', n))),
          graph('Cache evictions', cacheNodes.map((n) => cacheMetric('Evictions', n, 'Sum'))),
        ],
        [
          graph('Runner invocations / errors / throttles', [fnMetric('Invocations', 'Sum')], [fnMetric('Errors', 'Sum'), fnMetric('Throttles', 'Sum')]),
          graph('Runner duration p99 (ms)', [fnMetric('Duration', 'p99')]),
          graph('Runner concurrent executions', [fnMetric('ConcurrentExecutions', 'Maximum')]),
        ],
      ],
    });

    // ── budget ──────────────────────────────────────────────────────────────
    if (config.monthlyBudgetUsd > 0) {
      const subscribers = config.alarmEmail ? [{ subscriptionType: 'EMAIL', address: config.alarmEmail }] : [];
      const notify = (notificationType: 'ACTUAL' | 'FORECASTED', threshold: number) => ({
        notification: { notificationType, comparisonOperator: 'GREATER_THAN', threshold, thresholdType: 'PERCENTAGE' },
        subscribers,
      });
      new budgets.CfnBudget(this, 'MonthlyBudget', {
        budget: {
          budgetName: `${prefix}-monthly`,
          budgetType: 'COST',
          timeUnit: 'MONTHLY',
          budgetLimit: { amount: config.monthlyBudgetUsd, unit: 'USD' },
        },
        notificationsWithSubscribers: subscribers.length ? [notify('ACTUAL', 80), notify('ACTUAL', 100), notify('FORECASTED', 100)] : undefined,
      });
    }
  }
}
