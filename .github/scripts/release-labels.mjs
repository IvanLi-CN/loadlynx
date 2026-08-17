#!/usr/bin/env node
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const DEFAULT_POLICY_PATH = ".github/release-label-policy.json";
const RELEASE_COMMENT_MARKER = "<!-- loadlynx-release-version-comment -->";
const SOURCE_PULL_REQUEST_ATTEMPTS = 4;
const SOURCE_PULL_REQUEST_RETRY_DELAYS_MS = [2_000, 4_000, 8_000];
const MAX_RETRY_AFTER_MS = 30_000;
const SOURCE_PULL_REQUEST_QUERY = `
  query SourcePullRequest($owner: String!, $name: String!, $expression: String!) {
    repository(owner: $owner, name: $name) {
      object(expression: $expression) {
        ... on Commit {
          associatedPullRequests(first: 100) {
            nodes {
              number
              state
              mergedAt
              baseRefName
              mergeCommit {
                oid
              }
            }
          }
        }
      }
    }
  }
`;

class GitHubApiError extends Error {
  constructor(message, { source, status = null, retryAfterMs = null, retryable = false } = {}) {
    super(message);
    this.name = "GitHubApiError";
    this.source = source;
    this.status = status;
    this.retryAfterMs = retryAfterMs;
    this.retryable = retryable;
  }
}

export function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  return JSON.parse(readFileSync(policyPath, "utf8"));
}

function valueOf(label, prefix) {
  assert(label.startsWith(prefix), `${label} does not start with ${prefix}`);
  return label.slice(prefix.length);
}

export function validateLabels(labels, policy = loadPolicy()) {
  const labelNames = labels.map((label) =>
    typeof label === "string" ? label : label.name,
  );
  const groups = {};
  const errors = [];

  for (const group of policy.label_groups) {
    const allowed = new Set(group.allowed);
    const matching = labelNames.filter((label) =>
      label.startsWith(group.prefix),
    );
    const unknown = matching.filter((label) => !allowed.has(label));

    if (unknown.length > 0) {
      errors.push(
        `Unknown ${group.name} label(s): ${unknown.sort().join(", ")}`,
      );
    }
    if (group.required && matching.length === 0) {
      errors.push(`Missing required ${group.name} label (${group.prefix}*)`);
    }
    if (group.cardinality === "exactly-one" && matching.length > 1) {
      errors.push(
        `Expected exactly one ${group.name} label, got ${matching
          .sort()
          .join(", ")}`,
      );
    }

    groups[group.name] = matching
      .filter((label) => allowed.has(label))
      .map((label) => valueOf(label, group.prefix));
  }

  if (errors.length > 0) {
    const error = new Error(errors.join("\n"));
    error.errors = errors;
    throw error;
  }

  return {
    labels: labelNames.sort(),
    type: groups.type[0],
    channel: groups.channel[0],
    components: groups.component ?? [],
  };
}

export function bumpVersion(baseVersion, type) {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(baseVersion);
  if (!match) {
    throw new Error(`Invalid stable base version: ${baseVersion}`);
  }
  let [, majorRaw, minorRaw, patchRaw] = match;
  let major = Number(majorRaw);
  let minor = Number(minorRaw);
  let patch = Number(patchRaw);

  if (type === "major") {
    major += 1;
    minor = 0;
    patch = 0;
  } else if (type === "minor") {
    minor += 1;
    patch = 0;
  } else if (type === "patch") {
    patch += 1;
  } else if (type !== "none") {
    throw new Error(`Unsupported release type: ${type}`);
  }

  return `${major}.${minor}.${patch}`;
}

export function latestStableVersion() {
  const output = execFileSync(
    "git",
    ["tag", "--list", "v[0-9]*", "--sort=-version:refname"],
    { encoding: "utf8" },
  );
  const stableTag = output
    .split(/\r?\n/)
    .map((line) => line.trim())
    .find((tag) => /^v\d+\.\d+\.\d+$/.test(tag));
  return stableTag ? stableTag.slice(1) : "0.0.0";
}

