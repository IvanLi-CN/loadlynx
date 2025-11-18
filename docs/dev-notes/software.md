# 软件开发笔记（ESP32-S3 启动流程）

本文档说明 ESP32-S3 固件在上电后执行的初始化流程，并区分 MCU 片上（SoC 内部资源）与片外（经 FPC/连接器到板级器件）的步骤，供调试与审计使用。

## 1. 初始化顺序总览

### MCU 片上（SoC 内部资源）

1. **芯片与时钟上电（ROM → 应用入口）**
   - 运行 HAL 默认流程，完成最小化时钟、复用及电源域配置。
2. **关断模拟 5 V 控制路径**
   - GPIO34 立即设为推挽输出并保持低电平，确保 FPC Pin 14 (`ALG_EN`) 在 MCU 接管后仍保持关断（参考 `docs/power/netlists/digital-board-netlist.enet:1665-1694` 与 `docs/power/netlists/analog-board-netlist.enet:5074-5094`）。
   - 注意：真正的上电抖动抑制依赖 TPS82130 内部 400 kΩ 下拉与板上 RC；固件仅保证“接管后的默认关断”。
3. **本地数字外设初始化**
   - 完成 SPI2、显示控制 GPIO、背光 PWM 以及执行框架所需的缓冲区/资源配置，此时尚未调度任何任务。

### 控制板片外（同板其他数字资源）

4. **板载数字外设完结**
   - 记录数字域已就绪，插入约 10 ms 的启动延迟，确保控制板所有本地外设稳定。

### 模拟板片外（隔离域资源）

5. **TPS82130 使能**
   - 延时结束后把 `ALG_EN` 拉高，通过 FPC 启动 `5V_EN`，驱动模拟板上的 TPS82130SILR EN 引脚（`docs/power/netlists/analog-board-netlist.enet:5288-5308`），隔离 5 V 轨开始为模拟域供电。
6. **任务调度启动**
   - 启动任务执行器，此时模拟板 5 V 已经具备，可安全与 STM32/模拟前端协作。

## 2. 调参要点

- 启动延迟默认为 10 ms，可依据实测上电波形在固件中调整；若参数变化请同步此笔记。
- 若硬件改版后更换 5 V 模块或引脚，请同时更新：
  1. 固件中的 GPIO 选择逻辑；
  2. 本笔记以及相关网表引用，确保“软件 → 硬件”链路文档保持一致。

## 3. 快速验证

1. 构建：`(cd firmware/digital && cargo +esp build)`。
2. 烧录：`scripts/flash_s3.sh`（按需带端口参数）。
3. 观察串口日志：应依次看到系统上电、数字外设就绪、TPS82130 5 V 使能确认等信息。
4. 同时测量 FPC Pin 14/`ALG_EN`，确认延时后由 0 V 拉至 3.3 V，并验证模拟板 5 V 轨启动顺序。

以上流程为当前 ESP32-S3 固件的权威初始化记录；若固件或硬件改版，请同步修订本笔记。

## 4. 串口通信（最小验证 → FAST_STATUS 流）

目标：在 ESP32‑S3 与 STM32G431 之间建立稳定的 UART 链路，并基于共享协议 crate（`loadlynx-protocol`）持续传输 FAST_STATUS 遥测帧。

### ESP32‑S3 侧（UART1）

- 实例与引脚：`UART1`，TX=`GPIO17`，RX=`GPIO18`（见 `docs/interfaces/pinmaps/esp32-s3.md:120-121`）。
- 配置：`115200` baud，8N1，无流控；RX 侧使用较高 FIFO 满阈值与适度超时配置（见 `firmware/digital/src/main.rs` 中 `UartConfig`）。
- 代码位置：`firmware/digital/src/main.rs` 中 `uart_link_task` + `feed_decoder`。
- 行为：
  - 持续从 UART1 读取字节流并送入 `SlipDecoder`，以 SLIP 帧边界拆分完整帧。
  - 对每个完整帧调用 `decode_fast_status_frame`，解析出 `FastStatus`。
  - 将解析结果写入 UI 模型（`TelemetryModel::update_from_status`），并周期打印 `fast_status ok (count=...)`（默认每 32 帧一次）。
  - UART/协议错误通过限频日志计数：`UART RX error: ...`、`protocol decode error (...)` 等。

构建/烧录：
- 构建：`(cd firmware/digital && cargo +esp build --release)` 或 `make d-build`
- 烧录：`scripts/flash_s3.sh --release [--port /dev/tty.*]` 或 `make d-run PORT=/dev/tty.*`

### STM32G431 侧（USART3）

