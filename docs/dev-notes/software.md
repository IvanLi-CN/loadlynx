# 软件开发笔记（数字/模拟板启动与串口链路）

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

1. **板载数字外设完结**
   - 记录数字域已就绪，插入约 10 ms 的启动延迟，确保控制板所有本地外设稳定。

### 模拟板片外（隔离域资源）

1. **TPS82130 使能**
   - 延时结束后把 `ALG_EN` 拉高，通过 FPC 启动 `5V_EN`，驱动模拟板上的 TPS82130SILR EN 引脚（`docs/power/netlists/analog-board-netlist.enet:5288-5308`），隔离 5 V 轨开始为模拟域供电。
2. **软复位模拟板（协议层，无需掉电）**
   - 数字侧通过 `SoftReset` 消息（0x26，`reason=fw_update`）请求模拟板进入安全态并清空故障/积分等内部状态。
   - 当前固件在启动时最多尝试 3 次发送 SoftReset 请求，每次间隔约 150 ms；若在重试窗口内收到带 `FLAG_IS_ACK` 的应答则认为软复位成功。
   - 若始终未收到 ACK，固件会在日志中给出类似 “soft_reset ack not received; proceed with caution” 的告警，但仍继续后续握手；推荐在 UI 中对这类情况额外标注“软复位可能未完成”并提示必要时进行电源循环。
3. **任务调度启动**
   - 启动任务执行器，此时模拟板 5 V 已经具备且软复位流程已尝试完成，可安全与 STM32/模拟前端协作。

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

## 4. 串口链路与任务分工（当前实现）

目标：在 ESP32‑S3 与 STM32G431 之间建立稳定的 UART 链路，并基于共享协议 crate（`loadlynx-protocol`）完成 FastStatus 遥测 + SetPoint/SoftReset/SetEnable/CalWrite 控制闭环。

### ESP32‑S3 侧（UART1 + UHCI DMA）

- 实例与引脚：`UART1`，TX=`GPIO17`，RX=`GPIO18`（见 `docs/interfaces/pinmaps/esp32-s3.md`）。
- 物理配置：`115200` baud，8N1；RX 侧使用较高 FIFO 满阈值（约 120 字节）与适度超时配置（`UART_RX_FIFO_FULL_THRESHOLD` / `UART_RX_TIMEOUT_SYMS`）。
- 链路实现：
  - 默认使用 `UART1 + UHCI DMA` 环形搬运（`uart_link_task_dma`），DMA 块交给 `SlipDecoder` 逐字节推送。
  - `feed_decoder` 负责从 SLIP 流中拆出完整帧，先用 `decode_frame` 做头部 + CRC 校验，再按 `msg` 分发：
    - `MSG_HELLO` → 更新 `HELLO_SEEN`/`LINK_UP` 标志；
    - `MSG_FAST_STATUS` → 调用 `apply_fast_status` 更新 `TelemetryModel`，记录电压/电流/温度与故障标志；
    - `MSG_SET_POINT`（带 `FLAG_IS_ACK`）→ 调用 `handle_setpoint_ack`，驱动 SetPoint 重传状态机；
    - `MSG_SOFT_RESET`（带 `FLAG_IS_ACK`）→ 调用 `handle_soft_reset_frame`，确认软复位握手。
  - 协议/SLIP 错误通过限频日志计数：`UART RX error: ...`、`protocol decode error (...)` 等。
- 链路健康：
  - 每次成功处理 `HELLO`/`FAST_STATUS`/ACK 帧都会刷新 `LAST_GOOD_FRAME_MS`；
  - `stats_task` 每秒检查一次该时间戳，若 >300 ms 未见有效帧则将 `LINK_UP=false` 并在日志中标记“link down”，SetPoint 发送任务会在 `LINK_UP=false` 时暂缓新的控制指令。

### STM32G431 侧（USART3 + 单任务主循环）

- 实例与引脚：`USART3`，TX=`PC10`，RX=`PC11`（见 `docs/interfaces/uart-link.md` 与 `loadlynx.ioc`）。
- 物理配置：`115200` baud，8N1。
- 结构：
  - `main` 是唯一的 Embassy task，负责外设初始化 + 采样 + FastStatus 打包与发送；
  - 另起一个 `uart_setpoint_rx_task` 使用 `RingBufferedUartRx + SlipDecoder` 接收控制帧。
