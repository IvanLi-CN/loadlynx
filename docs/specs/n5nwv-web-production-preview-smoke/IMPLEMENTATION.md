# Web production preview smoke 与 chunk-cycle regression Implementation

## Summary

- 移除了独立 `react-vendor` manual chunk，让 React runtime 回到常规 `vendor` 初始化路径，消除 production bundle 的循环初始化崩溃。
- 新增 `bun run test:preview-smoke`、`web/playwright.preview.config.ts` 与 `web/tests/e2e/preview-smoke.spec.ts`，专门针对已构建 `dist` 运行生产预览冒烟。
- 将 preview smoke 接入 `.github/workflows/web-check.yml` 与 `.github/workflows/web-pages.yml`，并同步 workflow hygiene 契约。
- 更新 `web/README.md`，把新的 preview smoke 脚本和用途写入人类入口文档。
- 2026-07-08: `recharts` 相关 dashboard route 在 production bundle 下再次出现 route-level crash。修复方式是保持整个 `recharts` 运行时留在单一 `recharts-vendor` chunk，并把 preview smoke 扩展到 `/$deviceId/cc`，覆盖真实仪表盘首屏而不是只看 Overview 首页。

## Verification

- `cd web && bun run build`
- `cd web && bun run check:bundle:app`
- `cd web && bun run test:preview-smoke`
- `cd web && bun run test:e2e`

## Specification Companion Notes

`SPEC.md` 保留长期契约；这里记录实现覆盖、验证与 companion 级维护事实。

### Spec Metadata Context

- Spec ID: `n5nwv`
- Lifecycle: `active`
- Status: `implemented`

### Project Docs Updated

- `web/README.md`

### Follow-up Notes

- 如果未来需要继续细分 vendor chunk，必须先证明不会重新引入 React runtime 初始化循环，再允许新增 chunk 规则。
