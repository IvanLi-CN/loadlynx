---
name: loadlynx-release-decision
description: "Decide LoadLynx release labels and backfill releases from merged PRs. Use when Codex needs to choose type/channel/component labels, decide whether skill/docs changes require a GitHub Release, update a merged PR's release intent, or dispatch Release (LoadLynx) with a PR number."
---

# LoadLynx Release Decision

Use this skill before choosing or changing LoadLynx PR release labels, before declaring a PR
`type:none`, and before backfilling a release from an already-merged PR.

## Source Of Truth

- PR labels are the release intent source of truth.
- Every PR must have exactly one `type:major|minor|patch|none` label and exactly one
  `channel:stable|beta|dev` label.
- `component:firmware|web|host-tools|docs` labels describe the affected surface; they do
  not decide whether a release is created.
- `type:none` is an explicit no-release decision. It must not be used for changes that
  modify owner-facing or user-facing operation contracts.

## Decision Rules

- Use `type:patch` or higher when a PR changes any owner-facing or user-facing operation
  contract, including released CLI/Web/firmware/installer behavior, `README.md` or
  `AGENTS.md` released-operation guidance, and `skills/loadlynx-user-operations` or
  `skills/loadlynx-developer-operations` instructions that affect what an operator may
  install, run, verify, or trust.
- Use `type:patch` or higher when a skill/docs-only PR changes the promised released CLI
  surface, hardware operation path, install path, firmware asset expectation, safety gate,
  or release/backfill procedure.
- Use `type:none` only for internal documentation, spec/solution maintenance, comments,
  or tooling notes that do not change an owner-facing or user-facing operation contract.
- Choose `type:minor` or `type:major` instead of `type:patch` when the underlying product
  change is additive or breaking by the repository's normal versioning policy.
- Keep `channel:stable` for normal user-facing releases. Use prerelease channels only when
  the owner explicitly wants beta/dev distribution.

## Backfill A Merged PR

Use this path when a merged PR should have released but carried `type:none` or the wrong
release type.

1. Verify the latest stable release and the source PR labels:

```bash
gh release list --repo IvanLi-CN/loadlynx --limit 10
gh pr view <pr-number> --repo IvanLi-CN/loadlynx --json state,mergedAt,mergeCommit,labels
```

2. Update only the release-intent labels needed for the source PR:

```bash
gh pr edit <pr-number> --repo IvanLi-CN/loadlynx --remove-label type:none --add-label type:patch
```

3. Dispatch the release workflow against the source PR:

```bash
gh workflow run release.yml --repo IvanLi-CN/loadlynx -f pr_number=<pr-number>
```

4. Watch the workflow to completion and verify the release plus PR comment:

```bash
gh run watch <run-id> --repo IvanLi-CN/loadlynx --interval 30
gh release view <tag> --repo IvanLi-CN/loadlynx --json tagName,url,isPrerelease,assets
gh pr view <pr-number> --repo IvanLi-CN/loadlynx --json comments
```

## Guardrails

- Do not create a release by editing package manifests or manually uploading partial
  assets. The `Release (LoadLynx)` workflow builds all official release artifacts.
- Do not call a skill/docs PR `type:none` merely because no Rust, firmware, or Web code
  changed. If it changes operator behavior, it is a release contract change.
- Before dispatching a backfill, check whether a newer stable tag already exists. If so,
  report the version-order risk before continuing.
