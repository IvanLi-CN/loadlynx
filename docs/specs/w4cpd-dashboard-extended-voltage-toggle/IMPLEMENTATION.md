# Implementation

## Status

- Current status: 已完成
- Last updated: 2026-03-10

## Implementation Summary

This companion document records implementation status for the canonical spec. Existing implementation evidence remains in the spec body, referenced PRs, visual evidence, and related project documents.

## Remaining Gaps

- Refresh this implementation summary when the spec is next materially updated.

## Specification Companion Notes

`SPEC.md` owns the long-lived topic contract. Implementation progress, rollout records, documentation maintenance notes, and prior catalog state live in this companion document.

### Catalog Context
- Prior catalog status: 已完成
- Prior catalog timestamp: 2026-03-10
- Prior catalog implementation note: PR #70；实现已完成；HIL 可选

### 状态

- Status: 已完成
- Created: 2026-03-09
- Last: 2026-03-10
- Notes: PR #70；HIL 可选（Safe5V 起步、开关开/关重协商、红态锁存）

### 实现前置条件（Definition of Ready / Preconditions）

- 已满足：主人确认当前 mock 的控件布局、左侧 `PD` 文案、右侧设置入口图标与灰/蓝/红三态语义。
- 进入实现前无需再做新的主界面设计决策。

### 文档更新（Docs to Update）

- `docs/interfaces/main-display-ui.md`: 主界面 control row 下方两个按钮的语义与布局说明。
- `docs/specs/h3gz5-usb-pd-sink-toggle/SPEC.md`: 作为历史方案引用时补充指向新 spec 的说明（如仍保留）。
- `docs/specs/wjhba-dashboard-pd-button-label/SPEC.md`: 作为历史设计依据时补充指向新 spec 的说明（如仍保留）。

### 计划资产（Spec assets）

- Directory: `docs/specs/w4cpd-dashboard-extended-voltage-toggle/assets/`
- In-plan references:
  - `![states](./assets/dashboard-extended-voltage-states-v1.png)`
  - `![safe5v](./assets/dashboard-safe5v-only-v1.png)`
  - `![extended](./assets/dashboard-extended-voltage-on-v1.png)`
  - `![error](./assets/dashboard-extended-voltage-error-v1.png)`
- PR visual evidence source: 待实现阶段补充。

### 资产晋升（Asset promotion）

None

### 实现里程碑（Milestones / Delivery checklist）

- [x] M1: 数字侧引入 `allow_extended_voltage` 持久化字段，并完成旧 EEPROM blob 迁移默认值。
- [x] M2: 主界面左右两个按钮完成语义重排与三态渲染，移除 on-screen LOAD hit-test。
- [x] M3: `PD settings` Apply、attach/link 自动下发、以及 `GET/PUT /api/v1/pd` 全部接入 Safe5V 门控，并完成验证。
