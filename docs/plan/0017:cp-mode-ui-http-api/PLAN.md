# CP 模式：本机屏幕界面 + HTTP API（#0017）

## 状态

- Status: 部分完成（5/6）
- Created: 2026-01-15
- Last: 2026-01-18

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
  - 目标功率可编辑范围：`0 <= target_p_mw <= max_p_mw`（`max_p_mw` 来自当前 preset 的功率上限；默认值见 `docs/plan/0002:cv-mode-presets/PLAN.md`，为 `100_000 mW`；当前固件硬上限参考 `LIMIT_PROFILE_DEFAULT.max_p_mw = 100_000 mW`）；
  - 当 `output_enabled=false` 或触发 safing（fault/uv latch/link down）时，等效输出必须为安全关闭态；
  - 在 `v_main_mv` 合理且未触发限值时，稳态功率误差应满足本计划“CP 编程精度（规格书口径，冻结）”。
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
  Then 稳态功率误差满足下方“CP 编程精度（规格书口径，冻结）”。

- Given `output_enabled=true` 且模拟板无故障、链路健康，
  When 在 CP 模式下对 `target_p_mw` 做功率步进，
  Then 响应速度满足下方“CP 瞬态响应（规格书口径，冻结）”。

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

- [x] M1: 冻结 CP 的 “数据模型 + 协议 + HTTP API” 契约（本计划 contracts + 开放问题闭环）
- [x] M2: `loadlynx-protocol` 支持 CP（枚举 + SetMode 字段 + FastStatus mode 映射）并通过单测
- [x] M3: analog：实现 CP 等效电流计算与限值 gating（含基本滤波/限速）
- [x] M4: digital：本机 UI 支持 CP（模式选择 + 目标功率编辑 + 安全关断规则）
- [x] M5: digital：HTTP API 支持 CP（/identity + /presets + /control）并与文档一致
- [ ] M6: HIL 验证（不同电压下功率维持、限流/欠压/故障路径；并按本计划的“CP 编程精度/瞬态响应”口径复测）

### M6：HIL 验证记录

> 说明：本节记录端到端链路验证与观测结果；**验收以本计划的“CP 编程精度 / CP 瞬态响应（规格书口径，冻结）”为准**。其中编程精度的 `P` 以外部仪表 `V*I` 计算为准，`FastStatus.raw.calc_p_mw` 仅作为 UI/HTTP 展示读数。

- 设备与链路
  - digital HTTP：`loadlynx-d68638.local`（mDNS），IP `192.168.31.216`，端口 `80`
  - `just agentd-get-port analog`：`0d28:0204:2BDC77EE006DCE9B589D7AD8F22BD989`
  - `just agentd-get-port digital`：`/dev/cu.usbmodem412101`
- 刷写与启动
  - build：`just a-build`、`just d-build`
  - flash：`just agentd flash analog`、`just agentd flash digital`
  - `GET /api/v1/identity`：`capabilities.cp_supported=true`
- CP 写入链路（HTTP → digital → UART → analog）
  - `PUT /api/v1/presets`：`mode="cp"` + `target_p_mw=10000` 写入成功
  - `POST /api/v1/presets/apply`：active preset 切换成功（mode=cp）
  - analog 日志可见 `SetMode received: ... mode=Cp target_p=10000mW ...` 与 ACK 往返
- 不同电压下恒功率达标（已覆盖两档：5V / 12V）
  - 通过 `PUT /api/v1/pd` 切换 PD contract（fixed PDO），再运行 CP=10W：
    - 5V：`object_pos=1`，`i_req_ma=2500`，`v_local_mv≈4.96V`；CP=10W：`avg_p≈9.949W`，`max_err≈0.102W`（PASS）
    - 12V：`object_pos=3`，`i_req_ma=1500`，`v_local_mv≈12.50V`；CP=10W：`avg_p≈10.060W`，`max_err≈0.108W`（PASS）
- 限流路径（CURRENT_LIMITED）
  - CP=10W、12V 下，将 `max_i_ma_total` 限到 `200mA`：`calc_p_mw≈2.6W`，`state_flags_decoded` 含 `CURRENT_LIMITED`（预期）
