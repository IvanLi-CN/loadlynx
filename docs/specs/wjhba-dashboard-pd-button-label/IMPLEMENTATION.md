# Implementation

## Status

- Current status: 已完成
- Last updated: 2026-01-19
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
- Prior catalog timestamp: 2026-01-19
- Prior catalog origin: legacy planning taxonomy import.

### SPEC Metadata Context
- Spec ID: wjhba
- Lifecycle: active
- Status: 已完成
- Last: 2026-01-19

### 状态

- Status: 已完成
- Created: 2026-01-16
- Last: 2026-01-19

### 文档更新（Docs to Update）

- `docs/interfaces/main-display-ui.md`: 补充/修订 `PD` 按钮两行文本规则（从 “PD/5V|20V” 更新为 Detach/PPS/Fixed 规则）。

### 里程碑（Milestones）

- [x] M1: 冻结三态文案映射（Detach/PPS/Fixed）与触发条件
- [x] M2: 冻结数值格式化与占位符规则（Fixed 整数 / PPS 一位小数 / `N/A`）
- [x] M3: 冻结“不可用灰显”判定来源（完整 caps/范围校验）并补齐文档更新点
