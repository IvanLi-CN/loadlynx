# USB-PD EPR 28V Sink Enablement（#t2f5j）

## 状态

- Status: 部分完成（3/4）
- Created: 2026-03-19
- Last: 2026-03-19
- Notes: fast-track；实现与构建已完成；HIL 受当前模拟板启动 panic 阻塞

## 背景 / 问题陈述

- 当前模拟板 USB-PD 依赖仍停留在 `usbpd 1.1.3` / `usbpd-traits 1.1.0`，数字侧模型与 EEPROM 也把 fixed 目标电压硬限制在 `<=21V`。
- 现有实现因此只能稳定支持 SPR fixed / SPR PPS；即使 Source 提供 EPR fixed 28V/36V/48V，协议与 UI 也无法完整识别、保存、选择和重协商。
- 主人要求为模拟板 USB PD 负载输入接口增加“28V 那种档位”的 sink，且先按标准 USB PD 3.1/3.2 EPR fixed 路径做端到端支持，而不是一次性特判。

## 目标 / 非目标

### Goals

- 升级模拟板 PD 依赖到 `usbpd 2.0.0` 与 `usbpd-traits 2.0.0`，把 EPR sink policy engine 接入当前模拟板实现。
- 让 fixed PDO 列表可见并可选择 EPR fixed 28V；保持现有 Safe5V-first / `allow_extended_voltage` 门控语义。
- 扩展板间协议与数字侧模型，使 fixed PDO 列表可承载 `>20V` 的 fixed PDO，并新增只读 EPR/AVS 状态字段。
- 完成真实 28V EPR 电源的 HIL 验收，证明从 Safe5V attach 到 EPR 28V 合同建立的路径可用。

### Non-goals

- 本轮不开放 AVS 的 on-device UI 编辑与 HTTP apply。
- 本轮不交付 36V/48V 用户流程，即使协议层能看见这些能力。
- 本轮不改 source-mode，不增加新的热保护/功率保护策略，也不把负载策略与 EPR 档位做额外联动。

## 范围（Scope）

### In scope

- `firmware/analog`：
  - `usbpd 2.0.0` API 迁移；
  - EPR mode entry / exit / keep-alive；
  - fixed PDO `8+` 的 28V 请求；
  - EPR fixed / AVS 能力摘要上报。
- `libs/protocol`：
  - `PdStatus` 增加 `epr_active`、`epr_avs_pdos`；
  - `PdSinkMode` 增加 `Avs`；
  - 扩大 PD capability list 容量。
- `firmware/digital`：
  - EEPROM blob、`PdConfig`、HTTP API、dashboard、PD settings 对 `>20V` fixed PDO 的支持；
  - 28V fixed 的保存、恢复、门控与失败语义。

### Out of scope

- AVS 写路径与 UI 交互。
- Web 前端额外的视觉 redesign。
- 非 PD 相关硬件改版。

## 需求（Requirements）

### MUST

- 模拟板 attach 后仍先建立 Safe5V 合同；仅在目标 fixed PDO 位于 EPR 区间时才进入 EPR。
- 28V fixed 必须通过标准 EPR fixed PDO 请求路径完成，而不是模拟成 PPS/AVS。
- 数字侧保存的 fixed 目标电压不得再因 `>21_000` 被 EEPROM / UI / API 拒绝。
- `GET /api/v1/pd` 与 on-device PD settings 都必须能看到 28V fixed PDO。
- `allow_extended_voltage=false` 时，设备仍必须停留在 Safe5V，不能因为保存值是 28V 就自动离开 Safe5V。
- Source 不支持 EPR、线材不支持 EPR、或 EPR mode entry 失败时，必须进入现有可诊断失败态，而不是崩溃或静默改写保存配置。

### SHOULD

- `fixed_pdos` 视图中同时保留 20V 以下的 SPR fixed 与 28V/36V/48V 的 EPR fixed，按 object position 原样暴露。
- AVS 能力以只读方式暴露给数字侧与 HTTP，作为后续能力基础。
- 协议与 scratch buffer 预留到至少 16 PDO，避免后续再次因上限过小而返工。

### COULD

- HTTP 视图可以额外暴露 `epr_active` 与 `epr_avs_pdos` 的摘要文案。
- 调试日志可以区分 `SPR` / `EPR` 当前模式，帮助 HIL 诊断。

## 功能与行为规格（Functional/Behavior Spec）

### Core flows

- Fixed 28V 选择：
  - 数字侧从 `PD_STATUS.fixed_pdos` 中读取所有 fixed PDO；
  - 当 `object_pos >= 8` 且该 PDO 是 `28_000mV` 时，允许用户像选择普通 fixed PDO 一样保存它；
  - 若 `allow_extended_voltage=true` 且当前 attach，数字侧下发 fixed 模式的 `PD_SINK_REQUEST`。
- 模拟板 fixed 请求：
  - 若目标 fixed PDO 位于 SPR（通常 `pos <= 7`），保持原有 SPR fixed 流程；
  - 若目标 fixed PDO 位于 EPR（通常 `pos >= 8`），则先完成 EPR mode entry，再发出 `EprRequest` 请求对应 fixed PDO；
  - 若当前在 EPR 且目标切回 SPR fixed，则主动退出 EPR 并回到 SPR 能力协商。
