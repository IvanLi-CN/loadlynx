#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync } from "node:fs";
import { basename, join, resolve } from "node:path";

function fail(message) {
  throw new Error(`Invalid release web bundle: ${message}`);
}

function expectedVersionForTag(tag) {
  const normalizedTag = tag.trim();
  if (!normalizedTag) {
    fail("release tag cannot be empty");
  }
  return normalizedTag.startsWith("v") ? normalizedTag.slice(1) : normalizedTag;
}

function parseTarEntries(archive) {
  return execFileSync("tar", ["-tzf", archive], { encoding: "utf8" })
    .split(/\r?\n/)
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function hasUnsafeTarLink(archive) {
  const verboseEntries = execFileSync("tar", ["-tvzf", archive], {
    encoding: "utf8",
  }).split(/\r?\n/);
  return verboseEntries.some((entry) => entry.startsWith("l") || entry.startsWith("h"));
}

function isUnsafeTarEntry(entry) {
  const normalized = entry.replace(/^\.\//, "");
  return (
    entry.startsWith("/") ||
    normalized === ".." ||
    normalized.startsWith("../") ||
    normalized.includes("/../")
  );
}

export function preparePagesReleaseBundle({ archive, tag, output }) {
  const resolvedArchive = resolve(archive);
  const resolvedOutput = resolve(output);
  const expectedVersion = expectedVersionForTag(tag);
  const expectedArchiveName = `loadlynx-web-${tag}.tar.gz`;

  if (!existsSync(resolvedArchive)) {
    fail(`archive does not exist: ${resolvedArchive}`);
  }
  if (basename(resolvedArchive) !== expectedArchiveName) {
    fail(`expected archive ${expectedArchiveName}, got ${basename(resolvedArchive)}`);
  }
  if (existsSync(resolvedOutput) && readdirSync(resolvedOutput).length > 0) {
    fail(`output directory must be empty: ${resolvedOutput}`);
  }

  const entries = parseTarEntries(resolvedArchive);
  if (entries.some(isUnsafeTarEntry)) {
    fail("archive contains an unsafe path");
  }
  if (hasUnsafeTarLink(resolvedArchive)) {
    fail("archive contains a symbolic or hard link");
  }

  mkdirSync(resolvedOutput, { recursive: true });
  execFileSync("tar", ["-xzf", resolvedArchive, "-C", resolvedOutput]);

  const versionPath = join(resolvedOutput, "version.json");
  const indexPath = join(resolvedOutput, "index.html");
  const fallbackPath = join(resolvedOutput, "404.html");
  if (!existsSync(versionPath) || !existsSync(indexPath) || !existsSync(fallbackPath)) {
    fail("archive must contain version.json, index.html, and 404.html at its root");
  }

  const versionPayload = JSON.parse(readFileSync(versionPath, "utf8"));
  if (versionPayload?.version !== expectedVersion) {
    fail(
      `version.json version must be ${expectedVersion}, got ${String(versionPayload?.version)}`,
    );
  }

  const shell = readFileSync(indexPath, "utf8");
  const shellVersion = shell.match(/data-shell-version="([^"]+)"/)?.[1];
  if (shellVersion !== expectedVersion) {
    fail(
      `index.html shell version must be ${expectedVersion}, got ${String(shellVersion)}`,
    );
  }

  const fallback = readFileSync(fallbackPath, "utf8");
  const fallbackVersion = fallback.match(/data-shell-version="([^"]+)"/)?.[1];
  if (fallbackVersion !== expectedVersion) {
    fail(
      `404.html shell version must be ${expectedVersion}, got ${String(fallbackVersion)}`,
    );
  }

  return { expectedVersion, output: resolvedOutput };
}

function parseArgs(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (!key?.startsWith("--") || value == null) {
      throw new Error("Usage: prepare-pages-release-bundle.mjs --archive <path> --tag <tag> --output <dir>");
    }
    values.set(key.slice(2), value);
  }
  return values;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const args = parseArgs(process.argv.slice(2));
  const result = preparePagesReleaseBundle({
    archive: args.get("archive") ?? "",
    tag: args.get("tag") ?? "",
    output: args.get("output") ?? "",
  });
  console.log(`Prepared Pages bundle ${result.expectedVersion} in ${result.output}`);
}
