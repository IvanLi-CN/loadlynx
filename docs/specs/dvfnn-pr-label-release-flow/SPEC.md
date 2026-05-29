# dvfnn · PR Label Release Flow

## Summary

- PR labels are the source of truth for release intent.
- `Label Gate` validates release labels before merge.
- `Release (LoadLynx)` consumes the merged PR labels on `main`, computes the next version, builds all releaseable project artifacts, creates the GitHub Release, and comments back on the source PR.
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
- Official release artifacts include analog ELF, digital ELF, host tools for supported host targets, and a Web bundle.
- Release builds inject the computed version into firmware and Web artifacts instead of rewriting package manifests.

## GitHub Integration

- `Label Gate` is declared in `.github/quality-gates.json` as the required check.
- The default branch is protected through GitHub ruleset/branch protection so release-intent labels cannot be bypassed before merge.
- The release workflow comments on the source PR after a successful release with version, release URL, asset list, workflow run URL, and merge commit.
- Telegram notification scope remains release failures only.

## Acceptance

- A PR without exactly one `type:*` and one `channel:*` fails `Label Gate`.
- A PR with `type:patch` and `channel:stable` on top of `v0.1.0` produces `v0.1.1`.
- A merged stable PR creates a GitHub Release containing analog, digital, host-tools, and Web artifacts.
- Firmware/Web release metadata reports the injected release version.
- The source PR receives the release completion comment.
- Ordinary PR CI failures do not trigger Telegram notifications.
