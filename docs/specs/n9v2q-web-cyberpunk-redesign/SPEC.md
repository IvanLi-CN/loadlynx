# Web Console Cyberpunk Redesign

## Metadata

- Spec ID: n9v2q
- Lifecycle: active
- Status: implemented
- Last: 2026-06-01

## Specification

### Background

The current Web Console mixes daisyUI defaults, custom instrument CSS and route-level one-off layouts. The result is functional but visually inconsistent, and it does not provide the requested cyberpunk neon direction. The redesign must preserve all existing Web behavior while replacing the UI system and adding bilingual support.

### Goals

- Rebuild the complete Web Console with a product-grade cyberpunk neon style.
- Remove daisyUI entirely from dependencies, CSS plugins and source class names.
- Replace Iconify navigation icons with lucide-react icons.
- Introduce a local shadcn-style component system using Tailwind CSS 4 and accessible primitives.
- Add `zh-CN` and `en` i18n, with Chinese as the default UI language.
- Preserve current API calls, mock backend behavior, devd flows, calibration behavior and firmware dry-run semantics.
- Provide Storybook and visual evidence across mobile, tablet and desktop.

### Non-goals

- No firmware, protocol or HTTP API behavior changes.
- No real hardware verification requirement for this visual redesign.
- No automatic PR merge.
- No daisyUI compatibility layer.

### UI Library Policy

- Keep: React 19, Tailwind CSS 4, TanStack Router, TanStack Query, Storybook, Playwright.
- Add: local shadcn-style components, Radix primitives where needed, lucide-react, i18next, react-i18next.
- Remove: daisyUI and all `@iconify/*` packages.

### i18n Policy

- Default locale: `zh-CN`.
- Supported fallback locale: `en`.
- Technical terms may remain English in Chinese copy when they are device-domain terms: CC, CV, CP, USB-PD, PPS, PDO, APDO, Firmware, dry-run, lease, devd, UART.
- Storybook and E2E tests should use stable translated accessible names or deterministic test ids where translation would create excessive brittleness.

### Visual Direction Probes

The final direction is Probe B: dense bench-console neon. It uses a cyberpunk shell and signal color while keeping data panels low-glare.

- Probe A: [mobile neon drawer](./assets/probe-mobile-shell.svg)
- Probe B: [desktop bench console](./assets/probe-desktop-console.svg)
- Probe C: [calibration data lab](./assets/probe-calibration-lab.svg)

### Acceptance Criteria

- `web/package.json` contains no daisyUI or Iconify dependencies.
- `web/src/index.css` contains no daisyUI plugin import.
- Source code contains no daisyUI semantic class tokens such as `btn`, `card`, `input`, `select`, `menu`, `badge`, `alert`, `table`, `mockup-code` or `loading`.
- The app shell, drawer, route pages and common dialogs are styled with local components and Tailwind classes.
- The UI can be used at mobile, tablet and desktop widths without page-level horizontal scrolling for primary workflows.
- Chinese and English locale resources cover the visible Web Console shell and route copy.
- Storybook keeps focused route/layout/state coverage with mock data.
- Demo mode is a data/API mode on the normal console routes, enabled by `?demo=true` and remembered in `localStorage`; it must not introduce a separate demo route or page.
- Visual evidence is stored in this spec under `## Visual Evidence`.

### Test Plan

- `cd web && bun install`
- `cd web && bun run check`
- `cd web && bun run build`
- `cd web && bun run test:storybook:ci`
- `cd web && bun run test:e2e`
- `rg` self-check for removed daisyUI and Iconify tokens.

## Visual Evidence

Captured from the production-built console routes using pure frontend mock data. Viewports cover mobile, tablet, desktop and wide desktop, with Chinese default UI and one English switch coverage.

| Surface | Locale | Viewport | Evidence |
| --- | --- | --- | --- |
| Devices | zh-CN | 375x800 | [demo-mode-devices-mobile-zh.png](./assets/visual-evidence/demo-mode-devices-mobile-zh.png) |
| CC Control | zh-CN | 768x1024 | [demo-mode-cc-tablet-zh.png](./assets/visual-evidence/demo-mode-cc-tablet-zh.png) |
| Status | zh-CN | 1200x800 | [demo-mode-status-desktop-zh.png](./assets/visual-evidence/demo-mode-status-desktop-zh.png) |
| USB-PD | zh-CN | 1440x900 | [demo-mode-pd-wide-zh.png](./assets/visual-evidence/demo-mode-pd-wide-zh.png) |
| Settings | zh-CN | 375x800 | [demo-mode-settings-mobile-zh.png](./assets/visual-evidence/demo-mode-settings-mobile-zh.png) |
| Firmware | zh-CN | 768x1024 | [demo-mode-firmware-tablet-zh.png](./assets/visual-evidence/demo-mode-firmware-tablet-zh.png) |
| Calibration | zh-CN | 1200x800 | [demo-mode-calibration-desktop-zh.png](./assets/visual-evidence/demo-mode-calibration-desktop-zh.png) |
| Demo mode language switch | en | 1440x900 | [demo-mode-shell-en-wide.png](./assets/visual-evidence/demo-mode-shell-en-wide.png) |