export function resolveVersion(intent, { baseVersion, runNumber, sha, now }) {
  const base = baseVersion ?? latestStableVersion();
  const shortSha = sha.slice(0, 7);
  const nextStable = intent.type === "none" ? base : bumpVersion(base, intent.type);

  if (intent.type === "none") {
    return {
      base_version: base,
      version: "",
      tag: "",
      prerelease: false,
      should_release: false,
    };
  }
  if (intent.channel === "stable") {
    return {
      base_version: base,
      version: nextStable,
      tag: `v${nextStable}`,
      prerelease: false,
      should_release: true,
    };
  }
  if (intent.channel === "beta") {
    const version = `${nextStable}-beta.${runNumber}`;
    return {
      base_version: base,
      version,
      tag: `v${version}`,
      prerelease: true,
      should_release: true,
    };
  }
  if (intent.channel === "dev") {
    const stamp = formatTimestamp(now ?? new Date());
    const version = `dev-${stamp}-${shortSha}`;
    return {
      base_version: base,
      version,
      tag: version,
      prerelease: true,
      should_release: true,
    };
  }
  throw new Error(`Unsupported release channel: ${intent.channel}`);
}

export function resolveExplicitTag(releaseTag, { baseVersion } = {}) {
  const tag = releaseTag.trim();
  if (!tag) throw new Error("Explicit release tag cannot be empty");
  const version = tag.startsWith("v") ? tag.slice(1) : tag;
  return {
    base_version: baseVersion ?? latestStableVersion(),
    version,
    tag,
    prerelease: version.includes("-") || tag.startsWith("dev-"),
    should_release: true,
  };
}

function formatTimestamp(date) {
  const pad = (value) => `${value}`.padStart(2, "0");
  return `${date.getUTCFullYear()}${pad(date.getUTCMonth() + 1)}${pad(
    date.getUTCDate(),
  )}-${pad(date.getUTCHours())}${pad(date.getUTCMinutes())}${pad(
    date.getUTCSeconds(),
  )}`;
}

function parseArgs(argv) {
  const args = { _: [] };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (!arg.startsWith("--")) {
      args._.push(arg);
      continue;
    }
    const key = arg.slice(2);
    const next = argv[index + 1];
    if (next == null || next.startsWith("--")) {
      args[key] = true;
    } else {
      args[key] = next;
      index += 1;
    }
  }
  return args;
}

function labelsFromEvent(event) {
  if (!event.pull_request) {
    throw new Error("Event does not contain pull_request labels");
  }
  return event.pull_request.labels ?? [];
}

async function githubApi(endpoint, { method = "GET", body } = {}) {
  const token = process.env.GITHUB_TOKEN;
  const repository = process.env.GITHUB_REPOSITORY;
  if (!token) throw new Error("GITHUB_TOKEN is required");
  if (!repository) throw new Error("GITHUB_REPOSITORY is required");

  let response;
  try {
    response = await fetch(`https://api.github.com/repos/${repository}${endpoint}`, {
      method,
      headers: {
        Accept: "application/vnd.github+json",
        Authorization: `Bearer ${token}`,
        "X-GitHub-Api-Version": "2022-11-28",
        ...(body == null ? {} : { "Content-Type": "application/json" }),
      },
      body: body == null ? undefined : JSON.stringify(body),
    });
  } catch {
    throw new GitHubApiError(
      `GitHub REST ${method} ${endpoint} failed: network error`,
      { source: "REST", retryable: true },
    );
  }

  if (!response.ok) {
    throw new GitHubApiError(
      `GitHub REST ${method} ${endpoint} failed: HTTP ${response.status}`,
      {
        source: "REST",
        status: response.status,
        retryAfterMs: retryAfterMsFromHeaders(response.headers),
        retryable: isRetryableStatus(response.status, response.headers),
      },
    );
  }
  if (response.status === 204) return null;
  return response.json();
}

function githubRepository() {
  const repository = process.env.GITHUB_REPOSITORY;
  if (!repository) throw new Error("GITHUB_REPOSITORY is required");
  const [owner, name] = repository.split("/", 2);
  if (!owner || !name) {
    throw new Error(`Invalid GITHUB_REPOSITORY: ${repository}`);
  }
  return { owner, name };
}

export function isRetryableStatus(status, headers = new Headers()) {
  return (
    status === 408
    || status === 429
    || status >= 500
    || (status === 403 && isRateLimited(headers))
  );
}

function isRateLimited(headers) {
  return (
    headers.get("retry-after") != null
    || headers.get("x-ratelimit-remaining") === "0"
  );
}

function retryAfterMsFrom(value) {
  if (!value) return null;
  const seconds = Number(value);
  if (Number.isFinite(seconds) && seconds >= 0) {
    return Math.min(seconds * 1_000, MAX_RETRY_AFTER_MS);
  }
  const timestamp = Date.parse(value);
  if (Number.isNaN(timestamp)) return null;
  return Math.min(Math.max(timestamp - Date.now(), 0), MAX_RETRY_AFTER_MS);
}

