#!/usr/bin/env node

import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import {
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const rendererPath = join(repoRoot, "scripts/render-marketing-posters.mjs");
const sourceDirectory = join(repoRoot, "docs/assets/marketing/source");
const outputDirectory = join(repoRoot, "docs/assets/marketing");
const darkOutputName = "loadlynx-project-poster-dark.png";
const lightOutputName = "loadlynx-project-poster-light.png";
const darkSourceName = "loadlynx-project-poster-dark.svg";
const lockName = ".loadlynx-marketing-poster.lock";
const recoveryMarkerName = "recovery.json";

function createFixture() {
  const root = realpathSync(mkdtempSync(join(tmpdir(), "loadlynx-marketing-poster-publication-")));
  const fixtureScripts = join(root, "scripts");
  const fixtureSource = join(root, "docs/assets/marketing/source");
  const fixtureOutput = join(root, "docs/assets/marketing");
  mkdirSync(fixtureScripts, { recursive: true });
  mkdirSync(fixtureOutput, { recursive: true });
  copyFileSync(rendererPath, join(fixtureScripts, "render-marketing-posters.mjs"));
  cpSync(sourceDirectory, fixtureSource, { recursive: true });
  copyFileSync(join(outputDirectory, darkOutputName), join(fixtureOutput, darkOutputName));
  copyFileSync(join(outputDirectory, lightOutputName), join(fixtureOutput, lightOutputName));
  return {
    root,
    renderer: join(fixtureScripts, "render-marketing-posters.mjs"),
    source: fixtureSource,
    output: fixtureOutput,
  };
}

function runRenderer(fixture, args, overrides = {}) {
  const environment = { ...process.env };
  delete environment.LOADLYNX_MARKETING_TEST_INTERRUPT_AFTER_OUTPUT_RENAME;
  delete environment.LOADLYNX_MARKETING_TEST_INTERRUPT_AFTER_ROLLBACK_RENAME;
  delete environment.LOADLYNX_MARKETING_TEST_INTERRUPT_BEFORE_LOCK_PUBLISH;
  delete environment.LOADLYNX_MARKETING_TEST_PAUSE_AFTER_LOCK_PUBLISH_MS;
  delete environment.LOADLYNX_MARKETING_TEST_PAUSE_AFTER_RECOVERY_CLAIM_MS;
  delete environment.LOADLYNX_MARKETING_TEST_PAUSE_AFTER_LOCK_RETIRE_MS;
  Object.assign(environment, overrides);
  return spawnSync(process.execPath, [fixture.renderer, ...args], {
    cwd: fixture.root,
    encoding: "utf8",
    env: environment,
  });
}

function pause(milliseconds) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, milliseconds);
}

function waitForFile(path, timeoutMilliseconds) {
  const deadline = Date.now() + timeoutMilliseconds;
  while (!existsSync(path)) {
    if (Date.now() >= deadline) {
      throw new Error(`timed out waiting for ${path}`);
    }
    pause(20);
  }
}

function waitForAbsence(path, timeoutMilliseconds) {
  const deadline = Date.now() + timeoutMilliseconds;
  while (existsSync(path)) {
    if (Date.now() >= deadline) {
      throw new Error(`timed out waiting for ${path} to disappear`);
    }
    pause(20);
  }
}

function waitForChild(child) {
  if (child.exitCode !== null || child.signalCode !== null) {
    return Promise.resolve({ status: child.exitCode, signal: child.signalCode });
  }
  return new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("close", (status, signal) => resolve({ status, signal }));
  });
}

function outputBuffers(fixture) {
  return {
    dark: readFileSync(join(fixture.output, darkOutputName)),
    light: readFileSync(join(fixture.output, lightOutputName)),
  };
}

function makeDarkRenderDifferent(fixture) {
  const sourcePath = join(fixture.source, darkSourceName);
  const source = readFileSync(sourcePath, "utf8");
  const replaced = source.replace(
    /<\/svg>\s*$/,
    '<rect x="0" y="0" width="48" height="48" fill="#ff00ff"/></svg>\n',
  );
  assert.notEqual(replaced, source, "fixture dark poster source must end with an SVG root");
  writeFileSync(sourcePath, replaced);
}

function removeDarkSource(fixture) {
  rmSync(join(fixture.source, darkSourceName));
}

function prepareInterruptedPair(fixture) {
  const original = outputBuffers(fixture);
  makeDarkRenderDifferent(fixture);
  const interrupted = runRenderer(fixture, ["--all"], {
    LOADLYNX_MARKETING_TEST_INTERRUPT_AFTER_OUTPUT_RENAME: "1",
  });
  assert.equal(interrupted.status, 86, `expected deterministic publication interruption: ${interrupted.stderr}`);
  assert.notDeepEqual(outputBuffers(fixture).dark, original.dark, "interruption must occur after dark output replacement");
  return original;
}

