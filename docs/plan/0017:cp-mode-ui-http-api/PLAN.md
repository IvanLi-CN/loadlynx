# CP 模式：本机屏幕界面 + HTTP API（#0017）

## 状态

- Status: 待实现
- Created: 2026-01-15
- Last: 2026-01-16

## 背景 / 问题陈述

- 当前系统已具备 CC/CV 的闭环与 UI/HTTP 控制能力，但缺少恒功率（CP, Constant Power）工作模式。
- 目标是在 **不依赖 Web UI** 的前提下，让用户可以通过本机屏幕/旋钮/触控与 HTTP API 使用 CP 模式，并为后续 Web UI（见 #0018）提供稳定接口。

## 目标 / 非目标

### Goals

- 新增 CP 工作模式：在允许出力且无故障的前提下，按目标功率调节等效电流，实现近似恒功率输出（受软/硬限值约束）。
- 本机屏幕 UI：
  - 支持选择 CP 模式，并编辑目标功率（单位 W，内部 mW）；
  - 与既有 CC/CV/preset 的交互规则一致（尤其是模式切换与出力安全规则）。
- HTTP API：
  - 支持读写 CP 所需字段（模式、目标功率、限值）；
  - `identity.capabilities.cp_supported=true`，并通过明确错误码处理不支持/越界/链路不可用等情况。
- UART 协议（digital↔analog）具备 CP 所需字段与枚举，且 FastStatus 能反映当前 mode。

### Non-goals

- 不实现 Web UI（另起计划：`docs/plan/0018:web-cp-mode-ui/PLAN.md`）。
- 不实现 CR（恒阻）或更复杂的功率曲线/脚本控制。
- 不引入账号体系/远程云访问；默认局域网直连设备。

## 用户与场景（Users & Scenarios）

- 只有硬件面板的场景：无浏览器/无网络时，通过屏幕 + 旋钮 + 触控完成 CP 模式启停与设定。
- 自动化/脚本场景：通过 HTTP API 在局域网内切换 CP 模式并设定目标功率，用于测试/验证电源能力与热设计。

## 需求（Requirements）

### MUST

- CP 模式基本语义：
  - 目标值 `target_p_mw` 为恒功率设定（mW），UI 显示为 W；
  - 目标功率编辑分辨率：**0.1 W**（即 `100 mW`）；UI 必须按该步进调整数值；
  - 目标功率可编辑范围：`0 <= target_p_mw <= max_p_mw`（`max_p_mw` 来自当前 preset 的功率上限；默认值见 `docs/plan/0002:cv-mode-presets/PLAN.md`，为 `150_000 mW`；当前固件硬上限参考 `LIMIT_PROFILE_DEFAULT.max_p_mw = 250_000 mW`）；
  - 当 `output_enabled=false` 或触发 safing（fault/uv latch/link down）时，等效输出必须为安全关闭态；
  - 在 `v_main_mv` 合理且未触发限值时，`calc_p_mw` 应在稳定时间窗口内收敛到 `target_p_mw` 附近（允差与窗口在“验收标准”冻结）。
- 限值与保护（与现有语义对齐）：
  - `max_i_ma_total` 作为电流上限（OCP/软件电流限值）。
  - `max_p_mw` 作为功率上限（OPP/软件功率限值），且必须满足 `target_p_mw <= max_p_mw`（否则请求无效）。
  - `min_v_mv`（UVLO/欠压阈值）仍按既有语义参与 gate；当触发欠压语义时必须执行“停机保护”（强制输出关闭），欠压锁存清除规则不变。
- 本机 UI：
  - 能切换到 CP 模式并编辑 `target_p_mw`；
  - 模式切换导致实际输出语义变化时，必须执行“安全关断输出”（避免静默切换继续出力）。
- HTTP API：
  - 允许读写包含 CP 模式所需字段的资源；
  - 对非法输入返回 `400 INVALID_REQUEST` 或 `422 LIMIT_VIOLATION`；
  - 对链路/就绪状态返回 `503 LINK_DOWN` / `409 ANALOG_NOT_READY` / `409 ANALOG_FAULTED` 等错误。
- 协议与遥测：
  - `LoadMode` 扩展 CP 枚举值，并在 `FastStatus.mode` 中可观察到（用于 UI badge 与诊断）。

### SHOULD

