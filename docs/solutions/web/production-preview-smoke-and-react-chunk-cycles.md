---
title: Production preview smoke and runtime-coupled chunk-cycle guardrails
module: web
problem_type: production_runtime_crash
component: vite-build
tags: [vite, manual-chunks, react, github-pages, playwright, smoke-test]
status: active
related_specs: [n5nwv, m8k2v, rjkcw, xpm5n]
symptoms:
  - GitHub Pages served HTML/JS/CSS successfully but the page stayed blank.
  - dev server worked while built preview crashed on first paint.
root_cause: manual chunk boundaries split runtime-coupled libraries into production-only initialization cycles
resolution_type: guardrail
---

# Production preview smoke and runtime-coupled chunk-cycle guardrails

## Context

Vite manual chunking can reduce app bundle size, but production runtime behavior depends on emitted chunk initialization order rather than source import order alone. A split that looks harmless in dev may only fail after Rollup emits separate runtime chunks.

## Symptoms

- Hosted GitHub Pages bundle returns `200` for `index.html` and assets, but the app never mounts.
- `bun run dev` works, while `bun run build && bun run preview` reproduces a first-paint crash.
- Browser runtime reports errors such as `Cannot set properties of undefined (setting 'Activity')` during module initialization.

## Root cause

Manual chunk boundaries split libraries that expect a single initialization path into separate production chunks. Historical failures used `vendor` ↔ `react-vendor`; the same class reappeared when `recharts` internals were split across `recharts-*` and `state-vendor`. In both cases the emitted graph formed a cycle or half-initialized read, so one side observed undefined bindings and crashed before the app or dashboard route could mount.

## Resolution

- Keep runtime-coupled libraries on a single initialization path instead of forcing them across multiple vendor chunks.
- React should stay on the normal vendor initialization path, and `recharts` should not be split across internal runtime/state chunk boundaries unless the emitted graph is proven acyclic.
- If bundle budgets need more headroom, split only pure non-core dependencies into extra chunks or adjust the explicit budget gate by the smallest defensible amount.
- Add a dedicated production preview smoke that runs against built `dist` through `vite preview`, not the dev server.
- Make the smoke fail on both uncaught `pageerror` and browser `console error`, so white-screen regressions are caught before deploy.
- Extend the smoke beyond the Overview homepage to at least one real dashboard route so route-level chart crashes cannot hide behind a healthy app shell.
- For PWA apps, include a production preview smoke that waits for service worker control, switches the browser offline, reloads, and verifies the app shell still mounts while API fetches remain network-only.
- For PWA apps, add a stale-shell bootstrap guard ahead of the hashed entry bundle so an old cached HTML shell can compare its embedded version against `/version.json` and self-recover before it requests dead assets from a newer deploy.
- When a broken client is already pinned to a specific old entry asset name, ship a migration shim at that legacy asset path for at least one release so the stale client can clear old PWA state and reload into the current shell.

## Guardrails / Reuse Notes

- Treat `bun run build && bun run preview` as the canonical reproduction path for any “works in dev, blank in prod” report.
- Do not classify a fully fetched white screen as “performance” until production preview proves the app actually mounted.
- Manual chunk boundaries around `react` / `react-dom` or chart runtimes like `recharts` are high risk unless the emitted graph is inspected for cycles.
- PWA client helpers such as `workbox-window` can be split into a dedicated vendor chunk to preserve bundle budgets, but React should stay on the normal vendor path.
- If hashed entry assets change across deploys, do not rely on an old cached HTML shell to “eventually” self-heal; give the shell its own stable bootstrap path that can clear stale service-worker/cache state before loading app code.
- If the bad deploy is already live and some clients are pinned to an old entry asset path, the stable bootstrap alone is not enough; add a temporary compatibility asset at the legacy path and cover it with preview smoke.
- CI should run production preview smoke before publishing static artifacts; bundle budgets alone do not prove runtime health.

## References

- `web/scripts/chunking.ts`
- `web/playwright.preview.config.ts`
- `web/tests/e2e/preview-smoke.spec.ts`
- `web/tests/e2e/pwa-preview.spec.ts`
- `web/public/pwa-shell-guard.js`
- `web/public/assets/index-SkMVprsZ.js`
- `docs/specs/n5nwv-web-production-preview-smoke/SPEC.md`
- `docs/specs/xpm5n-web-pwa-offline-updates/SPEC.md`