function assertRecoveryPreservesApprovedPair(fixture, original, result) {
  assert.notEqual(result.status, 0, `expected staged render to fail: ${result.stderr}`);
  assert.deepEqual(outputBuffers(fixture), original, "recovery must restore the approved poster pair");
  assert.equal(
    existsSync(join(fixture.output, lockName)),
    false,
    `recovery must remove the transaction lock: ${result.stderr}`,
  );
}

function verifyInterruptedPublication(args) {
  const fixture = createFixture();
  try {
    const original = outputBuffers(fixture);
    makeDarkRenderDifferent(fixture);
    const interrupted = runRenderer(fixture, args, {
      LOADLYNX_MARKETING_TEST_INTERRUPT_AFTER_OUTPUT_RENAME: "1",
    });
    assert.equal(interrupted.status, 86, `expected deterministic publication interruption: ${interrupted.stderr}`);
    assert.notDeepEqual(outputBuffers(fixture).dark, original.dark, "interruption must occur after dark output replacement");

    removeDarkSource(fixture);
    assertRecoveryPreservesApprovedPair(fixture, original, runRenderer(fixture, ["--all"]));
  } finally {
    rmSync(fixture.root, { force: true, recursive: true });
  }
}

function verifyPendingLockIsNeverPublished() {
  const fixture = createFixture();
  try {
    const original = outputBuffers(fixture);
    const interrupted = runRenderer(fixture, ["--all"], {
      LOADLYNX_MARKETING_TEST_INTERRUPT_BEFORE_LOCK_PUBLISH: "1",
    });
    assert.equal(interrupted.status, 85, `expected deterministic pre-publication interruption: ${interrupted.stderr}`);
    assert.equal(existsSync(join(fixture.output, lockName)), false, "incomplete transaction must not become visible as a lock");
    assert.deepEqual(outputBuffers(fixture), original, "pre-publication interruption must not alter outputs");

    removeDarkSource(fixture);
    assertRecoveryPreservesApprovedPair(fixture, original, runRenderer(fixture, ["--all"]));
  } finally {
    rmSync(fixture.root, { force: true, recursive: true });
  }
}

function verifyIncompleteLockDoesNotBlockRecovery() {
  const fixture = createFixture();
  try {
    const original = outputBuffers(fixture);
    mkdirSync(join(fixture.output, lockName));
    removeDarkSource(fixture);
    assertRecoveryPreservesApprovedPair(fixture, original, runRenderer(fixture, ["--all"]));
  } finally {
    rmSync(fixture.root, { force: true, recursive: true });
  }
}

function verifyPidReuseDoesNotOwnStaleLock() {
  const fixture = createFixture();
  try {
    const original = outputBuffers(fixture);
    const lockDirectory = join(fixture.output, lockName);
    const darkOutput = join(fixture.output, darkOutputName);
    mkdirSync(lockDirectory);
    writeFileSync(
      join(lockDirectory, "transaction.json"),
      `${JSON.stringify({
        version: 3,
        pid: process.pid,
        ownerStartedAt: "stale-owner-identity",
        phase: "rendering",
        lockDirectory,
        jobs: [
          {
            output: darkOutput,
            stagedOutput: join(lockDirectory, `staged-0-${darkOutputName}`),
            backupOutput: join(lockDirectory, `previous-0-${darkOutputName}`),
            discardedOutput: join(lockDirectory, `discarded-0-${darkOutputName}`),
            restoreOutput: join(lockDirectory, `restoring-0-${darkOutputName}`),
            hadOriginal: true,
            rollbackState: "pending",
          },
        ],
      })}\n`,
    );

    removeDarkSource(fixture);
    const result = runRenderer(fixture, ["--all"]);
    assert.doesNotMatch(result.stderr, /Another poster render owns the transaction lock/);
    assertRecoveryPreservesApprovedPair(fixture, original, result);
  } finally {
    rmSync(fixture.root, { force: true, recursive: true });
  }
}

function verifyRecoveryRollbackCanResume() {
  const fixture = createFixture();
  try {
    const original = prepareInterruptedPair(fixture);
    removeDarkSource(fixture);
    const interruptedRecovery = runRenderer(fixture, ["--all"], {
      LOADLYNX_MARKETING_TEST_INTERRUPT_AFTER_ROLLBACK_RENAME: "2",
    });
    assert.equal(interruptedRecovery.status, 87, `expected deterministic rollback interruption: ${interruptedRecovery.stderr}`);
    assert.equal(existsSync(join(fixture.output, lockName)), true, "rollback interruption must retain the transaction lock");
    assert.deepEqual(outputBuffers(fixture).light, original.light, "the first rollback rename must restore one output");

    const recovered = runRenderer(fixture, ["--all"]);
    assertRecoveryPreservesApprovedPair(fixture, original, recovered);
  } finally {
    rmSync(fixture.root, { force: true, recursive: true });
  }
}

