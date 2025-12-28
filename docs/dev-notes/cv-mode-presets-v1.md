# CV 模式 + Preset（v1）需求与概要设计

本文件冻结“CV（电压钳位）模式 + 5 组预设 Preset + EEPROM 持久化”的 v1 需求基线，并给出跨 MCU（ESP32‑S3 ↔ STM32G431）实现边界与接口形状（不包含实现代码）。

相关文档：

- 串口链路与消息集：`docs/interfaces/uart-link.md`
- 网络 HTTP API：`docs/interfaces/network-http-api.md`
- CC 负载开关语义（enable/target/effective）：`docs/dev-notes/cc-load-switch-toggle.md`
- 当前 firmware 行为梳理：`docs/dev-notes/software.md`

---

## 1. 背景与目标

### 1.1 背景

现有系统已经具备：

- STM32G431（analog）侧：基于 DAC 设定 + 硬件电流环的恒流（CC）控制路径；`FastStatus` 以 20 Hz 上报；
- ESP32‑S3（digital）侧：本地 UI 与 Web/HTTP 控制入口（以 CC 为主），并通过 UART 下发 `SetPoint`/`LimitProfile` 等控制帧。

但 CV 模式仍处于预留状态（协议常量存在，payload/执行逻辑未落地）。

### 1.2 目标（v1）

- 新增 **CV（电压钳位型电子负载）模式**：当电压高于目标时吸流拉回；低于目标时退流至 0。
- 引入 **5 组 Preset**（编号 1..5，无名称）：每组包含 `mode + target + limits`，并 **持久化到 EEPROM**。
- 模式切换与应用预设必须安全：**强制先关闭输出**，用户再手动启动出力。
- CV 算法必须运行在 **STM32G431** 侧（避免网络/主控调节过慢）。

---

## 2. 范围与非目标

### 2.1 In scope（本期交付范围）

- 支持 `CC` 与 `CV` 两种工作模式。
- 5 组 Preset 的读写、应用、持久化（EEPROM）。
- 三项上限/约束（Preset 中定义）：`min_v_mv`、`max_i_ma_total`、`max_p_mw`。
- 欠压锁存（`uv_latched`）作为**非故障**状态：触发后退流并锁存，需用户“关→开”解除。
- 远端/近端电压的 `V_main` 选择规则（见 §6）。
- `FastStatus` 与 HTTP/Web/UI 的可观测性：必须能看到当前模式、目标、上限状态与欠压锁存状态。

### 2.2 Out of scope（明确不做）

- 本期不引入 `CP/CR`（恒功率/恒阻）。
- 欠压锁存不进入 `fault_flags`，不要求 SoftReset 才能恢复。
- 不做公网级鉴权/权限系统（仍按局域网控制假设）。

---

## 3. 关键约束（冻结）

### 3.1 硬件能力与系统硬限制（必须 clamp）

- 单通道最大电流：`5 A`
- 双通道合计最大电流：`10 A`
- 任何来源（EEPROM/HTTP/Web/本地 UI/UART）的设定值与上限，都必须在固件内 **clamp 到系统硬限制** 后才生效。

> 备注：系统级硬限制以固件硬编码为准；Preset 只提供用户层的“软件上限”，不得绕过系统硬限制。

### 3.2 通道分配策略（精度驱动的小设计）

为提升低电流区采样精度，冻结如下分配逻辑（对 CC 与 CV 的“总目标电流”一致适用）：

- 若 `I_total_target < 2_000 mA`：仅 CH1 承担全部电流，CH2=0；
- 若 `I_total_target ≥ 2_000 mA`：CH1/CH2 近似均分（奇数 mA 由 CH1 多承担 1 mA）。

并且：

- `I_ch1_target ≤ 5_000 mA`
- `I_ch2_target ≤ 5_000 mA`
- `I_total_target ≤ 10_000 mA`

---

## 4. Preset 数据模型（冻结）

### 4.1 Preset 定义（编号 1..5）

每个 Preset 包含：

