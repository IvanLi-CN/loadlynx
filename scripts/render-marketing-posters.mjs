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
import { basename, dirname, join, resolve } from "node:path";
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
    backupOutput: join(stagingDirectory, `previous-${basename(output)}`),
    discardedOutput: join(stagingDirectory, `discarded-${basename(output)}`),
  };
}

function restoreReplacements(staged) {
  const rollbackErrors = [];
  for (const job of [...staged].reverse()) {
    try {
      if (!job.replacementCommitted) {
        continue;
      }
      if (job.backupReady && existsSync(job.backupOutput)) {
        renameSync(job.backupOutput, job.output);
      } else if (existsSync(job.output)) {
        renameSync(job.output, job.discardedOutput);
      }
    } catch (error) {
      rollbackErrors.push(error instanceof Error ? error.message : String(error));
    }
  }
  return rollbackErrors;
}

function renderAtomically(jobs) {
  const staged = [];
  try {
    for (const job of jobs) {
      const stage = stageOutput(job.output);
      staged.push({ ...job, ...stage });
      renderPoster(job.variant, stage.stagedOutput);
    }

    try {
      for (const job of staged) {
        job.backupReady = false;
        job.replacementCommitted = false;
        if (existsSync(job.output)) {
          copyFileSync(job.output, job.backupOutput);
          job.backupReady = true;
        }
        renameSync(job.stagedOutput, job.output);
        job.replacementCommitted = true;
      }
    } catch (error) {
      const rollbackErrors = restoreReplacements(staged);
      const message = error instanceof Error ? error.message : String(error);
      if (rollbackErrors.length > 0) {
        throw new Error(`${message}; output rollback failed: ${rollbackErrors.join("; ")}`);
      }
      throw new Error(`${message}; poster outputs were restored`);
    }
  } finally {
    for (const job of staged) {
      rmSync(job.stagingDirectory, { force: true, recursive: true });
    }
  }
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