- 欠压锁存路径（UV_LATCHED）
  - 在输出已开启时，将 preset 的 `min_v_mv` 调高到高于当前 `v_local_mv`（例如 13_000mV）：FastStatus 进入 `UV_LATCHED`，且 `enable=false`（effective output=0）
  - 将 `min_v_mv` 恢复到 `0` 后，通过输出 `OFF → ON`（enable 上升沿）可清除 `UV_LATCHED`
- 瞬态初测（内部 `cp_perf`，仅用于自测回归；不作为冻结口径的验收依据）
  - 条件：PD `20V/5A`，CP 输出开启，步进 `10W ↔ 90W`；采样：`FastStatus.raw.calc_p_mw` 的 1ms 任务采样（非示波器 `V(t)`/`I(t)`）
  - 控制环调度抖动（analog 日志 `control_loop dt_us`）：`avg≈1007us`（1kHz，max~1.1ms 量级）
  - 时间到容差（`±(0.5%reading + 0.5%FS)`，FS_H=100W/FS_L=10W；内部用 `calc_p_mw` 估算）
    - “首次进入容差”（`enter_tol(1)`）：
      - `10W → 90W`：约 `3ms`
      - `90W → 10W`：约 `2ms`
    - “连续 3 次采样都在容差内”（`enter_tol(3)`）：
      - `10W → 90W`：约 `5ms`
      - `90W → 10W`：约 `4ms`
  - 内部 quick_check（非验收）：
    - `enter_tol(3) <= 5ms`：当前固件在上述条件下可稳定 PASS（用于回归；不替代示波器/外部仪表）
  - 备注：冻结的瞬态指标 `t_10_90/t_90_10 <= 1ms` 需用示波器测得 `P(t)=V(t)*I(t)` 复测；上述自测口径只能作为链路与固件回归参考

## 方案概述（Approach, high-level）

- 以“目标功率 → 等效电流”作为 CP 的主要控制量：`I_target ≈ target_p_mw / V_meas`，再叠加既有 OCP/OPP/UVLO gating。
- `Preset`/`ControlView` 扩展保持字段稳定：新增 `target_p_mw`，未用字段仍保留以减少未来演进成本。
- UI 与 API 的错误/越界策略沿用既有约定：客户端尽量阻止越界，服务端越界返回明确错误（不静默夹紧关键目标值）。

## 风险与开放问题（Risks & Open Questions）

### 风险

- CP 的 `I = P/V` 在低电压区域会导致电流需求骤增，需要明确的 UVLO / OCP 策略与 UI 呈现，避免误解为“失控”。
- 测量噪声会导致电流抖动，需要滤波/限速，否则可能引发热与稳定性问题。
- 冻结的 `t_10_90/t_90_10 <= 1 ms` 属于“动态/瞬态”指标；实现上通常要求模拟板控制环更新频率显著高于 1 kHz 并重调滤波/限速，且不应与 HTTP/UART 的指令处理延迟混为同一指标。
- 现有 “Preset/Control（v1 冻结）” 文档与实现需要同步演进，避免接口与 UI 口径分裂。

### 已冻结的决策（Freeze）

- CP **纳入 Preset**：`Preset.mode` 允许取 `cp`，并通过 “编辑 preset + apply” 路径生效。
- 目标功率编辑分辨率：`0.1 W`（`100 mW`）。
- 欠压策略：触发欠压语义时执行“停机保护”（强制输出关闭），并沿用既有欠压锁存清除规则。
- UI/HTTP 的功率读数来源：使用 `FastStatus.raw.calc_p_mw`（HTTP/本机 UI 均按此展示）。
- HTTP 写路径：不新增 “立即应用 CP” 专用端点；沿用 `/api/v1/presets` + `/api/v1/presets/apply` + `/api/v1/control`。
- 可诊断字段：`/api/v1/status` 增加 `state_flags_decoded`，其中必须包含 `POWER_LIMITED` / `CURRENT_LIMITED`（用于 UI/自动化解释“为何功率达不到”）。
- 验收口径：对标 **Chroma 6310A** 的 CP 编程精度口径（见下文“CP 编程精度（规格书口径，冻结；对标：Chroma 6310A）”）。

