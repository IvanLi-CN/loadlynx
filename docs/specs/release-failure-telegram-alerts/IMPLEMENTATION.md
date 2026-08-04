# Implementation

## Status

- Current status: 已更新
- Last updated: 2026-05-29

## Implementation Summary

- `notify-release-failure.yml` now watches the unified `Release (LoadLynx)` workflow.
- Release failure notifications stay scoped to release workflow failures; ordinary PR CI failures remain GitHub check feedback only.
- The release workflow summary and release intent snapshot carry PR label release context for failure investigation.

## Remaining Gaps

- Telegram smoke remains available through the workflow's manual dispatch path.
