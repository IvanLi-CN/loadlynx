#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import {
  closeSync,
  copyFileSync,
  existsSync,
  fsyncSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const sourceDirectory = resolve(repoRoot, "docs/assets/marketing/source");
const posterVariants = {
  dark: {
    source: resolve(sourceDirectory, "loadlynx-project-poster-dark.svg"),
    output: resolve(repoRoot, "docs/assets/marketing/loadlynx-project-poster-dark.png"),
    hero: resolve(sourceDirectory, "loadlynx-project-poster-dark-hero.png"),
    primary: resolve(sourceDirectory, "loadlynx-project-poster-dark-white.svg"),
    cyan: resolve(sourceDirectory, "loadlynx-project-poster-dark-cyan.svg"),
    decor: resolve(sourceDirectory, "loadlynx-project-poster-dark-decor.svg"),
  },
  light: {
    source: resolve(sourceDirectory, "loadlynx-project-poster-light.svg"),
    output: resolve(repoRoot, "docs/assets/marketing/loadlynx-project-poster-light.png"),
    hero: resolve(sourceDirectory, "loadlynx-project-poster-light-hero.png"),
    primary: resolve(sourceDirectory, "loadlynx-project-poster-light-primary.svg"),
    cyan: resolve(sourceDirectory, "loadlynx-project-poster-light-cyan.svg"),
    decor: resolve(sourceDirectory, "loadlynx-project-poster-light-decor.svg"),
  },
};
const outputWidth = 1122;
const outputHeight = 1402;
const transactionLockName = ".loadlynx-marketing-poster.lock";
const transactionJournalName = "transaction.json";
const transactionVersion = 2;
const approvedPosterOutputs = new Set(
  Object.values(posterVariants).map((variant) => variant.output),
);

const args = process.argv.slice(2);
if (args.includes("--help")) {
  console.log(
    "Usage: node scripts/render-marketing-posters.mjs [--variant <dark|light>] [--out <path>] [--all] [--verify] [--max-ae <non-negative integer>]",
  );
  console.log("Requires rsvg-convert (librsvg) and ImageMagick compare.");
  console.log("macOS: brew install librsvg imagemagick");
  console.log("Debian/Ubuntu: sudo apt-get install librsvg2-bin imagemagick");
  process.exit(0);
}

const outIndex = args.indexOf("--out");
const maxAeIndex = args.indexOf("--max-ae");
if (outIndex !== -1 && !args[outIndex + 1]) {
  console.error("--out requires a file path");
  process.exit(1);
}
if (maxAeIndex !== -1 && !args[maxAeIndex + 1]) {
  console.error("--max-ae requires a non-negative integer");
  process.exit(1);
}
if (maxAeIndex !== -1 && !args.includes("--verify")) {
  console.error("--max-ae can only be used with --verify");
  process.exit(1);
}
const maxAe = maxAeIndex === -1 ? 0 : Number(args[maxAeIndex + 1]);
if (!Number.isSafeInteger(maxAe) || maxAe < 0) {
  console.error("--max-ae requires a non-negative integer");
  process.exit(1);
}

const variantIndex = args.indexOf("--variant");
const variantName = variantIndex === -1 ? "dark" : args[variantIndex + 1];
if (variantIndex !== -1 && !variantName) {
  console.error("--variant requires dark or light");
  process.exit(1);
}
if (!(variantName in posterVariants)) {
  console.error(`Unknown poster variant: ${variantName}`);
  process.exit(1);
}
if (args.includes("--all") && outIndex !== -1) {
  console.error("--out cannot be combined with --all");
  process.exit(1);
}
if (
  args.includes("--verify") &&
  (args.includes("--all") || outIndex !== -1 || variantIndex !== -1)
) {
  console.error("--verify checks both committed poster variants and cannot be combined with --variant, --out, or --all");
  process.exit(1);
}

function dataUri(path, mimeType) {
  return `data:${mimeType};base64,${readFileSync(path).toString("base64")}`;
}