- 主循环职责（约 20 Hz）：
  - 初始化时钟、VREFBUF、ADC1/ADC2、DAC1、USART3、`LOAD_EN_CTL/LOAD_EN_TS` 与板载 LED 后进入循环；
  - 每 50 ms：
    - 通过 VrefInt 计算当前 `vref_mv`；
    - 采样本地/远端电压、电流、5 V 轨、电源温度、两路 NTC 与 MCU 内部温度；
    - 根据传感器换算得到 `v_local_mv`/`v_remote_mv`、`i_local_ma`/`i_remote_ma`、`calc_p_mw`、三路温度与 `dac_headroom_mv`；
    - 做远端 sense 判定：电压在 0.5–55 V 且 ADC 原码远离饱和，3 帧进入 / 2 帧退出，驱动 `STATE_FLAG_REMOTE_ACTIVE`；
    - 基于 `LAST_RX_GOOD_MS` 与 300 ms 超时时间计算链路是否健康，超时则认为 link fault：LED1 以约 2 Hz 闪烁，并在 FastStatus 的 `state_flags` 中清除 `STATE_FLAG_LINK_GOOD`；
    - 结合 `ENABLE_REQUESTED`、`CAL_READY` 与 `FAULT_FLAGS` 计算 `effective_enable_with_fault`，据此决定目标电流与 DAC 输出；
    - 打包 `FastStatus`（包括 `fault_flags`）并通过 `encode_fast_status_frame + slip_encode` 从 USART3 发出。

### UART RX 任务与控制帧处理（G431）

`uart_setpoint_rx_task` 在独立任务中消费 USART3 RX 环缓冲，逐帧解析控制消息：

- `MSG_SET_POINT`：
  - 成功解析后更新 `TARGET_I_LOCAL_MA`，并记录 `LAST_SETPOINT_SEQ`；
  - 对首次或新 `seq` 应用 setpoint，重复 `seq` 视为重传只回 ACK 不改目标（幂等）；
  - 通过 `encode_ack_only_frame(MSG_SET_POINT)` 立即返回 ACK，数字侧基于 `FLAG_IS_ACK` 驱动重传状态机。
- `MSG_SOFT_RESET`：
  - 收到请求帧后调用 `apply_soft_reset_safing`：
    - 拉低 `LOAD_EN_CTL/LOAD_EN_TS`，硬件上断开功率级；
    - 将目标电流与 DAC 输出清零，清除 `FAULT_FLAGS`；
    - 稍作延时后重新拉高 `LOAD_EN_*` 以允许重新上电；
  - 随后重发一次 `HELLO` 供数字侧确认版本；
  - 同时刷新 `LAST_RX_GOOD_MS`。
- `MSG_SET_ENABLE`：
  - 更新 `ENABLE_REQUESTED`（true/false），主循环按 `enable && cal_ready && (fault_flags == 0)` 决定是否真正出力。
- `MSG_CAL_WRITE`：
  - 当前版本仅使用 `index=0` 的下行写入，payload 视为不透明；
  - 成功解析任意一帧即置 `CAL_READY=true`，作为 enable gating 条件之一；
  - 未实现上行 `CAL_READ` 或多块标定同步流程。

### ESP32‑S3 UI 与遥测映射（摘要）

- UI 实现位于 `firmware/digital/src/ui`，当前布局大致为：
  - 左侧三张主卡片：主电压/主电流/主功率，对应 `FastStatus` 中选取的 main voltage/current/power；
  - 右上角电压对：REMOTE 与 LOCAL 电压，`REMOTE_ACTIVE` 置位时以远端为主，否则 REMOTE 显示 `--.--` 且条形图归零；
  - 右中部电流对：CH1/CH2 电流条，映射 `i_local_ma` 与 `i_remote_ma`；
  - 底部 5 行状态文本：运行时间、两路散热片温度、MCU 温度以及故障概要（`FAULT OK` 或 `FAULT 0xXXXXXXXX`）。
