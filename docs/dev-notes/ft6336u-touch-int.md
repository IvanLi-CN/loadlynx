# FT6336U 触控（P024C128-CTP）驱动与 digital 集成设计（草案）

## 背景

LoadLynx 现有 digital 固件已驱动 ST7789 显示，但尚未接入触控。P024C128-CTP 模组的 CTP IC 为 FT6336U，接口为 I2C + INT + RST，需要提供：

- 一个可复用的 Rust `#![no_std]` 驱动 crate，支持同步与异步两种 I2C 栈，并保持同一套 API。
- digital 固件侧的最小可用闭环：INT 触发读取触点寄存器，解析坐标/事件，日志输出 + 屏幕可视化反馈。

本文仅覆盖需求分析与概要设计，不包含实现代码与固件逻辑修改。

## 目标

### A. `ft6336u-async` crate（临时本地 crate）

- 在工作区根目录新增临时子目录：`ft6336u-async/`，并且它必须是**独立 git repo**（未来若需要，才迁移为可共享的远端仓库并作为真正 submodule 记录在主仓库中）。
- 约束：在远端仓库未就绪前，主仓库不得跟踪 `ft6336u-async/` 内容；本地开发可通过 `.git/info/exclude` 等方式做本地忽略，避免影响他人工作流。
- `#![no_std]`，Rust edition 与仓库一致（建议 2024）。
- 同一套 API 同时支持：
  - 同步：`embedded-hal = 1.0` I2C
  - 异步：启用 feature `async` 后使用 `embedded-hal-async = 1.0` I2C
- 使用 `maybe-async-cfg` 在同一 impl 上生成 sync/async 两个版本，组织方式需对齐 `sc8815-rs`。
- 默认 I2C 7-bit 地址为 `0x38`，构造时允许覆盖。
- 实现最小触点读取与解析（最多 2 点），并提供 raw report bytes 接口便于 bring-up。

### B. digital 固件集成（ESP32-S3）

- MUST：使用 `CTP_INT` 作为触点读取触发（除初始化/复位外，不允许纯轮询常驻读 I2C）。
- MUST：I2C0 与 EEPROM 共用总线，支持并发调度但事务必须串行化，且避免长时间持有 I2C 锁。
- MUST：最小可用验证闭环：
  - 串口日志打印触点事件（坐标、id、event）
  - 屏幕上显示一个可视化反馈（如光点/十字跟随），确认坐标映射正确

## 非目标

- 不发布 crates.io，不做远端仓库管理/版本发布流程。
- 不做工厂模式/诊断/flash 相关复杂能力。
- 不做复杂手势识别；优先支持触点坐标 + down/move/up（或其等价事件）。
- analog 固件不参与触控处理。

## 关键用例 / 用户流程

- 终端用户：触摸屏幕进行 UI 操作（按钮、选项、设置值）。
- 开发者 bring-up：
  1) 上电与 reset 时序正确
  2) `CTP_INT` 触发
  3) 固件读取 `0x02..0x0E` 并解析
  4) 串口与屏幕反馈一致（坐标映射正确）

## 设计 A：`ft6336u-async` crate

### 依赖与 feature（需对齐 `sc8815-rs`）

- `embedded-hal = { version = "1.0.0", default-features = false }`（默认启用）
- `embedded-hal-async = { version = "1.0.0", optional = true, default-features = false }`（仅 feature `async` 启用）
- `maybe-async-cfg = "0.2.4"`
- 可选：`defmt`（仅用于格式化，不能影响 core API）

features 结构建议：

- `async = ["dep:embedded-hal-async"]`
- `defmt = ["dep:defmt"]`（如需要）

### 模块边界

- `Ft6336u<I2C>`：仅关心 I2C 事务（寄存器读写、report burst read）。
- `TouchReport`/`TouchPoint`：安全解析后的领域模型。
- `RawTouchReport`：固定长度 raw bytes（用于调试/回归/抓包对比）。
- `parse`：纯解析逻辑（无 I/O），便于 host 侧单元测试（未来实现阶段放到 `libs/` 或 crate 内 test）。

### 寄存器与读取策略

最小读取集合（单次 burst，短事务）：

- `0x02 TD_STATUS`：触点数量（最多 2 点）
- `0x03..0x08`：点 1（XH/XL/YH/YL/WEIGHT/MISC）
- `0x09..0x0E`：点 2（同理）

