# Implementation

## Status

- Current status: in_progress
- Last updated: 2026-06-20

## Implementation Summary

- Introduce a sticky top shell in `ConsoleLayout` and remove the legacy sidebar / icon rail / mobile navigation drawer rendering path.
- Route `/` and `/devices` to the Overview experience, keep `/$deviceId/cc` as the dashboard canonical route, and add `/$deviceId/about`.
- Embed the full USB-PD query/apply flow inside the dashboard through `PdControlPanel`, while preserving `/$deviceId/pd` as a redirect-only compatibility entry.
- Add a pathless `SystemLayout` to provide unified secondary navigation for settings, calibration, status, firmware and about.
- Update Storybook route/layout coverage and Playwright selectors to align with the top-nav shell and dashboard PD panel workflow.

## Validation

- Pending full validation chain:
  - `cd web && bun run check`
  - `cd web && bun run build`
  - `cd web && bun run test:storybook:ci`
  - `cd web && bun run test:e2e`

## Remaining Gaps

- Capture desktop/mobile visual evidence on mock/demo data and link it from this spec if needed.
- Record final validation results and any residual compatibility notes.
