import * as fs from 'node:fs';
import * as path from 'node:path';
import { Annotations, CfnElement, CfnOutput, Duration, RemovalPolicy, SecretValue, Stack, StackProps } from 'aws-cdk-lib';
import * as acm from 'aws-cdk-lib/aws-certificatemanager';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';
import * as route53 from 'aws-cdk-lib/aws-route53';
import * as targets from 'aws-cdk-lib/aws-route53-targets';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as s3deploy from 'aws-cdk-lib/aws-s3-deployment';
import * as wafv2 from 'aws-cdk-lib/aws-wafv2';
import { Construct } from 'constructs';
import type { StageConfig } from '../config';
import { acknowledge, acknowledgeFindings, nagAccount, nagArn, nagRegion, REASONS } from '../nag';
import { ORIGIN_VERIFY_HEADER, VIEWER_IP_HEADER } from './ingress-stack';

export interface EdgeStackProps extends StackProps {
  readonly config: StageConfig;
  /** Host name of the API origin (ALB DNS name, or origin.<domain> over TLS). */
  readonly apiOriginDomainName: string;
  readonly apiOriginUsesHttps: boolean;
  /**
   * Name of the origin-verify secret (owned by the Ingress stack). Referenced by
   * name so the CloudFormation dynamic reference needs no cross-stack token.
   */
  readonly originVerifySecretName: string;
}

/**
 * Content-Security-Policy for the SPA. Everything is bundled by Vite (fonts
 * included), the API is same-origin, and nothing may frame the app.
 * `style-src` also allows inline styles because CodeMirror 6 (style-mod)
 * injects its theme as a runtime <style> element — a static SPA behind a CDN
 * has no per-response nonce to offer instead. Scripts stay strictly `'self'`
 * (the build has no inline scripts, eval or workers).
 */
export const CONTENT_SECURITY_POLICY = [
  "default-src 'self'",
  "script-src 'self'",
  "style-src 'self' 'unsafe-inline'",
  "font-src 'self'",
  "img-src 'self' data:",
  "connect-src 'self'",
  "manifest-src 'self'",
  "object-src 'none'",
  "base-uri 'self'",
  "form-action 'self'",
  "frame-ancestors 'none'",
  'upgrade-insecure-requests',
].join('; ');

/** Viewer-request function for the SPA: extension-less paths are client routes. */
const SPA_REWRITE_FUNCTION = `
function handler(event) {
  var request = event.request;
  var uri = request.uri;
  var last = uri.substring(uri.lastIndexOf('/') + 1);
  // "/", "/problems/two-sum", "/account/settings" -> the SPA shell.
  // "/favicon.svg", "/robots.txt" -> served as-is.
  if (last.indexOf('.') === -1) {
    request.uri = '/index.html';
  }
  return request;
}`;

/** Viewer-request function for /api: pins the real viewer IP for WAF rate limits. */
const VIEWER_IP_FUNCTION = `
function handler(event) {
  var request = event.request;
  // Overwrites anything the client sent under the same name.
  request.headers['${VIEWER_IP_HEADER}'] = { value: event.viewer.ip };
  return request;
}`;

/**
 * CloudFront in front of everything: the SPA from a private S3 bucket (OAC),
 * `/api/v1/content/*` cached from the ALB, and the rest of `/api/*` passed
 * through uncached with cookies and CSRF headers intact.
 */
export class EdgeStack extends Stack {
  public readonly distribution: cloudfront.Distribution;
  public readonly siteBucket: s3.Bucket;
  /** `https://<domain>` or `https://<id>.cloudfront.net`. */
  public readonly publicUrl: string;