- `mode`: `CC | CV`
- `target_i_ma`: CC 模式目标电流（mA）
- `target_v_mv`: CV 模式目标电压（mV）
- `min_v_mv`: 欠压阈值（mV），用于欠压锁存（见 §7）
- `max_i_ma_total`: 总电流上限（mA）
- `max_p_mw`: 总功率上限（mW）

### 4.2 默认值（EEPROM 空/无效时）

默认上限建议（v1 冻结）：

- `min_v_mv = 0`
- `max_i_ma_total = 10_000`
- `max_p_mw = 150_000`

默认 `mode/target` 不强行规定为“上电即出力”的值（因为上电默认输出关闭；见 §5）。

---

## 5. 输出开关与模式切换（冻结）

### 5.1 输出开关（OutputEnabled）

- `output_enabled=false` 时：等效输出为“0 吸流”（无论 CC/CV）。
- `output_enabled=true` 时：按 active preset 的 mode/target/limits 执行控制。

### 5.2 模式切换/应用 Preset 的安全规则

当发生以下任一事件：

- `ApplyPreset(preset_id)`（应用 1..5 预设）
- 修改 `mode`（CC↔CV）

必须执行：

- 强制 `output_enabled=false`（用户再手动开启）

目的：避免“切换后自动出力”带来的误操作风险。

---

## 6. 电压选择：V_main（控制与保护都用“更高的那路”）

CV 控制与 `min_v`/功率等保护约束，都统一使用：

```
V_main_mv = max(V_local_mv, V_remote_mv)
```

但为避免远端 Sense 线浮空/异常导致误控，`V_remote_mv` 必须通过“有效性判定”后才允许参与 max。有效性判定为实现细节，但 v1 设计要求：

- 必须存在明确的 remote-valid 判定与降级路径（invalid → 只用 local）；
- 必须具备可观测性（至少在 `FastStatus`/HTTP 上能看见当前电压来源/remote 是否有效）。

---

## 7. 欠压锁存（uv_latched，非 fault，冻结）

### 7.1 触发条件

当 `output_enabled=true` 且：

```
V_main_mv ≤ min_v_mv
```

触发欠压锁存。

### 7.2 行为

- 置位 `uv_latched=true`
- 强制输出退流（等效 `I_total_target=0`）
- `uv_latched` 不进入 `fault_flags`，不作为系统故障

### 7.3 清除条件（冻结）

只有当用户执行一次“关→开”（观察到 `output_enabled` 出现新的上升沿）时，才允许清除 `uv_latched` 并重新出力。

> SoftReset 可以额外清除 `uv_latched`（可选），但不作为正常恢复路径的必要条件。

---

## 8. CV 控制算法（冻结：运行在 G431）

### 8.1 架构

- 内环：现有硬件恒流环（DAC 设定电流）
- 外环：CV 逻辑在 G431 周期性运行，通过电压误差调节 **总目标电流** `I_total_target`，再经 §3.2 分配到 CH1/CH2

### 8.2 基本行为（冻结）

- 若 `output_enabled=false` 或 `uv_latched=true`：`I_total_target=0`
- 否则在 CV 模式下：
  - 若 `V_main_mv ≤ target_v_mv`：`I_total_target → 0`（退流）
  - 若 `V_main_mv > target_v_mv`：增大 `I_total_target` 以拉回电压（受限于 §9 的限值）

### 8.3 速度要求

CV 外环必须在 G431 侧独立运行，不依赖 S3/HTTP/Web 的更新频率；实现时采用“折中默认值”并保留可调参数（更新周期/死区/限速/PI 增益等），以便硬件实测整定。

---

## 9. 三限值联动（v1 必须真限）

Preset 中三限值必须对 CC/CV 生效：

### 9.1 电流上限（max_i_ma_total）

- 作为总电流上限，并叠加系统硬限制（总 10A、单通道 5A）。

### 9.2 功率上限（max_p_mw）

必须主动限功率（不再仅日志告警）。典型推导：

```
I_by_power_ma = (max_p_mw * 1000) / max(V_main_mv, V_min_for_div)
I_limit_ma    = min(max_i_ma_total, I_by_power_ma, system_i_max_ma)
```

