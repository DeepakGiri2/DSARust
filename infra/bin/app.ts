#!/usr/bin/env node
/**
 * DSA Visualized — cloud platform.
 *
 *   npx cdk synth  -c stage=dev
 *   npx cdk deploy -c stage=prod -c domainName=… --all
 *
 * Every parameter is documented in lib/config.ts and docs/platform/DEPLOY.md.
 */
import { App } from 'aws-cdk-lib';
import { loadStageConfig } from '../lib/config';
import { buildPlatform } from '../lib/platform';

const app = new App();
buildPlatform(app, loadStageConfig(app));
app.synth();
