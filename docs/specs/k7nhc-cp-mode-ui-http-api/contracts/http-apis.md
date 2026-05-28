# HTTP API（CP 模式相关增量）

本文件只描述本计划新增/修改的 HTTP API 契约；错误格式与通用约定沿用 `docs/interfaces/network-http-api.md`。

## `GET /api/v1/identity`

- 范围（Scope）: external
- 变更（Change）: Modify
- 鉴权（Auth）: none（局域网直连）

### 响应（Response）

- Success（200）：在 `capabilities` 中声明 CP 支持：

```jsonc
{
  "capabilities": {
    "cp_supported": true
  }
}
```

### 兼容性与迁移（Compatibility / migration）

- `cp_supported` 由 `false -> true`；旧 Web 客户端应忽略未知/不关心字段。

## `GET /api/v1/status`（snapshot / SSE）

- 范围（Scope）: external
- 变更（Change）: Modify
- 鉴权（Auth）: none

### 响应（Response）

在现有 `FastStatusView` 基础上，新增 `state_flags_decoded` 字段（数组），用于解释状态位并支撑 UI/自动化诊断。

示例（仅示意新增字段）：

```jsonc
{
  "state_flags_decoded": ["LINK_GOOD", "ENABLED", "POWER_LIMITED"]
}
```

允许值（大小写固定）：

- `REMOTE_ACTIVE`
- `LINK_GOOD`
- `ENABLED`
- `UV_LATCHED`
- `POWER_LIMITED`
- `CURRENT_LIMITED`

### 兼容性与迁移（Compatibility / migration）

- 新增字段：旧客户端可忽略；新客户端优先使用该字段解释 “功率达不到/被限流” 等状态。

## `GET /api/v1/presets`

- 范围（Scope）: external
- 变更（Change）: Modify
- 鉴权（Auth）: none

### 响应（Response）

- Success（200）：`Preset.mode` 扩展 `"cp"`，并新增 `target_p_mw`（mW）字段：

```jsonc
{
  "presets": [
    {
      "preset_id": 1,
      "mode": "cp",
      "target_p_mw": 45000,
      "target_i_ma": 0,
      "target_v_mv": 12000,
      "min_v_mv": 0,
      "max_i_ma_total": 10000,
      "max_p_mw": 150000
    }
  ]
}
```

### 兼容性与迁移（Compatibility / migration）

- 新字段 `target_p_mw`：
  - 旧客户端忽略即可；
  - 当 `mode!="cp"` 时，`target_p_mw` 仍可返回（用于稳定 schema），但实现侧可选择固定为 `0`。
- `mode="cp"` 对旧客户端而言属于未知枚举：旧客户端应按既有策略报 “MODE_UNSUPPORTED” 或隐藏该 preset（由 Web 实现决定）。

## `PUT /api/v1/presets`

- 范围（Scope）: external
- 变更（Change）: Modify
- 鉴权（Auth）: none

### 请求（Request）

当 `mode="cp"` 时，必须提供 `target_p_mw`（mW）：

```jsonc
{
  "preset_id": 1,
  "mode": "cp",
  "target_p_mw": 45000,
  "target_i_ma": 0,
  "target_v_mv": 12000,
  "min_v_mv": 0,
  "max_i_ma_total": 10000,
  "max_p_mw": 150000
}
```

### 响应（Response）

- Success（200）：返回更新后的 `Preset`（同请求结构）。

### 错误（Errors）

- `400 INVALID_REQUEST`：字段缺失/类型错误（例如 `mode="cp"` 缺少 `target_p_mw`）。
- `422 LIMIT_VIOLATION`：越界/不满足安全约束，例如：
  - `target_p_mw > max_p_mw`；
  - `target_p_mw` 超出硬件允许范围（由固件定义）。
- `409 CONFLICT`：EEPROM 忙/写保护/其它互斥状态（如适用）。
- `503 UNAVAILABLE`：EEPROM 写入失败或网络栈未就绪（沿用既有约定）。

## `GET /api/v1/control`

- 范围（Scope）: external
- 变更（Change）: Modify
- 鉴权（Auth）: none

### 响应（Response）

- Success（200）：当 active preset 的 `mode="cp"` 时，`preset.target_p_mw` 为必有字段：

```jsonc
{
  "active_preset_id": 1,
  "output_enabled": false,
  "uv_latched": false,
  "preset": {
    "preset_id": 1,
    "mode": "cp",
    "target_p_mw": 45000,
    "target_i_ma": 0,
    "target_v_mv": 12000,
    "min_v_mv": 0,
    "max_i_ma_total": 10000,
    "max_p_mw": 150000
  }
}
```

### 兼容性与迁移（Compatibility / migration）

- `ControlView` 的 `preset` schema 与 `/api/v1/presets` 同步扩展。
