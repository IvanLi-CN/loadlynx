# LoadLynx 网络 HTTP API 协议约定（v1）

本文档规范 ESP32‑S3 数字板暴露给 Web 客户端的 HTTP API，包括：

- 统一的请求/响应格式；
- 各端点 URL、方法与请求/响应字段；
- 域内枚举常量（状态/错误码等）。

本协议面向局域网环境下的 Web 控制台，默认由新建的 `web/` 前端子项目通过浏览器调用。

## 1. 通用约定

### 1.1 传输与基础路径

- 协议：`HTTP/1.1`
- 默认端口：固件内部可配置，建议默认 `80` 或 `8080`。
- 基础路径前缀：`/api/v1`
  - 所有端点均以此为前缀，例如：`GET /api/v1/identity`。

### 1.2 编码与媒体类型

- 请求体与响应体：`JSON`，编码为 `UTF-8`。
- `Content-Type`：
  - 请求：`application/json; charset=utf-8`（无请求体的端点可省略）。
  - 响应：`application/json; charset=utf-8`。

### 1.3 成功与错误响应统一格式

- 成功（HTTP 2xx）：
  - 直接返回资源对象，不额外包一层 `data`。
- 错误（HTTP 4xx/5xx）：
  - 统一返回 `ErrorResponse`：

```jsonc
{
  "error": {
    "code": "LINK_DOWN",
    "message": "UART link is down (no frames for 850 ms)",
    "retryable": true,
    "details": {
      "last_frame_age_ms": 850
    }
  }
}
```

#### 1.3.1 ErrorCode 枚举

`error.code` 统一使用字符串枚举，区分大小写，建议值：

- `INVALID_REQUEST`：JSON 解析失败、字段缺失/类型错误；
- `UNSUPPORTED_OPERATION`：端点存在，但当前固件不支持该操作；
- `LINK_DOWN`：数字板与模拟板 UART 链路不通或不健康；
- `ANALOG_FAULTED`：模拟板处于故障状态（如过流/过温），禁止执行相关操作；
- `ANALOG_NOT_READY`：模拟板未完成标定/上电初始化（CalMissing 等）；
- `LIMIT_VIOLATION`：请求参数超出安全软限值（电流/功率/电压等）；
- `MODE_UNSUPPORTED`：当前固件不支持请求中的模式/枚举值；
- `CONFLICT`：状态冲突（例如正在进行其他关键操作）；
- `RATE_LIMITED`：调用频率超过固件设定的安全阈值；
- `INTERNAL_ERROR`：未预期的内部错误；
- `UNAVAILABLE`：服务暂时不可用（Wi‑Fi 未连接、网络栈未就绪等）。

`error.retryable` 表示在不改变请求参数的前提下，重试是否有意义：

- 对于 `LINK_DOWN`、`ANALOG_NOT_READY`、`UNAVAILABLE`、`RATE_LIMITED` 等，多数情况下为 `true`；
- 对 `INVALID_REQUEST`、`LIMIT_VIOLATION`、`MODE_UNSUPPORTED` 等为 `false`。

#### 1.3.2 HTTP 状态码映射建议

| HTTP 状态 | 典型 ErrorCode          | 说明                           |
| --------- | ----------------------- | ------------------------------ |
| 200       | —                       | 成功，返回资源对象             |
| 202       | —                       | 已接受，异步操作（当前未使用） |
| 204       | —                       | 成功，无返回体                 |
| 400       | `INVALID_REQUEST`       | JSON 或字段非法                |
| 404       | `UNSUPPORTED_OPERATION` | 端点不存在或关掉               |
| 409       | `CONFLICT`              | 状态冲突                       |
| 422       | `LIMIT_VIOLATION`       | 参数超出安全可接受范围         |
| 429       | `RATE_LIMITED`          | 被限流                         |
| 503       | `LINK_DOWN`/`UNAVAILABLE` | 链路/服务不可用             |
| 500       | `INTERNAL_ERROR`        | 未分类固件内部错误             |

