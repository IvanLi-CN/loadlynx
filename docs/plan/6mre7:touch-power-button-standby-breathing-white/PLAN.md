# 触摸电源按键：睡眠待机白光低频呼吸（#6mre7）

## 状态

- Status: 已完成
- Created: 2026-02-01
- Last: 2026-02-03

## 背景 / 问题陈述

- 数字板（ESP32‑S3）已有“屏幕自动调暗/熄屏”的省电策略（Plan #0015；实现见 `libs/screen-power/src/lib.rs`、`firmware/digital/src/main.rs` 的 `ScreenPowerState::{Active,Dim,Off}` 与 `display.sleep()/wake()`）。
- 现在新增了一个独立的触摸电源按键；在设备进入“睡眠待机”时，需要一个低干扰、无需看屏幕的白光呼吸指示，便于识别设备仍处于待机而非断电。

## 目标 / 非目标

### Goals

- 明确“睡眠待机”的**可观测判定条件**（固件内可直接引用的状态源），并把它作为指示灯策略的唯一触发条件。
- 当设备处于睡眠待机时，电源按键指示灯以**白光低节奏呼吸**显示（可配置节奏/亮度上限，便于 HIL 调参）。
- 不引入新的对外协议或 API；仅改动 digital 固件内部状态机与外设驱动/调度（实现阶段）。

### Non-goals

- 不做 ESP32 系统级 deep sleep / wakeup（本计划聚焦“待机灯效”，不把系统睡眠形态升级为 deep sleep）。
- 不在本计划内重新设计整机“电源管理/开关机语义”（例如真正断电/上电流程、长按关机等）。
- 不在本计划内重做 #0021 的完整 RGB 状态映射（本计划只冻结“睡眠待机 → 白光呼吸”的最小口径；其余状态灯效如需统一另开计划或明确纳入 #0021）。

## 用户与场景

- 用户：桌面/台架使用者。
- 场景：
  - 设备因无操作进入待机（屏幕已熄）；用户希望从远处快速判断“设备仍在待机运行”。
  - 用户触摸电源键希望唤醒屏幕（口径冻结为：熄屏时只唤醒且输入被消费，不改变业务状态）。

## 需求（Requirements）

### MUST

- **待机判定（State source）**
  - “睡眠待机”判定 **等价于** `ScreenPowerState::Off`：
    - 领域模型：`libs/screen-power/src/lib.rs` 的 `ScreenPowerState::Off`。
    - digital 侧状态源：`firmware/digital/src/main.rs` 的 `SCREEN_POWER_STATE`（原子变量）。
  - 判定条件必须稳定、可日志化（便于 HIL 验收）。

- **触摸电源按键输入（Touch power button input）**
  - 触摸电源按键 **等价于** `TOUCH_SPRING`（Touch Sensor）：
    - GPIO：`GPIO14` / `TouchPad14`（见 `docs/interfaces/pinmaps/esp32-s3.md`、Plan #0021）。
    - 网表：`docs/power/netlists/digital-board-netlist.enet` 中存在 `TOUCH_SPRING` 网络。

- **电源按键指示灯硬件形态（Indicator HW）**
  - 指示灯为 RGB LED（3 路 PWM）：
    - 网络：`RGB_R_PWM` / `RGB_G_PWM` / `RGB_B_PWM`（见 `docs/power/netlists/digital-board-netlist.enet`）。
    - GPIO 规划与电气约束沿用 `docs/interfaces/pinmaps/esp32-s3.md` 与 `docs/interfaces/touch-switch-and-rgb-led.md`（共阳/COM=3V3、GPIO 灌电流、active‑low）。

- **白光低频呼吸（Breathing pattern）**
  - 待机时指示灯输出为白光（RGB 同时发光或等价白光通道）。
  - 呼吸节奏为“低节奏”：周期 `T_ms` 可配置（固件常量或配置结构体均可），实现阶段允许按实际观感调参，并在 HIL 验收记录最终取值。
  - 呼吸亮度上限 `max_brightness` 可配置，避免夜间刺眼与无谓耗电；实现阶段允许按实际观感调参，并在 HIL 验收记录最终取值。
  - 动画更新应限频（例如 20–50ms 一步），避免占用过多 CPU 时间与日志带宽。

- **熄屏输入处理（Wake-only & consume）**
  - 在 `ScreenPowerState::Off` 期间，触摸电源按键输入 **只用于唤醒** 且 **输入被消费**：不得改变任何业务状态（例如不得切换 `load_enabled`、不得改 setpoint）。