- 实例与引脚：`USART3`，TX=`PC10`，RX=`PC11`（见 `docs/interfaces/uart-link.md` 与 `loadlynx.ioc`）。
- 配置：`115200` baud，8N1。
- 代码位置：`firmware/analog/src/main.rs` 中 `Uart::new(...)` 与主循环。
- 行为：
  - 以约 60 Hz 周期构造 `FastStatus` 模拟数据（电压、电流、功率、温度等字段）。
  - 使用 `encode_fast_status_frame` 生成带 CRC 的帧，再通过 `slip_encode` 封装为 SLIP 流。
  - 通过 USART3 发送整帧；若 UART 写入失败，会打印 `uart tx error; dropping frame` 但继续重试。

构建/烧录：
- 构建：`(cd firmware/analog && cargo build --release)` 或 `make a-build`
- 烧录运行：`make a-run PROBE=<VID:PID[:SER]>` 或 `scripts/flash_g431.sh release PROBE=<...>`

### 联调与期望日志

1. 先刷写 STM32 固件（analog），确认 probe 与供电正常；再刷写 ESP32‑S3 固件并打开监视串口。
2. 在 ESP32‑S3 监视窗口中应看到：
   - `LoadLynx digital alive; initializing local peripherals`（本地外设初始化）
   - `UART link task starting`（串口任务启动）
   - 随着链路稳定，周期出现 `fast_status ok (count=...)` 以及周期性的 UI 刷新日志（如有）。
3. 如无 `fast_status ok` 日志或 UART/协议错误计数持续增加，请检查：
   - 板间隔离器与引脚方向（见 `docs/interfaces/uart-link.md`）。
   - 双端波特率/引脚是否一致（G431 使用 USART3 PC10/PC11；S3 使用 UART1 GPIO17/18）。
   - G431 侧是否正常启动并打印 `LoadLynx analog alive; streaming mock FAST_STATUS frames`。

当前链路已经实现 SLIP/CBOR/CRC 的最小可用版本，后续消息集与可靠性（ACK/重试/心跳）将按 `docs/interfaces/uart-link.md` 中的协议规划逐步扩展。

## 5. STM32G431 模拟板：电压/电流采样与 CC 恒流（0.5 A/通道测试版）

本节针对 `firmware/analog/`（STM32G431）侧的基础功能规划：在不引入复杂协议与 UI 交互的前提下，先打通本地电压/电流采样链路，并实现“固定 0.5 A/通道”的 CC 恒流模式，用于功率板与环路的 Bring‑up。

### 5.1 目标与边界

- 功能目标
  - 建立稳定的 ADC 采样路径：
    - 采样每个通道的负载电流（分流电阻 → ADC）。
    - 采样负载端电压（分压 → ADC）。
    - 预留若干通道给温度/电源监控（NTC、VBUS/VIN 等），但当前阶段只保证接口打通。
  - 在 MCU 侧提供简单 CC 功能：
    - 上电后使用固定 setpoint=0.5 A/通道（测试阶段常量），通过 DAC/设定路径驱动模拟 CC 环路。
    - 提供基础的过流/过压/过温保护钩子（先做检测与报警，再逐步完善关断策略）。
- 范围与假设
  - CC 闭环主带宽由板上运放 + 分流电阻构成的模拟环路提供（参见 `docs/boards/analog-board.md` 与 `docs/components/opamps/selection.md`）；MCU 仅负责设定与监督，不直接做高速数字环路。
  - ADC、DAC 资源按 `loadlynx.ioc` 当前配置使用（ADC1/ADC2、DAC1），具体通道/引脚映射在实现阶段根据网表与 CubeMX 再细化到代码。

### 5.2 硬件路径梳理（基于现有文档）

- 电流采样
  - 分流电阻 Kelvin 接入 ADC（`docs/boards/analog-board.md`）；假定每个功率通道均有独立分流+运放。
  - 至少一组 ADC 通道用于差分或带增益的电流采样（`loadlynx.ioc` 中 ADC2 存在差分配置）。
- 电压采样
  - 负载端电压通过分压 + RC 接入 ADC1 单端通道，用于：
    - 显示与日志（远/近端电压）。
    - 检测是否进入“电压钳位”区（为后续 CV/CP 模式预留）。
- DAC 与设定路径
  - DAC1 输出作为运放 / 比较环路的电流设定基准，驱动 NMOS 栅极缓冲（参见 TPS22810 + OPA2365 相关文档）。
  - 固件侧以“电流设定（安培）→ DAC 码值”的一阶线性模型进行标定，系数从实测中回填。