其中 `V_min_for_div` 为实现细节，用于避免除零与低压异常放大。

### 9.3 欠压下限（min_v_mv）

按 §7 的欠压锁存语义执行（锁存、需用户关开恢复）。

---

## 10. 跨模块接口边界（概要）

### 10.1 UART：原子下发 Active Control（冻结）

v1 冻结将 `MSG_SET_MODE (0x21)` 定义为“原子 Active Control”控制帧，以满足：

- 一次下发：`preset_id + output_enabled + mode + target + limits`
- 带 ACK_REQ/ACK，便于可靠传输与“关→开”边沿语义

Payload 形状（示意）：

```text
{
  preset_id: u8 (1..5),
  output_enabled: bool,
  mode: u8 (CC=1, CV=2),
  target_i_ma: i32,
  target_v_mv: i32,
  min_v_mv: i32,
  max_i_ma_total: i32,
  max_p_mw: u32,
}
```

> 说明：即使 Preset 存在 S3 的 EEPROM 中，analog 仍需要收到“当前生效的 active preset 内容”，以执行 CV/限功率/欠压锁存等逻辑。

### 10.2 FastStatus / 状态可观测性

v1 需要补齐：

- `mode`：必须真实反映 CC/CV
- `target_value`：语义需与 mode 对齐（实现阶段决定：复用一个字段随 mode 变化，或新增字段）
- `state_flags` 或扩展字段：
  - `uv_latched`
  - `power_limited` / `current_limited`（建议）
  - `voltage_source`（local/remote/max，或 remote_valid 指示，建议）

### 10.3 HTTP/Web/UI（Web 优先）

Web/HTTP 必须覆盖：

- Preset 1..5 的读写与应用
- 输出开关 `output_enabled`
- 模式与目标值（CC: I、CV: V）
- 关键状态展示：`uv_latched` 与当前受限原因（建议）

v1 冻结使用统一端点与固定路径（详见 `docs/interfaces/network-http-api.md`）：

- `GET /api/v1/presets` → `{ "presets": Preset[] }`（必须恰好 5 条）
- `PUT /api/v1/presets` → 更新单个 Preset（请求体必须包含完整 Preset payload + `preset_id`），返回更新后的 Preset
- `POST /api/v1/presets/apply` → `{ "preset_id": number }`，应用并 **强制输出关闭**，返回 `ControlView`
- `GET /api/v1/control` → `ControlView`
- `PUT /api/v1/control` → `{ "output_enabled": boolean }`（`uv_latched` 仅能通过 “关→开” 边沿清除）

---

## 11. EEPROM 持久化（v1 冻结）

### 11.1 目标

- 5 组 Preset 必须存入外置 EEPROM。
- 必须版本化并带校验（CRC32 或等价），并在无效时回退默认值。
- 不持久化 `output_enabled`（上电默认关闭）。

### 11.2 EEPROM 地址规划（建议）

当前 calibration profile 使用：

- base：`0x0000`
- len：`1024` bytes（v3）

v1 建议将 Preset blob 放在后续地址段（不重叠），例如：

- `0x0400..0x04FF`（256 bytes）用于 presets（具体长度由实现阶段最终结构决定）

---

## 12. 验收标准（实现阶段）

### 12.1 功能验收

- 可从 Web 应用 Preset 1..5，并观察到 `output_enabled` 被强制关闭。
- 用户手动开启输出后：
  - CC：按目标电流出力并受 `max_i/max_p` 限制；
  - CV：电压高于目标时能拉回到目标附近；低于目标时退流。
- 欠压锁存：
  - 触发后退流并锁存；
  - 仅在用户“关→开”后恢复（无需 SoftReset）。

### 12.2 安全与上限

- 单通道不超过 5A、总不超过 10A（被 clamp）。
- 功率上限为真限：当达到 `max_p_mw` 时会主动限流。

### 12.3 可观测性

- FastStatus/HTTP 能反映 mode、目标、`uv_latched`、受限原因（建议项）。