export function retryAfterMsFromHeaders(headers) {
  return (
    retryAfterMsFrom(headers.get("retry-after"))
    ?? rateLimitResetMsFrom(headers.get("x-ratelimit-reset"))
  );
}

function rateLimitResetMsFrom(value) {
  const seconds = Number(value);
  if (!Number.isFinite(seconds)) return null;
  return Math.min(Math.max((seconds * 1_000) - Date.now(), 0), MAX_RETRY_AFTER_MS);
}

export function isRetryableGraphqlErrors(errors) {
  return errors.some((error) => {
    const detail = [error?.type, error?.extensions?.code, error?.message]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
    return /rate[ _-]?limit|timeout|temporar(?:y|ily)|service unavailable/.test(detail);
  });
}

async function githubGraphql(query, variables) {
  const token = process.env.GITHUB_TOKEN;
  if (!token) throw new Error("GITHUB_TOKEN is required");

  let response;
  try {
    response = await fetch("https://api.github.com/graphql", {
      method: "POST",
      headers: {
        Accept: "application/vnd.github+json",
        Authorization: `Bearer ${token}`,
        "X-GitHub-Api-Version": "2022-11-28",
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ query, variables }),
    });
  } catch {
    throw new GitHubApiError("GitHub GraphQL query failed: network error", {
      source: "GraphQL",
      retryable: true,
    });
  }

  if (!response.ok) {
    throw new GitHubApiError(`GitHub GraphQL query failed: HTTP ${response.status}`, {
      source: "GraphQL",
      status: response.status,
      retryAfterMs: retryAfterMsFromHeaders(response.headers),
      retryable: isRetryableStatus(response.status, response.headers),
    });
  }

  const payload = await response.json();
  if (payload.errors?.length > 0) {
    throw new GitHubApiError("GitHub GraphQL query returned errors", {
      source: "GraphQL",
      retryAfterMs: retryAfterMsFromHeaders(response.headers),
      retryable: isRetryableGraphqlErrors(payload.errors),
    });
  }
  return payload.data;
}

async function githubAssociatedPullRequests({ sha }) {
  const { owner, name } = githubRepository();
  const data = await githubGraphql(SOURCE_PULL_REQUEST_QUERY, {
    owner,
    name,
    expression: sha,
  });
  return data?.repository?.object?.associatedPullRequests?.nodes ?? [];
}

function isCanonicalSourcePullRequest(pull, { sha, baseRef }) {
  return Boolean(
    pull?.merged_at
      && pull.base?.ref === baseRef
      && pull.merge_commit_sha === sha,
  );
}

function isGraphqlSourcePullRequest(pull, { sha, baseRef }) {
  return Boolean(
    pull?.state === "MERGED"
      && pull.mergedAt
      && pull.baseRefName === baseRef
      && pull.mergeCommit?.oid === sha,
  );
}

function candidateNumbers(pulls, matches = () => true) {
  return [...new Set(pulls.filter(matches).map((pull) => pull.number))].sort((left, right) => left - right);
}

function sourcePullRequestNotFoundError(sha, sources) {
  return new Error(
    `No merged pull request found for commit ${sha} after ${SOURCE_PULL_REQUEST_ATTEMPTS} attempts via ${[...sources].join(", ")}`,
  );
}

function sourcePullRequestAmbiguousError(sha, numbers) {
  const error = new Error(
    `Multiple merged pull requests match commit ${sha}: ${numbers.map((number) => `#${number}`).join(", ")}`,
  );
  error.code = "SOURCE_PULL_REQUEST_AMBIGUOUS";
  return error;
}

async function resolveCanonicalCandidates(numbers, criteria, rest) {
  const pulls = [];
  for (const number of numbers) {
    const pull = await rest(`/pulls/${number}`);
    if (isCanonicalSourcePullRequest(pull, criteria)) {
      pulls.push(pull);
    }
  }
  return pulls;
}

function retryDelayMs(attempt, retryAfterMs) {
  return retryAfterMs ?? SOURCE_PULL_REQUEST_RETRY_DELAYS_MS[attempt - 1];
}

function retryableError(error) {
  return Boolean(error?.retryable);
}

function ambiguousSourcePullRequestError(error) {
  return error?.code === "SOURCE_PULL_REQUEST_AMBIGUOUS";
}

function sourceErrorSummary(source, error) {
  if (error instanceof GitHubApiError && error.status != null) {
    return `${source} HTTP ${error.status}`;
  }
  return `${source} error`;
}