function requireCommand(command, packageHint) {
  const result = spawnSync(command, ["--version"], { stdio: "ignore" });
  if (result.error?.code === "ENOENT") {
    throw new Error(`${command} is required; install ${packageHint}`);
  }
  if (result.error) {
    throw new Error(`${command} could not start: ${result.error.message}`);
  }
  if (result.status !== 0) {
    throw new Error(`${command} preflight failed with exit code ${result.status}`);
  }
}

function renderPoster(variant, output) {
  const renderedSvg = readFileSync(variant.source, "utf8")
    .replace("__HERO_RENDER_DATA_URI__", dataUri(variant.hero, "image/png"))
    .replace("__PRIMARY_VECTOR_DATA_URI__", dataUri(variant.primary, "image/svg+xml"))
    .replace("__CYAN_VECTOR_DATA_URI__", dataUri(variant.cyan, "image/svg+xml"))
    .replace("__DECOR_VECTOR_DATA_URI__", dataUri(variant.decor, "image/svg+xml"));

  if (/__[A-Z_]+__/.test(renderedSvg)) {
    throw new Error("A layered poster source placeholder is missing");
  }

  const renderedSvgPath = join(dirname(output), `${basename(output)}.svg`);
  writeFileSync(renderedSvgPath, renderedSvg);

  const result = spawnSync(
    "rsvg-convert",
    [
      "--width",
      String(outputWidth),
      "--height",
      String(outputHeight),
      "--output",
      output,
      renderedSvgPath,
    ],
    { stdio: "inherit" },
  );

  if (result.error) {
    throw new Error(`rsvg-convert could not start: ${result.error.message}`);
  }
  if (result.status !== 0) {
    throw new Error(`rsvg-convert failed with exit code ${result.status ?? 1}`);
  }
}