  constructor(scope: Construct, id: string, props: EdgeStackProps) {
    super(scope, id, props);
    const { config } = props;
    const prefix = config.prefix;
    const domain = config.domain;

    // ── buckets ─────────────────────────────────────────────────────────────
    const logsBucket = config.isProd
      ? new s3.Bucket(this, 'EdgeLogs', {
          encryption: s3.BucketEncryption.S3_MANAGED,
          blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
          enforceSSL: true,
          // CloudFront standard logging writes with an ACL grant, which needs ACLs enabled.
          objectOwnership: s3.ObjectOwnership.BUCKET_OWNER_PREFERRED,
          lifecycleRules: [{ expiration: Duration.days(config.logRetentionDays), abortIncompleteMultipartUploadAfter: Duration.days(1) }],
          removalPolicy: RemovalPolicy.RETAIN,
        })
      : undefined;
    if (logsBucket) {
      acknowledge(logsBucket, 'This bucket is the access-log destination itself; logging it to another bucket would recurse.', 'AwsSolutions-S1');
    }

    this.siteBucket = new s3.Bucket(this, 'SiteBucket', {
      encryption: s3.BucketEncryption.S3_MANAGED,
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      objectOwnership: s3.ObjectOwnership.BUCKET_OWNER_ENFORCED,
      enforceSSL: true,
      versioned: config.isProd,
      serverAccessLogsBucket: logsBucket,
      serverAccessLogsPrefix: logsBucket ? 's3/site/' : undefined,
      lifecycleRules: [
        { abortIncompleteMultipartUploadAfter: Duration.days(1) },
        ...(config.isProd ? [{ noncurrentVersionExpiration: Duration.days(30) }] : []),
      ],
      // The bucket only holds build output, which CI can always re-create.
      removalPolicy: config.isProd ? RemovalPolicy.RETAIN : RemovalPolicy.DESTROY,
      autoDeleteObjects: !config.isProd,
    });
    if (!logsBucket) {
      acknowledge(this.siteBucket, 'dev skips S3 server access logs (cost); the bucket is private and only CloudFront (OAC) can read it.', 'AwsSolutions-S1');
    }

    // ── functions and policies ──────────────────────────────────────────────
    const spaRewrite = new cloudfront.Function(this, 'SpaRewrite', {
      functionName: `${prefix}-spa-rewrite`,
      comment: 'Serve /index.html for client-side routes',
      runtime: cloudfront.FunctionRuntime.JS_2_0,
      code: cloudfront.FunctionCode.fromInline(SPA_REWRITE_FUNCTION),
    });
    const viewerIp = new cloudfront.Function(this, 'ViewerIp', {
      functionName: `${prefix}-viewer-ip`,
      comment: `Set ${VIEWER_IP_HEADER} for WAF rate limiting`,
      runtime: cloudfront.FunctionRuntime.JS_2_0,
      code: cloudfront.FunctionCode.fromInline(VIEWER_IP_FUNCTION),
    });

    // index.html carries `no-store`; with MinTTL 0 the edge honours it, so a
    // deploy is visible immediately.
    const shellCache = new cloudfront.CachePolicy(this, 'ShellCache', {
      cachePolicyName: `${prefix}-spa-shell`,
      comment: 'SPA shell and root files: honour origin Cache-Control, default no caching',
      minTtl: Duration.seconds(0),
      defaultTtl: Duration.seconds(0),
      maxTtl: Duration.days(1),
      enableAcceptEncodingGzip: true,
      enableAcceptEncodingBrotli: true,
    });
    const assetsCache = new cloudfront.CachePolicy(this, 'AssetsCache', {
      cachePolicyName: `${prefix}-assets`,
      comment: 'Content-hashed Vite assets: cache for a year',
      minTtl: Duration.days(1),
      defaultTtl: Duration.days(365),
      maxTtl: Duration.days(365),
      enableAcceptEncodingGzip: true,
      enableAcceptEncodingBrotli: true,
    });
    // Public content: the key is path + query only, no cookies or headers, and
    // the API's Cache-Control (public, ETag = content version) decides the TTL.
    const contentCache = new cloudfront.CachePolicy(this, 'ContentCache', {
      cachePolicyName: `${prefix}-api-content`,
      comment: 'GET /api/v1/content/*: keyed on path + query, TTL from origin Cache-Control',
      minTtl: Duration.seconds(0),
      defaultTtl: Duration.seconds(0),
      maxTtl: Duration.days(1),
      queryStringBehavior: cloudfront.CacheQueryStringBehavior.all(),
      headerBehavior: cloudfront.CacheHeaderBehavior.none(),
      cookieBehavior: cloudfront.CacheCookieBehavior.none(),
      enableAcceptEncodingGzip: true,
      enableAcceptEncodingBrotli: true,
    });
    const contentOriginRequest = new cloudfront.OriginRequestPolicy(this, 'ContentOriginRequest', {
      originRequestPolicyName: `${prefix}-api-content`,
      comment: 'Cached content: forward the query string and the viewer IP only (no cookies)',
      queryStringBehavior: cloudfront.OriginRequestQueryStringBehavior.all(),
      headerBehavior: cloudfront.OriginRequestHeaderBehavior.allowList(VIEWER_IP_HEADER),
      cookieBehavior: cloudfront.OriginRequestCookieBehavior.none(),
    });

    const securityHeaders = new cloudfront.ResponseHeadersPolicy(this, 'SecurityHeaders', {
      responseHeadersPolicyName: `${prefix}-security-headers`,
      comment: 'Security headers for the SPA and the API',
      securityHeadersBehavior: {
        contentSecurityPolicy: { contentSecurityPolicy: CONTENT_SECURITY_POLICY, override: true },
        contentTypeOptions: { override: true },
        frameOptions: { frameOption: cloudfront.HeadersFrameOption.DENY, override: true },
        referrerPolicy: { referrerPolicy: cloudfront.HeadersReferrerPolicy.STRICT_ORIGIN_WHEN_CROSS_ORIGIN, override: true },
        ...(config.isProd
          ? { strictTransportSecurity: { accessControlMaxAge: Duration.days(730), includeSubdomains: true, preload: false, override: true } }
          : {}),
      },
      customHeadersBehavior: {
        customHeaders: [
          { header: 'Permissions-Policy', value: 'camera=(), microphone=(), geolocation=(), payment=()', override: true },
          { header: 'Cross-Origin-Opener-Policy', value: 'same-origin', override: true },
        ],
      },
      removeHeaders: ['Server', 'X-Powered-By'],
    });

    // ── origins ─────────────────────────────────────────────────────────────
    const siteOrigin = origins.S3BucketOrigin.withOriginAccessControl(this.siteBucket, {
      originAccessLevels: [cloudfront.AccessLevel.READ],
    });
    const apiOrigin = new origins.HttpOrigin(props.apiOriginDomainName, {
      protocolPolicy: props.apiOriginUsesHttps ? cloudfront.OriginProtocolPolicy.HTTPS_ONLY : cloudfront.OriginProtocolPolicy.HTTP_ONLY,
      originSslProtocols: [cloudfront.OriginSslPolicy.TLS_V1_2],
      // Synchronous runs can take tens of seconds before the first byte.
      readTimeout: Duration.seconds(60),
      // Below the ALB idle timeout, so the ALB never closes a connection CloudFront still considers open.
      keepaliveTimeout: Duration.seconds(60),
      connectionAttempts: 3,
      connectionTimeout: Duration.seconds(10),
      customHeaders: {
        // A CloudFormation dynamic reference: resolved at deploy time, never in the template.
        [ORIGIN_VERIFY_HEADER]: SecretValue.secretsManager(props.originVerifySecretName).unsafeUnwrap(),
      },
    });

    const edgeAcl = this.createEdgeWebAcl(config);

    this.distribution = new cloudfront.Distribution(this, 'Distribution', {
      comment: `${prefix}: DSA Visualized web + API`,
      defaultRootObject: 'index.html',
      domainNames: domain ? [domain.domainName] : undefined,
      certificate: domain ? acm.Certificate.fromCertificateArn(this, 'Certificate', domain.certificateArn) : undefined,
      minimumProtocolVersion: domain ? cloudfront.SecurityPolicyProtocol.TLS_V1_2_2021 : undefined,
      httpVersion: cloudfront.HttpVersion.HTTP2_AND_3,
      enableIpv6: true,
      priceClass: cloudfront.PriceClass[config.priceClass === 'PriceClass_All' ? 'PRICE_CLASS_ALL' : config.priceClass === 'PriceClass_200' ? 'PRICE_CLASS_200' : 'PRICE_CLASS_100'],
      enableLogging: logsBucket !== undefined,
      logBucket: logsBucket,
      logFilePrefix: logsBucket ? 'cloudfront/' : undefined,
      webAclId: edgeAcl?.attrArn,
      defaultBehavior: {
        origin: siteOrigin,
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        allowedMethods: cloudfront.AllowedMethods.ALLOW_GET_HEAD,
        cachePolicy: shellCache,
        responseHeadersPolicy: securityHeaders,
        compress: true,
        functionAssociations: [{ function: spaRewrite, eventType: cloudfront.FunctionEventType.VIEWER_REQUEST }],
      },
      // Order matters: CloudFront matches behaviours top to bottom, so the
      // cacheable content prefix must precede the catch-all /api/*.
      additionalBehaviors: {
        '/assets/*': {
          origin: siteOrigin,
          viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
          allowedMethods: cloudfront.AllowedMethods.ALLOW_GET_HEAD,
          cachePolicy: assetsCache,
          responseHeadersPolicy: securityHeaders,
          compress: true,
        },
        '/api/v1/content/*': {
          origin: apiOrigin,
          viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.HTTPS_ONLY,
          allowedMethods: cloudfront.AllowedMethods.ALLOW_GET_HEAD_OPTIONS,
          cachedMethods: cloudfront.CachedMethods.CACHE_GET_HEAD,
          cachePolicy: contentCache,
          originRequestPolicy: contentOriginRequest,
          responseHeadersPolicy: securityHeaders,
          compress: true,
          functionAssociations: [{ function: viewerIp, eventType: cloudfront.FunctionEventType.VIEWER_REQUEST }],
        },
        '/api/*': {
          origin: apiOrigin,
          viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.HTTPS_ONLY,
          allowedMethods: cloudfront.AllowedMethods.ALLOW_ALL,
          cachePolicy: cloudfront.CachePolicy.CACHING_DISABLED,
          // Cookies (dsa_session), X-CSRF-Token and Origin must reach the API;
          // Host stays the origin's name so TLS to the ALB validates.
          originRequestPolicy: cloudfront.OriginRequestPolicy.ALL_VIEWER_EXCEPT_HOST_HEADER,
          responseHeadersPolicy: securityHeaders,
          // The API compresses itself (and must not have SSE streams buffered).
          compress: false,
          functionAssociations: [{ function: viewerIp, eventType: cloudfront.FunctionEventType.VIEWER_REQUEST }],
        },
      },
    });

    this.publicUrl = `https://${domain ? domain.domainName : this.distribution.distributionDomainName}`;

    if (domain?.hostedZoneId && domain.hostedZoneName) {
      const zone = route53.HostedZone.fromHostedZoneAttributes(this, 'Zone', {
        hostedZoneId: domain.hostedZoneId,
        zoneName: domain.hostedZoneName,
      });
      const target = route53.RecordTarget.fromAlias(new targets.CloudFrontTarget(this.distribution));
      new route53.ARecord(this, 'SiteA', { zone, recordName: domain.domainName, target });
      new route53.AaaaRecord(this, 'SiteAaaa', { zone, recordName: domain.domainName, target });
    }

    this.deployWebsite(config);

    // ── cdk-nag acknowledgements ────────────────────────────────────────────
    acknowledge(this.distribution, 'DSA Visualized is a global product; there is no country to restrict.', 'AwsSolutions-CFR1');
    if (!edgeAcl) {
      acknowledge(
        this.distribution,
        'dev has no edge web ACL (cost); /api is still filtered by the regional WAF on the ALB, and everything else is static, public build output. prod attaches an edge ACL.',
        'AwsSolutions-CFR2',
      );
    }
    if (!logsBucket) {
      acknowledge(this.distribution, 'dev skips CloudFront standard logs (cost); prod ships them to S3.', 'AwsSolutions-CFR3');
    }
    if (!domain) {
      acknowledge(
        this.distribution,
        'Without a custom domain the distribution uses the default *.cloudfront.net certificate, whose viewer TLS policy cannot be changed; with a domain it enforces TLSv1.2_2021.',
        'AwsSolutions-CFR4',
      );
      acknowledge(
        this.distribution,
        'Without a custom domain there is no certificate the ALB could present for its AWS DNS name, so CloudFront → ALB is HTTP (still gated by the prefix-list security group and the origin-verify header). Configure domainName for TLS end to end.',
        'AwsSolutions-CFR5',
      );
    }

    new CfnOutput(this, 'PublicUrl', { value: this.publicUrl, description: 'The platform URL (SPA + /api)' });
    new CfnOutput(this, 'DistributionId', { value: this.distribution.distributionId });
    new CfnOutput(this, 'DistributionDomainName', { value: this.distribution.distributionDomainName });
    new CfnOutput(this, 'SiteBucketName', { value: this.siteBucket.bucketName });
  }