- Telemetry 模型在后台持续积分 `energy_wh`，当前 UI 尚未单独绘制该值，但已在 `UiSnapshot` 中保留字段，便于后续扩展或上位机导出。
- UI 不直接控制任何安全逻辑，仅反映 `FastStatus` 内容；控制路径通过上文的 SetPoint/SoftReset/SetEnable 完成。
- 风扇 PWM / Tach 控制虽然在引脚与协议层预留了钩子，但当前固件尚未实现相应驱动与控制逻辑。

### 联调与期望日志

1. 先刷写 STM32 固件（analog），确认 probe 与供电正常；再刷写 ESP32‑S3 固件并打开监视串口。
2. 在 ESP32‑S3 监视窗口中应看到：
   - `LoadLynx digital firmware version: ...`；
   - `LoadLynx digital alive; initializing local peripherals`（本地外设初始化）；
   - `spawning uart link task (UHCI DMA)` 与 `SetPoint TX task starting`；
   - 随着链路稳定，周期出现 `fast_status ok (count=...)` 与 `stats: fast_status_ok=...` 等行。
3. 在 G431 RTT 日志中应看到：
   - `LoadLynx analog alive; init VREFBUF/ADC/DAC/UART (CC 0.5A, real telemetry)`；
   - VREFBUF/ADC 校准信息与周期性的 `sense: v_loc=...` 行；
   - SoftReset、CalWrite、SetEnable、fault latch 等事件日志。
4. 如长时间看不到 `fast_status ok` 或 UART/协议错误计数持续增加，请检查：
   - 板间隔离器与引脚方向（见 `docs/interfaces/uart-link.md`）；
   - 双端波特率/引脚是否一致（G431 使用 USART3 PC10/PC11；S3 使用 UART1 GPIO17/18）；
   - G431 侧是否正常启动并打印上述初始化/遥测日志。

当前链路已实现 `HELLO`、`FAST_STATUS`、`SET_POINT + ACK`、`SoftReset`、`SetEnable` 与单块 `CalWrite` 的 v0 最小闭环，其余消息类型与带宽规划见 `docs/interfaces/uart-link.md`。

## 5. STM32G431 模拟板：ADC 采样、CC 恒流与保护（当前实现）

本节简要描述 `firmware/analog/src/main.rs` 的当前架构。更细的协议字段与热设计仍以 `docs/interfaces/uart-link.md` 与 `docs/thermal/*` 为准。

### 5.1 启动流程与外设初始化

- 使用 Embassy executor，仅保留一个 `main` 任务配合一个 UART RX 任务：
  - 在 `main` 中初始化时钟、VREFBUF、ADC1/ADC2、DAC1、USART3、负载使能 GPIO（`LOAD_EN_CTL/LOAD_EN_TS`）以及板载 LED。
  - 将 `LOAD_EN_CTL` 与 `LOAD_EN_TS` 拉高，使 CC 通道处于“硬件允许输出但由固件 gating”的状态。
  - 通过 VREFBUF CSR 配置将内部基准设置到约 2.9 V 档位，为 ADC/DAC 提供稳定参考。
- 启动 UART 环形缓冲接收与 `uart_setpoint_rx_task` 后，主循环进入 20 Hz 采样与 FastStatus 发送节奏。

### 5.2 采样路径与 FastStatus 字段

每个 FastStatus 周期内，主循环完成：

- **参考电压与 5 V 轨**
  - 多次采样 VrefInt 计算当前 `vref_mv`，用于将其余 ADC 原码转换为 mV/mA。
  - 采样 `_5V_SNS` 得到模拟板 5 V 轨电压 `v_5v_mv`。
- **电流与电压**
  - 采样 `CUR1_SNS`/`CUR2_SNS`，根据硬件增益关系将测得电压换算为 `i_local_ma`/`i_remote_ma`（约等于 `2 × V_CUR[mV]`，覆盖 0–5 A 测试范围）。
  - 采样 `V_NR_SNS`（近端）与 `V_RMT_SNS`（远端），结合差分放大器缩放系数得到 `v_local_mv`/`v_remote_mv`。
  - 计算瞬时功率 `calc_p_mw = i_local_ma * v_local_mv / 1000`。
