# Web Bundle Budget Gates Implementation

## Summary

- Added `web/scripts/check-bundles.ts` to enforce JS chunk budgets against emitted build artifacts.
- Added `bun run check:bundle:app`, `bun run check:bundle:storybook`, and `bun run check:bundle`.
- Wired app bundle checks into `.github/workflows/web-check.yml` and `.github/workflows/web-pages.yml`.
- Wired Storybook bundle checks into `bun run test:storybook:ci` so the static build is budgeted before the test runner starts.
- Raised Storybook's Vite `chunkSizeWarningLimit` to `1200` in production builds because repository policy now relies on explicit bundle budgets instead of Vite's generic warning.

## Current Budgets

- App JS chunk budget: `252 kB`
- Storybook preview JS chunk budget: `250 kB`
- Storybook framework mocker runtime budget (`vite-inject-mocker-entry.js`): `1200 kB`

## 2026-07-08 budget note

- The app JS chunk budget moved from `250 kB` to `252 kB` so production-safe `recharts` chunk consolidation can stay within the explicit repository gate without reintroducing manual chunk cycles.

## Verification

- `cd web && bun run check`
- `cd web && bun run build`
- `cd web && bun run check:bundle:app`
- `cd web && bun run build-storybook --quiet`
- `cd web && bun run check:bundle:storybook`

## Specification Companion Notes

`SPEC.md` owns the long-lived topic contract. Implementation progress, rollout records, documentation maintenance notes, and prior catalog state live in this companion document.

### Catalog Context
- Prior catalog status: 已完成
- Prior catalog timestamp: 2026-06-07
- Prior catalog implementation note: 显式 app/Storybook bundle budget、Storybook framework runtime 单独验收、CI 门禁

### SPEC Metadata Context
- Spec ID: m8k2v
- Lifecycle: active
- Status: 已完成
- Last: 2026-06-07

### Docs to Update

- `web/README.md`
- `.github/workflows/web-check.yml`
- `.github/workflows/web-pages.yml`