- 平滑性：电压抖动/测量噪声不应导致电流剧烈抖动（允许通过滤波/限速实现）。
- 可诊断性：在日志与/或 UI 状态行中能区分 “目标达到 / 受电流限值 / 受功率限值 / 欠压 gating” 等主要原因。

### COULD

- 增加一个只读摘要字段（例如 `cp_limited_reason`）用于 UI/HTTP 诊断（不影响核心闭环）。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `/api/v1/identity` | HTTP API | external | Modify | ./contracts/http-apis.md | digital | web / tools | `cp_supported=true` |
| `/api/v1/presets` | HTTP API | external | Modify | ./contracts/http-apis.md | digital | web / local ui | 扩展 `Preset` 支持 `cp` 与 `target_p_mw` |
| `/api/v1/control` | HTTP API | external | Modify | ./contracts/http-apis.md | digital | web / local ui | `ControlView.preset` 扩展同上 |
| `/api/v1/status` | HTTP API | external | Modify | ./contracts/http-apis.md | digital | web / local ui | 增加 `state_flags_decoded` 用于受限原因解释 |
| `MSG_SET_MODE(SetMode)` | RPC | internal | Modify | ./contracts/rpc.md | protocol | digital / analog | 增加 CP 模式与 `target_p_mw` |
| `FastStatus.mode` | RPC | internal | Modify | ./contracts/rpc.md | protocol | digital / web | 增加 CP mode 数值映射 |

### 契约文档（按 Kind 拆分）

- [contracts/README.md](./contracts/README.md)
- [contracts/http-apis.md](./contracts/http-apis.md)
- [contracts/rpc.md](./contracts/rpc.md)

## 验收标准（Acceptance Criteria）

### 本机 UI

- Given 设备处于 `analog_state="ready"` 且 `output_enabled=false`，
  When 用户在本机 UI 选择 CP 并设置 `target_p_mw`，
  Then UI 显示值与内部值一致（W↔mW 换算一致），且不会自动开启输出。

- Given `output_enabled=true` 且当前模式为 CC/CV，
  When 用户将模式切换为 CP，
  Then `output_enabled` 被强制置为 `false`（安全关断），并在 UI 上可见。

### 闭环语义

- Given `output_enabled=true` 且模拟板无故障、链路健康，
  When 进入 CP 模式并设置 `target_p_mw=T`（T 在硬件允许范围内），
  Then `calc_p_mw` 在稳定窗口内收敛到 `T±tol`（`tol` 与窗口长度见下方“验收档位（已选：B）”）。

- Given `target_p_mw` 对应电流需求超过 `max_i_ma_total`，
  When 设备处于 CP 模式且输出开启，
  Then 实际电流被限制在 `max_i_ma_total`，且 `calc_p_mw < target_p_mw`（不报错，但应可诊断为“受电流限值”）。

### HTTP API

- Given `GET /api/v1/control`，
  When active preset 的 `mode="cp"`，
  Then 响应中的 `preset` 包含 `target_p_mw` 且数值单位为 mW。

- Given `PUT /api/v1/presets`，
  When `mode="cp"` 且 `target_p_mw > max_p_mw`，
  Then 返回 `422 LIMIT_VIOLATION`（`retryable=false`）并附带可读 message。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Protocol: 在 `libs/protocol` 增加单元测试覆盖：
  - `LoadMode` 的 u8 映射（含 CP）；
  - `SetMode` 新字段的 encode/decode（含缺省兼容行为）。
- Firmware: 对关键纯逻辑（clamp/换算/状态机）尽量下沉到可测试模块（若已有对应模式则复用），并补齐单元测试（能跑在 host 上的部分）。

### Quality checks

- Rust format: `cargo fmt --all`（仓库既有）
- Lints: 仅在已有配置/目标可用处运行 `cargo clippy ...`（不引入新 lint 工具）

## 文档更新（Docs to Update）

- `docs/interfaces/network-http-api.md`: 更新 `LoadMode`、`Preset`、`ControlView` 与相关端点描述，补齐 CP 语义与示例。
- `docs/interfaces/network-control.md`: 若该文档仍作为控制接口参考，需要同步加入 CP（至少在“将来可扩展”处升级为明确契约/或链接到本计划 contracts）。
- `docs/plan/README.md`: 新增本计划索引行（已完成）。

## 里程碑（Milestones）

