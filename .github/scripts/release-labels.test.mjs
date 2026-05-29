#!/usr/bin/env node
import assert from "node:assert/strict";
import {
  buildReleaseComment,
  bumpVersion,
  resolveExplicitTag,
  resolveVersion,
  validateLabels,
} from "./release-labels.mjs";

const policy = {
  label_groups: [
    {
      name: "type",
      prefix: "type:",
      required: true,
      cardinality: "exactly-one",
      allowed: ["type:major", "type:minor", "type:patch", "type:none"],
    },
    {
      name: "channel",
      prefix: "channel:",
      required: true,
      cardinality: "exactly-one",
      allowed: ["channel:stable", "channel:beta", "channel:dev"],
    },
    {
      name: "component",
      prefix: "component:",
      required: false,
      cardinality: "zero-or-more",
      allowed: ["component:firmware", "component:web"],
    },
  ],
};

function mustThrow(name, fn, contains) {
  assert.throws(fn, (error) => {
    assert.match(error.message, contains, name);
    return true;
  });
}

assert.deepEqual(
  validateLabels(["type:patch", "channel:stable", "component:firmware"], policy),
  {
    labels: ["channel:stable", "component:firmware", "type:patch"],
    type: "patch",
    channel: "stable",
    components: ["firmware"],
  },
);

mustThrow("missing type", () => validateLabels(["channel:stable"], policy), /Missing required type/);
mustThrow("missing channel", () => validateLabels(["type:patch"], policy), /Missing required channel/);
mustThrow(
  "duplicate type",
  () => validateLabels(["type:patch", "type:minor", "channel:stable"], policy),
  /Expected exactly one type/,
);
mustThrow(
  "unknown channel",
  () => validateLabels(["type:patch", "channel:prod"], policy),
  /Unknown channel/,
);

assert.equal(bumpVersion("0.1.0", "patch"), "0.1.1");
assert.equal(bumpVersion("0.1.0", "minor"), "0.2.0");
assert.equal(bumpVersion("0.1.0", "major"), "1.0.0");
assert.equal(bumpVersion("0.1.0", "none"), "0.1.0");

assert.deepEqual(
  resolveVersion(
    { type: "patch", channel: "stable" },
    { baseVersion: "0.1.0", runNumber: "42", sha: "1fac33c634", now: new Date("2026-05-29T00:00:00Z") },
  ),
  {
    base_version: "0.1.0",
    version: "0.1.1",
    tag: "v0.1.1",
    prerelease: false,
    should_release: true,
  },
);

assert.equal(
  resolveVersion(
    { type: "patch", channel: "dev" },
    { baseVersion: "0.1.0", runNumber: "42", sha: "1fac33c634", now: new Date("2026-05-29T01:02:03Z") },
  ).tag,
  "dev-20260529-010203-1fac33c",
);

assert.deepEqual(
  resolveVersion(
    { type: "none", channel: "stable" },
    { baseVersion: "0.1.0", runNumber: "42", sha: "1fac33c634", now: new Date("2026-05-29T00:00:00Z") },
  ),
  {
    base_version: "0.1.0",
    version: "",
    tag: "",
    prerelease: false,
    should_release: false,
  },
);

assert.deepEqual(resolveExplicitTag("v0.1.1", { baseVersion: "0.1.0" }), {
  base_version: "0.1.0",
  version: "0.1.1",
  tag: "v0.1.1",
  prerelease: false,
  should_release: true,
});

const comment = buildReleaseComment(
  {
    tag: "v0.1.1",
    channel: "stable",
    type: "patch",
    merge_commit_sha: "abc123",
    run_url: "https://github.com/IvanLi-CN/loadlynx/actions/runs/1",
  },
  "https://github.com/IvanLi-CN/loadlynx/releases/tag/v0.1.1",
  ["loadlynx-web-v0.1.1.tar.gz"],
);
assert.match(comment, /loadlynx-release-version-comment/);
assert.match(comment, /Version: `v0\.1\.1`/);
assert.match(comment, /loadlynx-web-v0\.1\.1\.tar\.gz/);

console.log("release-labels tests passed");