async function verifyRecoveryClaimSerializesSecondWriter() {
  const fixture = createFixture();
  const lockPath = join(fixture.output, lockName);
  const recoveryPath = join(lockPath, recoveryMarkerName);
  const original = prepareInterruptedPair(fixture);
  removeDarkSource(fixture);
  const child = spawn(process.execPath, [fixture.renderer, "--all"], {
    cwd: fixture.root,
    env: {
      ...process.env,
      LOADLYNX_MARKETING_TEST_PAUSE_AFTER_RECOVERY_CLAIM_MS: "2000",
    },
    stdio: "ignore",
  });
  try {
    waitForFile(recoveryPath, 5000);
    const contender = runRenderer(fixture, ["--all"]);
    assert.notEqual(contender.status, 0, "a second writer must not enter recovery while the claim is active");
    assert.match(contender.stderr, /Another poster recovery owns the transaction lock/);
    assert.equal(existsSync(lockPath), true, "the active recovery claim must retain the transaction lock");
    assert.deepEqual(await waitForChild(child), { status: 1, signal: null });
    assert.deepEqual(outputBuffers(fixture), original, "the single recovery owner must restore the approved pair");
    assert.equal(existsSync(lockPath), false, "the recovery owner must remove the transaction lock before render failure");
  } finally {
    if (child.exitCode === null && child.signalCode === null) {
      child.kill("SIGKILL");
    }
    rmSync(fixture.root, { force: true, recursive: true });
  }
}

async function verifyLockRetirementDoesNotDeleteNewWriter() {
  const fixture = createFixture();
  const lockPath = join(fixture.output, lockName);
  const original = prepareInterruptedPair(fixture);
  removeDarkSource(fixture);
  const recovery = spawn(process.execPath, [fixture.renderer, "--all"], {
    cwd: fixture.root,
    env: {
      ...process.env,
      LOADLYNX_MARKETING_TEST_PAUSE_AFTER_LOCK_RETIRE_MS: "2500",
    },
    stdio: "ignore",
  });
  let writer;
  try {
    waitForAbsence(lockPath, 5000);
    writer = spawn(process.execPath, [fixture.renderer, "--variant", "dark"], {
      cwd: fixture.root,
      env: {
        ...process.env,
        LOADLYNX_MARKETING_TEST_PAUSE_AFTER_LOCK_PUBLISH_MS: "4000",
      },
      stdio: "ignore",
    });
    waitForFile(lockPath, 5000);
    assert.deepEqual(await waitForChild(recovery), { status: 1, signal: null });
    assert.equal(existsSync(lockPath), true, "retiring the old lock must not remove the new writer lock");
    assert.deepEqual(await waitForChild(writer), { status: 1, signal: null });
    assert.deepEqual(outputBuffers(fixture), original, "lock retirement must preserve the recovered approved pair");
    assert.equal(existsSync(lockPath), false, "the new writer must clean up its own lock");
  } finally {
    for (const child of [recovery, writer]) {
      if (child?.exitCode === null && child.signalCode === null) {
        child.kill("SIGKILL");
      }
    }
    rmSync(fixture.root, { force: true, recursive: true });
  }
}

async function verifyActiveSingleWriterBlocksPairPublication() {
  const fixture = createFixture();
  const lockPath = join(fixture.output, lockName);
  const child = spawn(process.execPath, [fixture.renderer, "--variant", "dark"], {
    cwd: fixture.root,
    env: {
      ...process.env,
      LOADLYNX_MARKETING_TEST_PAUSE_AFTER_LOCK_PUBLISH_MS: "2000",
    },
    stdio: "ignore",
  });
  try {
    waitForFile(lockPath, 5000);
    const contender = runRenderer(fixture, ["--all"]);
    assert.notEqual(contender.status, 0, "pair publication must not run while a single writer owns the lock");
    assert.match(contender.stderr, /Another poster render owns the transaction lock/);
    assert.deepEqual(await waitForChild(child), { status: 0, signal: null });
  } finally {
    if (child.exitCode === null && child.signalCode === null) {
      child.kill("SIGKILL");
    }
    rmSync(fixture.root, { force: true, recursive: true });
  }
}

verifyInterruptedPublication(["--all"]);
verifyInterruptedPublication(["--variant", "dark"]);
verifyPendingLockIsNeverPublished();
verifyIncompleteLockDoesNotBlockRecovery();
verifyPidReuseDoesNotOwnStaleLock();
verifyRecoveryRollbackCanResume();
await verifyRecoveryClaimSerializesSecondWriter();
await verifyLockRetirementDoesNotDeleteNewWriter();
await verifyActiveSingleWriterBlocksPairPublication();
console.log("marketing poster publication recovery passed");
