# Implementation

## Status

- Current status: implemented
- Last updated: 2026-06-01

## Implementation Summary

Implemented the Web Console redesign with a local cyberpunk neon design system, lucide icons, normal-route demo mode and bilingual shell i18n. The Web UI no longer depends on daisyUI or Iconify.

Key implementation points:

- Replaced daisyUI package/plugin/classes with `ll-*` local component classes and OKLCH tokens in `web/src/index.css`.
- Added local shadcn-style primitives under `web/src/components/ui/` plus shared `cn()` utility.
- Replaced Iconify icon data with `lucide-react` icons through the existing `AppIcon` wrapper.
- Added `i18next` + `react-i18next`, default `zh-CN`, English fallback, shell language switcher and Storybook locale toolbar.
- Updated console layout, dialogs, route surfaces, focused Storybook stories and E2E selectors to use the new component vocabulary.
- Added `?demo=true|false` as the persistent mode switch for the normal console routes. Demo mode swaps the device store/API data source without adding a separate route or page.
- Stored final visual evidence from normal console routes in `assets/visual-evidence/`.

## Verification

- `cd web && bun run check`
- `cd web && bun run build`
- `cd web && bun run test:storybook:ci`
- `cd web && bun run test:e2e`
- `rg` self-check confirmed no daisyUI package/plugin/Iconify references in Web source/dependency files and no old daisyUI semantic class tokens in `web/src`/`web/tests`.

## Specification Companion Notes

`SPEC.md` owns the long-lived topic contract. Implementation progress, rollout records, documentation maintenance notes, and prior catalog state live in this companion document.

### Catalog Context
- Prior catalog status: implemented
- Prior catalog timestamp: 2026-06-01
- Prior catalog implementation note: 全 Web Console 赛博朋克重做；移除 daisyUI/Iconify；新增 shadcn 风格组件与 i18n

### SPEC Metadata Context
- Spec ID: n9v2q
- Lifecycle: active
- Status: implemented
- Last: 2026-06-01
