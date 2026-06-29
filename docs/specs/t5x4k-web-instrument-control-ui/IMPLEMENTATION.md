# Implementation

## Status

- Current status: 已完成
- Last updated: 2026-01-20
- Origin: migrated from legacy planning docs.

## Implementation Summary

This canonical companion document was initialized from the legacy plan migration. Detailed implementation evidence remains in the migrated spec body and any referenced PR/HIL notes.

2026-06-20 update:

- `device-cc` is now surfaced to owners as `仪表盘`, while retaining the `/$deviceId/cc` route for compatibility.
- The full USB-PD query / validation / Apply workflow is embedded into the dashboard via `PdControlPanel`.
- The dashboard now lives under the top-nav shell introduced by `m3n8p`, rather than under a left sidebar shell.

2026-06-22 update:

- The dashboard information architecture is being tightened around three explicit priorities:
  - `P0 primary information`: device-critical live measurements and the time-series view.
  - `P1 secondary information`: thermal, fault, link, protection and PD summary context.
  - `P2 control/general information`: all write actions, mode changes, setpoints and advanced controls.
- The left column is the operator's read-first surface; the right column is the control-first surface.

## Milestones

No explicit milestones were recorded in the legacy plan.

## Remaining Gaps

## Specification Companion Notes

`SPEC.md` owns the long-lived topic contract. Implementation progress, rollout records, documentation maintenance notes, and prior catalog state live in this companion document.

### Catalog Context
- Prior catalog status: 已完成
- Prior catalog timestamp: 2026-01-20
- Prior catalog origin: legacy planning taxonomy import.

### SPEC Metadata Context
- Spec ID: t5x4k
- Lifecycle: active
- Status: 已完成
- Last: 2026-01-20

### 状态

- Status: 已完成
- Created: 2026-01-16
- Last: 2026-01-20

### 文档更新（Docs to Update）

- `web/README.md`: 更新页面结构与入口说明（如果已有 UI 导航文档）。
- `docs/specs/README.md`: 新增本计划索引行。

### 里程碑（Milestones）

- [x] M1: 需求冻结（读数优先级与主读数策略确认）
- [x] M2: 组件与布局方案确定（UI 结构与信息层级）
- [x] M3: 验收标准冻结（含异常与响应式）
- [x] M4: 仪器面板 UI 实现（Storybook 可预览）
- [x] M5: 高保真样式对齐设计图 + E2E 绿灯
