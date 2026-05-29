#!/usr/bin/env node
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const DEFAULT_POLICY_PATH = ".github/release-label-policy.json";
const RELEASE_COMMENT_MARKER = "<!-- loadlynx-release-version-comment -->";

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

  const response = await fetch(`https://api.github.com/repos/${repository}${endpoint}`, {
    method,
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${token}`,
      "X-GitHub-Api-Version": "2022-11-28",
      ...(body == null ? {} : { "Content-Type": "application/json" }),
    },
    body: body == null ? undefined : JSON.stringify(body),
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(`GitHub API ${method} ${endpoint} failed: ${response.status} ${text}`);
  }
  if (response.status === 204) return null;
  return response.json();
}

async function findPullRequestForSha(sha, prNumber) {
  if (prNumber) {
    return githubApi(`/pulls/${prNumber}`);
  }
  const pulls = await githubApi(`/commits/${sha}/pulls`);
  if (Array.isArray(pulls) && pulls.length > 0) {
    const merged = pulls.find((pull) => pull.merged_at) ?? pulls[0];
    return githubApi(`/pulls/${merged.number}`);
  }
  throw new Error(`No pull request associated with commit ${sha}`);
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
  const pull = await findPullRequestForSha(sha, prNumber);
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