## 2. 类型与枚举定义

以下为 HTTP API 层暴露的 JSON 结构与枚举，不直接暴露二进制 UART 帧细节；底层已经由 `loadlynx-protocol` 封装。

### 2.1 设备与网络类型

```ts
type DeviceId = string; // 建议短 ID，如 "llx-1a2b3c"，来源于 MAC 等

interface NetworkInfo {
  ip: string;        // IPv4 字符串，如 "192.168.1.100"
  mac: string;       // MAC 地址，如 "aa:bb:cc:dd:ee:ff"
  hostname: string;  // 设备主机名
}

interface DeviceCapabilities {
  cc_supported: boolean;
  cv_supported: boolean;
  cp_supported: boolean;
  presets_supported: boolean;
  preset_count: number; // 固定为 5
  api_version: string; // HTTP API 语义版本，如 "2.0.0"
}
```

### 2.2 模拟板状态枚举

HTTP API 暴露的 `analog_state` 与数字板内部 `AnalogState` 枚举对应（`firmware/digital/src/ui/mod.rs`）：

```ts
type AnalogState =
  | "offline"     // 无 UART 链路或长期未收到帧
  | "cal_missing" // 模拟板未收到标定数据，禁止出力
  | "faulted"     // 模拟板处于故障状态（过流/过温/过压等）
  | "ready";      // 模拟板就绪，可正常工作
```

### 2.3 CC 控制相关类型

```ts
type CcProtectionMode =
  | "off"      // 关闭该维度保护（仅监控）
  | "protect"  // 保护模式：触发即强制降流/断开
  | "maintain"; // 维持模式：偏向保持输出，策略由固件定义

interface CcLimitProfile {
  max_i_ma: number;            // 软件电流上限（mA）
  max_p_mw: number;            // 软件功率上限（mW）
  ovp_mv: number;              // 软过压阈值（mV）
  temp_trip_mc: number;        // 软过温阈值（m°C）
  thermal_derate_pct: number;  // 热降额百分比（0–100）
}

interface CcProtectionConfig {
  voltage_mode: CcProtectionMode; // 电压维度保护策略
  power_mode: CcProtectionMode;   // 功率维度保护策略
}

interface CcControlView {
  // 注意：从 capabilities.api_version="2.0.0" 起，enable/target_i_ma 的语义为
  // “负载开关 + 设置值”模型，不再表示模拟板的 SetEnable 状态。
  enable: boolean;          // 负载开关（load switch）：true=应用设置值，false=生效值为 0
  target_i_ma: number;      // 设置值（setpoint，mA），UI 展示值；enable=false 时也允许为非 0
  effective_i_ma: number;   // 生效值（effective，mA），实际下发 SetPoint.target_i_ma
  limit_profile: CcLimitProfile;
  protection: CcProtectionConfig;

  // 派生/实时状态（来自 FastStatus）
  i_total_ma: number;       // 实际总电流（mA）
  v_main_mv: number;        // 主电压（mV，具体选远端/本地见实现）
  p_main_mw: number;        // 实际功率（mW）
}
```

### 2.4 FastStatus 映射（简化）

`loadlynx-protocol::FastStatus` 会被映射为 JSON 对象，字段含义与协议 crate 保持一致（见 `libs/protocol/src/lib.rs`）：