### 开放问题（需要主人决策；剩余）

None.

## 假设（Assumptions）

- 默认采用 `LoadMode` 数值映射：`CC=1`、`CV=2`、`CP=3`（若需其它值，请在开放问题中确认后再冻结）。

## CP 编程精度（规格书口径，冻结；对标：Chroma 6310A）

说明：

- 参考商用电子负载规格，CP 相关误差通常以 “`% of reading + %FS` / `% + absolute offset`” 的形式给出；本计划选择对标 Chroma 6310A 的 `±(0.5% of reading + 0.5%FS)` 口径，便于与规格书直接对齐。

### 指标定义（Programming Accuracy）

本计划在 CP 模式下对标规格书中的“编程精度（Programming Accuracy）”指标，仅描述**稳态误差**，不在本计划中给出动态/带宽指标。

定义：

- 满量程功率（Full Scale, FS）：本项目功率上限固定为 `FS_H = 100 W`；为对标商用负载的“双量程”口径，同时定义低量程 `FS_L = 10 W`（即 10%FS）。
- 目标功率：`T`（单位 W；实现与遥测内部为 `target_p_mw`，单位 mW）。
- 被测功率：`P`（单位 W；用外部仪表测得 `V` 与 `I` 并计算 `P = V * I`；`FastStatus.raw.calc_p_mw` 仅作为 UI/HTTP 展示读数，不作为该指标的测量来源）。
- 容差（对标 Chroma 6310A：`±(0.5% of reading + 0.5%FS)`）：
  - 低量程（`0 < T <= FS_L`）：`tol_L(T) = 0.005 * T + 0.005 * FS_L`
  - 高量程（`FS_L < T <= FS_H`）：`tol_H(T) = 0.005 * T + 0.005 * FS_H`

### PASS 条件

- 在 CP 模式且输出已开启的稳态下：
  - 若 `T <= FS_L`：`|P - T| <= tol_L(T)`；
  - 若 `T > FS_L`：`|P - T| <= tol_H(T)`。

参考（仅用于解释“为何用双项 tol 形式”）：

- Keysight EL30000 系列电子负载：CP 模式给出 “`% + absolute`” 的编程精度口径（例如高量程项含 `+ 1.6 W` 的绝对项）。

## CP 瞬态响应（规格书口径，冻结）

说明：

- 商用电子负载对“速度”的规格通常以 **Slew rate（A/µs）**、**Transient response time（10%→90%）**（或 rise/fall time）等形式给出。
- 本计划冻结的是 CP 模式下“功率设定步进”的 **10%→90% / 90%→10% 瞬态响应时间**，不使用 UI/遥测的判稳窗口口径。

### 测试条件（Test Conditions）

- `FS = 100 W`。
- 固定输入电压：建议用 PD `20 V` contract（避免电流上限导致无法覆盖高功率区间；如无 PD，则使用可稳定输出 20 V 的电源）。
- 模式：CP；输出开启；无欠压/故障/降额；`max_i_ma_total` 与 `max_p_mw` 不应成为限制因素。

### 指标定义（Transient Response Time）

- 令功率设定发生一步阶跃：`T1 → T2`（建议取 `T1=0.1*FS`，`T2=0.9*FS`，并同时测试下降沿 `T2 → T1`）。
- 令 `P(t)` 表示被测功率随时间变化（用示波器同时测 `V(t)` 与 `I(t)` 并计算得到；**不使用** `FastStatus` 作为该指标的测量来源）。
- 上升沿响应时间 `t_10_90`：`P(t)` 从 `P1 + 0.1*(P2-P1)` 上升到 `P1 + 0.9*(P2-P1)` 的时间。
- 下降沿响应时间 `t_90_10`：`P(t)` 从 `P2 - 0.1*(P2-P1)` 下降到 `P2 - 0.9*(P2-P1)` 的时间。

### PASS 条件

- `t_10_90 <= 1 ms` 且 `t_90_10 <= 1 ms`。
