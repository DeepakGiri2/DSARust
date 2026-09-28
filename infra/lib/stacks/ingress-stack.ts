import { CfnOutput, Duration, RemovalPolicy, Stack, StackProps } from 'aws-cdk-lib';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as elbv2 from 'aws-cdk-lib/aws-elasticloadbalancingv2';
import * as route53 from 'aws-cdk-lib/aws-route53';
import * as targets from 'aws-cdk-lib/aws-route53-targets';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import * as wafv2 from 'aws-cdk-lib/aws-wafv2';
import { Construct } from 'constructs';
import type { StageConfig } from '../config';
import { acknowledge } from '../nag';
import { API_PORT, type PlatformSecurityGroups } from './network-stack';

/** Header CloudFront adds to every origin request and the ALB insists on. */
export const ORIGIN_VERIFY_HEADER = 'X-Origin-Verify';
/**
 * Header a CloudFront Function sets to the viewer's IP on every /api request.
 * The ALB only ever sees CloudFront's addresses, and the left-most
 * X-Forwarded-For entry is client-controlled, so WAF rate limits key on this.
 */
export const VIEWER_IP_HEADER = 'x-viewer-ip';

export interface IngressStackProps extends StackProps {
  readonly config: StageConfig;
  readonly vpc: ec2.IVpc;
  readonly securityGroups: PlatformSecurityGroups;
}

/**
 * The origin tier: a public ALB that only CloudFront can reach (prefix-list
 * security group + secret header), a regional WAF, and the API target group.
 */
export class IngressStack extends Stack {
  public readonly loadBalancer: elbv2.ApplicationLoadBalancer;
  public readonly targetGroup: elbv2.ApplicationTargetGroup;
  public readonly originVerifySecret: secretsmanager.Secret;
  public readonly originVerifySecretName: string;
  public readonly webAcl: wafv2.CfnWebACL;
  /** Host name CloudFront uses for the API origin. */
  public readonly originDomainName: string;
  /** Whether CloudFront reaches the origin over TLS. */
  public readonly originUsesHttps: boolean;

