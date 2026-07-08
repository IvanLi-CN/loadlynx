# Web PWA Offline Shell and Update Prompt Implementation

## Summary

- Added `vite-plugin-pwa` with generated service worker, manifest, Workbox precache, prompt-style update registration, and outdated-cache cleanup.
- Moved `web` build version generation before Vite build so `public/version.json` is present before precache generation.
- Added root-level PWA update prompt runtime and a separate Storybook-safe view component.
- Added active `/version.json` probing in the PWA prompt runtime so stale GitHub Pages tabs can discover a newer deployed build and offer an operator-confirmed refresh even before the browser surfaces a Workbox `needRefresh` event.
- Added Storybook states for update-ready, offline-ready, registration-error, and hidden prompt states.
- Added production preview smoke coverage for offline app-shell reload and network-only API behavior.
- Split `workbox-window` into `pwa-vendor` so app and Storybook JS bundle budgets remain within the existing `250 kB` cap.

## Verification

- `cd web && bun run check`
- `cd web && bun run test:unit`
- `cd web && bun run build`
- `cd web && bun run check:bundle:app`
- `cd web && bun run test:storybook:ci`
- `cd web && bun run test:preview-smoke`
- `cd web && bun run test:e2e`

## Remaining Gaps

- Browser install prompt UX is left to the browser; LoadLynx does not show a custom install CTA.
- Offline mode intentionally does not offer cached device data or queued write actions.
- The upgrade prompt still depends on the client revisiting the page, regaining visibility, or waiting for the periodic version probe; it does not push remote notifications into closed tabs.
