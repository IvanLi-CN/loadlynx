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
- The quality-gates declaration check now also verifies that each declared workflow name resolves to a local workflow file and that every declared branch-protection job name still exists in the actual GitHub Actions workflow even if the workflow is internally split into additional jobs.
- Refactored the quality-gates checker into reusable validation logic plus a dedicated regression test script so workflow/job contract drift is covered both by script-level fixtures and by the live repository declaration check.
- Added a separate workflow hygiene checker so every repo-local GitHub Actions workflow must declare explicit top-level `permissions` and job-level `timeout-minutes`; `Code Check` runs the same script in CI.

## Verification

- Local label validator tests cover valid, missing, duplicate, and unknown labels.
- Local quality-gates validation covers protected branch name, PR-only semantics, zero required approvals, the `Label Gate` required check contract, and workflow/job-name drift between `.github/quality-gates.json` and `.github/workflows/*.yml`.
- `npm run test:quality-gates` now runs both fixture-style regression tests for the checker and the live repository declaration check; `Code Check` consumes the same script in CI.
- `npm run test:workflow-hygiene` runs fixture-style and live-repository checks for workflow `permissions` and `timeout-minutes`; `Code Check` consumes the same script in CI.
- Release intent dry-run confirms `v0.1.0` plus `type:patch` resolves to `v0.1.1`.
- Full release validation completes after this PR merges and the automatic `Release (LoadLynx)` run finishes.
