---
title: Production preview smoke and React chunk-cycle guardrails
module: web
problem_type: production_runtime_crash
component: vite-build
tags: [vite, manual-chunks, react, github-pages, playwright, smoke-test]
status: active
related_specs: [n5nwv, m8k2v, rjkcw, xpm5n]
symptoms:
  - GitHub Pages served HTML/JS/CSS successfully but the page stayed blank.
  - dev server worked while built preview crashed on first paint.
root_cause: manual chunk boundaries created a vendor-to-react runtime initialization cycle
resolution_type: guardrail
---

# Production preview smoke and React chunk-cycle guardrails

## Context

Vite manual chunking can reduce app bundle size, but production runtime behavior depends on emitted chunk initialization order rather than source import order alone. A split that looks harmless in dev may only fail after Rollup emits separate runtime chunks.

## Symptoms

- Hosted GitHub Pages bundle returns `200` for `index.html` and assets, but the app never mounts.
- `bun run dev` works, while `bun run build && bun run preview` reproduces a first-paint crash.
- Browser runtime reports errors such as `Cannot set properties of undefined (setting 'Activity')` during module initialization.

## Root cause

React runtime code and helper modules that expected a single initialization path were split into a standalone `react-vendor` chunk, while other emitted vendor chunks still imported pieces back from that runtime. The emitted graph formed a `vendor` ↔ `react-vendor` cycle, so one side observed partially initialized bindings and crashed before React could mount.

## Resolution

- Keep React runtime on the normal vendor initialization path instead of forcing a standalone `react-vendor` chunk.
- If bundle budgets need more headroom, split only pure non-React dependencies into extra chunks.
- Add a dedicated production preview smoke that runs against built `dist` through `vite preview`, not the dev server.
- Make the smoke fail on both uncaught `pageerror` and browser `console error`, so white-screen regressions are caught before deploy.
- For PWA apps, include a production preview smoke that waits for service worker control, switches the browser offline, reloads, and verifies the app shell still mounts while API fetches remain network-only.

## Guardrails / Reuse Notes

- Treat `bun run build && bun run preview` as the canonical reproduction path for any “works in dev, blank in prod” report.
- Do not classify a fully fetched white screen as “performance” until production preview proves the app actually mounted.
- Manual chunk boundaries around `react` / `react-dom` are high risk unless the emitted graph is inspected for cycles.
- PWA client helpers such as `workbox-window` can be split into a dedicated vendor chunk to preserve bundle budgets, but React should stay on the normal vendor path.
- CI should run production preview smoke before publishing static artifacts; bundle budgets alone do not prove runtime health.

## References

- `web/scripts/chunking.ts`
- `web/playwright.preview.config.ts`
- `web/tests/e2e/preview-smoke.spec.ts`
- `web/tests/e2e/pwa-preview.spec.ts`
- `docs/specs/n5nwv-web-production-preview-smoke/SPEC.md`
- `docs/specs/xpm5n-web-pwa-offline-updates/SPEC.md`