- **资源与互不干扰**
  - 指示灯驱动不得抢占/破坏现有 LEDC 资源分配（当前已使用：背光=Timer0/Channel0、风扇=Timer1/Channel1、蜂鸣器=Timer2/Channel2；见 `firmware/digital/src/main.rs`）。
  - 待机灯效不得影响 `ScreenPowerState::Off` 时的“跳过屏幕渲染”策略（Plan #0015 已在 `display_task` 中 `continue`）。

## 接口契约（Interfaces & Contracts）

None（本计划不新增/修改对外接口；仅涉及 digital 固件内部状态/外设策略）。

## 约束与风险

- **“睡眠待机”语义需对齐**：若主人期望的是“系统 light sleep/deep sleep”，则 LEDC 时钟/任务调度可能发生变化，需要重新评估灯效是否能在睡眠中持续运行。
- **与 #0021 的灯效策略协调**：#0021 已冻结“异常/绿/红”映射；本计划新增的“待机白光呼吸”需定义优先级并避免互相打架。

## 验收标准（Acceptance Criteria）

- Given 设备满足“睡眠待机”的判定条件
  When 进入该状态后的 1 秒内
  Then 电源按键指示灯开始以白光呼吸显示，且周期与亮度上限遵循固件配置（可肉眼验证 + 至少一次状态日志）。

- Given 设备处于睡眠待机且白光呼吸正在运行
  When 设备退出睡眠待机（例如用户操作导致屏幕唤醒，或 `ScreenPowerState::Off → Active`）
  Then 白光呼吸在 1 秒内停止；后续灯效由“非待机策略”接管（建议与 #0021 一致：异常/绿/红）。

- Given 设备处于睡眠待机
  When 用户触摸电源按键一次
  Then 仅唤醒屏幕且输入被消费（不改变业务状态），且白光呼吸随状态切换正确停止。

## 实现前置条件（Definition of Ready / Preconditions）

- 已冻结“睡眠待机”判定：`ScreenPowerState::Off`
- 已冻结触摸电源按键输入：`TOUCH_SPRING`（GPIO14 / TouchPad14）
- 已冻结“熄屏触摸电源键”语义：只唤醒且输入被消费
- 已确认指示灯硬件与电气约束：RGB 三路 PWM（active‑low / COM=3V3；详见 `docs/interfaces/touch-switch-and-rgb-led.md`）

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- 单元测试（如适用）：把“呼吸曲线生成（t → brightness）”提炼为纯逻辑函数，放到 `libs/`（或现有可测模块）并覆盖：
  - `t=0/t=T/2/t=T` 边界值
  - 亮度裁剪（0..=max）
  - 不同 `T_ms` 的周期性

### Quality checks

- `just fmt`
- `just d-build`（digital release build）

## 文档更新（Docs to Update）

- `docs/interfaces/pinmaps/esp32-s3.md`：补充/确认电源按键与其指示灯的网络名、GPIO 占用与电气注意事项（避免与上电毛刺/strapping 冲突）。
- `docs/dev-notes/software.md`（或等价位置）：补充“睡眠待机指示灯策略（白光低频呼吸）”与验收口径，便于后续排查与一致性。

## 计划资产（Plan assets）

- None

## 资产晋升（Asset promotion）

None

## 实现里程碑（Milestones）

- [x] M1: digital：增加“待机白光呼吸”灯效状态机（触发条件=ScreenPowerState::Off，退出条件=Off→Active）
- [x] M2: digital：接入 RGB 三路 LEDC PWM（Timer3/Channel3-5 或等价不冲突配置）并实现白光呼吸波形（可调 `T_ms/max_brightness`）
- [x] M3: digital：熄屏期间触摸电源键输入仅唤醒且被消费（不改变业务状态）
- [x] M4: HIL：记录最终 `T_ms/max_brightness` 取值与观感结论，并补充到 `docs/dev-notes/software.md`（final: T_ms=14000, max_brightness=12%, update_ms=10）

## 方案概述（Approach, high-level）

- 以 `ScreenPowerState::Off` 作为待机灯效触发条件：
  - 进入 Off：启动呼吸动画（白光）；
  - 退出 Off：停止呼吸动画并进入非待机策略。
- 灯效生成使用定时 tick（或与现有 ticker 复用），并将 duty 更新做限频与去抖，避免对主循环造成可感知影响。
- LEDC 资源分配遵循现状：为指示灯使用独立的 timer/channel（建议 Timer3/Channel3-5），避免与背光/风扇/蜂鸣器互相牵连。

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：见「约束与风险」。
- 开放问题：None
- 假设：None