```ts
interface FastStatusJson {
  uptime_ms: number;
  mode: number;             // 当前工作模式（1=CC, 2=CV, 3=CP；其他值保留）
  state_flags: number;      // 状态位掩码（uint32）
  enable: boolean;          // 来自模拟板 FastStatus；与 /api/v1/cc 中的 enable（负载开关）不是同一概念
  target_value: number;     // 目标总电流（mA）
  i_local_ma: number;
  i_remote_ma: number;
  v_local_mv: number;
  v_remote_mv: number;
  calc_p_mw: number;
  dac_headroom_mv: number;
  loop_error: number;
  sink_core_temp_mc: number;
  sink_exhaust_temp_mc: number;
  mcu_temp_mc: number;
  fault_flags: number;      // 故障位掩码（uint32）

  // Optional raw calibration telemetry (present only in calibration mode).
  cal_kind?: number;        // 1=voltage, 2=current_ch1, 3=current_ch2
  raw_v_nr_100uv?: number;  // near-end ADC pin voltage, 100 µV/LSB (i16)
  raw_v_rmt_100uv?: number; // remote ADC pin voltage, 100 µV/LSB (i16)
  raw_cur_100uv?: number;   // current-sense ADC pin voltage, 100 µV/LSB (i16)
  raw_dac_code?: number;    // DAC code for selected channel (u16)
}

type FaultFlag =
  | "OVERCURRENT"
  | "OVERVOLTAGE"
  | "MCU_OVER_TEMP"
  | "SINK_OVER_TEMP";

type StateFlag =
  | "REMOTE_ACTIVE"
  | "LINK_GOOD"
  | "ENABLED"
  | "UV_LATCHED"
  | "POWER_LIMITED"
  | "CURRENT_LIMITED";

interface FastStatusView {
  raw: FastStatusJson;
  link_up: boolean;          // 数字板根据 LAST_GOOD_FRAME_MS 推导
  hello_seen: boolean;       // 是否收到 HELLO
  analog_state: AnalogState; // 映射自数字板内部状态机
  fault_flags_decoded: FaultFlag[]; // 从 fault_flags 位掩码解码出的列表
  state_flags_decoded: StateFlag[]; // 从 state_flags 位掩码解码出的列表
}
```

### 2.5 Preset/Control（v1 冻结）

```ts
type LoadMode = "cc" | "cv" | "cp";

interface Preset {
  preset_id: number;        // 1..=5
  mode: LoadMode;

  // Targets (units fixed; unused field still present for wire stability).
  target_p_mw: number;      // mW (used when mode="cp")
  target_i_ma: number;      // mA (used when mode="cc")
  target_v_mv: number;      // mV (used when mode="cv")

  // Limits
  min_v_mv: number;         // mV (undervoltage threshold for uv latch)
  max_i_ma_total: number;   // mA (total current limit)
  max_p_mw: number;         // mW (power limit)
}

interface ControlView {
  active_preset_id: number; // 1..=5
  output_enabled: boolean;  // user output switch
  uv_latched: boolean;      // undervoltage latched (clears on off->on edge)
  preset: Preset;           // snapshot of the active preset contents
}
```

## 3. API 端点定义

### 3.1 `GET /api/v1/identity`

读取设备标识与基础信息。

- 请求：无请求体。
- 响应（200）：

```jsonc
{
  "device_id": "llx-1a2b3c",
  "digital_fw_version": "digital 0.1.0 (profile release, v0.1.0-5-gf0393b8, src 0x1234567890abcdef)",
  "analog_fw_version": "analog 0.1.0 (profile release, v0.1.0-3-gdeadbeef, src 0xabcdef0123456789)",
  "protocol_version": 1,
  "uptime_ms": 123456,
  "network": {
    "ip": "192.168.1.100",
    "mac": "aa:bb:cc:dd:ee:ff",
    "hostname": "loadlynx-01"
  },
  "capabilities": {
    "cc_supported": true,
    "cv_supported": true,
    "cp_supported": true,
    "presets_supported": true,
    "preset_count": 5,
    "api_version": "2.0.0"
  }
}
```

- 错误：
  - 若网络栈未就绪，可返回 `503 UNAVAILABLE`。

### 3.2 `GET /api/v1/status`

读取当前遥测与链路状态，支持 **一次性快照** 与 **SSE 流式** 两种模式。

#### 3.2.1 一次性快照（JSON）

- 请求：
  - 方法：`GET`
  - 头部：`Accept: application/json`（或缺省）
  - 无请求体。
- 响应（200）：