- 保护与监控
  - 内部通道：利用 Vref、Vbat、温度传感器用于自校准与监控供电/结温。
  - 外部通道：预留 NTC 采样通道，用于后续实现热降额与关断逻辑。

### 5.3 固件架构与任务划分

现状：`firmware/analog/src/main.rs` 仅实现 UART3 Echo，未使用 `Spawner` 与 Embassy 任务模型。

规划中的任务划分（Embassy executor）：

1. `main` 启动阶段
   - 初始化时钟、GPIO、ADC1/ADC2、DAC1、USART3。
   - 启动以下异步任务：
     - `adc_sampler_task`：周期性采样所有监控通道。
     - `cc_supervisor_task`：基于采样结果与 setpoint 执行 CC 监督与限幅。
     - `uart_telemetry_task`：通过 UART3 输出最小调试信息（电流/电压快照、告警）。

2. `adc_sampler_task`
   - 使用 Embassy 的 ADC 驱动对一组通道轮询采样，形成结构化数据：
     - `I_sense[ch]`：每个通道的电流原始码值。
     - `V_load[ch]`：每个通道的负载电压原始码值。
     - `T_ntc[ch]` / `V_in` / `V_ref`：按需要追加。
   - 采样周期：先以“数百 Hz–几 kHz”级别轮询（具体频率根据后续环路/噪声测试再定），通过 `embassy_time::Ticker` 或定时触发机制实现。
   - 对每个通道做简单的滑动平均/中值滤波，减小噪声对保护逻辑的瞬时触发。

3. `cc_supervisor_task`
   - 维护每个通道的目标电流 `I_set[ch]`，当前阶段固定为 0.5 A。
   - 将 `I_set[ch]` 通过线性映射转换为 DAC 码值：
     - `dac_code[ch] = k_gain[ch] * I_set[ch] + k_offset[ch]`。
     - `k_gain / k_offset` 在 Bring‑up 阶段通过实测标定后写入常量或查表。
   - 对 DAC 输出添加限幅与斜率限制：
     - 限幅：确保 DAC 输出不超过功率级允许的最大电流/电压。
     - 斜率限制：后续支持从 0 A → 目标电流的软启动，避免瞬时电流阶跃。
   - 基于采样值实现基础保护：
     - 过流：若 `I_sense[ch]` 明显高于目标且持续一定时间，拉低对应通道设定（甚至硬关断）。
     - 过压：若 `V_load[ch]` 超过设计上限，降低设定或关断通道。
     - 过温：若 NTC 显示温度超限，执行降额（降低 setpoint）或整体关断。

4. `uart_telemetry_task`
   - 维持当前的 UART3 链路，用于：
     - 周期输出简短状态行（如 `CH1: 0.50A 12.0V OK`）。
     - 输出告警事件（OV/OC/OT）。
   - 当前阶段仍然可以保持简单 Echo 功能，便于上位机简单交互；后续再按 `docs/interfaces/uart-link.md` 替换为正式协议。

### 5.4 实现步骤（建议开发顺序）

1. **基础外设初始化**
   - 在 G431 固件中初始化 ADC1/ADC2 与 DAC1，确认可对任意单通道进行采样与输出。
   - 通过 UART3 打印单通道的原始 ADC 码值与 DAC 输出对应的实际电流/电压，完成“单点”连通性验证。
2. **多通道采样与数据结构**
   - 定义 `ChannelConfig` / `ChannelState` 结构体，包含：
     - ADC 通道索引、DAC 通道索引。
     - 标定系数（A/LSB、V/LSB）。
   - 在 `adc_sampler_task` 中轮询多通道，并将结果写入 `ChannelState`。
3. **固定 0.5 A CC 模式**
   - 在上电初始化后，默认把所有通道的 `I_set[ch]` 置为 0.5 A，并通过 DAC 输出对应码值。
   - 通过长时间运行（数十分钟）观察散热与稳定性，结合 `docs/thermal/*` 文档评估是否需要降额。
4. **保护与限幅**
   - 基于实测数据确定过流/过压/过温阈值，先实现“告警不关断”，再逐步加入关断/降额逻辑。
5. **对接数字板与 UI**
   - 在模拟侧确认 CC 0.5 A 测试稳定后，再通过 UART 协议与 ESP32‑S3 对接，将 setpoint/测量值纳入 UI 与上位机控制。

以上设计作为 G431 模拟板电压/电流采样及固定 0.5 A 恒流 Bring‑up 的基础规划，后续若硬件参数或环路补偿有更新，请同步修订本节（特别是 ADC/DAC 通道映射与标定模型）。

