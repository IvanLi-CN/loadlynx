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

## 4. 串口通信（最小验证）

目标：实现 ESP32‑S3 与 STM32G431 之间最小可用的 UART 链路，用于“可达性”验证（非最终协议）。

### ESP32‑S3 侧（UART1）

- 实例与引脚：`UART1`，TX=`GPIO17`，RX=`GPIO18`（见 `docs/interfaces/pinmaps/esp32-s3.md:120-121`）。
- 配置：`460800` baud，8N1，无流控。
- 代码位置：`firmware/digital/src/main.rs` 中 `uart_link_task`。
- 行为：周期发送 `"PING\n"`，非阻塞读取并打印接收字节计数（defmt）。

构建/烧录：
- 构建：`(cd firmware/digital && cargo +esp build)`
- 烧录：`scripts/flash_s3.sh [--port /dev/tty.*]`

### STM32G431 侧（USART1）

- 实例与引脚：`USART1`，TX=`PA9`，RX=`PA10`（如与硬件不符，请在源码中按板图调整）。
- 配置：`460800` baud，8N1。
- 代码位置：`firmware/analog/src/main.rs`（`Uart::new(...)` + 简单回环）。
- 行为：按字节回显（echo），收到什么回什么。

构建/烧录：
- 构建：`(cd firmware/analog && cargo build)` 或 `make g431-build`
- 烧录运行：`make g431-run`（基于 `probe-rs`）

### 联调与期望日志

1. 先刷写 STM32 固件，后刷写 ESP32‑S3 固件并打开监视串口。
2. 在 ESP32‑S3 监视窗口中应看到：
   - `LoadLynx digital alive...`（本地外设初始化）
   - `UART link task starting`（串口任务启动）
   - 周期 `PING` 后收到的 `uart rx N bytes`（来自 STM32 的回显），表示链路可达。
3. 如无回显，请检查：
   - 板间隔离器与引脚方向（见 `docs/interfaces/uart-link.md`）。
   - 双端波特率/引脚是否一致；必要时在固件中调整。

备注：当前为“功能验证”最小实现，未实现 SLIP/CBOR/CRC 等正式协议与可靠性控制，后续将按 `docs/interfaces/uart-link.md` 逐步演进。
