#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
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
    renameSync(stage.stagedOutput, job.output);
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

function writeTransactionJournal(transaction) {
  const journalPath = transactionJournalPath(transaction.lockDirectory);
  const temporaryPath = join(transaction.lockDirectory, `transaction-${process.pid}.tmp`);
  writeFileSync(temporaryPath, `${JSON.stringify(transaction)}\n`);
  renameSync(temporaryPath, journalPath);
}

function isProcessRunning(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return error?.code === "EPERM";
  }
}

function transactionMatchesJobs(transaction, lockDirectory, jobs) {
  if (
    !transaction ||
    transaction.version !== 1 ||
    !Number.isSafeInteger(transaction.pid) ||
    transaction.pid <= 0 ||
    transaction.lockDirectory !== lockDirectory ||
    !["rendering", "publishing", "published"].includes(transaction.phase) ||
    !Array.isArray(transaction.jobs)
  ) {
    return false;
  }

  const expectedOutputs = jobs.map((job) => job.output).sort();
  const recordedOutputs = transaction.jobs.map((job) => job.output).sort();
  if (
    expectedOutputs.length !== recordedOutputs.length ||
    expectedOutputs.some((output, index) => output !== recordedOutputs[index])
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

function readTransaction(lockDirectory, jobs) {
  const journalPath = transactionJournalPath(lockDirectory);
  if (!existsSync(journalPath)) {
    throw new Error(`Poster transaction lock is incomplete: ${lockDirectory}`);
  }

  let transaction;
  try {
    transaction = JSON.parse(readFileSync(journalPath, "utf8"));
  } catch (error) {
    throw new Error(`Poster transaction journal is unreadable: ${error.message}`);
  }
  if (!transactionMatchesJobs(transaction, lockDirectory, jobs)) {
    throw new Error(`Poster transaction journal does not match this output set: ${lockDirectory}`);
  }
  return transaction;
}

function restoreTransactionOutputs(transaction) {
  const rollbackErrors = [];
  for (const job of [...transaction.jobs].reverse()) {
    try {
      if (job.hadOriginal && existsSync(job.backupOutput)) {
        renameSync(job.backupOutput, job.output);
      } else if (!job.hadOriginal && !existsSync(job.stagedOutput) && existsSync(job.output)) {
        renameSync(job.output, job.discardedOutput);
      } else if (job.hadOriginal) {
        throw new Error(`missing backup for ${job.output}`);
      }
    } catch (error) {
      rollbackErrors.push(error instanceof Error ? error.message : String(error));
    }
  }
  return rollbackErrors;
}

function removeTransactionLock(lockDirectory) {
  rmSync(lockDirectory, { force: true, recursive: true });
}

function recoverInterruptedTransaction(outputDirectory, jobs) {
  const lockDirectory = join(outputDirectory, transactionLockName);
  if (!existsSync(lockDirectory)) {
    return;
  }

  const transaction = readTransaction(lockDirectory, jobs);
  if (isProcessRunning(transaction.pid)) {
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
  try {
    mkdirSync(lockDirectory);
  } catch (error) {
    if (error?.code === "EEXIST") {
      throw new Error(`Another poster render acquired the transaction lock: ${lockDirectory}`);
    }
    throw error;
  }

  const transaction = {
    version: 1,
    pid: process.pid,
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
    writeTransactionJournal(transaction);
  } catch (error) {
    removeTransactionLock(lockDirectory);
    throw error;
  }
  return transaction;
}

function renderPosterPairAtomically(jobs) {
  const outputDirectory = commonOutputDirectory(jobs);
  recoverInterruptedTransaction(outputDirectory, jobs);

  let transaction;
  try {
    transaction = createTransaction(outputDirectory, jobs);
    const stagedJobs = jobs.map((job, index) => ({
      ...job,
      ...transaction.jobs[index],
    }));
    for (const job of stagedJobs) {
      renderPoster(job.variant, job.stagedOutput);
    }

    for (const job of transaction.jobs) {
      if (job.hadOriginal) {
        copyFileSync(job.output, job.backupOutput);
      }
    }
    transaction.phase = "publishing";
    writeTransactionJournal(transaction);
    for (const job of transaction.jobs) {
      renameSync(job.stagedOutput, job.output);
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
  if (jobs.length === 1) {
    renderSingleAtomically(jobs[0]);
    return;
  }
  renderPosterPairAtomically(jobs);
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