```jsonc
{
  "status": {
    "uptime_ms": 123456,
    "mode": 0,
    "state_flags": 3,
    "enable": true,
    "target_value": 1500,
    "i_local_ma": 1400,
    "i_remote_ma": 100,
    "v_local_mv": 12000,
    "v_remote_mv": 11950,
    "calc_p_mw": 180000,
    "dac_headroom_mv": 500,
    "loop_error": 10,
    "sink_core_temp_mc": 45000,
    "sink_exhaust_temp_mc": 42000,
    "mcu_temp_mc": 40000,
    "fault_flags": 0
  },
  "link_up": true,
  "hello_seen": true,
  "analog_state": "ready",
  "fault_flags_decoded": [],
  "state_flags_decoded": ["REMOTE_ACTIVE", "LINK_GOOD"]
}
```

- 错误：
  - 若 UART 链路长时间无数据，可返回 `503 LINK_DOWN`，并在 `details` 中给出 `last_frame_age_ms`。

#### 3.2.2 流式模式（SSE）

- 适用场景：需要较高刷新率（建议 2–10 Hz）的实时状态展示。
- 请求：
  - 方法：`GET`
  - 头部：`Accept: text/event-stream`
  - 无请求体。
- 响应头：
  - `HTTP/1.1 200 OK`
  - `Content-Type: text/event-stream`
  - `Cache-Control: no-cache`
  - `Connection: keep-alive`
- 事件格式（示例）：

```text
event: status
data: {"status":{"uptime_ms":123456,"mode":0,...},"link_up":true,"hello_seen":true,"analog_state":"ready","fault_flags_decoded":[],"state_flags_decoded":["REMOTE_ACTIVE","LINK_GOOD"]}

```

- 刷新频率与资源限制：
  - 内部 UART FastStatus 仍按设计以约 20 Hz 更新缓存；
  - SSE 端每次仅在有新状态且达到最小发送间隔时推送，建议上限 5–10 Hz；
  - 固件可限制同时存在的 SSE 连接数量（例如最多 1–2 条），超出时返回 `429 RATE_LIMITED` 或拒绝新连接。
- 链路异常：
  - 当 UART 链路判定为断开或不健康时，可以：
    - 继续发送 `event: status`，其中 `link_up=false`、`analog_state="offline"`；
    - 或发送一次 `event: error` 后关闭连接，由前端负责重连。

#### 3.2.3 校准 Raw 附加（按 Tab 选择性上报）

为减轻链路压力，Raw ADC/DAC 字段**只在校准模式出现**，且按校准类型选择性附加：

- Web 进入/切换校准 Tab 时先调用 `POST /api/v1/calibration/mode` 选择 `kind`；
- ESP 将该 `kind` 转发为 UART `CalMode(0x25)` 给 G431；
- G431 在 FastStatus 中按 `kind` 附加 Raw 字段：
  - `kind="voltage"`：附加 `raw_v_nr_100uv`、`raw_v_rmt_100uv`；
  - `kind="current_ch1"` 或 `"current_ch2"`：附加 `raw_cur_100uv`、`raw_dac_code`（仅当前通道）；
  - `kind="off"`：不附加任何 Raw 字段。

Raw 字段单位：`*_100uv` 为 ADC 引脚电压（100 µV/LSB 的 i16）；`raw_dac_code` 为 DAC 码（u16）。

#### 3.2.4 `POST /api/v1/calibration/mode`

选择 STM32 的 Raw 遥测模式，仅在校准界面使用。

- 请求：

```jsonc
{ "kind": "off" | "voltage" | "current_ch1" | "current_ch2" }
```

- 响应（200）：无返回体或返回当前 kind。

- 错误：
  - UART 链路不可用 → `503 LINK_DOWN`；
  - 模拟板故障 → `503 ANALOG_FAULTED`。

### 3.3 `GET /api/v1/cc`

读取当前 CC 控制视图。

- 请求：无请求体。
- 响应（200）：

