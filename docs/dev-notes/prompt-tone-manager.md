# 提示音管理器（蜂鸣器 Prompt Tone）设计

## 背景

LoadLynx 控制板（ESP32‑S3）已集成无源蜂鸣器，但当前固件缺少统一的提示音机制。为了让操作者在不看屏幕的情况下也能获得明确反馈，需要在数字侧固件中增加：

- **提示音管理器**：以事件驱动、非阻塞播放、可扩展的“提示音表”与调度策略。
- **操作反馈音**：所有本地交互（触摸、旋钮 detent、按键）都要有声音反馈，且音量偏小（靠占空比控制）。
- **故障报警音**：模拟板故障（`fault_flags!=0`）时持续报警，直到故障消除且用户进行一次本地确认交互后才停止。

本设计只覆盖控制板本地蜂鸣器（GPIO21）提示音，不涉及 UI 视觉提示或远程接口反馈。

## 目标

- 提供两层架构：
  - `buzzer`：蜂鸣器硬件 PWM 驱动（LEDC），提供最小、非阻塞控制接口。
  - `prompt_tone`：提示音管理器（事件 → 音型 → 调度），无堆分配。
- **故障报警**（最高优先级）：
  - `fault_flags != 0`：循环报警，抑制其它提示音；
  - `fault_flags == 0`：仍继续报警，直到用户发生一次**本地交互**（触摸/旋钮/按键）后停止。
  - 远程操作（Wi‑Fi/HTTP 等）不计入“确认交互”。
- **操作反馈**（低音量）：
  - 触摸：每次 touch-down 有声；
  - 旋钮：每个 detent 有声；若极快旋转导致播放跟不上，需要排队；
  - 按键：短按/长按均有声；失败音只用于“业务拒绝”，不用于 ACK/超时等内部通信失败。

## 范围与非目标

### 范围

- 数字侧固件 `firmware/digital/` 增加提示音模块与集成点（仅在实现阶段进行）。
- 基于现有协议与状态：`FastStatus.fault_flags`、编码器/触摸事件。

### 非目标

- 不实现静音/音量调节入口（包括“夜间模式”）。
- 不用“ACK/timeout”作为失败音触发条件（内部通信问题不在本次范围内）。
- 不实现“故障提示界面/消音按钮”视觉 UI（未来再做）。

## 硬件与约束（从文档/网表提取）

- 引脚：`GPIO21=BUZZER`（`docs/interfaces/pinmaps/esp32-s3.md`）。
- 器件：`BUZZER1=MLT-7525`，标称 `2.7kHz`（`docs/power/netlists/digital-board-netlist.enet`）。
- 拓扑：`GPIO21 → Q1(SS8050)` 驱动蜂鸣器；并联钳位二极管（同网表）。
- 软件侧安全策略：默认占空比偏低、GPIO 驱动强度保守，避免长时间高占空比。

## 核心用例 / 用户流程

1) **正常操作**
- 旋钮每个 detent：播放 `UiTick`（短促低音量）。
- 触摸屏幕（touch-down）：播放 `UiOk`（短促低音量）。
- 编码器按键短按/长按：播放 `UiOk` 或 `UiFail`（仅业务拒绝触发 `UiFail`）。

2) **进入故障**
- 接收到 `fault_flags` 从 `0 → 非0`：立即进入 `FaultAlarm` 循环，抑制其它音。

3) **故障消除但需要人工确认**
- `fault_flags` 从 `非0 → 0`：继续 `FaultAlarm` 循环（不立刻停）。
- 下一次本地交互（触摸/旋钮/按键）发生后：停止 `FaultAlarm`，并正常播放该次交互的提示音。

## 模块边界与接口形状（概要）

### 1) `buzzer`：硬件驱动层（LEDC PWM）

职责：
- 绑定 `GPIO21`，用 LEDC 输出方波（非阻塞）。
- 提供最小接口，保证 `stop()` 强制静音。

接口形状（示意）：

```text
trait BuzzerControl {
  start_tone(freq_hz: u32, duty_pct: u8) -> Result<(), BuzzerError>
  stop() -> Result<(), BuzzerError>
}
```

### 2) `prompt_tone`：提示音管理层（事件→音型→调度）

