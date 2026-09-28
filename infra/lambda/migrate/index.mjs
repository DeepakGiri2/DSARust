// CloudFormation custom resource: run `dsa-api migrate` as a one-off Fargate
// task and succeed only if it exits 0.
//
// The API service depends on this resource, so on every deployment that
// changes the migration task definition (i.e. every new API image) the
// database is migrated *before* the service rolls to the new code, and a failed
// migration fails the stack update while the old tasks keep serving.
//
// Plain ESM with no dependencies: the AWS SDK v3 ships with the Lambda Node.js
// runtime, so there is nothing to bundle.
import { CloudFormationClient, DescribeStacksCommand } from '@aws-sdk/client-cloudformation';
import { CloudWatchLogsClient, GetLogEventsCommand } from '@aws-sdk/client-cloudwatch-logs';
import { DescribeTasksCommand, ECSClient, RunTaskCommand, StopTaskCommand } from '@aws-sdk/client-ecs';

const ecs = new ECSClient({});
const cfn = new CloudFormationClient({});
const logs = new CloudWatchLogsClient({});

const POLL_MS = 10_000;
// Leave enough time to stop the task and report before Lambda kills us.
const SAFETY_MARGIN_MS = 60_000;

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

export const handler = async (event, context) => {
  const physicalId = event.PhysicalResourceId ?? `${event.LogicalResourceId}-migrations`;
  console.log(JSON.stringify({ requestType: event.RequestType, stackId: event.StackId, logicalId: event.LogicalResourceId }));
  try {
    if (event.RequestType === 'Delete') {
      return await respond(event, 'SUCCESS', physicalId, 'Nothing to undo: migrations are forward-only.');
    }
    if (event.RequestType === 'Update' && (await isRollingBack(event.StackId))) {
      // Re-running the *previous* image's migrate against a schema that is
      // already newer would fail and wedge the rollback (UPDATE_ROLLBACK_FAILED).
      // Migrations are expand/contract, so the old code runs on the new schema.
      return await respond(event, 'SUCCESS', physicalId, 'Skipped: stack is rolling back.');
    }
    const outcome = await runMigrations(event.ResourceProperties, context);
    return await respond(event, outcome.ok ? 'SUCCESS' : 'FAILED', physicalId, outcome.reason, { TaskArn: outcome.taskArn ?? '' });
  } catch (err) {
    console.error(err);
    return respond(event, 'FAILED', physicalId, `Migration runner error: ${err?.message ?? String(err)}`);
  }
};

async function isRollingBack(stackId) {
  const out = await cfn.send(new DescribeStacksCommand({ StackName: stackId }));
  const status = out.Stacks?.[0]?.StackStatus ?? '';
  return status.includes('ROLLBACK');
}

async function runMigrations(p, context) {
  const run = await startTask(p);
  const taskArn = run.taskArn;
  console.log(`started ${taskArn}`);

  for (;;) {
    if (context.getRemainingTimeInMillis() < SAFETY_MARGIN_MS) {
      await ecs.send(new StopTaskCommand({ cluster: p.Cluster, task: taskArn, reason: 'Migration exceeded the custom resource deadline' }));
      return { ok: false, taskArn, reason: `Migration did not finish in time and was stopped (${taskArn}). ${await tail(p, taskArn)}` };
    }
    await sleep(POLL_MS);
    const described = await ecs.send(new DescribeTasksCommand({ cluster: p.Cluster, tasks: [taskArn] }));
    const task = described.tasks?.[0];
    if (!task || task.lastStatus !== 'STOPPED') continue;

    const container = task.containers?.find((c) => c.name === p.ContainerName);
    const exitCode = container?.exitCode;
    if (exitCode === 0) {
      return { ok: true, taskArn, reason: `Migrations applied (${taskArn}).` };
    }
    const why = [task.stopCode, task.stoppedReason, container?.reason].filter(Boolean).join(' / ');
    return {
      ok: false,
      taskArn,
      reason: `Migration task failed (exit code ${exitCode ?? 'none'}${why ? `; ${why}` : ''}). ${await tail(p, taskArn)}`,
    };
  }
}

async function startTask(p) {
  // RunTask can fail transiently (capacity, ENI limits); retry a few times.
  let lastError = 'unknown';
  for (let attempt = 1; attempt <= 5; attempt++) {
    const out = await ecs.send(
      new RunTaskCommand({
        cluster: p.Cluster,
        taskDefinition: p.TaskDefinition,
        launchType: 'FARGATE',
        platformVersion: 'LATEST',
        count: 1,
        startedBy: 'cfn-migrations',
        networkConfiguration: {
          awsvpcConfiguration: {
            subnets: p.Subnets,
            securityGroups: p.SecurityGroups,
            assignPublicIp: 'DISABLED',
          },
        },
      }),
    );
    const task = out.tasks?.[0];
    if (task?.taskArn) return task;
    lastError = (out.failures ?? []).map((f) => `${f.arn ?? ''} ${f.reason ?? ''} ${f.detail ?? ''}`.trim()).join('; ') || 'no task returned';
    console.warn(`RunTask attempt ${attempt} failed: ${lastError}`);
    await sleep(attempt * 5_000);
  }
  throw new Error(`RunTask failed: ${lastError}`);
}

/** The last few log lines of the task, so the failure reason is actionable from the CloudFormation console. */
async function tail(p, taskArn) {
  try {
    const taskId = taskArn.split('/').pop();
    const out = await logs.send(
      new GetLogEventsCommand({
        logGroupName: p.LogGroup,
        logStreamName: `${p.LogStreamPrefix}/${p.ContainerName}/${taskId}`,
        limit: 8,
        startFromHead: false,
      }),
    );
    const lines = (out.events ?? []).map((e) => (e.message ?? '').trim()).filter(Boolean);
    return lines.length ? `Last log lines: ${lines.join(' | ')}` : `No log output (log group ${p.LogGroup}).`;
  } catch (err) {
    return `Logs unavailable (${err?.name ?? 'error'}); see log group ${p.LogGroup}.`;
  }
}

async function respond(event, status, physicalId, reason, data) {
  // CloudFormation caps the response body at 4 KiB.
  const body = JSON.stringify({
    Status: status,
    Reason: String(reason).slice(0, 3000),
    PhysicalResourceId: physicalId,
    StackId: event.StackId,
    RequestId: event.RequestId,
    LogicalResourceId: event.LogicalResourceId,
    Data: data,
  });
  console.log(`responding ${status}: ${reason}`);
  for (let attempt = 1; attempt <= 3; attempt++) {
    try {
      const res = await fetch(event.ResponseURL, { method: 'PUT', body, headers: { 'content-type': '' } });
      if (res.ok) return;
      console.error(`response PUT returned ${res.status}`);
    } catch (err) {
      console.error(`response PUT failed: ${err?.message ?? err}`);
    }
    await sleep(attempt * 2_000);
  }
}
