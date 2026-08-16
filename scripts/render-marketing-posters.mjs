#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { mkdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
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
    "Usage: node scripts/render-marketing-posters.mjs [--variant <dark|light>] [--out <path>] [--all]",
  );
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

function dataUri(path, mimeType) {
  return `data:${mimeType};base64,${readFileSync(path).toString("base64")}`;
}

function renderPoster(variant, output) {
  mkdirSync(dirname(output), { recursive: true });

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

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

try {
  if (args.includes("--all")) {
    for (const variant of Object.values(posterVariants)) {
      renderPoster(variant, variant.output);
    }
  } else {
    const variant = posterVariants[variantName];
    const output = outIndex === -1 ? variant.output : resolve(process.cwd(), args[outIndex + 1]);
    renderPoster(variant, output);
  }
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