```jsonc
{
  "enable": false,
  "target_i_ma": 1500,
  "effective_i_ma": 0,
  "limit_profile": {
    "max_i_ma": 5000,
    "max_p_mw": 60000,
    "ovp_mv": 40000,
    "temp_trip_mc": 80000,
    "thermal_derate_pct": 100
  },
  "protection": {
    "voltage_mode": "protect",
    "power_mode": "protect"
  },
  "i_total_ma": 1480,
  "v_main_mv": 11980,
  "p_main_mw": 177000
}
```

- 错误：
  - `503 LINK_DOWN`：链路不可用；
  - `503 UNAVAILABLE`：Wi‑Fi/网络栈未就绪；
  - `500 INTERNAL_ERROR`：内部状态读取失败。

### 3.4 `PUT /api/v1/cc`

更新 CC 控制参数（负载开关、设置值及软限值配置）。

- 请求：

```jsonc
{
  "enable": true,
  "target_i_ma": 1500,
  "max_i_ma": 5000,     // 可选：覆盖当前 max_i_ma
  "max_p_mw": 60000,    // 可选
  "ovp_mv": 40000,      // 可选
  "temp_trip_mc": 80000,// 可选
  "thermal_derate_pct": 100, // 可选
  "voltage_mode": "protect", // 可选
  "power_mode": "protect"    // 可选
}
```

字段策略：

- `enable` 与 `target_i_ma` 为必填；
- `enable` 表示“负载开关”，`target_i_ma` 表示“设置值”；二者语义从 `api_version="2.0.0"` 起生效；
- **强制规则（A）**：当 `target_i_ma == 0` 时，服务端必须将 `enable` 纠正为 `false`（即使请求传入 `true`）；
- 软限值与保护模式字段为可选，若省略则保持当前固件中已有配置；
- 固件需对范围进行安全检查，例如：
  - `0 <= target_i_ma <= max_i_ma`；
  - `0 < max_i_ma <= 硬件额定值`；
  - `0 < max_p_mw <= 硬件额定功率`；
  - `0 < ovp_mv <= 安全上限电压`。

- 响应（200）：
  - 返回更新后的完整 `CcControlView`（同 `GET /api/v1/cc`，包含 `effective_i_ma`）。

- 典型错误：
  - `400 INVALID_REQUEST`：JSON 无法解析或类型错误；
  - `422 LIMIT_VIOLATION`：目标值/限值超出允许范围；
  - `503 LINK_DOWN`：链路未就绪；
  - `409 ANALOG_FAULTED`：模拟板当前处于故障状态；
  - `409 ANALOG_NOT_READY`：模拟板未标定或未完成上电流程。

### 3.5 `GET /api/v1/pd`

读取 USB‑PD 连接状态、当前合同（Active contract）、Source 能力列表（Fixed/PPS）以及数字板保存的 PD 配置。

- 请求：无请求体。
- 响应（200）：

```jsonc
{
  "attached": true,
  "contract_mv": 9000,
  "contract_ma": 2000,
  "fixed_pdos": [
    { "pos": 1, "mv": 5000, "max_ma": 3000 },
    { "pos": 4, "mv": 20000, "max_ma": 5000 }
  ],
  "pps_pdos": [
    { "pos": 2, "min_mv": 3300, "max_mv": 21000, "max_ma": 5000 }
  ],
  "saved": {
    "mode": "fixed",
    "fixed_object_pos": 4,
    "pps_object_pos": 2,
    "target_mv": 9000,
    "i_req_ma": 2000
  },
  "apply": {
    "pending": false,
    "last": { "code": "ok", "at_ms": 123456 }
  }
}
```

字段说明：

- `fixed_pdos[].pos` 与 `pps_pdos[].pos` 均为 **object position（1-based）**；若模拟板能力列表未携带 `pos`（旧格式），数字板以列表索引 `idx+1` 生成稳定的 `pos`。
- `saved.target_mv` 在 `mode="pps"` 时表示 PPS 目标电压（mV）；在 `mode="fixed"` 时不参与协商（Fixed 的电压由所选 PDO 决定）。
- `contract_mv` / `contract_ma` 在 `attached=false`（或合同未知）时为 `null`。