  /**
   * Edge (CLOUDFRONT-scope) ACL: IP reputation must be evaluated where the
   * viewer's address is visible. At the ALB every request arrives from a
   * CloudFront address, so the managed IP-reputation list can only work here.
   * CLOUDFRONT-scope ACLs can only be created in us-east-1.
   */
  private createEdgeWebAcl(config: StageConfig): wafv2.CfnWebACL | undefined {
    if (!config.edgeWaf) return undefined;
    if (this.region !== 'us-east-1') {
      Annotations.of(this).addWarningV2(
        'dsa:edgeWafRegion',
        'edgeWaf requires the stage to be deployed in us-east-1 (CLOUDFRONT-scope web ACLs live there); skipping the edge ACL.',
      );
      return undefined;
    }
    const prefix = config.prefix;
    return new wafv2.CfnWebACL(this, 'EdgeWebAcl', {
      name: `${prefix}-edge`,
      description: `${prefix}: viewer-IP reputation at the edge`,
      scope: 'CLOUDFRONT',
      defaultAction: { allow: {} },
      visibilityConfig: { cloudWatchMetricsEnabled: true, sampledRequestsEnabled: true, metricName: `${prefix}-edge` },
      rules: [
        {
          name: 'AWSManagedRulesAmazonIpReputationList',
          priority: 0,
          overrideAction: { none: {} },
          statement: { managedRuleGroupStatement: { vendorName: 'AWS', name: 'AWSManagedRulesAmazonIpReputationList' } },
          visibilityConfig: { cloudWatchMetricsEnabled: true, sampledRequestsEnabled: true, metricName: `${prefix}-edge-ip-reputation` },
        },
      ],
    });
  }