建议 `read_raw_touch_report()` 一次性读取 `0x02..0x0E` 共 13 bytes，以减少 I2C 事务次数与锁持有时间。

### 解析规则（固定）

以点 1 为例（点 2 同理）：

- `P1_XH`：
  - `bit7..6`：触摸事件标志（event raw）
  - `bit3..0`：X 高 4 位
- `P1_XL`：X 低 8 位
- `P1_YH`：
  - `bit7..4`：ID
  - `bit3..0`：Y 高 4 位
- `P1_YL`：Y 低 8 位

### API 形状（同步/异步同一套）

说明：以下以 `async fn` 形状描述，由 `maybe-async-cfg` 生成 sync/async 两套实现；函数名与行为保持一致。

构造/释放：

- `new(i2c, address)` / `new_default(i2c)`
- `release(self) -> I2C`
- `address()` / `set_address()`

基础寄存器访问（bring-up）：

- `read_reg(reg) -> u8`
- `write_reg(reg, val)`
- `read_many(start_reg, buf)`

触点读取：

- `read_raw_touch_report() -> RawTouchReport`
- `parse_touch_report(&RawTouchReport) -> TouchReport`（纯解析）
- `read_touch_report() -> TouchReport`（raw + parse）

可选初始化/配置（先提供 raw 能力，避免猜测语义）：

- `ensure_workmode_normal()`：写 `0x00 WORKMODE = 0x00`（可选步骤）
- `set_g_mode_raw(val)`：写 `0xA4`（语义待实测确认）
- `read_chip_id()`（`0xA3`）、`read_fw_ver()`（`0xA6`）、`read_vendor_id()`（`0xA8`）

### 错误模型

- I2C 总线错误与解析错误分离，便于固件侧计数与定位：
  - `Error::I2c(...)`
  - `Error::InvalidTouchCount(...) / InvalidEvent(...) / InvalidPointId(...)`
  - `ParseError::*`（供纯解析 API 返回）

## 设计 B：digital 固件集成（ESP32-S3）

### 任务模型（无 INT 不读 I2C）

新增 `touch_task`，遵循：

1) 等待 `CTP_INT`（默认 falling-edge）
2) 触发后短事务读取 `0x02..0x0E`（13 bytes）
3) 解析出触点事件并：
   - defmt 日志输出（坐标、id、event）
   - 将事件发送给 UI 绘制路径（例如 channel/queue），与显示刷新解耦
4) 立即释放 I2C 锁并 `yield`，避免阻塞 UI/通信任务

### INT 等待策略（edge 与 level 兼容）

硬件常见两类行为：

- falling-edge：中断为短脉冲或边沿触发（默认）
- low-level：INT 保持低电平直到读出/清除

设计上应支持两种等待模式：

- `wait_for_falling_edge()`：默认路径（FocalTech Qualcomm 参考驱动在 IRQ 注册中固定使用 `IRQF_TRIGGER_FALLING`）
- `wait_for_low()` +（必要时）限次/限时补读直到 `wait_for_high()`：用于“保持低”场景，避免 stuck-low 死循环

### Reset/上电时序（最小要求）

按 datasheet 的最小参数约束：

- `Trst >= 5ms`：RST 低电平保持时间
- `Trsi ≈ 300ms`：reset 后开始报点的时间窗口
- `Tpon ≈ 300ms`：上电后开始报点的时间窗口

固件策略（实现阶段）：

- 初始化阶段执行一次 RST 序列并等待 `>=300ms`
- 可选写 `WORKMODE=0x00` 确保普通模式
- 之后进入 “等待 INT → 短事务读 I2C” 的常态工作模式

### I2C0 共享（EEPROM + Touch）

现状：EEPROM 使用 `Mutex<...>` 持有 I2C0。

目标：两设备可并发调度，但 I2C 事务必须串行化，且触控读取不能长期占锁。

建议实现策略之一（实现阶段二选一）：

1) 总线级 `Mutex<I2c0>`：每个设备驱动在单次 read/write 前 lock，总线事务结束后立即 unlock。
2) 引入通用 shared-bus/hal-bus 适配层，为每个设备生成轻量 I2C 访问器（每次事务内部短持锁）。

