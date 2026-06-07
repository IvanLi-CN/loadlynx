# Implementation

## Status

- Current status: 已更新
- Last updated: 2026-06-07

## Implementation Summary

- Added a repo-local release label policy and `Label Gate` workflow.
- Added `.github/quality-gates.json` to declare `Label Gate` as the required release-intent check and to keep the review policy at `0` approvals for the protected `main` branch.
- Added release intent tooling for label validation, version computation, and PR release comments.
- Refactored release automation to consume merged PR labels and inject the computed release version into artifacts.
- Release asset assembly now publishes host-tools installer scripts, firmware catalog JSON, Web bundle, and `SHA256SUMS` covering all release files before creating the GitHub Release.
- Added a repo-local declaration check so CI fails if the quality-gates contract drifts away from the expected PR-only + zero-approval policy.

## Verification

- Local label validator tests cover valid, missing, duplicate, and unknown labels.
- Local quality-gates validation covers protected branch name, PR-only semantics, zero required approvals, and the `Label Gate` required check contract.
- Release intent dry-run confirms `v0.1.0` plus `type:patch` resolves to `v0.1.1`.
- Full release validation completes after this PR merges and the automatic `Release (LoadLynx)` run finishes.