  /**
   * Uploads `web/dist` in three passes so each object gets the right
   * Cache-Control and a new shell never references an asset that is not
   * there yet: assets first (immutable, never pruned — tabs still running the
   * previous build keep lazy-loading their chunks), then root files, then
   * index.html (no-store) with a CloudFront invalidation.
   */
  private deployWebsite(config: StageConfig): void {
    const dist = config.webDistDir;
    if (!fs.existsSync(path.join(dist, 'index.html'))) {
      Annotations.of(this).addWarningV2(
        'dsa:webDistMissing',
        `${dist} has no index.html: the SPA is not deployed by this synth. Run "npm ci && npm run build" in web/ first.`,
      );
      return;
    }
    // Source maps expose the original TypeScript; prod does not publish them.
    const source = s3deploy.Source.asset(dist, { exclude: config.publishSourceMaps ? [] : ['**/*.map'] });
    const common = { sources: [source], destinationBucket: this.siteBucket, memoryLimit: 1024 };

    const assets = new s3deploy.BucketDeployment(this, 'DeployAssets', {
      ...common,
      exclude: ['*'],
      include: ['assets/*'],
      prune: false,
      cacheControl: [s3deploy.CacheControl.fromString('public, max-age=31536000, immutable')],
    });
    const rootFiles = new s3deploy.BucketDeployment(this, 'DeployRootFiles', {
      ...common,
      exclude: ['assets/*', 'index.html'],
      prune: true,
      cacheControl: [s3deploy.CacheControl.fromString('public, max-age=3600')],
    });
    const shell = new s3deploy.BucketDeployment(this, 'DeployShell', {
      ...common,
      exclude: ['*'],
      include: ['index.html'],
      prune: false,
      cacheControl: [s3deploy.CacheControl.fromString('no-cache, no-store, must-revalidate')],
      distribution: this.distribution,
      distributionPaths: ['/index.html'],
    });
    shell.node.addDependency(assets, rootFiles);

    // The three deployments share one CDK-managed handler Lambda (a singleton).
    const handler = this.node.children.find((c) => c.node.id.startsWith('Custom::CDKBucketDeployment'));
    if (handler) {
      acknowledge(handler, REASONS.cdkManagedRuntime, 'AwsSolutions-L1');
      acknowledgeFindings(
        handler,
        'AwsSolutions-IAM5',
        [
          'Action::s3:GetBucket*',
          'Action::s3:GetObject*',
          'Action::s3:List*',
          'Action::s3:Abort*',
          'Action::s3:DeleteObject*',
          `Resource::${nagArn(this, 's3', `cdk-${config.cdkQualifier}-assets-${nagAccount(this)}-${nagRegion(this)}/*`, { global: true, noAccount: true })}`,
          `Resource::<${this.getLogicalId(this.siteBucket.node.defaultChild as CfnElement)}.Arn>/*`,
          'Resource::*',
        ],
        'CDK-managed BucketDeployment handler: its grants are the aws s3 sync verbs on the CDK asset bucket and on this site bucket only; ' +
          'CloudFront invalidation actions support no resource-level permissions.',
      );
      acknowledgeFindings(
        handler,
        'AwsSolutions-IAM4',
        ['Policy::arn:<AWS::Partition>:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole'],
        REASONS.lambdaBasicExecution,
      );
    }
  }
}
