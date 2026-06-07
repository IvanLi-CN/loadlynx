# Web Bundle Budget Gates Implementation

## Summary

- Added `web/scripts/check-bundles.ts` to enforce JS chunk budgets against emitted build artifacts.
- Added `bun run check:bundle:app`, `bun run check:bundle:storybook`, and `bun run check:bundle`.
- Wired app bundle checks into `.github/workflows/web-check.yml` and `.github/workflows/web-pages.yml`.
- Wired Storybook bundle checks into `bun run test:storybook:ci` so the static build is budgeted before the test runner starts.
- Raised Storybook's Vite `chunkSizeWarningLimit` to `1200` in production builds because repository policy now relies on explicit bundle budgets instead of Vite's generic warning.

## Current Budgets

- App JS chunk budget: `250 kB`
- Storybook preview JS chunk budget: `250 kB`
- Storybook framework mocker runtime budget (`vite-inject-mocker-entry.js`): `1200 kB`

## Verification

- `cd web && bun run check`
- `cd web && bun run build`
- `cd web && bun run check:bundle:app`
- `cd web && bun run build-storybook --quiet`
- `cd web && bun run check:bundle:storybook`
