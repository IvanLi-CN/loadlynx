# dvfnn · PR Label Release Flow

## Summary

- PR labels are the source of truth for release intent.
- `Label Gate` validates release labels before merge.
- `Release (LoadLynx)` consumes the merged PR labels on `main`, computes the next version, builds all releaseable project artifacts, deploys the validated release Web artifact to GitHub Pages, creates the GitHub Release, and comments back on the source PR.
- Release failures continue to use the Telegram notifier; ordinary PR CI failures do not.

## Label Contract

- Required exactly once: `type:major`, `type:minor`, `type:patch`, or `type:none`.
- Required exactly once: `channel:stable`, `channel:beta`, or `channel:dev`.
- Optional zero or more: `component:firmware`, `component:web`, `component:host-tools`, `component:docs`.
- Unknown labels with one of these prefixes fail the gate.
- Duplicate required groups fail the gate.

## Release Behavior

- `type:none` records a no-release intent and does not create a GitHub Release.
- `channel:stable` creates `vX.Y.Z` from the latest stable `v*` tag and the selected `type`.
- `channel:beta` creates a prerelease `vX.Y.Z-beta.<run-number>`.
- `channel:dev` creates a prerelease `dev-<timestamp>-<sha>`.
- Stable version discovery ignores `dev-*` tags and other non-stable tags.
- Official release artifacts include analog ELF, digital ELF, firmware catalog, host tools for supported host targets, user installer scripts, Web bundle, and `SHA256SUMS` covering every release asset.
- Release builds inject the computed version into firmware, Web, and released host-tools version metadata instead of rewriting package manifests.
- The release Web bundle is budget-checked and production-preview-smoked before packaging. GitHub Pages receives that exact tarball only after its embedded shell version and `version.json` match the resolved release tag; a Pages failure blocks release creation.
- The manual Pages recovery workflow accepts a required published `release_tag`, downloads the matching Web asset, and performs the same validation before deployment.
- Released host tools must report that injected version through owner-facing `loadlynx -v` and `loadlynx-devd --version` output.

## Release Decision Matrix

- PRs that change released CLI, Web, firmware, installer, firmware catalog, release workflow, or user-facing artifact behavior must use `type:patch` or higher.
- PRs that change owner-facing/user-facing operation contracts in `README.md`, `AGENTS.md`, `skills/loadlynx-user-operations`, or `skills/loadlynx-developer-operations` must use `type:patch` or higher even when the diff is docs/skill-only.
- A skill/docs PR changes the operation contract when it changes what an operator may install, run, verify, trust, or treat as released behavior.
- `type:none` is allowed only for internal documentation, spec/solution maintenance, comments, or tooling notes that do not change an owner-facing or user-facing operation contract.
- Release label decisions and release backfills are governed by `skills/loadlynx-release-decision/SKILL.md`.
- If a merged PR should have released but carried `type:none`, update the source PR labels and dispatch `Release (LoadLynx)` with `workflow_dispatch` input `pr_number=<PR>`.

## GitHub Integration

- `Label Gate` is declared in `.github/quality-gates.json` as the required check.
- `Code Check` and `Web Check` remain informational PR checks; embedded digital firmware coverage is consolidated into `Code Check` instead of a separate `Digital Check` workflow.
- The default branch is protected through GitHub branch protection as a PR-only branch: direct pushes are disallowed, admins are still enforced, and the PR review requirement remains at `0` approvals.
- The release workflow comments on the source PR after a successful release with version, release URL, asset list, workflow run URL, and merge commit.
- Telegram notification scope remains release failures only.

## Acceptance

- A PR without exactly one `type:*` and one `channel:*` fails `Label Gate`.
- A PR with `type:patch` and `channel:stable` on top of `v0.1.0` produces `v0.1.1`.
- A merged stable PR creates a GitHub Release containing analog, digital, firmware catalog, host-tools, installer, Web, and `SHA256SUMS` artifacts.
- A docs/skill PR that changes owner-facing released operation guidance is labeled `type:patch` or higher; `type:none` is rejected by review or contract checks for that class of change.
- Firmware/Web/host-tools release metadata reports the injected release version.
- GitHub Pages returns the same version as the release Web tarball and cannot be updated by an independent `main` source build.
- Released `loadlynx` and `loadlynx-devd` binaries report the release tag version instead of the crate package version.
- The source PR receives the release completion comment.
- Ordinary PR CI failures do not trigger Telegram notifications.
