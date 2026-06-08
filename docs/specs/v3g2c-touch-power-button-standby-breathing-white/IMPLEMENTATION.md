# Implementation

## Status

- Current status: 已完成
- Last updated: 2026-02-03
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
- Prior catalog timestamp: 2026-02-03
- Prior catalog origin: legacy planning taxonomy import.

### SPEC Metadata Context
- Spec ID: v3g2c
- Lifecycle: active
- Status: 已完成
- Last: 2026-02-03

### 状态

- Status: 已完成
- Created: 2026-02-01
- Last: 2026-02-03

### 实现前置条件（Definition of Ready / Preconditions）

- 已冻结“睡眠待机”判定：`ScreenPowerState::Off`
- 已冻结触摸电源按键输入：`TOUCH_SPRING`（GPIO14 / TouchPad14）
- 已冻结“熄屏触摸电源键”语义：只唤醒且输入被消费
- 已确认指示灯硬件与电气约束：RGB 三路 PWM（active‑low / COM=3V3；详见 `docs/interfaces/touch-switch-and-rgb-led.md`）

### 文档更新（Docs to Update）

- `docs/interfaces/pinmaps/esp32-s3.md`：补充/确认电源按键与其指示灯的网络名、GPIO 占用与电气注意事项（避免与上电毛刺/strapping 冲突）。
- `docs/dev-notes/software.md`（或等价位置）：补充“睡眠待机指示灯策略（白光低频呼吸）”与验收口径，便于后续排查与一致性。

### 计划资产（Spec assets）

- None

### 资产晋升（Asset promotion）

None

### 实现里程碑（Milestones）

- [x] M1: digital：增加“待机白光呼吸”灯效状态机（触发条件=ScreenPowerState::Off，退出条件=Off→Active）
- [x] M2: digital：接入 RGB 三路 LEDC PWM（Timer3/Channel3-5 或等价不冲突配置）并实现白光呼吸波形（可调 `T_ms/max_brightness`）
- [x] M3: digital：熄屏期间触摸电源键输入仅唤醒且被消费（不改变业务状态）
- [x] M4: HIL：记录最终 `T_ms/max_brightness` 取值与观感结论，并补充到 `docs/dev-notes/software.md`（final: T_ms=14000, max_brightness=12%, update_ms=10）