function syncFile(path) {
  const descriptor = openSync(path, "r");
  try {
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
}

function syncDirectory(directory) {
  const descriptor = openSync(directory, "r");
  try {
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
}

function renameAndSync(source, destination) {
  renameSync(source, destination);
  syncDirectory(dirname(destination));
}

function interruptPublicationForTest(point) {
  if (
    (point === "before-lock-publish" && process.env.LOADLYNX_MARKETING_TEST_INTERRUPT_BEFORE_LOCK_PUBLISH === "1") ||
    (point.startsWith("after-output-") &&
      process.env.LOADLYNX_MARKETING_TEST_INTERRUPT_AFTER_OUTPUT_RENAME === point.slice("after-output-".length))
  ) {
    process.exit(point === "before-lock-publish" ? 85 : 86);
  }
}

function pauseAfterLockPublicationForTest() {
  const duration = Number(process.env.LOADLYNX_MARKETING_TEST_PAUSE_AFTER_LOCK_PUBLISH_MS);
  if (Number.isSafeInteger(duration) && duration > 0) {
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, duration);
  }
}

function stageOutput(output) {
  const outputDirectory = dirname(output);
  mkdirSync(outputDirectory, { recursive: true });
  const stagingDirectory = mkdtempSync(join(outputDirectory, `.${basename(output)}.`));
  return {
    stagingDirectory,
    stagedOutput: join(stagingDirectory, basename(output)),
  };
}

function renderSingleAtomically(job) {
  const stage = stageOutput(job.output);
  try {
    renderPoster(job.variant, stage.stagedOutput);
    syncFile(stage.stagedOutput);
    renameAndSync(stage.stagedOutput, job.output);
  } finally {
    rmSync(stage.stagingDirectory, { force: true, recursive: true });
  }
}

function commonOutputDirectory(jobs) {
  const directories = [...new Set(jobs.map((job) => dirname(job.output)))];
  if (directories.length !== 1) {
    throw new Error("Multi-poster rendering requires one shared output directory");
  }
  return directories[0];
}

function isPathInside(parent, child) {
  const pathFromParent = relative(parent, child);
  return pathFromParent !== "" && !pathFromParent.startsWith("..");
}

function transactionJournalPath(lockDirectory) {
  return join(lockDirectory, transactionJournalName);
}

function writeTransactionJournal(transaction, journalDirectory = transaction.lockDirectory) {
  const journalPath = transactionJournalPath(journalDirectory);
  const temporaryPath = join(journalDirectory, `transaction-${process.pid}.tmp`);
  const descriptor = openSync(temporaryPath, "w");
  try {
    writeFileSync(descriptor, `${JSON.stringify(transaction)}\n`);
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
  renameAndSync(temporaryPath, journalPath);
}

function linuxProcessStartIdentity(pid) {
  let stat;
  try {
    stat = readFileSync(`/proc/${pid}/stat`, "utf8");
  } catch (error) {
    if (error?.code === "ENOENT") {
      return existsSync("/proc") ? null : undefined;
    }
    if (error?.code === "EACCES") {
      return undefined;
    }
    throw error;
  }
  const commandEnd = stat.lastIndexOf(") ");
  if (commandEnd === -1) {
    return null;
  }
  const fields = stat.slice(commandEnd + 2).trim().split(/\s+/);
  const startTime = fields[19];
  if (!startTime) {
    return null;
  }
  let bootId;
  try {
    bootId = readFileSync("/proc/sys/kernel/random/boot_id", "utf8").trim();
  } catch (error) {
    if (error?.code === "ENOENT" || error?.code === "EACCES") {
      bootId = "unknown-boot";
    } else {
      throw error;
    }
  }
  return `linux:${bootId}:${startTime}`;
}

function processStartIdentity(pid) {
  const linuxIdentity = linuxProcessStartIdentity(pid);
  if (linuxIdentity !== undefined) {
    return linuxIdentity;
  }
  const result = spawnSync("ps", ["-o", "lstart=", "-p", String(pid)], {
    encoding: "utf8",
  });
  if (result.error) {
    throw new Error(`ps could not inspect poster transaction owner: ${result.error.message}`);
  }
  if (result.status !== 0) {
    return null;
  }
  const identity = result.stdout.trim().replace(/\s+/g, " ");
  return identity ? `ps:${identity}` : null;
}

function isTransactionOwnerRunning(transaction) {
  return processStartIdentity(transaction.pid) === transaction.ownerStartedAt;
}

function transactionMatchesJobs(transaction, lockDirectory, permittedOutputs) {
  if (
    !transaction ||
    transaction.version !== transactionVersion ||
    !Number.isSafeInteger(transaction.pid) ||
    transaction.pid <= 0 ||
    typeof transaction.ownerStartedAt !== "string" ||
    !transaction.ownerStartedAt ||
    transaction.lockDirectory !== lockDirectory ||
    !["rendering", "publishing", "published"].includes(transaction.phase) ||
    !Array.isArray(transaction.jobs)
  ) {
    return false;
  }

  const recordedOutputs = transaction.jobs.map((job) => job.output).sort();
  if (
    recordedOutputs.length === 0 ||
    recordedOutputs.length !== new Set(recordedOutputs).size ||
    recordedOutputs.some((output) => !permittedOutputs.has(output))
  ) {
    return false;
  }

  return transaction.jobs.every(
    (job) =>
      typeof job.hadOriginal === "boolean" &&
      typeof job.output === "string" &&
      typeof job.stagedOutput === "string" &&
      typeof job.backupOutput === "string" &&
      typeof job.discardedOutput === "string" &&
      isPathInside(lockDirectory, job.stagedOutput) &&
      isPathInside(lockDirectory, job.backupOutput) &&
      isPathInside(lockDirectory, job.discardedOutput),
  );
}

function readTransaction(lockDirectory, permittedOutputs) {
  const journalPath = transactionJournalPath(lockDirectory);
  if (!existsSync(journalPath)) {
    return null;
  }

  let transaction;
  try {
    transaction = JSON.parse(readFileSync(journalPath, "utf8"));
  } catch (error) {
    throw new Error(`Poster transaction journal is unreadable: ${error.message}`);
  }
  if (!transactionMatchesJobs(transaction, lockDirectory, permittedOutputs)) {
    throw new Error(`Poster transaction journal does not match this output set: ${lockDirectory}`);
  }
  return transaction;
}

function restoreTransactionOutputs(transaction) {
  const rollbackErrors = [];
  for (const job of [...transaction.jobs].reverse()) {
    try {
      if (job.hadOriginal && existsSync(job.backupOutput)) {
        renameAndSync(job.backupOutput, job.output);
      } else if (!job.hadOriginal && !existsSync(job.stagedOutput) && existsSync(job.output)) {
        renameAndSync(job.output, job.discardedOutput);
      } else if (job.hadOriginal) {
        throw new Error(`missing backup for ${job.output}`);
      }
    } catch (error) {
      rollbackErrors.push(error instanceof Error ? error.message : String(error));
    }
  }
  if (rollbackErrors.length === 0) {
    syncDirectory(transaction.lockDirectory);
  }
  return rollbackErrors;
}

function removeTransactionLock(lockDirectory) {
  rmSync(lockDirectory, { force: true, recursive: true });
  syncDirectory(dirname(lockDirectory));
}

function recoverInterruptedTransaction(outputDirectory, permittedOutputs) {
  const lockDirectory = join(outputDirectory, transactionLockName);
  if (!existsSync(lockDirectory)) {
    return;
  }

  const transaction = readTransaction(lockDirectory, permittedOutputs);
  if (transaction === null) {
    removeTransactionLock(lockDirectory);
    return;
  }
  if (isTransactionOwnerRunning(transaction)) {
    throw new Error(`Another poster render owns the transaction lock: ${lockDirectory}`);
  }

  if (transaction.phase === "publishing") {
    const rollbackErrors = restoreTransactionOutputs(transaction);
    if (rollbackErrors.length > 0) {
      throw new Error(`Interrupted poster transaction could not be restored: ${rollbackErrors.join("; ")}`);
    }
  }
  removeTransactionLock(lockDirectory);
}

function createTransaction(outputDirectory, jobs) {
  const lockDirectory = join(outputDirectory, transactionLockName);
  const ownerStartedAt = processStartIdentity(process.pid);
  if (ownerStartedAt === null) {
    throw new Error("ps could not identify poster transaction owner");
  }
  const pendingLockDirectory = mkdtempSync(join(outputDirectory, `.${transactionLockName}.pending-`));
  const transaction = {
    version: transactionVersion,
    pid: process.pid,
    ownerStartedAt,
    phase: "rendering",
    lockDirectory,
    jobs: jobs.map((job, index) => ({
      output: job.output,
      stagedOutput: join(lockDirectory, `staged-${index}-${basename(job.output)}`),
      backupOutput: join(lockDirectory, `previous-${index}-${basename(job.output)}`),
      discardedOutput: join(lockDirectory, `discarded-${index}-${basename(job.output)}`),
      hadOriginal: existsSync(job.output),
    })),
  };
  try {
    writeTransactionJournal(transaction, pendingLockDirectory);
    interruptPublicationForTest("before-lock-publish");
    try {
      renameSync(pendingLockDirectory, lockDirectory);
    } catch (error) {
      if (error?.code === "EEXIST" || error?.code === "ENOTEMPTY") {
        throw new Error(`Another poster render acquired the transaction lock: ${lockDirectory}`);
      }
      throw error;
    }
    syncDirectory(outputDirectory);
    pauseAfterLockPublicationForTest();
  } catch (error) {
    rmSync(pendingLockDirectory, { force: true, recursive: true });
    throw error;
  }
  return transaction;
}

function renderPosterSetAtomically(jobs, permittedOutputs) {
  const outputDirectory = commonOutputDirectory(jobs);
  recoverInterruptedTransaction(outputDirectory, permittedOutputs);

  let transaction;
  try {
    transaction = createTransaction(outputDirectory, jobs);
    const stagedJobs = jobs.map((job, index) => ({
      ...job,
      ...transaction.jobs[index],
    }));
    for (const job of stagedJobs) {
      renderPoster(job.variant, job.stagedOutput);
      syncFile(job.stagedOutput);
    }

    for (const job of transaction.jobs) {
      if (job.hadOriginal) {
        copyFileSync(job.output, job.backupOutput);
        syncFile(job.backupOutput);
      }
    }
    syncDirectory(transaction.lockDirectory);
    transaction.phase = "publishing";
    writeTransactionJournal(transaction);
    for (const [index, job] of transaction.jobs.entries()) {
      renameAndSync(job.stagedOutput, job.output);
      interruptPublicationForTest(`after-output-${index + 1}`);
    }
    transaction.phase = "published";
    writeTransactionJournal(transaction);
  } finally {
    if (transaction?.phase === "publishing") {
      const rollbackErrors = restoreTransactionOutputs(transaction);
      if (rollbackErrors.length > 0) {
        throw new Error(`Poster outputs could not be restored: ${rollbackErrors.join("; ")}`);
      }
      removeTransactionLock(transaction.lockDirectory);
    } else if (transaction?.phase === "rendering" || transaction?.phase === "published") {
      removeTransactionLock(transaction.lockDirectory);
    }
  }
}

function renderAtomically(jobs) {
  const approvedOutputJobs = jobs.filter((job) => approvedPosterOutputs.has(job.output));
  if (approvedOutputJobs.length > 0) {
    if (approvedOutputJobs.length !== jobs.length) {
      throw new Error("Approved poster outputs cannot be mixed with custom output paths");
    }
    renderPosterSetAtomically(jobs, approvedPosterOutputs);
    return;
  }
  if (jobs.length === 1) {
    renderSingleAtomically(jobs[0]);
    return;
  }
  renderPosterSetAtomically(jobs, new Set(jobs.map((job) => job.output)));
}

function verifyPosters(maxAllowedAe) {
  const verificationDirectory = mkdtempSync(join(tmpdir(), "loadlynx-marketing-posters-"));
  try {
    const jobs = Object.entries(posterVariants).map(([name, variant]) => ({
      variant,
      output: join(verificationDirectory, `${name}.png`),
    }));
    renderAtomically(jobs);

    for (const [name, variant] of Object.entries(posterVariants)) {
      const actual = join(verificationDirectory, `${name}.png`);
      const result = spawnSync(
        "compare",
        ["-metric", "AE", actual, variant.output, "null:"],
        { encoding: "utf8" },
      );
      if (result.error) {
        throw new Error(`compare could not start: ${result.error.message}`);
      }
      if (result.status !== 0 && result.status !== 1) {
        throw new Error(`compare failed with exit code ${result.status ?? 1}`);
      }
      const metricOutput = `${result.stdout}${result.stderr}`.trim();
      const metricMatch = metricOutput.match(/(?:^|\s)(\d+)(?:\s+\(|$)/);
      const metric = metricMatch ? Number(metricMatch[1]) : Number.NaN;
      if (!Number.isSafeInteger(metric)) {
        throw new Error(`${name} poster comparison returned an unreadable AE metric: ${metricOutput || "unknown"}`);
      }
      console.log(`${name} poster AE ${metric} (limit ${maxAllowedAe})`);
      if (metric > maxAllowedAe) {
        throw new Error(`${name} poster differs from its committed output (AE ${metric}, limit ${maxAllowedAe})`);
      }
    }
  } finally {
    rmSync(verificationDirectory, { force: true, recursive: true });
  }
}

try {
  requireCommand("rsvg-convert", "librsvg (macOS: brew install librsvg; Debian/Ubuntu: sudo apt-get install librsvg2-bin)");
  if (args.includes("--verify")) {
    requireCommand("compare", "ImageMagick (macOS: brew install imagemagick; Debian/Ubuntu: sudo apt-get install imagemagick)");
    verifyPosters(maxAe);
  } else {
    const jobs = args.includes("--all")
      ? Object.values(posterVariants).map((variant) => ({ variant, output: variant.output }))
      : [
          {
            variant: posterVariants[variantName],
            output: outIndex === -1
              ? posterVariants[variantName].output
              : resolve(process.cwd(), args[outIndex + 1]),
          },
        ];
    renderAtomically(jobs);
  }
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
