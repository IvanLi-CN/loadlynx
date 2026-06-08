# Implementation

## Status

- Current status: 已完成
- Last updated: 2026-01-21
- Origin: migrated from legacy planning docs.

## Implementation Summary

This canonical companion document was initialized from the legacy plan migration. Detailed implementation evidence remains in the migrated spec body and any referenced PR/HIL notes.

## Milestones

No explicit milestones were recorded in the legacy plan.

## Remaining Gaps

## Specification Companion Notes

`SPEC.md` owns the long-lived topic contract. Implementation progress, rollout records, documentation maintenance notes, and prior catalog state live in this companion document.

### Catalog Context
- Prior catalog status: 已完成
- Prior catalog timestamp: 2026-01-21
- Prior catalog origin: legacy planning taxonomy import.

### SPEC Metadata Context
- Spec ID: yff7v
- Lifecycle: active
- Status: 已完成
- Last: 2026-01-21

### 状态

- Status: 已完成
- Created: 2026-01-20
- Last: 2026-01-21

### 文档更新

- `web/README.md`：补充“版本信息如何生成/展示、如何跳转到 GitHub 溯源”的说明。

### 里程碑（Milestones）

- [x] 构建期注入：将版本/溯源信息编译进 Vite bundle（`VITE_APP_VERSION` / `VITE_APP_GIT_*` / `VITE_GITHUB_REPO`）
- [x] 在 `ConsoleLayout` 增加 `AppVersionLink` 展示位（并在 Storybook runtime 隐藏；数据主来源为构建期注入）
- [x] 本地预览与 Pages 验证：版本展示正常、GitHub 跳转正确
