# Implementation

## Status

- Current status: 已完成
- Last updated: 2026-02-05
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
- Prior catalog timestamp: 2026-02-05
- Prior catalog origin: legacy planning taxonomy import.

### SPEC Metadata Context
- Spec ID: shkmx
- Lifecycle: active
- Status: 已完成
- Last: 2026-02-05

### 状态

- Status: 已完成
- Created: 2026-02-01
- Last: 2026-02-05

### 实现前置条件（Definition of Ready / Preconditions）

- 已冻结：仅支持扬声器输出；不做硬件版本区分；不引入蜂鸣器回退路径。
- 引脚分配为既定事实：不得改动既有引脚分配与网络命名；实现只按现有分配使用 `I2S_*` 与 `AMP_SD_MODE`，且不得试图复用/重映射 `GPIO21=BUZZER`。

### 文档更新（Docs to Update）

说明：计划阶段不修改 `docs/` 下非本 spec 目录的文档；以下更新在实现阶段随代码一起落地并作为验收的一部分。

- `docs/boards/control-board.md`: 明确声明当前硬件版本已经没贴装蜂鸣器相关器件；音频提示走扬声器（MAX98357A/I²S）。

### 计划资产（Spec assets）

- None

### 资产晋升（Asset promotion）

None

### 实现里程碑（Milestones）

- [x] M1: 数字板：新增扬声器音频输出后端（I²S + MAX98357A），提供 play/stop/mute 的最小接口
- [x] M2: 数字板：`prompt_tone` 输出从蜂鸣器迁移到扬声器（保持既有告警/ack 语义）
- [x] M3: HIL：在当前硬件验证可听到 UI 反馈音与告警音，并记录日志/结论
