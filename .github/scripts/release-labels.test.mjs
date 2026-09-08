#!/usr/bin/env node
import assert from "node:assert/strict";
import {
  bumpVersion,
  loadPolicy,
  isRetryableGraphqlErrors,
  isRetryableStatus,
  retryAfterMsFromHeaders,
  releaseMergeCommitSha,
  resolveExplicitTag,
  resolveSourcePullRequest,
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

async function mustReject(name, fn, contains) {
  await assert.rejects(fn, (error) => {
    assert.match(error.message, contains, name);
    return true;
  });
}

async function withoutWarnings(fn) {
  const originalWarn = console.warn;
  console.warn = () => {};
  try {
    return await fn();
  } finally {
    console.warn = originalWarn;
  }
}

const mergeSha = "46f2161d8d083c3dadf2fba59b0779516c4a4ca9";

function pull(number, overrides = {}) {
  return {
    number,
    merged_at: "2026-08-17T00:00:00Z",
    merge_commit_sha: mergeSha,
    base: { ref: "main" },
    labels: [
      { name: "type:none" },
      { name: "channel:stable" },
      { name: "component:docs" },
    ],
    ...overrides,
  };
}

function graphqlPull(number, overrides = {}) {
  return {
    number,
    state: "MERGED",
    mergedAt: "2026-08-17T00:00:00Z",
    baseRefName: "main",
    mergeCommit: { oid: mergeSha },
    ...overrides,
  };
}

function retryableFailure(message, retryAfterMs = null) {
  const error = new Error(message);
  error.retryable = true;
  error.retryAfterMs = retryAfterMs;
  return error;
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
assert.equal(
  releaseMergeCommitSha({ merge_commit_sha: "source-pr-merge" }, "workflow-head"),
  "source-pr-merge",
);
assert.equal(releaseMergeCommitSha({}, "workflow-head"), "workflow-head");
assert.equal(isRetryableGraphqlErrors([{ type: "RATE_LIMITED" }]), true);
assert.equal(isRetryableGraphqlErrors([{ message: "Field does not exist" }]), false);
assert.equal(isRetryableStatus(403, new Headers({ "retry-after": "30" })), true);
assert.equal(isRetryableStatus(403, new Headers()), false);
assert.equal(
  retryAfterMsFromHeaders(new Headers({
    "x-ratelimit-reset": `${Math.ceil((Date.now() + 90_000) / 1_000)}`,
  })),
  30_000,
);

const pr126 = pull(126);
const pr126RestCalls = [];
const resolvedPr126 = await resolveSourcePullRequest(
  { sha: mergeSha },
  {
    graphql: async ({ sha, baseRef }) => {
      assert.equal(sha, mergeSha);
      assert.equal(baseRef, "main");
      return [graphqlPull(126)];
    },
    rest: async (endpoint) => {
      pr126RestCalls.push(endpoint);
      if (endpoint === "/pulls/126") return pr126;
      if (endpoint === `/commits/${mergeSha}/pulls?per_page=100`) return [];
      throw new Error(`Unexpected REST endpoint: ${endpoint}`);
    },
    sleep: async () => assert.fail("PR #126 should not retry"),
  },
);
assert.equal(resolvedPr126.number, 126);

const originalFetch = globalThis.fetch;
const originalToken = process.env.GITHUB_TOKEN;
const originalRepository = process.env.GITHUB_REPOSITORY;
process.env.GITHUB_TOKEN = "test-token";
process.env.GITHUB_REPOSITORY = "IvanLi-CN/loadlynx";
globalThis.fetch = async (url, options) => {
  assert.equal(url, "https://api.github.com/repos/IvanLi-CN/loadlynx/pulls/126");
  assert.equal(options.method, "GET");
  return new Response(JSON.stringify(pr126), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
};
try {
  const resolvedWithDefaultRest = await resolveSourcePullRequest({
    sha: mergeSha,
    prNumber: 126,
  });
  assert.equal(resolvedWithDefaultRest.number, 126);
} finally {
  globalThis.fetch = originalFetch;
  if (originalToken == null) delete process.env.GITHUB_TOKEN;
  else process.env.GITHUB_TOKEN = originalToken;
  if (originalRepository == null) delete process.env.GITHUB_REPOSITORY;
  else process.env.GITHUB_REPOSITORY = originalRepository;
}

assert.deepEqual(pr126RestCalls, ["/pulls/126"]);
assert.deepEqual(validateLabels(resolvedPr126.labels, loadPolicy()), {
  labels: ["channel:stable", "component:docs", "type:none"],
  type: "none",
  channel: "stable",
  components: ["docs"],
});
assert.equal(
  resolveVersion(
    validateLabels(resolvedPr126.labels, loadPolicy()),
    { baseVersion: "0.6.0", runNumber: "1", sha: mergeSha },
  ).should_release,
  false,
);

let retryGraphqlCalls = 0;
const retryDelays = [];
const resolvedAfterRetry = await withoutWarnings(() => resolveSourcePullRequest(
  { sha: mergeSha },
  {
    graphql: async () => {
      retryGraphqlCalls += 1;
      if (retryGraphqlCalls < 3) {
        throw retryableFailure("GitHub GraphQL unavailable");
      }
      return [graphqlPull(126)];
    },
    rest: async (endpoint) => {
      if (endpoint === `/commits/${mergeSha}/pulls?per_page=100`) return [];
      if (endpoint === "/pulls/126") return pr126;
      throw new Error(`Unexpected REST endpoint: ${endpoint}`);
    },
    sleep: async (milliseconds) => retryDelays.push(milliseconds),
  },
));
assert.equal(resolvedAfterRetry.number, 126);
assert.deepEqual(retryDelays, [2_000, 4_000]);

let rateLimitedGraphqlCalls = 0;
const retryAfterDelays = [];
const resolvedAfterRetryAfter = await withoutWarnings(() => resolveSourcePullRequest(
  { sha: mergeSha },
  {
    graphql: async () => {
      rateLimitedGraphqlCalls += 1;
      if (rateLimitedGraphqlCalls === 1) {
        throw retryableFailure("GitHub GraphQL rate limited", 30_000);
      }
      return [graphqlPull(126)];
    },
    rest: async (endpoint) => {
      if (endpoint === `/commits/${mergeSha}/pulls?per_page=100`) return [];
      if (endpoint === "/pulls/126") return pr126;
      throw new Error(`Unexpected REST endpoint: ${endpoint}`);
    },
    sleep: async (milliseconds) => retryAfterDelays.push(milliseconds),
  },
));
assert.equal(resolvedAfterRetryAfter.number, 126);
assert.deepEqual(retryAfterDelays, [30_000]);

const restFallbackCalls = [];
const resolvedFromRest = await resolveSourcePullRequest(
  { sha: mergeSha },
  {
    graphql: async () => [],
    rest: async (endpoint) => {
      restFallbackCalls.push(endpoint);
      if (endpoint === `/commits/${mergeSha}/pulls?per_page=100`) return [pr126];
      if (endpoint === "/pulls/126") return pr126;
      throw new Error(`Unexpected REST endpoint: ${endpoint}`);
    },
    sleep: async () => assert.fail("REST fallback should not retry"),
  },
);
assert.equal(resolvedFromRest.number, 126);
assert.deepEqual(restFallbackCalls, [`/commits/${mergeSha}/pulls?per_page=100`, "/pulls/126"]);

const missingDelays = [];
await withoutWarnings(() => mustReject(
  "rejects candidates with wrong base or merge SHA",
  () => resolveSourcePullRequest(
    { sha: mergeSha },
    {
      graphql: async () => [graphqlPull(126, { baseRefName: "release" })],
      rest: async (endpoint) => {
        if (endpoint === `/commits/${mergeSha}/pulls?per_page=100`) {
          return [pull(126, { merge_commit_sha: "different-sha" })];
        }
        if (endpoint === "/pulls/126") return pull(126, { merge_commit_sha: "different-sha" });
        throw new Error(`Unexpected REST endpoint: ${endpoint}`);
      },
      sleep: async (milliseconds) => missingDelays.push(milliseconds),
    },
  ),
  /No merged pull request found for commit 46f2161d8d083c3dadf2fba59b0779516c4a4ca9 after 4 attempts via GraphQL, REST/,
));
assert.deepEqual(missingDelays, [2_000, 4_000, 8_000]);

await withoutWarnings(() => mustReject(
  "rejects ambiguous exact source pull requests",
  () => resolveSourcePullRequest(
    { sha: mergeSha },
    {
      graphql: async () => [graphqlPull(126), graphqlPull(127)],
      rest: async (endpoint) => {
        if (endpoint === "/pulls/126") return pull(126);
        if (endpoint === "/pulls/127") return pull(127);
        throw new Error(`Unexpected REST endpoint: ${endpoint}`);
      },
      sleep: async () => assert.fail("ambiguous matches should fail immediately"),
    },
  ),
 /Multiple merged pull requests match commit .*: #126, #127/,
));

let explicitGraphqlCalled = false;
const resolvedExplicitPull = await resolveSourcePullRequest(
  { sha: mergeSha, prNumber: 126 },
  {
    graphql: async () => {
      explicitGraphqlCalled = true;
      throw new Error("association lookup must be skipped");
    },
    rest: async (endpoint) => {
      assert.equal(endpoint, "/pulls/126");
      return pr126;
    },
    sleep: async () => assert.fail("explicit PR lookup should not retry"),
  },
);
assert.equal(resolvedExplicitPull.number, 126);
assert.equal(explicitGraphqlCalled, false);

await mustReject(
  "rejects an explicit release backfill for an unmerged PR",
  () => resolveSourcePullRequest(
    { sha: mergeSha, prNumber: 128 },
    {
      graphql: async () => assert.fail("explicit PR lookup must skip associations"),
      rest: async (endpoint) => {
        assert.equal(endpoint, "/pulls/128");
        return pull(128, { merged_at: null });
      },
    },
  ),
  /Explicit release backfill requires merged pull request #128 targeting main/,
);

let mixedFailureCalls = 0;
const mixedFailureDelays = [];
const permanentGraphqlFailure = new Error("GitHub GraphQL access denied");
permanentGraphqlFailure.retryable = false;
const resolvedAfterMixedFailures = await withoutWarnings(() => resolveSourcePullRequest(
  { sha: mergeSha },
  {
    graphql: async () => {
      throw permanentGraphqlFailure;
    },
    rest: async (endpoint) => {
      if (endpoint === `/commits/${mergeSha}/pulls?per_page=100`) {
        mixedFailureCalls += 1;
        if (mixedFailureCalls < 3) throw retryableFailure("GitHub REST unavailable");
        return [pr126];
      }
      if (endpoint === "/pulls/126") return pr126;
      throw new Error(`Unexpected REST endpoint: ${endpoint}`);
    },
    sleep: async (milliseconds) => mixedFailureDelays.push(milliseconds),
  },
));
assert.equal(resolvedAfterMixedFailures.number, 126);
assert.deepEqual(mixedFailureDelays, [2_000, 4_000]);

const authFailure = new Error("GitHub GraphQL access denied");
authFailure.retryable = false;
const restAuthFailure = new Error("GitHub REST access denied");
restAuthFailure.retryable = false;
await withoutWarnings(() => mustReject(
  "fails immediately when both API sources reject authorization",
  () => resolveSourcePullRequest(
    { sha: mergeSha },
    {
      graphql: async () => {
        throw authFailure;
      },
      rest: async () => {
        throw restAuthFailure;
      },
      sleep: async () => assert.fail("authorization failures should not retry"),
    },
  ),
  /GitHub GraphQL access denied/,
));

console.log("release-labels tests passed");