  constructor(scope: Construct, id: string, props: IngressStackProps) {
    super(scope, id, props);
    const { config, vpc, securityGroups } = props;
    const prefix = config.prefix;
    const domain = config.domain;

    // Shared secret between CloudFront (origin custom header), the ALB listener
    // rule and the API (ORIGIN_VERIFY_SECRET). 64 hex characters = 256 bits.
    // Bumping originVerifyGeneration creates a fresh secret and re-points all three.
    this.originVerifySecretName = `${prefix}/edge/origin-verify-g${config.originVerifyGeneration}`;
    this.originVerifySecret = new secretsmanager.Secret(this, 'OriginVerifySecret', {
      secretName: this.originVerifySecretName,
      description: `${prefix}: value of the ${ORIGIN_VERIFY_HEADER} header CloudFront sends to the ALB`,
      generateSecretString: {
        passwordLength: 64,
        excludeUppercase: true,
        excludePunctuation: true,
        excludeCharacters: 'ghijklmnopqrstuvwxyz',
        includeSpace: false,
        requireEachIncludedType: false,
      },
      removalPolicy: RemovalPolicy.DESTROY,
    });
    acknowledge(
      this.originVerifySecret,
      'Rotated by bumping originVerifyGeneration (a new secret re-points CloudFront, the listener rule and the API in one deployment); an unattended rotation would desynchronise them. See DEPLOY.md.',
      'AwsSolutions-SMG4',
    );

    this.loadBalancer = new elbv2.ApplicationLoadBalancer(this, 'Alb', {
      loadBalancerName: `${prefix}-alb`,
      vpc,
      internetFacing: true,
      vpcSubnets: { subnetType: ec2.SubnetType.PUBLIC },
      securityGroup: securityGroups.alb,
      // ≥ CloudFront's 60 s keep-alive, and long enough for runs and AI streams.
      idleTimeout: Duration.seconds(config.albIdleTimeoutSeconds),
      http2Enabled: true,
      dropInvalidHeaderFields: true,
      desyncMitigationMode: elbv2.DesyncMitigationMode.DEFENSIVE,
      deletionProtection: config.isProd,
    });

    if (config.isProd) {
      const accessLogs = new s3.Bucket(this, 'AlbAccessLogs', {
        // ALB log delivery supports SSE-S3 only.
        encryption: s3.BucketEncryption.S3_MANAGED,
        blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
        enforceSSL: true,
        objectOwnership: s3.ObjectOwnership.BUCKET_OWNER_ENFORCED,
        lifecycleRules: [{ expiration: Duration.days(config.logRetentionDays), abortIncompleteMultipartUploadAfter: Duration.days(1) }],
        removalPolicy: RemovalPolicy.RETAIN,
      });
      this.loadBalancer.logAccessLogs(accessLogs, 'alb');
      acknowledge(accessLogs, 'This bucket is the access-log destination itself; logging it to another bucket would recurse.', 'AwsSolutions-S1');
    } else {
      acknowledge(this.loadBalancer, 'dev skips ALB access logs (cost); CloudFront and application logs cover debugging. prod ships them to S3.', 'AwsSolutions-ELB2');
    }

    this.targetGroup = new elbv2.ApplicationTargetGroup(this, 'ApiTargets', {
      targetGroupName: `${prefix}-api`,
      vpc,
      port: API_PORT,
      protocol: elbv2.ApplicationProtocol.HTTP,
      targetType: elbv2.TargetType.IP,
      // Runs and AI streams have very uneven durations; route to the least busy task.
      loadBalancingAlgorithmType: elbv2.TargetGroupLoadBalancingAlgorithmType.LEAST_OUTSTANDING_REQUESTS,
      deregistrationDelay: Duration.seconds(config.isProd ? 120 : 15),
      healthCheck: {
        path: '/readyz',
        healthyHttpCodes: '200',
        interval: Duration.seconds(15),
        timeout: Duration.seconds(5),
        healthyThresholdCount: 2,
        unhealthyThresholdCount: 3,
      },
    });

    const forbidden = elbv2.ListenerAction.fixedResponse(403, { contentType: 'text/plain', messageBody: 'Forbidden' });
    let listener: elbv2.ApplicationListener;
    if (domain) {
      listener = this.loadBalancer.addListener('Https', {
        port: 443,
        protocol: elbv2.ApplicationProtocol.HTTPS,
        certificates: [elbv2.ListenerCertificate.fromArn(domain.albCertificateArn)],
        sslPolicy: elbv2.SslPolicy.RECOMMENDED_TLS,
        open: false,
        defaultAction: forbidden,
      });
      // Plain HTTP is never served: it only redirects. (The security group does
      // not open port 80 — CloudFront talks HTTPS to this origin.)
      this.loadBalancer.addListener('HttpRedirect', {
        port: 80,
        protocol: elbv2.ApplicationProtocol.HTTP,
        open: false,
        defaultAction: elbv2.ListenerAction.redirect({ protocol: 'HTTPS', port: '443', permanent: true }),
      });
    } else {
      // Without a certificate (no custom domain) CloudFront reaches the ALB by
      // its AWS DNS name over HTTP; the header check still applies.
      listener = this.loadBalancer.addListener('Http', {
        port: 80,
        protocol: elbv2.ApplicationProtocol.HTTP,
        open: false,
        defaultAction: forbidden,
      });
    }
    // Only requests carrying the CloudFront secret reach the API; anything else
    // (a scanner that found the ALB name, another CloudFront distribution) gets 403.
    listener.addAction('FromCloudFront', {
      priority: 10,
      conditions: [elbv2.ListenerCondition.httpHeader(ORIGIN_VERIFY_HEADER, [this.originVerifySecret.secretValue.unsafeUnwrap()])],
      action: elbv2.ListenerAction.forward([this.targetGroup]),
    });

    this.originUsesHttps = domain !== undefined;
    this.originDomainName = domain ? domain.originDomainName : this.loadBalancer.loadBalancerDnsName;
    if (domain?.hostedZoneId && domain.hostedZoneName) {
      const zone = route53.HostedZone.fromHostedZoneAttributes(this, 'Zone', {
        hostedZoneId: domain.hostedZoneId,
        zoneName: domain.hostedZoneName,
      });
      new route53.ARecord(this, 'OriginAlias', {
        zone,
        recordName: domain.originDomainName,
        target: route53.RecordTarget.fromAlias(new targets.LoadBalancerTarget(this.loadBalancer)),
        comment: `${prefix}: CloudFront → ALB origin`,
      });
    }

    this.webAcl = this.createWebAcl(config);
    new wafv2.CfnWebACLAssociation(this, 'WebAclAssociation', {
      resourceArn: this.loadBalancer.loadBalancerArn,
      webAclArn: this.webAcl.attrArn,
    });

    new CfnOutput(this, 'AlbDnsName', {
      value: this.loadBalancer.loadBalancerDnsName,
      description: domain && !domain.hostedZoneId ? `Create a CNAME ${domain.originDomainName} → this name` : 'ALB DNS name (CloudFront-only)',
    });
    new CfnOutput(this, 'TargetGroupArn', { value: this.targetGroup.targetGroupArn, description: 'Used by the deploy smoke test' });
  }

