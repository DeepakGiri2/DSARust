#!/usr/bin/env node
// Offline structural validation of the platform GitHub Actions workflows.
//
// GitHub only reports a broken workflow after it is pushed; this catches the
// common mistakes locally and in CI: YAML errors and duplicate keys, jobs
// without runs-on/uses, steps with both or neither of uses/run, `needs` that
// name no job, reusable workflows that do not declare workflow_call, OIDC jobs
// without an environment, and `${{ … }}` expressions interpolated straight
// into shell scripts (script injection) instead of passed through env.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseDocument } from 'yaml';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const dir = path.join(repoRoot, '.github', 'workflows');
const files = fs.readdirSync(dir).filter((f) => /^platform-.*\.ya?ml$/.test(f));
const problems = [];
const fail = (file, msg) => problems.push(`${file}: ${msg}`);

if (files.length === 0) fail('.github/workflows', 'no platform-*.yml workflows found');

for (const file of files) {
  const doc = parseDocument(fs.readFileSync(path.join(dir, file), 'utf8'), { uniqueKeys: true, prettyErrors: true });
  for (const e of [...doc.errors, ...doc.warnings]) fail(file, e.message);
  if (doc.errors.length) continue;
  const wf = doc.toJS();

  if (typeof wf.name !== 'string') fail(file, 'missing top-level name');
  if (!wf.on || typeof wf.on !== 'object') fail(file, 'missing "on" triggers');
  if (!wf.jobs || typeof wf.jobs !== 'object') {
    fail(file, 'missing jobs');
    continue;
  }

  for (const [id, job] of Object.entries(wf.jobs)) {
    const where = `jobs.${id}`;
    if (!job['runs-on'] && !job.uses) fail(file, `${where}: needs runs-on or uses`);
    for (const need of [job.needs ?? []].flat()) {
      if (!(need in wf.jobs)) fail(file, `${where}: needs unknown job "${need}"`);
    }
    if (typeof job.uses === 'string' && job.uses.startsWith('./')) {
      const target = path.join(repoRoot, job.uses);
      if (!fs.existsSync(target)) {
        fail(file, `${where}: reusable workflow ${job.uses} does not exist`);
      } else {
        const called = parseDocument(fs.readFileSync(target, 'utf8')).toJS();
        if (!called.on || !('workflow_call' in called.on)) fail(file, `${where}: ${job.uses} does not declare on.workflow_call`);
      }
    }
    if (job.permissions?.['id-token'] === 'write' && !job.environment) {
      fail(file, `${where}: requests an OIDC token without a GitHub environment (deploy roles trust environment-scoped subjects)`);
    }
    (job.steps ?? []).forEach((step, i) => {
      const s = `${where}.steps[${i}]${step.name ? ` (${step.name})` : ''}`;
      if (!!step.uses === !!step.run) fail(file, `${s}: exactly one of uses/run is required`);
      if (typeof step.run === 'string' && /\$\{\{\s*(github\.event|inputs|vars|secrets)\./.test(step.run)) {
        fail(file, `${s}: interpolates an untrusted/config expression into the script; pass it through env instead`);
      }
      if (typeof step.uses === 'string' && !step.uses.startsWith('./') && !/@[\w.-]+$/.test(step.uses)) {
        fail(file, `${s}: action ${step.uses} is not pinned to a version`);
      }
    });
  }
}

if (problems.length) {
  console.error(`Workflow validation failed:\n  ${problems.join('\n  ')}`);
  process.exit(1);
}
console.log(`Validated ${files.length} workflow(s): ${files.join(', ')}`);