职责：
- 定义音效 ID（`SoundId`）与默认音型表（静态步骤序列）。
- 接收事件（本地交互、故障变化），按策略排队/抢占，驱动蜂鸣器播放。

关键状态：
- `fault_active`：`fault_flags != 0`。
- `fault_cleared_wait_ack`：`fault_flags == 0`，但尚未收到一次本地交互确认，因此继续报警。
- `pending_ticks`：旋钮 detent 的待播放计数（允许累积/排队）。

事件形状（示意）：

```text
SoundEvent:
  - FaultEnter(flags: u32)
  - FaultCleared
  - LocalInteraction   // 任何本地交互：touch-down / detent / 按键
  - UiTick(count: u16) // 每个 detent 计数
  - UiOk
  - UiFail             // 业务拒绝（非通信失败）
```

> 说明：实现时可将 `LocalInteraction` 作为“副作用事件”，由输入端统一上报；在 `fault_cleared_wait_ack` 期间，`LocalInteraction` 会先停止报警，再允许本次交互的声音继续播放。

## 调度策略（冻结为实现基线）

### 优先级与抢占

- 优先级：`FaultAlarm` > `UiFail` > `UiOk` > `UiTick`。
- `FaultAlarm` 必须立即抢占并持续；故障未消除时抑制其它音。
- `UiOk/UiFail` 可以抢占正在播放的 `UiTick`，但不得丢失 `pending_ticks`。

### detent 排队（必须）

- 每个 detent 产生一次 `UiTick`；若播放赶不上，累积到 `pending_ticks`。
- 设计目标：正常旋转速度下不应形成可感知 backlog；极快旋转允许 backlog。

### 故障消除后的“确认停止”

- 当 `fault_flags` 清零后进入 `fault_cleared_wait_ack=true`：
  - 继续播放 `FaultAlarm`；
  - 等待第一次本地交互（`LocalInteraction`）到来后：停止报警并清除此状态。
- 远程操作不产生 `LocalInteraction`。

## 默认音型与参数建议（需实机试听后微调）

基准频率建议：`~2200Hz`（低于 2.7kHz 共振点，听感更柔和）。  
占空比建议：操作音更小、故障音可略高但仍保守（最终以实机风险与听感为准）。

建议的音型（示意）：

- `UiTick`：`Tone(2200Hz, 3%, 12ms)` + `Silence(8ms)`（总 20ms/ detent）
- `UiOk`：`Tone(2200Hz, 3%, 25ms)`
- `UiFail`：`Tone(2200Hz, 3%, 25ms)` + `Silence(30ms)` + `Tone(2200Hz, 3%, 25ms)`
- `FaultAlarm`：`Tone(2200Hz, 6%, 300ms)` + `Silence(700ms)` 循环

## 集成点（仅定义入口，不定义实现细节）

- `fault_flags` 变化检测：在数字侧消费 `FastStatus` 的位置检测 `0↔非0` 边沿，并向 `prompt_tone` 上报 `FaultEnter/FaultCleared`。
- 本地交互事件：
  - 旋钮：每个 detent 上报 `LocalInteraction + UiTick(1)`。
  - 编码器按键：按下/释放上报 `LocalInteraction + UiOk/UiFail`。
  - 触摸：任意 touch-down 上报 `LocalInteraction + UiOk`（即使未命中控件也算“交互”）。

业务失败（`UiFail`）定义：仅当本地状态机明确拒绝该操作时触发；不得因为 ACK/timeout/链路波动触发。

## 兼容性与迁移

- 不修改协议与模拟侧固件行为；只消费现有 `fault_flags` 与本地输入事件。
- 不引入配置/存储迁移。

## 风险与注意事项

- 长时间报警：故障未消除与“已消除但未确认”两种状态都会持续报警，需确保 `stop()` 可靠、不会卡鸣。
- 硬件驱动保守性：默认低占空比/保守驱动强度，避免潜在过流/过热风险。
- detent 高频：需要确保 `UiTick` 足够短（建议 ≤20ms/次）以满足“正常旋转不排队”的体验目标。

## 验收标准

- `fault_flags!=0` 时：持续 `FaultAlarm` 循环，且抑制其它音。
- `fault_flags` 清零后：报警仍持续；直到发生一次本地交互后停止；该交互仍会发出对应操作音。
- 旋钮每个 detent 都有声；极快旋转时允许排队但不丢失计数。

