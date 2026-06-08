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
- Spec ID: rjkcw
- Lifecycle: active
- Status: 已完成
- Last: 2026-01-21

### 状态

- Status: 已完成
- Created: 2026-01-20
- Last: 2026-01-21

### 文档更新

- `web/README.md`：补充 lockfile 策略与“修改依赖后应更新哪些文件”的说明（避免未来再次出现 lockfile 不一致）。

### 里程碑（Milestones）

- [x] 对齐 Bun 版本与依赖安装命令（`web-pages` / `web-check` 一致；推荐使用 `bun ci` 或继续使用 `bun install --frozen-lockfile`）
- [x] 明确并落地 lockfile 策略（仅保留并维护一种 lockfile；避免自动迁移/双锁不一致）
- [x] GitHub Actions 验证：`web-check` + `web-pages` 在 `main` 上均成功
- [x] GitHub Pages 验证：部署 commit 与 `main` HEAD 对齐（通过 `head_sha`）

### 验证记录

- GitHub Actions (2026-01-21):
  - `Web Check` #105: success
  - `Web Deploy (GitHub Pages)` #16: success (`build` + `deploy`)
  - `head_sha`: `93e86974e960dbd61ca1cb4b651f42e63bd89635`（= `main` HEAD）