  private createWebAcl(config: StageConfig): wafv2.CfnWebACL {
    const prefix = config.prefix;
    const visibility = (metricName: string): wafv2.CfnWebACL.VisibilityConfigProperty => ({
      cloudWatchMetricsEnabled: true,
      sampledRequestsEnabled: true,
      metricName,
    });
    const viewerIpKey: wafv2.CfnWebACL.RateBasedStatementCustomKeyProperty = {
      header: { name: VIEWER_IP_HEADER, textTransformations: [{ priority: 0, type: 'NONE' }] },
    };
    const rateLimited: wafv2.CfnWebACL.RuleActionProperty = {
      block: { customResponse: { responseCode: 429, customResponseBodyKey: 'rate_limited' } },
    };
    const managed = (
      name: string,
      priority: number,
      countOnly: string[] = [],
    ): wafv2.CfnWebACL.RuleProperty => ({
      name,
      priority,
      overrideAction: { none: {} },
      statement: {
        managedRuleGroupStatement: {
          vendorName: 'AWS',
          name,
          ruleActionOverrides: countOnly.length ? countOnly.map((rule) => ({ name: rule, actionToUse: { count: {} } })) : undefined,
        },
      },
      visibilityConfig: visibility(`${prefix}-${name}`),
    });

    // Body signatures run in COUNT mode: request bodies here are user programs
    // (≤ 128 KiB of arbitrary C++/Java/Go/Python), drafts and progress imports,
    // on which XSS/LFI/RFI/SQLi/size signatures false-positive constantly. That
    // code only ever executes inside the network-less runner, and the API uses
    // bound SQL parameters; URI, query-string, header and cookie signatures
    // still block. COUNT keeps the telemetry.
    const bodyRulesCrs = ['SizeRestrictions_BODY', 'CrossSiteScripting_BODY', 'GenericLFI_BODY', 'GenericRFI_BODY', 'EC2MetaDataSSRF_BODY'];
    const bodyRulesSqli = ['SQLi_BODY', 'SQLiExtendedPatterns_BODY'];

    return new wafv2.CfnWebACL(this, 'WebAcl', {
      name: `${prefix}-alb`,
      description: `${prefix}: rate limits and managed rule groups in front of dsa-api`,
      scope: 'REGIONAL',
      defaultAction: { allow: {} },
      visibilityConfig: visibility(`${prefix}-alb`),
      customResponseBodies: {
        rate_limited: {
          contentType: 'APPLICATION_JSON',
          content: '{"error":{"code":"rate_limited","message":"Too many requests. Slow down and try again shortly."}}',
        },
      },
      rules: [
        {
          // Credential stuffing / sign-up abuse: a much tighter budget on /api/v1/auth/*.
          name: 'AuthRateLimitPerViewer',
          priority: 0,
          action: rateLimited,
          statement: {
            rateBasedStatement: {
              limit: config.wafAuthRateLimit,
              evaluationWindowSec: 300,
              aggregateKeyType: 'CUSTOM_KEYS',
              customKeys: [viewerIpKey],
              scopeDownStatement: {
                byteMatchStatement: {
                  fieldToMatch: { uriPath: {} },
                  positionalConstraint: 'STARTS_WITH',
                  searchString: '/api/v1/auth/',
                  textTransformations: [
                    { priority: 0, type: 'NORMALIZE_PATH' },
                    { priority: 1, type: 'LOWERCASE' },
                  ],
                },
              },
            },
          },
          visibilityConfig: visibility(`${prefix}-auth-rate`),
        },
        {
          name: 'ApiRateLimitPerViewer',
          priority: 1,
          action: rateLimited,
          statement: {
            rateBasedStatement: {
              limit: config.wafRateLimit,
              evaluationWindowSec: 300,
              aggregateKeyType: 'CUSTOM_KEYS',
              customKeys: [viewerIpKey],
            },
          },
          visibilityConfig: visibility(`${prefix}-api-rate`),
        },
        managed('AWSManagedRulesCommonRuleSet', 2, bodyRulesCrs),
        managed('AWSManagedRulesKnownBadInputsRuleSet', 3),
        managed('AWSManagedRulesSQLiRuleSet', 4, bodyRulesSqli),
      ],
    });
  }
}
