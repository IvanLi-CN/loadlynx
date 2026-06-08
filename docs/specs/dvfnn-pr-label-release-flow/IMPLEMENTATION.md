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
- Hardened `Code Check` so the host-tools job explicitly provisions Node.js 20 and installs root `npm` dependencies before running repo-local workflow and quality-gate validators, removing the previous implicit dependence on runner preinstalls.
- Removed the legacy standalone `Digital Check` workflow after its digital formatting/build coverage had already been subsumed by `Code Check`, reducing duplicate CI maintenance and eliminating a second, weaker source of truth for digital firmware verification.
- Added repo-level `just check*` entrypoints so local developers can run CI-aligned policy, host, embedded and web verification without reconstructing the command matrix by hand.
- Added explicit `just deps*` entrypoints and dependency-presence guards so local check commands fail fast with actionable setup guidance instead of surfacing opaque downstream tool errors.
- Added an explicit local guard for the vendored `third_party/embassy` submodule so embedded lint paths fail with a concrete submodule-init hint instead of a deep Cargo path-resolution error.
- Added explicit local guards for the ESP toolchain alias (`cargo +esp`) and `export-esp.sh`, so digital formatting/build paths fail with actionable `espup` guidance before entering the underlying Xtensa toolchain bootstrap.
- Made `just d-build` source `export-esp.sh` itself so local and CI digital build entrypoints share one self-contained contract instead of requiring callers to remember an extra shell prelude.
- Added a fail-fast guard for the digital Wi-Fi compile-time config so `just d-build` now points operators to `.env.example` before compilation instead of failing late inside the firmware build script.
- Aligned local `check-root` with CI by including the release-label regression test, and added the same `third_party/embassy` submodule guard to the embedded build path so `check-embedded` fails fast with the same actionable hint as `lint-embedded`.
- Closed the remaining digital firmware lint gap by adding repo-local `just d-clippy`, wiring `lint-embedded` through it, and making the `Code Check` workflow's `digital-firmware` job fail on `cargo +esp clippy -- -D warnings` before the release build step.
- Centralized the GitHub Actions Node.js runtime version into repo-root `.node-version` and switched every `actions/setup-node` callsite to `node-version-file`, so workflow runtime upgrades no longer require editing four separate inline literals.

## Verification

- Local label validator tests cover valid, missing, duplicate, and unknown labels.
- Local quality-gates validation covers protected branch name, PR-only semantics, zero required approvals, the `Label Gate` required check contract, and workflow/job-name drift between `.github/quality-gates.json` and `.github/workflows/*.yml`.
- `npm run test:quality-gates` now runs both fixture-style regression tests for the checker and the live repository declaration check; `Code Check` consumes the same script in CI.
- `npm run test:workflow-hygiene` runs fixture-style and live-repository checks for workflow `permissions` and `timeout-minutes`; `Code Check` consumes the same script in CI.
- `Code Check` now also re-triggers when the root `package-lock.json` changes, so CI dependency bootstrap changes cannot drift silently past the workflow that consumes them.
- `.github/quality-gates.json` now declares only the surviving informational checks (`check`, `web-check`), while digital firmware remains verified via the `Code Check` workflow's `digital-firmware` job.
- Local embedded lint verification now includes digital firmware clippy under dummy Wi-Fi compile-time config, matching CI's `digital-firmware` lint semantics without requiring operators to create a real `.env` just to run the lint gate.
- Workflow hygiene now rejects inline `setup-node` versions and requires `node-version-file: ".node-version"` whenever GitHub Actions provisions Node.js.
- Release intent dry-run confirms `v0.1.0` plus `type:patch` resolves to `v0.1.1`.
- Full release validation completes after this PR merges and the automatic `Release (LoadLynx)` run finishes.
