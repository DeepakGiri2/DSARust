import * as fs from 'node:fs';
import * as path from 'node:path';
import { App, CfnElement, Stack } from 'aws-cdk-lib';
import { Template } from 'aws-cdk-lib/assertions';
import type { IConstruct } from 'constructs';
import { loadStageConfig } from '../lib/config';
import { buildPlatform, type Platform } from '../lib/platform';

const INFRA_DIR = path.resolve(__dirname, '..');

/** The feature flags from cdk.json, so tests synthesize exactly like the CLI does. */
const CDK_JSON_CONTEXT = JSON.parse(fs.readFileSync(path.join(INFRA_DIR, 'cdk.json'), 'utf8')).context as Record<string, unknown>;

/** Tiny stand-in repository: stub Dockerfiles and a two-file web/dist. */
export const FIXTURE_CONTEXT = { repoRoot: 'test/fixtures/repo' };

export const DOMAIN_CONTEXT = {
  domainName: 'dsa.example.com',
  hostedZoneId: 'Z0123456789ABCDEFGHIJ',
  hostedZoneName: 'dsa.example.com',
  certificateArn: 'arn:aws:acm:us-east-1:111111111111:certificate/11111111-2222-3333-4444-555555555555',
};

export type StackKey = 'network' | 'data' | 'runner' | 'ingress' | 'edge' | 'api' | 'observability';

export interface Built {
  readonly app: App;
  readonly platform: Platform;
  readonly templates: Record<StackKey, Template>;
  /** Raw template JSON per stack. */
  readonly json: Record<StackKey, TemplateJson>;
}

export interface TemplateJson {
  Resources: Record<string, { Type: string; Properties?: any; DependsOn?: string | string[]; DeletionPolicy?: string }>;
  Outputs?: Record<string, any>;
}

export function buildApp(context: Record<string, unknown>): Built {
  const app = new App({ context: { ...CDK_JSON_CONTEXT, ...FIXTURE_CONTEXT, ...context } });
  const platform = buildPlatform(app, loadStageConfig(app, INFRA_DIR));
  const keys: StackKey[] = ['network', 'data', 'runner', 'ingress', 'edge', 'api', 'observability'];
  const templates = Object.fromEntries(keys.map((k) => [k, Template.fromStack(platform[k])])) as Record<StackKey, Template>;
  const json = Object.fromEntries(keys.map((k) => [k, templates[k].toJSON() as TemplateJson])) as Record<StackKey, TemplateJson>;
  return { app, platform, templates, json };
}

export function logicalId(construct: IConstruct): string {
  const element = (construct.node.defaultChild ?? construct) as CfnElement;
  return Stack.of(construct).getLogicalId(element);
}

export function resourcesOfType(t: TemplateJson, type: string): Array<[string, any]> {
  return Object.entries(t.Resources).filter(([, r]) => r.Type === type);
}

/** True when a CloudFormation value (Ref/GetAtt/cross-stack output) points at the given logical id. */
export function refersTo(value: unknown, id: string): boolean {
  return JSON.stringify(value ?? null).includes(id);
}