- **温度**
  - 采样两路 NTC（`TS1/TS2`）并通过 Steinhart–Hart 近似转换为 `sink_core_temp_mc` 与 `sink_exhaust_temp_mc`（单位 m°C）。
  - 使用片上温度传感器得到 `mcu_temp_mc`，同样以 m°C 上报。
- **远端 sense 判定**
  - 对 `v_remote_mv` 做范围与饱和检查：0.5–55 V 且 ADC 原码远离 0 与满量程。
  - 满足条件的连续 3 帧置位 `STATE_FLAG_REMOTE_ACTIVE`，失败连续 2 帧清除，形成 3 帧进入 / 2 帧退出的软判定。
- **链路健康指示**
  - 基于 `LAST_RX_GOOD_MS` 与 300 ms 超时时间计算 `link_fault`：
    - 正常时 `STATE_FLAG_LINK_GOOD` 置位且 LED 熄灭；
    - 超时则 LED 以约 2 Hz 闪烁，仅作为链路健康指示，不直接参与 enable gating。

### 5.3 使能 gating 与恒流控制

- 目标电流由数字板通过 `SetPoint` 下发，存入 `TARGET_I_LOCAL_MA`，单位 mA。
- 有效出力条件为：

  ```text
  effective_enable_with_fault =
      ENABLE_REQUESTED && CAL_READY && (FAULT_FLAGS == 0);
  ```

- 当上述条件不满足时，主循环强制目标电流为 0 mA，并将 DAC 输出拉到对应零点。
- 当条件满足时：
  - 将 `TARGET_I_LOCAL_MA` 限制在 `[0, 5_000]` mA 的安全区间；
  - 按线性比例将目标电流映射为 DAC 码值（0.5 A → `CC_0P5A_DAC_CODE_CH1`），并更新 DAC CH1；
  - 计算 `dac_headroom_mv = vref_mv - V_DAC`，用于观察环路是否即将打满。

主循环同时计算闭环误差 `loop_error = target_i_local_ma - i_local_ma` 并写入 FastStatus，供数字侧 UI 与诊断使用。

### 5.4 故障检测与 SoftReset safing

- **故障判定与锁存**
  - 使用协议 crate 中的 4 个 fault bit：
    - `FAULT_OVERCURRENT`：`i_local_ma > 5.5 A` 近似；
    - `FAULT_OVERVOLTAGE`：`v_local_mv > 55 V`；
    - `FAULT_MCU_OVER_TEMP`：`mcu_temp_mc > 110 °C`；
    - `FAULT_SINK_OVER_TEMP`：`sink_core_temp_mc > 100 °C`。
  - 一旦任意条件触发即将对应 bit OR 进 `FAULT_FLAGS`，并通过日志打印首次 latch；`FAULT_FLAGS` 在 FastStatus 中原样上报。
  - `FAULT_FLAGS != 0` 会参与 `effective_enable_with_fault` 计算，将目标电流强制压到 0 mA，实现“故障即刻失能”的兜底保护。
- **软复位 safing（数字侧触发）**
  - 数字板通过 `SoftReset` 消息请求模拟侧清空状态；模拟侧收到请求后：
    1. 拉低 `LOAD_EN_CTL/LOAD_EN_TS`，硬件上断开功率级；
    2. 将目标电流与 DAC 输出清零，清除 `FAULT_FLAGS`；
    3. 稍作延时后重新拉高 `LOAD_EN_*` 以允许重新上电；
    4. 重发一次 `HELLO`，供数字侧确认版本并重新握手；
    5. 等待新的 `SetEnable(true)` 与 `SetPoint` 重新建立闭环。

### 5.5 历史 Bring‑up 规划（简述）

早期版本中，G431 侧仅实现 UART Echo 与固定 0.5 A 输出，规划是拆分为 `adc_sampler_task`、`cc_supervisor_task` 与 `uart_telemetry_task` 三个 Embassy 任务。本节前述当前实现已经在单任务主循环中落地了绝大部分规划要点（ADC 采样、恒流设定、基础保护与 FastStatus 遥测），后续若需要多通道扩展或更复杂的环路控制，可在此基础上再拆分任务与结构。

