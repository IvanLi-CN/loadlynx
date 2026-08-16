#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync } from "node:fs";
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
    "Usage: node scripts/render-marketing-posters.mjs [--variant <dark|light>] [--out <path>] [--all] [--verify]",
  );
  console.log("Requires rsvg-convert (librsvg) and ImageMagick compare.");
  console.log("macOS: brew install librsvg imagemagick");
  console.log("Debian/Ubuntu: sudo apt-get install librsvg2-bin imagemagick");
  process.exit(0);
}

const outIndex = args.indexOf("--out");
if (outIndex !== -1 && !args[outIndex + 1]) {
  console.error("--out requires a file path");
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

  const result = spawnSync(
    "rsvg-convert",
    [
      "--width",
      String(outputWidth),
      "--height",
      String(outputHeight),
      "--output",
      output,
      "-",
    ],
    { input: renderedSvg, stdio: ["pipe", "inherit", "inherit"] },
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

function renderAtomically(jobs) {
  const staged = [];
  try {
    for (const job of jobs) {
      const stage = stageOutput(job.output);
      staged.push({ ...job, ...stage });
      renderPoster(job.variant, stage.stagedOutput);
    }

    for (const job of staged) {
      renameSync(job.stagedOutput, job.output);
    }
  } finally {
    for (const job of staged) {
      rmSync(job.stagingDirectory, { force: true, recursive: true });
    }
  }
}

function verifyPosters() {
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
      if (result.status !== 0) {
        const metric = `${result.stdout}${result.stderr}`.trim() || "unknown";
        throw new Error(`${name} poster differs from its committed output (AE ${metric})`);
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
    verifyPosters();
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
