#!/usr/bin/env node
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { preparePagesReleaseBundle } from "./prepare-pages-release-bundle.mjs";

function writeBundle(root, version, { fallbackVersion = version, includeFallback = true } = {}) {
  const dist = join(root, "dist");
  mkdirSync(dist, { recursive: true });
  writeFileSync(join(dist, "version.json"), `${JSON.stringify({ version })}\n`);
  writeFileSync(
    join(dist, "index.html"),
    `<script data-shell-version="${version}"></script>\n`,
  );
  if (includeFallback) {
    writeFileSync(
      join(dist, "404.html"),
      `<script data-shell-version="${fallbackVersion}"></script>\n`,
    );
  }
  return dist;
}

{
  const root = mkdtempSync(join(tmpdir(), "loadlynx-pages-bundle-missing-fallback-"));
  const dist = writeBundle(root, "1.2.3", { includeFallback: false });
  const archive = join(root, "loadlynx-web-v1.2.3.tar.gz");
  execFileSync("tar", ["-czf", archive, "-C", dist, "."]);
  assert.throws(
    () => preparePagesReleaseBundle({ archive, tag: "v1.2.3", output: join(root, "pages") }),
    /404\.html/,
  );
}

function archiveBundle(root, tag, version) {
  const dist = writeBundle(root, version);
  const archive = join(root, `loadlynx-web-${tag}.tar.gz`);
  execFileSync("tar", ["-czf", archive, "-C", dist, "."]);
  return archive;
}

for (const [tag, version] of [
  ["v1.2.3", "1.2.3"],
  ["v1.2.4-beta.7", "1.2.4-beta.7"],
  ["dev-20260712-010203-abcdef0", "dev-20260712-010203-abcdef0"],
]) {
  const root = mkdtempSync(join(tmpdir(), "loadlynx-pages-bundle-"));
  const archive = archiveBundle(root, tag, version);
  const output = join(root, "pages");

  const result = preparePagesReleaseBundle({ archive, tag, output });
  assert.equal(result.expectedVersion, version);
  assert.deepEqual(JSON.parse(readFileSync(join(output, "version.json"), "utf8")), {
    version,
  });
}

{
  const root = mkdtempSync(join(tmpdir(), "loadlynx-pages-bundle-invalid-"));
  const archive = archiveBundle(root, "v1.2.3", "1.2.2");
  assert.throws(
    () => preparePagesReleaseBundle({ archive, tag: "v1.2.3", output: join(root, "pages") }),
    /version\.json version must be 1\.2\.3/,
  );
}

{
  const root = mkdtempSync(join(tmpdir(), "loadlynx-pages-bundle-invalid-fallback-"));
  const dist = writeBundle(root, "1.2.3", { fallbackVersion: "1.2.2" });
  const archive = join(root, "loadlynx-web-v1.2.3.tar.gz");
  execFileSync("tar", ["-czf", archive, "-C", dist, "."]);
  assert.throws(
    () => preparePagesReleaseBundle({ archive, tag: "v1.2.3", output: join(root, "pages") }),
    /404\.html shell version must be 1\.2\.3/,
  );
}

console.log("Pages release bundle tests passed");
