# Dashboard Boot Link Recovery（#fqmns）

## 状态

- Status: 已完成（HIL selector 缺失，软件验证完成）
- Created: 2026-04-25
- Last: 2026-04-25
- Notes: 软件实现与构建验证完成；HIL 因当前 worktree 缺失 analog/digital selector 阻断。

## 背景 / 问题陈述

- 数字板 Dashboard 的生产初始状态复用了 `UiSnapshot::demo()`，当 UART 从未收到模拟板有效帧时，屏幕仍显示一组看似真实的 demo 电压/电流/功率。
- 整机上电偶发出现 Dashboard 数值冻结，但 FPS 仍波动；reset 任意一个 MCU 后通常 1–2 秒恢复，说明 UI 渲染任务仍活着，风险集中在 MCU↔MCU 链路冷启动/握手恢复。
- 现有数字侧恢复逻辑主要覆盖 `LINK_UP=true` 之后的 `CalMissing`，对 `LAST_GOOD_FRAME_MS==0` 的“从未收到帧”冷启动窗口缺少限频重握手。

## 目标 / 非目标

### Goals

- 生产固件在无遥测时显示 offline/unknown 状态，不显示 demo 数值。
- 数字板在冷启动从未收到 `HELLO` / `FAST_STATUS`，或链路持续 down 时，自动限频重发完整启动控制握手。
- SoftReset ACK 等待必须绑定本次握手，不能被旧 ACK 状态误判为成功。
- reset 任意 MCU 后，链路可在数秒内自动恢复，无需主人手动再 reset 另一侧。

### Non-goals

- 不改变 UART 物理引脚、波特率、SLIP/CBOR frame shape 或 HTTP API payload。
- 不重做显示布局或 Dashboard 交互。
- 不自动切换 `.stm32-port` / `.esp32-port` 缓存选择。

## 范围（Scope）

### In scope

- `firmware/digital` 的 UI 初始状态、SoftReset ACK 状态机、SetMode TX 冷启动 recovery watchdog。
- 必要的模拟板 HELLO/FastStatus 配合，仅限恢复链路可观测性与握手鲁棒性。
- 与本行为相关的固件设计文档、spec 与复用经验文档。

### Out of scope

- Web 前端、HTTP API contract、PD policy、校准算法、显示性能调参。
- 任意破坏性硬件操作或 selector 变更。

## 需求（Requirements）

### MUST

- `TelemetryModel::new()` 不能在生产路径显示 demo 数值；无帧状态必须表现为 `AnalogState::Offline`、`link_up=false`、`hello_seen=false`。
- 数字板 SetMode TX 任务必须覆盖 `LAST_GOOD_FRAME_MS==0` 的冷启动无帧恢复路径。
- 恢复握手必须包含 SoftReset、CalWrite 全量曲线、SetEnable(true)，并强制后续发送当前 SetMode snapshot。
- SoftReset ACK 判定必须使用本次握手的 seq/ack baseline，避免陈旧 boolean 让新握手直接短路。
- 恢复重试必须限频，避免在模拟板未供电或线缆断开时刷爆 UART。

### SHOULD

- 链路恢复日志应明确区分 `never-seen-frame` 与 `stale-link-down`，方便 HIL 排查。
- 保持现有 CalMissing 恢复路径，用于模拟板 reset 后校准曲线丢失的场景。

## 功能与行为规格（Functional/Behavior Spec）

### Core flows

- 冷启动无模拟板帧：
  - Dashboard 首帧显示 0/unknown + OFF/LNK 状态；
  - 数字板在短暂 grace 后限频发起 link recovery handshake；
  - 任一 `HELLO`、`FAST_STATUS` 或 ACK 到达后刷新 `LAST_GOOD_FRAME_MS`，正常链路逻辑接管。
- 链路曾经建立后 down：
  - `stats_task` 将 `LINK_UP=false`；
  - SetMode TX watchdog 在无 pending 控制 ACK 时限频重发启动握手；
  - 恢复后强制 SetMode snapshot，避免模拟板 reset 后停在默认 active control。
- SoftReset ACK：
  - 每次 handshake 记录发送前 ACK total 与本次 seq；
  - 只有 ACK total 增加且 ACK seq 匹配，才视为本次 SoftReset 成功。

### Edge cases / errors

- 模拟板未上电或串口断线时，恢复逻辑只产生日志与限频 TX，不得阻塞 UI/输入/HTTP。
- 若输出开关处于 ON 且链路 down，仍保持现有安全 gate：不得为了恢复而盲发 output_enabled=true。
- 旧 ACK 晚到时只刷新统计，不得让后续不同 seq 的 SoftReset handshake 误成功。

## 验收标准（Acceptance Criteria）

- Given 数字板启动后尚未收到任何 UART 有效帧，When Dashboard 首帧渲染，Then 不显示 demo 的 24.50V/12.00A/294W，而显示 offline/zero/unknown 状态。
- Given 整机同时上电，When 无人工 reset，Then 数字板数秒内能收到 `HELLO` 或 `FAST_STATUS`，`fast_status ok` 递增，Dashboard 数值开始刷新。
- Given reset analog 且 digital 不 reset，When 模拟板重新启动，Then digital 自动重发校准与 SetMode snapshot，链路恢复。
- Given reset digital 且 analog 不 reset，When 数字板重新启动，Then SoftReset ACK 不被旧状态短路，后续启动握手真实发出。
- Given 模拟板断开，When watchdog 运行，Then 日志限频且 UI 仍刷新 FPS/状态，不发生 panic 或长时间阻塞。

## 实现前置条件（Definition of Ready / Preconditions）

- 已锁定快车道 `merge-ready`；HIL 仅允许使用当前缓存/已确认 selector。

## 非功能性验收 / 质量门槛（Quality Gates）

- `cargo test -p loadlynx-protocol` 或受影响 host-testable crate 通过。
- `just d-build` 通过；如修改模拟板则 `just a-build` 通过。
- HIL 可用时，flash/monitor digital 与 analog，并比对 `tmp/{digital,analog}-fw-version.txt` 与 boot log。

## 文档更新（Docs to Update）

- `docs/dev-notes/software.md`
- `docs/specs/README.md`
- 如形成可复用经验，新增或刷新 `docs/solutions/firmware/**`。

## 实现里程碑（Milestones）

- [x] M1: 生产 UI 初始状态改为 offline/unknown，demo 仅保留给 mock/test。
- [x] M2: SoftReset ACK 改为本次 seq/baseline 绑定。
- [x] M3: SetMode TX 增加冷启动/持续 down 的限频恢复握手。
- [x] M4: 构建与可用 HIL 验证完成，文档同步（HIL selector 缺失，记录为阻断证据）。

## 风险 / 开放问题 / 假设

- 风险：若根因是硬件供电时序或隔离器方向异常，软件 watchdog 只能提升恢复概率，不能替代硬件修复。
- 假设：模拟板在供电稳定后能接收数字板 UART TX，并能通过现有 SoftReset/CalWrite/SetEnable/SetMode 流恢复。

## References

- `docs/solutions/firmware/boot-link-recovery-watchdog.md`
- `docs/plan/0009:uart-comm-troubleshooting/PLAN.md`（legacy UART troubleshooting record；pending canonical migration/deletion decision）
- `docs/interfaces/uart-link.md`
- `docs/dev-notes/software.md`
