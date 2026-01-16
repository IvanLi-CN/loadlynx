# UI Components / Routes

本文件冻结 Web 侧 CP 页面相关的“路由与组件接口”形状（props/state），便于后续实现与测试对齐。

外部 HTTP API 契约引用：

- `docs/plan/0017:cp-mode-ui-http-api/contracts/http-apis.md`

## Route：`/:deviceId/cc`（existing; extended with CP）

- 范围（Scope）: internal
- 变更（Change）: Modify

### Params

- `deviceId: string`（与现有设备路由一致）

### Data dependencies

- `GET /api/v1/identity`
- `GET /api/v1/control`
- `GET /api/v1/status`（可选：用于功率/电压/电流摘要）
- `GET /api/v1/presets` / `PUT /api/v1/presets` / `POST /api/v1/presets/apply`（写路径，若选择 preset 模型）

### States

- `Unsupported`：`cp_supported=false`
- `LinkDownLike`：`HttpApiError` 为 `LINK_DOWN|UNAVAILABLE` 等
- `Ready`：可读写（具备必要依赖数据）

## Component：`DeviceCcRoute`（existing; extended）

- 范围（Scope）: internal
- 变更（Change）: Modify

### Responsibilities

- 负责数据加载（query/loader）、错误态渲染与整体布局（标题/导航），并在 preset editor 中支持 `mode="cp"`。

### Child components（建议）

- `CpPresetEditor`：目标功率与限值编辑
- `CpStatusSummary`：功率/电压/电流/模式摘要

## Component：`CpPresetEditor`

- 范围（Scope）: internal
- 变更（Change）: New

### Props（建议）

- `presetId: number`
- `value: { mode: \"cp\"; target_p_mw: number; max_p_mw: number; max_i_ma_total: number; min_v_mv: number }`（单位：`target_p_mw/max_p_mw` 为 mW）
- `onChange(next): void`
- `onSave(): Promise<void>`
- `onApply(): Promise<void>`（若提供 apply 按钮）

### Validation

- `target_p_mw <= max_p_mw`（越界应禁用保存/或显示错误）

## Component：`CpStatusSummary`

- 范围（Scope）: internal
- 变更（Change）: New

### Display fields（建议）

- 当前模式（badge：CC/CV/CP/CR）
- 目标功率（W）与实际功率（W）
- 电压（V）与电流（A）