export async function resolveSourcePullRequest(
  { sha, prNumber = null, baseRef = "main" },
  {
    rest = githubApi,
    graphql = githubAssociatedPullRequests,
    sleep = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)),
  } = {},
) {
  if (prNumber) {
    const pull = await rest(`/pulls/${prNumber}`);
    if (!pull?.merged_at || pull.base?.ref !== baseRef) {
      throw new Error(
        `Explicit release backfill requires merged pull request #${prNumber} targeting ${baseRef}`,
      );
    }
    return pull;
  }

  const criteria = { sha, baseRef };
  const sources = new Set();
  let retryAfterMs = null;

  for (let attempt = 0; attempt < SOURCE_PULL_REQUEST_ATTEMPTS; attempt += 1) {
    if (attempt > 0) {
      await sleep(retryDelayMs(attempt, retryAfterMs));
      retryAfterMs = null;
    }

    const errors = [];
    try {
      sources.add("GraphQL");
      const graphqlNumbers = candidateNumbers(
        await graphql({ sha, baseRef }),
        (pull) => isGraphqlSourcePullRequest(pull, criteria),
      );
      const pulls = await resolveCanonicalCandidates(graphqlNumbers, criteria, rest);
      if (pulls.length > 1) {
        throw sourcePullRequestAmbiguousError(sha, pulls.map((pull) => pull.number));
      }
      if (pulls.length === 1) return pulls[0];
    } catch (error) {
      if (ambiguousSourcePullRequestError(error)) throw error;
      errors.push({ source: "GraphQL", error });
      console.warn(`Source PR lookup attempt ${attempt + 1}: ${sourceErrorSummary("GraphQL", error)}`);
    }

    try {
      sources.add("REST");
      const restNumbers = candidateNumbers(
        await rest(`/commits/${sha}/pulls?per_page=100`),
      );
      const pulls = await resolveCanonicalCandidates(restNumbers, criteria, rest);
      if (pulls.length > 1) {
        throw sourcePullRequestAmbiguousError(sha, pulls.map((pull) => pull.number));
      }
      if (pulls.length === 1) return pulls[0];
    } catch (error) {
      if (ambiguousSourcePullRequestError(error)) throw error;
      errors.push({ source: "REST", error });
      console.warn(`Source PR lookup attempt ${attempt + 1}: ${sourceErrorSummary("REST", error)}`);
    }

    const transientErrors = errors.filter(({ error }) => retryableError(error));
    if (transientErrors.length > 0) {
      retryAfterMs = Math.max(
        ...transientErrors.map(({ error }) => error.retryAfterMs ?? 0),
        0,
      ) || null;
      continue;
    }
    if (errors.length > 0) throw errors[0].error;
  }

  throw sourcePullRequestNotFoundError(sha, sources);
}

export function releaseMergeCommitSha(pull, fallbackSha) {
  return pull.merge_commit_sha ?? fallbackSha;
}

export function buildReleaseComment(snapshot, releaseUrl, assets = []) {
  return [
    RELEASE_COMMENT_MARKER,
    "LoadLynx release completed for this PR.",
    "",
    `- Version: \`${snapshot.tag}\``,
    `- Channel: \`${snapshot.channel}\``,
    `- Type: \`${snapshot.type}\``,
    `- Release: ${releaseUrl}`,
    `- Merge commit: \`${snapshot.merge_commit_sha}\``,
    `- Workflow run: ${snapshot.run_url}`,
    assets.length > 0 ? `- Assets: ${assets.map((asset) => `\`${asset}\``).join(", ")}` : null,
  ]
    .filter(Boolean)
    .join("\n");
}

function writeOutputs(outputs) {
  if (!process.env.GITHUB_OUTPUT) return;
  const lines = Object.entries(outputs).map(([key, value]) => `${key}=${value}`);
  appendFileSync(process.env.GITHUB_OUTPUT, `${lines.join("\n")}\n`, "utf8");
}

function appendSummary(markdown) {
  if (!process.env.GITHUB_STEP_SUMMARY) return;
  appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${markdown}\n`, "utf8");
}