- 状态上报：
  - `fixed_pdos` 上报全部 fixed PDO；
  - `epr_avs_pdos` 上报全部 AVS APDO；
  - `epr_active=true` 表示当前协商状态机已处于 EPR 模式。

### Edge cases / errors

- EPR 进入失败、EPR 能力缺失、或选中的 EPR fixed PDO 不在当前 capabilities 中时：
  - 本次请求视为失败；
  - 保存配置不回退；
  - 数字侧按现有 extended-voltage failure latch 处理。
- 若 source 已回落到 SPR 或 detach：
  - `epr_active` 清零；
  - 后续重新 attach 仍从 Safe5V 起步。
- AVS 本轮只读：
  - 可以显示；
  - 不允许通过 UI / HTTP 设置成生效目标。

## 接口契约（Interfaces & Contracts）

### Internal protocol

- `PdSinkMode`
  - `Fixed`: 同时表示 SPR fixed 与 EPR fixed 请求。
  - `Pps`: 保持现状。
  - `Avs`: 新增，仅作为只读基础能力预留。
- `PdStatus`
  - 新增 `epr_active: bool`
  - 新增 `epr_avs_pdos: [pos, min_mv, max_mv, pdp_w]...`
  - `fixed_pdos` 允许 `pos >= 8`

### Digital persistence / API

- `PdConfig` 允许 fixed target 保存到 `48_000mV`。
- `GET /api/v1/pd`
  - `fixed_pdos` 可返回 28V/36V/48V fixed PDO；
  - 新增 `epr_active`；
  - 新增 `epr_avs_pdos`。
- `PUT /api/v1/pd`
  - 继续只接受 `fixed` / `pps`；
  - fixed 模式允许 `object_pos` 指向 EPR fixed PDO。

## 验收标准（Acceptance Criteria）

- Given 设备连接支持 EPR fixed 28V 的 Source
  When `allow_extended_voltage=true` 且保存目标是 28V fixed PDO
  Then 设备先建立 Safe5V 合同，再进入 EPR，并最终得到约 `28_000mV` 的合同。
- Given 当前合同为 28V EPR fixed
  When 用户切回 5V 或 20V fixed
  Then 设备退出 EPR 并稳定回到 SPR fixed 合同。
- Given 保存的目标是 28V fixed
  When 重启后 `allow_extended_voltage=false`
  Then UI 仍显示保存目标为 28V，但运行时有效策略保持 Safe5V。
- Given Source 不支持 EPR 或 EPR 进入失败
  When 用户请求 28V fixed
  Then 失败态可诊断，保存配置不被改写，设备不崩溃。
- Given `GET /api/v1/pd`
  When 当前 Source 暴露 EPR fixed / AVS 能力
  Then 响应包含 28V fixed PDO 与只读 EPR 状态字段。

## 实现前置条件（Definition of Ready / Preconditions）

- 已确认依赖升级目标固定为 `usbpd 2.0.0` 与 `usbpd-traits 2.0.0`。
- 已确认本轮只交付 EPR fixed 28V 的用户路径，AVS 保持只读。
- HIL 需要真实 28V-capable EPR source、合规 5A 线材与现有板卡。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- 协议 encode/decode tests：覆盖 `PdSinkMode::Avs`、`PdStatus.epr_active`、`PdStatus.epr_avs_pdos`。
- EEPROM migration tests：覆盖 fixed target `>20_000mV` 的保存/恢复。
- 数字侧逻辑 tests：覆盖 fixed 28V 选择、Safe5V 门控与 failure latch。
- 构建：`just a-build`、`just d-build`、相关 host/unit tests。
- HIL：真实 28V EPR attach / upgrade / downgrade / failure cases。

### Quality checks

- `cargo fmt --all`
- 与改动直接相关的 host/unit tests
- `just a-build`
- `just d-build`

## 文档更新（Docs to Update）

- `docs/interfaces/uart-link.md`
- `docs/interfaces/network-http-api.md`
- `docs/interfaces/main-display-ui.md`

## Visual Evidence (PR)

待实现阶段补充真实 HIL / UI 证据。

## 实现里程碑（Milestones / Delivery checklist）

- [x] M1: 新 spec / index 落盘，并冻结 EPR fixed 28V 的范围与验收口径。
- [x] M2: 模拟板升级到 `usbpd 2.0.0` 并完成 EPR fixed 28V 协商路径接入。
- [x] M3: 共享协议与数字侧模型/UI/API 支持 28V fixed + EPR 只读状态。
- [ ] M4: 构建、测试与 HIL 完成，具备 merge-ready 证据。

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：现有硬件在 28V 下的电源路径与热裕量仍需真机确认。
- 风险：`usbpd 2.0.0` 的 API 迁移面较大，模拟板可能需要一起调整 `embassy-futures` / `heapless` 的依赖兼容。
- 风险：当前模拟板固件在板上启动时会先于 PD 协商阶段触发 `ADC1 uses sys clock, which is not running` panic，导致本轮 HIL 无法完成。
- 假设：本轮 HIL 环境可用，且现有 Type-C/PD 前端硬件满足 EPR sink 基本条件。
- 假设：AVS 只读不会影响本轮 28V fixed 的用户理解。