- [ ] M1: 冻结 CP 的 “数据模型 + 协议 + HTTP API” 契约（本计划 contracts + 开放问题闭环）
- [ ] M2: `loadlynx-protocol` 支持 CP（枚举 + SetMode 字段 + FastStatus mode 映射）并通过单测
- [ ] M3: analog：实现 CP 等效电流计算与限值 gating（含基本滤波/限速）
- [ ] M4: digital：本机 UI 支持 CP（模式选择 + 目标功率编辑 + 安全关断规则）
- [ ] M5: digital：HTTP API 支持 CP（/identity + /presets + /control）并与文档一致
- [ ] M6: HIL 验证（不同电压下功率维持、限流/欠压/故障路径）

## 方案概述（Approach, high-level）

- 以“目标功率 → 等效电流”作为 CP 的主要控制量：`I_target ≈ target_p_mw / V_meas`，再叠加既有 OCP/OPP/UVLO gating。
- `Preset`/`ControlView` 扩展保持字段稳定：新增 `target_p_mw`，未用字段仍保留以减少未来演进成本。
- UI 与 API 的错误/越界策略沿用既有约定：客户端尽量阻止越界，服务端越界返回明确错误（不静默夹紧关键目标值）。

## 风险与开放问题（Risks & Open Questions）

### 风险

- CP 的 `I = P/V` 在低电压区域会导致电流需求骤增，需要明确的 UVLO / OCP 策略与 UI 呈现，避免误解为“失控”。
- 测量噪声会导致电流抖动，需要滤波/限速，否则可能引发热与稳定性问题。
- 现有 “Preset/Control（v1 冻结）” 文档与实现需要同步演进，避免接口与 UI 口径分裂。

### 已冻结的决策（Freeze）

- CP **纳入 Preset**：`Preset.mode` 允许取 `cp`，并通过 “编辑 preset + apply” 路径生效。
- 目标功率编辑分辨率：`0.1 W`（`100 mW`）。
- 欠压策略：触发欠压语义时执行“停机保护”（强制输出关闭），并沿用既有欠压锁存清除规则。
- 功率达标验收真值：使用 `FastStatus.raw.calc_p_mw`（HTTP/本机 UI 均按此展示与判断）。
- HTTP 写路径：不新增 “立即应用 CP” 专用端点；沿用 `/api/v1/presets` + `/api/v1/presets/apply` + `/api/v1/control`。
- 可诊断字段：`/api/v1/status` 增加 `state_flags_decoded`，其中必须包含 `POWER_LIMITED` / `CURRENT_LIMITED`（用于 UI/自动化解释“为何功率达不到”）。
- 验收档位：**B（均衡）**（见下文“验收档位”）。

### 开放问题（需要主人决策；剩余）

None.

## 假设（Assumptions）

- 默认采用 `LoadMode` 数值映射：`CC=1`、`CV=2`、`CP=3`（若需其它值，请在开放问题中确认后再冻结）。

## 验收档位（已选：B）

说明：

- `FastStatus` 发送周期当前为 **20 Hz（50 ms）**，因此稳定窗口最小粒度建议以 `N` 个连续样本定义（`N = window / 50ms`）。
- 参考商用电子负载规格，CP 相关误差通常以 “`% of reading + absolute offset`” 的形式给出；我们用同样的双项形式定义 `tol = max(abs_w, rel_pct*T)`，便于覆盖低功率与高功率场景。

候选档位（任选其一冻结到“验收标准”）：

| 档位 | `tol` 定义 | `t_settle`（允许收敛时间） | `t_window`（稳定窗口） | 适用场景 |
| --- | --- | --- | --- | --- |
| A（严格） | `max(0.2 W, 2% * T)` | `<= 1.0 s` | `>= 0.5 s` | 自动化回归、对比测试、希望 UI 反馈更“跟手” |
| **B（已选）** | `max(0.2 W, 3% * T)` | `<= 1.5 s` | `>= 1.0 s` | 日常使用、HIL 验证、脚本控制（兼顾稳定与实现成本） |
| C（保守） | `max(0.5 W, 5% * T)` | `<= 2.0 s` | `>= 2.0 s` | 早期硬件/校准未完善、强调“可用优先”与误差容忍 |

参考（仅用于解释“为何用双项 tol 形式”）：

- Keysight EL30000 系列电子负载：CP 模式给出 “`% + absolute`” 的编程精度口径（例如高量程项含 `+ 1.6 W` 的绝对项）。