可用性约定（重要）：

- `GET /api/v1/pd` **不应因为 PD 未 attach 或状态尚未就绪而返回错误**。
  - 当尚未收到 `PD_STATUS`（或 PD 未 attach）时，固件应返回 `200`，并设置：
    - `attached=false`
    - `contract_mv=null`、`contract_ma=null`
    - `fixed_pdos=[]`、`pps_pdos=[]`（若暂无 Source 能力）
  - 这样 Web UI 可以稳定显示 “DETACHED/未知能力”，而不是把“未插 PD 电源”误报成设备异常。

- 错误：
  - `503 LINK_DOWN`：UART 链路不可用；
  - `409 ANALOG_FAULTED`：模拟板故障。

### 3.6 `POST /api/v1/pd`

更新并应用（Apply）USB‑PD 配置；固件会把配置写入 EEPROM，并触发数字板→模拟板的 `PD_SINK_REQUEST`（ACK/NACK 仅表示“接收/拒绝策略”，最终合同以 `contract_*` 为准）。

兼容性约定：

- 为避免浏览器私网预检（preflight），Web 端可使用 `POST` 且 `Content-Type: text/plain`，body 为 JSON 字符串；
- 固件不强制校验 `Content-Type`，只解析 body 内容。

- 请求（Fixed）：

```jsonc
{ "mode": "fixed", "object_pos": 4, "i_req_ma": 3000 }
```

- 请求（PPS）：

```jsonc
{ "mode": "pps", "object_pos": 2, "target_mv": 9000, "i_req_ma": 2000 }
```

- 响应（200）：返回更新后的 `GET /api/v1/pd` 视图。

- 典型错误：
  - `400 INVALID_REQUEST`：字段缺失/类型错误（例如 PPS 缺少 `target_mv`）；
  - `409 NOT_ATTACHED`：PD 未 attach（或 PD 状态不可用），拒绝 apply；
  - `422 LIMIT_VIOLATION`：`object_pos` 不存在于当前能力列表、`target_mv` 越界、或 `i_req_ma` 超过 Imax；
  - `503 LINK_DOWN`：UART 链路不可用；
  - `503 UNAVAILABLE`：EEPROM 写入失败。

### 3.7 `POST /api/v1/soft-reset`（预留）

触发数字板→模拟板的 SoftReset 握手，清除故障并恢复安全状态。

- 请求：

```jsonc
{
  "reason": "manual"
}
```

`reason` 建议枚举：

- `"manual"`：用户手动触发；
- `"firmware_update"`：固件升级后恢复；
- `"ui_recover"`：UI 异常恢复；
- `"link_recover"`：链路异常恢复。

可与 `loadlynx-protocol::SoftResetReason` 对应。

- 响应（200）：

```jsonc
{
  "accepted": true,
  "reason": "manual"
}
```

- 典型错误：
  - `503 LINK_DOWN`：链路不可用；
  - `503 UNAVAILABLE`：Wi‑Fi/网络未就绪；
  - `409 CONFLICT`：当前已有 SoftReset 操作进行中（若固件实现互斥）。

### 3.6 `GET /api/v1/presets`（冻结）

读取 5 组 Preset（必须始终返回 **恰好 5 条**，按 `preset_id` 1..5 排序）。

- 请求：无请求体。
- 响应（200）：

```jsonc
{
  "presets": [
    {
      "preset_id": 1,
      "mode": "cc",
      "target_p_mw": 0,
      "target_i_ma": 1500,
      "target_v_mv": 12000,
      "min_v_mv": 0,
      "max_i_ma_total": 10000,
      "max_p_mw": 150000
    }
    // ... preset_id=2..5
  ]
}
```

### 3.7 `PUT /api/v1/presets`（冻结）

更新单个 Preset。请求体必须包含完整 Preset payload（包括 `preset_id`），固件需对范围做校验；对 CP 关键字段越界应返回 `422 LIMIT_VIOLATION`（不应静默夹紧目标功率）。