async function validateCommand(args) {
  const policy = loadPolicy(args.policy ?? DEFAULT_POLICY_PATH);
  const event = JSON.parse(readFileSync(args.event ?? process.env.GITHUB_EVENT_PATH, "utf8"));
  if (event.merge_group) {
    const snapshot = {
      event: "merge_group",
      merge_group_head_sha: event.merge_group.head_sha ?? null,
      labels: [],
      should_validate: false,
      reason: "Release labels are validated on pull_request_target events before merge queue grouping.",
    };
    if (args.output) {
      writeFileSync(args.output, `${JSON.stringify(snapshot, null, 2)}\n`, "utf8");
    }
    appendSummary("## Label Gate\n\nMerge queue event accepted; release labels are validated on PR events.");
    console.log(JSON.stringify(snapshot, null, 2));
    return;
  }
  const intent = validateLabels(labelsFromEvent(event), policy);
  const snapshot = {
    pull_request: event.pull_request?.number ?? null,
    labels: intent.labels,
    type: intent.type,
    channel: intent.channel,
    components: intent.components,
  };
  if (args.output) {
    writeFileSync(args.output, `${JSON.stringify(snapshot, null, 2)}\n`, "utf8");
  }
  appendSummary(`## Label Gate\n\nValidated release intent: \`${intent.type}\` / \`${intent.channel}\``);
  console.log(JSON.stringify(snapshot, null, 2));
}

async function resolveCommand(args) {
  const policy = loadPolicy(args.policy ?? DEFAULT_POLICY_PATH);
  const eventPath = args.event ?? process.env.GITHUB_EVENT_PATH;
  const event = eventPath ? JSON.parse(readFileSync(eventPath, "utf8")) : {};
  const sha = args.sha ?? process.env.GITHUB_SHA;
  const runNumber = args["run-number"] ?? process.env.GITHUB_RUN_NUMBER ?? "0";
  const releaseTag = args["release-tag"] ?? event.inputs?.release_tag ?? null;
  if (!sha) throw new Error("A commit sha is required");

  const prNumber = args["pr-number"] || event.inputs?.pr_number || null;
  const pull = await resolveSourcePullRequest({ sha, prNumber });
  const intent = validateLabels(pull.labels ?? [], policy);
  const version = releaseTag
    ? resolveExplicitTag(releaseTag)
    : resolveVersion(intent, { runNumber, sha });
  const snapshot = {
    pull_request: pull.number,
    pull_request_url: pull.html_url,
    merge_commit_sha: releaseMergeCommitSha(pull, sha),
    head_sha: pull.head?.sha ?? null,
    labels: intent.labels,
    type: intent.type,
    channel: intent.channel,
    components: intent.components,
    ...version,
    artifact_names: [],
    run_url: `${process.env.GITHUB_SERVER_URL}/${process.env.GITHUB_REPOSITORY}/actions/runs/${process.env.GITHUB_RUN_ID}`,
  };

  if (args.output) {
    writeFileSync(args.output, `${JSON.stringify(snapshot, null, 2)}\n`, "utf8");
  }
  writeOutputs({
    pr_number: snapshot.pull_request,
    type: snapshot.type,
    channel: snapshot.channel,
    components: snapshot.components.join(","),
    base_version: snapshot.base_version,
    version: snapshot.version,
    tag: snapshot.tag,
    prerelease: snapshot.prerelease,
    should_release: snapshot.should_release,
  });
  appendSummary(`## Release Intent\n\n\`\`\`json\n${JSON.stringify(snapshot, null, 2)}\n\`\`\``);
  console.log(JSON.stringify(snapshot, null, 2));
}

async function commentCommand(args) {
  const snapshot = JSON.parse(readFileSync(args.snapshot, "utf8"));
  const releaseUrl = args["release-url"];
  const assets = (args.assets ?? "")
    .split(",")
    .map((asset) => asset.trim())
    .filter(Boolean);
  const body = buildReleaseComment(snapshot, releaseUrl, assets);

  const comments = await githubApi(`/issues/${snapshot.pull_request}/comments?per_page=100`);
  const existing = comments.find(
    (comment) =>
      comment.body?.includes(RELEASE_COMMENT_MARKER) && comment.user?.type === "Bot",
  );
  if (existing) {
    await githubApi(`/issues/comments/${existing.id}`, {
      method: "PATCH",
      body: { body },
    });
    console.log(`Updated release comment on PR #${snapshot.pull_request}`);
    return;
  }

  await githubApi(`/issues/${snapshot.pull_request}/comments`, {
    method: "POST",
    body: { body },
  });
  console.log(`Created release comment on PR #${snapshot.pull_request}`);
}

async function main() {
  const [command, ...rest] = process.argv.slice(2);
  const args = parseArgs(rest);
  if (command === "validate") return validateCommand(args);
  if (command === "resolve") return resolveCommand(args);
  if (command === "comment") return commentCommand(args);
  throw new Error(`Unknown command: ${command}`);
}

const entrypoint = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (entrypoint === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error.message);
    if (error.errors) {
      for (const item of error.errors) console.error(`- ${item}`);
    }
    process.exit(1);
  });
}
