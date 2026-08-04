# Implementation

## Status

- Current status: 已完成
- Last updated: 2026-06-08

## Implementation Summary

This companion document records implementation status for the canonical spec.

- 2026-06-08 的本地续推将 calibration route 进一步拆分为 store、status、mode-sync、draft/toast/dialog 等子模块，降低了单文件复杂度；对应实现已本地提交为 `8b5c785`。
- 本地验证已覆盖 `web` 单测、生产构建、Storybook 静态构建，以及 `libs/protocol` 与 `firmware/digital` 的编译/测试路径。
- 最新 Storybook canvas 视觉证据已刷新到 spec 资产目录，并绑定到 calibration route 重构提交 `8b5c785`。

## Remaining Gaps

- No code or verification gaps were found in the current local pass.

## Specification Companion Notes

`SPEC.md` owns the long-lived topic contract. Implementation progress, rollout records, documentation maintenance notes, and prior catalog state live in this companion document.

### Catalog Context
- Prior catalog status: 已完成
- Prior catalog timestamp: 2026-04-16
- Prior catalog implementation note: 本地校准页稳定性修复、Storybook/E2E 回归与视觉证据已完成

### 状态

- Status: 已完成
- Created: 2026-04-16
- Last: 2026-06-08
- Notes: 2026-06-08 已完成 calibration route 组件化重构、单测/构建/Storybook 刷新与视觉证据回采；相关实现与视觉证据绑定本地提交 `8b5c785`。

### 实现前置条件（Definition of Ready / Preconditions）

- 已满足：主人已确认问题表现来自校准页数据/模式偶发错位，并授权直接实现与更新 SPEC。

### 文档更新（Docs to Update）

- `docs/specs/README.md`
- `docs/solutions/web/calibration-mode-single-owner-and-status-fallback.md`

### 实现里程碑（Milestones / Delivery checklist）

- [x] M1: 收口 calibration mode 协调入口，移除页面内重复 cleanup `off`，并让 storage hydrate 早于自动 sync。
- [x] M2: 为 calibration 页补齐 last-good status + fallback polling，并对 mismatch 的 RAW / DAC 展示做显式门控。
- [x] M3: 补齐 Storybook / E2E 回归，保留稳定视觉证据与文档更新。