- 请求：

```jsonc
{
  "preset_id": 3,
  "mode": "cv",
  "target_p_mw": 0,
  "target_i_ma": 1500,
  "target_v_mv": 12000,
  "min_v_mv": 0,
  "max_i_ma_total": 10000,
  "max_p_mw": 150000
}
```

- 响应（200）：返回更新后的 Preset（同请求结构）。

- 约束（CP）：
  - 当 `mode="cp"` 时必须提供 `target_p_mw`（mW），且满足 `target_p_mw <= max_p_mw`；不满足则返回 `422 LIMIT_VIOLATION`。

### 3.8 `POST /api/v1/presets/apply`（冻结）

应用指定 `preset_id` 作为 active preset，并 **必须强制输出关闭**（`output_enabled=false`），用户需后续通过 `/api/v1/control` 手动开启输出。

- 请求：

```jsonc
{ "preset_id": 3 }
```

- 响应（200）：返回更新后的 `ControlView`。

### 3.9 `GET /api/v1/control`（冻结）

读取统一的控制视图（active preset + 输出开关 + 欠压锁存）。

- 请求：无请求体。
- 响应（200）：`ControlView`。

### 3.10 `PUT /api/v1/control`（冻结）

切换输出开关。

- 请求：

```jsonc
{ "output_enabled": true }
```

- 语义（冻结）：
  - 仅切换输出开关，不修改 preset 内容；
  - `uv_latched` **仅能通过** `output_enabled` 的 “关→开” 边沿清除（即必须先 `false` 再 `true`）。
- 响应（200）：`ControlView`。

## 4. 错误码一览表（建议实现）

| ErrorCode           | 含义                                           |
| ------------------- | ---------------------------------------------- |
| `INVALID_REQUEST`   | 请求体格式错误或字段非法                       |
| `UNSUPPORTED_OPERATION` | 端点不存在或当前固件禁用该操作           |
| `LINK_DOWN`         | UART 链路不可用或状态不健康                   |
| `ANALOG_FAULTED`    | 模拟板处于故障状态，禁止执行相关操作         |
| `ANALOG_NOT_READY`  | 模拟板未完成标定/上电初始化                   |
| `LIMIT_VIOLATION`   | 参数超出软限值/安全范围                       |
| `MODE_UNSUPPORTED`  | 请求中的模式/枚举值当前固件不支持             |
| `CONFLICT`          | 当前状态与请求操作冲突                         |
| `RATE_LIMITED`      | 调用超过固件设定的频率限制                     |
| `UNAVAILABLE`       | Wi‑Fi/网络服务未就绪                           |
| `INTERNAL_ERROR`    | 固件内部未预期错误                             |

## 5. 兼容性与演进

- API 版本号：
  - URL 路径中固定使用 `/api/v1`；
  - 细粒度版本信息由 `identity.capabilities.api_version` 提供（如 `"2.0.0"`）。
- 向后兼容策略：
  - 新增字段应保证前端在忽略该字段时仍能正常工作；
  - 新增端点不得改变现有端点语义；
  - 删除或重大变更现有端点/字段时，需通过升级 `api_version` 并在前端做兼容处理。

### 5.1 `api_version` 变更记录（与 `/api/v1/cc` 相关）

- `1.x`（历史语义）
  - `enable=false` 等价于“将目标设为 0”（会覆盖用户设置值）；
  - `target_i_ma` 表示当前目标（更接近“生效值”）。
- `2.0.0`（本文档语义）
  - `enable` 表示“负载开关”（load switch），关闭时不清空设置值；
  - `target_i_ma` 表示“设置值”（setpoint）；
  - 新增 `effective_i_ma` 表示“生效值”（effective）；
  - `target_i_ma==0` 时强制 `enable=false`（A 规则）。

前端应以“宽松读取、严格发送”为原则：读取时容忍多余字段，发送时遵循本文档定义的字段与枚举，避免给固件引入不必要的不确定性。
