#!/usr/bin/env node
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const workflowPath = new URL("../workflows/notify-release-failure.yml", import.meta.url);
const workflow = await readFile(workflowPath, "utf8");
const oidruneReference =
  "IvanLi-CN/oidrune/.github/workflows/notify.yml@e48822f99c6402a753ed86557ea029754cbab20b";

function countOccurrences(source, value) {
  return source.split(value).length - 1;
}

function jobBlock(jobId, nextJobId) {
  const start = workflow.indexOf(`  ${jobId}:\n`);
  assert.notEqual(start, -1, `missing job ${jobId}`);
  const end = nextJobId ? workflow.indexOf(`  ${nextJobId}:\n`, start) : workflow.length;
  assert.notEqual(end, -1, `missing boundary after job ${jobId}`);
  return workflow.slice(start, end);
}

assert.match(workflow, /^name: Notify failed release$/m);
assert.match(workflow, /  workflow_run:\n/);
assert.match(workflow, /    workflows:\n      - Release \(LoadLynx\)\n/);
assert.match(workflow, /    types:\n      - completed\n/);
assert.match(workflow, /    branches:\n      - main\n/);
assert.match(workflow, /  workflow_dispatch:\n/);
assert.match(workflow, /^permissions:\n  id-token: write\n/m);
assert.equal(countOccurrences(workflow, "      - Release (LoadLynx)"), 1);

assert.equal(countOccurrences(workflow, oidruneReference), 2);
assert.doesNotMatch(workflow, /IvanLi-CN\/github-workflows/);
assert.doesNotMatch(workflow, /\.github\/workflows\/[^\n]+@main/);
assert.doesNotMatch(workflow, /^\s+secrets:\s*$/m);
assert.doesNotMatch(workflow, /^\s+(gateway_url|oidc_audience):/m);
assert.doesNotMatch(workflow, /^\s+(push|pull_request|schedule):/m);

const failureJob = jobBlock("notify_failure", "smoke_test");
assert.match(
  failureJob,
  /if: \$\{\{ github\.event_name == 'workflow_run' && github\.event\.workflow_run\.conclusion == 'failure' \}\}/,
);
assert.match(failureJob, new RegExp(`uses: ${oidruneReference.replaceAll("/", "\\/")}`));
assert.match(failureJob, /      outcome: failure\n      summary: \|\n/);
assert.match(failureJob, /^        Project: LoadLynx$/m);
assert.match(failureJob, /^        Status: failure$/m);
assert.match(failureJob, /^        Result: failure$/m);
assert.match(failureJob, /^        Target SHA: \$\{\{ github\.event\.workflow_run\.head_sha \}\}$/m);
assert.match(failureJob, /^        Run URL: \$\{\{ github\.event\.workflow_run\.html_url \}\}$/m);
assert.match(failureJob, /^        Failure title: Release \(LoadLynx\) failed$/m);
assert.doesNotMatch(failureJob, /^        Smoke title:/m);
assert.match(failureJob, /^        Release context: /m);

const smokeJob = jobBlock("smoke_test");
assert.match(smokeJob, /if: \$\{\{ github\.event_name == 'workflow_dispatch' \}\}/);
assert.match(smokeJob, new RegExp(`uses: ${oidruneReference.replaceAll("/", "\\/")}`));
assert.match(smokeJob, /      outcome: failure\n      summary: \|\n/);
assert.match(smokeJob, /^        Project: LoadLynx$/m);
assert.match(smokeJob, /^        Status: failure$/m);
assert.match(smokeJob, /^        Result: failure$/m);
assert.match(smokeJob, /^        Target SHA: \$\{\{ github\.sha \}\}$/m);
assert.match(
  smokeJob,
  /^        Run URL: \$\{\{ format\('\{0\}\/\{1\}\/actions\/runs\/\{2\}', github\.server_url, github\.repository, github\.run_id\) \}\}$/m,
);
assert.match(smokeJob, /^        Smoke title: Release failure notification smoke test$/m);
assert.doesNotMatch(smokeJob, /^        Failure title:/m);
assert.match(smokeJob, /^        Smoke context: manual notifier smoke test$/m);

console.log("release failure notifier workflow contract passed");
