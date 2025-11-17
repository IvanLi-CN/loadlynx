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