触控每次读取固定 13 bytes，目标是将 I2C 锁持有压到最低，避免影响 EEPROM 读写时延上限。

### 坐标系映射与 UI 反馈

- LCD 分辨率为 `240×320`（固件常量）。
- 需提供可配置的坐标变换（旋转/镜像/交换 XY），以适配 FT6336U 上报坐标与 LCD 方向可能不一致的情况。
- 最小可视化：在 framebuffer 上绘制一个光点/十字跟随最新触点（至少点 1），并可按事件区分颜色。

### 可观测性（日志与计数器）

至少包含：

- `touch_int_count`：INT 触发次数
- `touch_i2c_read_count`：I2C 读取次数（应接近 INT 次数；low-level 场景可能略高）
- `touch_parse_fail_count`：解析失败次数
- 触点事件日志：`id/event/x/y`

### 性能/实时性与测量

- 目标：处理最高 ~100Hz 报点，不阻塞 UI/通信任务。
- 延迟测量方法（实现阶段）：在 `wait_for_*` 返回时记 `t0`，解析完成记 `t1`，日志输出 `t1-t0` 的 max/avg（或直方统计），目标 < 50ms（可按实测调整）。

## 风险点与待确认问题（Open Questions）

1) **CTP_INT 行为与电气**
   - 参考驱动建议 IRQ 触发方式为 falling-edge（见 Qualcomm 参考驱动 IRQ 注册处）。
   - 仍需确认：是否开漏/是否需要上拉、以及是否存在“保持低直到清除”的行为（影响是否需要 low-level 兼容路径）。
   - 验证：示波器/逻辑分析仪抓 INT；固件在 bring-up 模式下对比 `wait_for_falling_edge` 与 `wait_for_low` 两条路径的稳定性与丢事件情况。

2) **`0xA4 (G_MODE)` 语义不确定**
   - 文档疑似笔误，需要实测确认 0/1 的含义与对 INT 模式的影响。
   - 验证：读/写不同值后观察 INT 行为与报点节奏；记录 chip/fw/vendor id 便于追溯。

3) **坐标方向与映射**
   - 是否需要旋转/镜像/交换 XY？
   - 验证：按压屏幕四角，观察绘制点是否落在对应位置，必要时调整 transform 配置。

4) **触控高频与 EEPROM 访问冲突**
   - 触控频繁抢占 I2C0 会否影响 EEPROM 访问时延上限？
   - 验证：在连续触控压力下并行执行 EEPROM 读写，记录 EEPROM 操作耗时分布，必要时引入优先级/节流策略。

## 参考资料（可核查来源）

- 模组信息（CTP IC=FT6336U、CTP 引脚）：`docs/other-datasheets/p024c128-ctp.md`
- 数字板网表（CTP_RST/INT/SDA/SCL）：`docs/power/netlists/digital-board-netlist.enet`
- digital 固件现状（LCD 240×320；I2C0=GPIO8/9@400kHz async）：`firmware/digital/src/main.rs`
- FT6336U 特性与时序：`docs/other-datasheets/d-ft6336u-datasheet-v1-1.md`
- 参考驱动文本（地址 0x38、IRQ falling edge 约定）：`/Users/ivan/Sync/Ivan-Personal/Datasheets/Display/P024C128-CTP/Focaltech_Touch_FT6336U_Driver_for_Qualcomm_V3.2_20200422/docs/focaltech-ts.txt`
- 参考驱动 IRQ 注册（固定 falling edge）：`/Users/ivan/Sync/Ivan-Personal/Datasheets/Display/P024C128-CTP/Focaltech_Touch_FT6336U_Driver_for_Qualcomm_V3.2_20200422/focaltech_touch/focaltech_core.c`
- 参考寄存器宏（chip id/fw ver/vendor id）：`/Users/ivan/Sync/Ivan-Personal/Datasheets/Display/P024C128-CTP/Focaltech_Touch_FT6336U_Driver_for_Qualcomm_V3.2_20200422/focaltech_touch/focaltech_common.h`
- maybe-async-cfg 双模式范例：`/Users/ivan/Projects/Ivan/sc8815-rs/`
- esp-hal async GPIO Wait trait 实现：`/Users/ivan/.cargo/registry/src/*/esp-hal-1.0.0/src/gpio/embedded_hal_impls.rs`
