# MCU↔MCU 串口通信问题说明与排查方案（记录规范 & 实测数据）

## 背景

- 数字板（ESP32‑S3）通过 UART1 与模拟板（STM32G431）进行 MCU↔MCU 通信，协议为自定义帧 + SLIP 封装。
- 数字板同时驱动 ST7789 显示屏（SPI2，60MHz），UI 每帧渲染整屏像素，近期已改为“分块渲染 + 块间让出”，随后启用 SPI DMA 并对齐帧缓冲。

## 现象（症状）

- 运行中持续出现 UART 硬件 FIFO 溢出：`UART RX error: FifoOverflowed`。
- 协议解码错误持续累积：`invalid version`、`payload length mismatch`、`length mismatch` 等，表现为 SLIP 帧被截断/粘包后 CRC/长度校验失败。
- `fast_status ok` 计数稳定增加，说明链路未中断，但存在持续丢字节/破帧。

## 初步结论

- 根因为“读取不够及时”导致 UART RX FIFO 溢出，继而破坏帧边界与 CRC。
- 触发因素叠加：
  - 显示渲染与 SPI 传输形成较重的执行片段，即便分块后仍存在较明显的调度占用；
  - 启用 SPI DMA 后，事务在启动/等待阶段更“粗粒度”，配合当前 UART 配置（阈值=1、超时=1）产生高频中断压力；
  - 中断压力 + 显示负载叠加，导致 `uart_link_task` 获得 CPU 的机会不足，RX FIFO 易溢出。

## 当前实现快照（关键代码位置）

- 帧率上限：`DISPLAY_MIN_FRAME_INTERVAL_MS`（当前 33ms ≈ 30FPS，与 analog 侧 30Hz FAST_STATUS 对齐）
  - `firmware/digital/src/main.rs:57–58`
- 分块参数：`DISPLAY_CHUNK_ROWS`（默认 8 行/块），`DISPLAY_CHUNK_YIELD_LOOPS`（默认 6）
  - `firmware/digital/src/main.rs:61–62`
- UART RX 阈值/超时：`with_fifo_full_threshold(120)`，`with_timeout(10)`
  - `firmware/digital/src/main.rs:900–908`
- SLIP 解码器容量：`SlipDecoder<FAST_STATUS_SLIP_CAPACITY>`（默认 1024）
  - 初始化与使用：`firmware/digital/src/main.rs:94, 648, 729`
- 模拟板 FAST_STATUS 周期：`FAST_STATUS_PERIOD_MS = 1000 / 30`（≈30Hz）
  - `firmware/analog/src/main.rs:20`

## 排查与优化步骤（按优先级）

### 推荐优化路径（按阶段刷新）

1) **UART UHCI DMA（现行主方案）**
   - 状态：单板（显示关闭）场景已验证 0 溢出，下一步是逐步恢复显示 DMA + 正常帧率。
   - 基线设置：`DMA_CH1`、`rx_buf=1024–2048`、`chunk_limit=buf_len`、`UART_RX_FIFO_FULL_THRESHOLD=32`、`UART_RX_TIMEOUT_SYMS=2`，日志统计周期 1s。
   - 行动：优先确保 UHCI DMA 版本为默认构建；保留旧的轮询实现备用，便于快速 A/B。
   - 观察：`uart_rx_err_total`、`fast_status ok`、`decode_errs`，以及 DMA 回调/任务栈占用。

2) **显示让出 + 帧率限制**
   - 目标：在 UHCI DMA 运行时减轻 UI 对 CPU 的竞争，避免任务饿死。
   - 建议重新扫描：`DISPLAY_MIN_FRAME_INTERVAL_MS=40–80`、`DISPLAY_CHUNK_ROWS=8–12`、`DISPLAY_CHUNK_YIELD_LOOPS=2–6`；每个组合记录 UI 体感与 UART 指标。
   - 若需要对照，可暂时关闭 `ENABLE_DISPLAY_SPI_UPDATES` 以验证串口单独表现。

3) **UART 阈值/超时调参**（与 2 并行）
   - UHCI DMA 状态下仍需调参以兼顾中断频率与每次搬运量。
   - 建议区间：`UART_RX_FIFO_FULL_THRESHOLD=16–64`、`UART_RX_TIMEOUT_SYMS=2–4`，观察 DMA 中断节奏与任务加载。

4) **A/B：波特率、FAST_STATUS、显示帧率回退**
   - 若仍存在 decode_errs 过高或 DMA 崩溃，可降至 `115200`、降低 `FAST_STATUS_HZ` 至 20Hz、或将帧率压到 ≈20FPS，以定位瓶颈。

5) **SPI 事务形态与接收缓冲冗余**
   - 关闭 SPI DMA / 调整双缓冲，或将 `SlipDecoder` 扩大到 `1024+` 并启用 seq 统计，用于观察是否仍有破帧。
   - 需要时继续深挖 DMA chunk/描述符参数，或探查 UHCI DMA 与 `esp-hal` 的交互问题。

## 记录规范（每次实验务必填写）

- **基础信息**
  - `build version`（analog/digital 各自的 fw version 日志行；确保与本地 tmp/{analog|digital}-fw-version.txt 一致）
    - analog 侧即便未改动也要写明版本字符串与 `tmp/analog-fw-version.txt` 来源；缺失时实验视为无效。
    - 记录本次 `loadlynx-devd` bounded logs/monitor（通过 `loadlynx monitor ...` 或 CLI/devd 日志导出汇总），方便后续对照。
  - `test duration (s)`
  - `baud` / `FAST_STATUS_HZ`（发送侧频率）
  - `DISPLAY_MIN_FRAME_INTERVAL_MS`、`DISPLAY_CHUNK_ROWS`、`DISPLAY_CHUNK_YIELD_LOOPS`
  - UART 阈值：`UART_RX_FIFO_FULL_THRESHOLD`、`UART_RX_TIMEOUT_SYMS`
  - 特殊开关：`ENABLE_DISPLAY_SPI_UPDATES`、`ENABLE_UART_LINK_TASK`、SPI DMA on/off
- **核心指标（平均/总量/速率）**
  - `uart_rx_err_total` 变化量与增长率（次/秒），分类型记录：`Rx(FifoOverflowed)` / 其他
  - `PROTO_DECODE_ERRS` 变化量与增长率（次/秒），记录突出错误类型
  - `fast_status ok` 总数与节奏（是否按预期帧率递增）
  - 若使用序号/seq，可记录丢帧估计（缺口/秒）
- **UI 性能**
  - 平均/中位 `dt_ms`（可用显示帧日志估计）
  - 是否出现卡顿/掉帧肉眼可见
- **日志摘录**
  - 关键 warn 行（例：`UART RX error: Rx(FifoOverflowed)` 首尾各几条）
  - 代表性的 `fast_status ok` 行（确认节奏）

> 以上信息缺一不可，确保测试不白测。

## 验收标准（A/B 验证 60s）

- UHCI DMA 版本：
  - `uart_rx_err_total` 维持 0；DMA 回调线程无 panic/timeout。
  - 在 `DISPLAY_MIN_FRAME_INTERVAL_MS<=50`、`DISPLAY_CHUNK_ROWS<=12`、`DISPLAY_CHUNK_YIELD_LOOPS<=4` 的正常 UI 负载下，`fast_status ok` 节奏与发送端一致。
- 轮询 fallback（若用于对照）：
  - `uart_rx_err_total` 增长率 < 1 次/秒（理想为 0）。
  - `decode_errs` 不增长或仅偶发（≪ 0.1 次/秒）。
- 所有实验需附 analog/digital 版本、日志路径，并确认 UI 帧率仍满足体验要求（无明显卡顿）。

## 复现实验与日志观测

- 构建：
  - `just d-build`（必要时 `FEATURES=... just d-build`）；analog 若也升级：`just a-build`。
- 烧录：
  - digital：`loadlynx flash digital --device <saved-id> --artifact <artifact-id>`；analog：`loadlynx flash analog --device <saved-id> --artifact <artifact-id>`。
- 日志采集：
  - `loadlynx monitor digital --device <saved-id> --reset`（停止时用 Ctrl+C）。
  - 只看已有日志：使用 CLI/devd bounded log 读取入口。
  - 双板同时观察：开两个终端分别运行 `loadlynx monitor analog --device <saved-id> --reset` 与 `loadlynx monitor digital --device <saved-id> --reset`。
- 关注要点：
  - 先读取 `tmp/digital-fw-version.txt` / `tmp/analog-fw-version.txt`，与日志开头的 version 行比对，确认一致再评估。
  - UART：`UART RX error: FifoOverflowed (total=...)`、DMA panic/nmi、`uhci_dma_rx_chunk...` 之类统计。
  - 协议：`protocol decode error (...)` 或 `decode_errs` 的速率。
  - 节奏：`fast_status ok (count=...)`、UI 帧日志、`DISPLAY dt_ms`。

## UHCI DMA 配置与注意事项

- 编译期开关：`ENABLE_UART_UHCI_DMA=true`（`firmware/digital/src/main.rs:81`）。禁用后将回到 async 轮询版本，用于快速对照。
- DMA 管线：`Uhci::new(UART1, UHCI0, DMA_CH1)`（同文件 831-879 行）并启用 `esp_hal::dma_buffers!(UART_DMA_BUF_LEN)`；推荐 `UART_DMA_BUF_LEN=1024` 首测，必要时扩到 2048 但需同步调整 `chunk_limit`。
- 配置要点：
  - `with_fifo_full_threshold(UART_RX_FIFO_FULL_THRESHOLD)` 与 `with_timeout(UART_RX_TIMEOUT_SYMS)` 仍生效；默认 32 / 2。
  - `uhci.apply_rx_config(...chunk_limit)` 必须 ≤ buffer 长度且 ≤4095。过大时会导致 `Breakpoint on ProCpu / Cp0Disabled`。
  - `DmaRxBuf` 由 `uart_link_task_dma` 独占，ISR 将数据放入 slip 解码器后及时 `recycle_desc`，否则 UHCI 报 chunk 枯竭。
  - `stats_task` 每秒报告 `fast_status_ok/decode_errs/uart_rx_err_total`，用于观察 DMA 稳定性。
- 调参流程：
  1. 在 `ENABLE_DISPLAY_SPI_UPDATES=false` 时确认 DMA 稳定（0 溢出 + 0 panic）。
  2. 逐步恢复显示分块、帧率，再观察 decode_errs；若 decode_errs 仍高，优先扩大 `SlipDecoder` 或在 analog 端 dump 原始 seq 以定位。
  3. 若需 A/B，切换 `ENABLE_UART_UHCI_DMA=false` 并重新运行日志脚本以确认问题随模式变化。

## 附：当前实现的关键点

- UART 接收路径现包含三个形态：
  - 首选：`ENABLE_UART_UHCI_DMA=true` 时通过 UHCI DMA 环形搬运，由 `uart_link_task_dma` 处理（详见上节）。
  - 备用：`ENABLE_UART_UHCI_DMA=false` 时走轮询式 `uart.read()`（无 DMA），在 `230400` + 60Hz 下可维持 0 溢出但 CPU 占用高。
  - Legacy：`read_async()` 版本已知在本项目负载下会频繁 `FifoOverflowed` 与协议解码错误，仅用于对照或问题回溯。
- 分块渲染 + 块间 `yield_now()` 已启用；
- SPI DMA 已启用且 framebuffer 32B 对齐；调试阶段可通过 `ENABLE_DISPLAY_SPI_UPDATES=false` 禁用所有 SPI 推屏，以单独验证串口链路。
- 仍建议优先通过“更细分块、更高让出频率、适度降帧”和“降低 UART 中断压力”两方向收敛；必要时采用 UART UHCI DMA。

## 外部参考与 HAL 版本记录

- 数字板使用的 HAL 版本：`esp-hal = \"=1.0.0-rc.1\"`（`firmware/digital/Cargo.toml`）。
  - 对应 Git tag：`esp-hal-v1.0.0-rc.1`，tag 时间：`2025-10-13T16:54:43Z`，指向 commit `7757c381a0bfb9c4f881bf6aab406beb257aec06`。
  - 官方 `esp-hal-v1.0.0` release 说明中指出：1.0.0 与 1.0.0-rc.1 之间无迁移差异，可视为接近最终 1.0 实现。

- Upstream 已知的 async UART 行为问题（ESP32-S3 + Embassy 负载场景）：
  - Issue `#3144`：`impl embedded_io_async::Read for UartRx<'_, Async> is very unreliable.`
    - 描述在多任务 + WiFi 负载下，基于 `read_async()` 的 UART 接收在高负载时存在可靠性问题（错过事件、溢出、丢字节）。
  - Issue `#3168`：`esp32s3: UART rx_fifo_count() produces garbage after RxError::FifoOverflowed.`
    - 描述硬件 FIFO 溢出后内部状态错乱、后续读出垃圾数据。
  - PR `#3142`：`UART: make async operations cancellation-safe, update others for consistency.`
    - 合并时间：`2025-02-20T08:34:10Z`，针对 async UART 做了较大改动并修复部分已知问题。
    - 由于 `esp-hal-v1.0.0-rc.1` 的 tag 晚于该 PR 且基于 `main`，本项目使用的 1.0.0-rc.1 **已经包含上述修复**。

- 在 `esp-hal 1.0.0-rc.1` 上的本项目结论：
  - 在 `UART_BAUD=230400`、FAST_STATUS≈60Hz 的连续流场景中，使用 `read_async()` 的实现即便在关闭显示 SPI 负载的情况下仍会出现：
    - 持续的 `RxError::FifoOverflowed`；
    - 频繁的协议解码错误（`invalid version`/`length mismatch` 等）。
  - 在相同发送端配置下，将接收端改为轮询 `uart.read()` 抽干 FIFO（无数据时主动 `yield_now()`）后：
    - `uart_rx_err_total` 不再增长；
    - `fast_status ok` 计数按预期节奏稳定增加。
  - 综合 upstream 报告与本地实验，当前版本下的 async UART `read_async()` 在本项目的高负载链路中仍存在调度/时序风险，需通过轮询或 UART UHCI DMA 等方式规避。

## 经验补充：MCU↔MCU SetPoint 链路在 DMA + Embassy 场景下的可靠用法

- 场景：数字板（ESP32‑S3）通过 UART1 + SLIP + CBOR 向模拟板（STM32G431）周期性发送 `SetPoint { target_i_ma }` 控制帧。
- 问题症状（模拟板侧）：
  - `uart_setpoint_rx_task` 逐字节读取 `UartRx<'static, Async>`（DMA 模式）时，只能看到类似
    `0xc0, 0x01, 0x05, 0x00, 0x58, 0xc0`、`[0x01, 0x22, 0x00, 0x00, 0x02, ...]` 的残缺 SLIP 帧；
  - `SlipDecoder<128>` 重构出的 `frame.len()` 只有 4 或 6 字节，`decode_set_point_frame()` 一直报 `InvalidPayloadLength`；
  - 数字板 log 显示发送的 SLIP 帧完整且 CRC 正确（15 字节，首尾 0xC0）；
  - 降低 FAST_STATUS 周期到 20 Hz、关闭 analog→digital FAST_STATUS TX 之后，症状依旧。
- 根因总结：
  - 在 Embassy STM32 HAL 中，`UartRx<'_, Async>` + DMA **并不保证在两次 `read()` 调用之间不会丢字节**；
  - 官方文档建议：在需要“持续接收、不中断”的场景，必须使用
    `UartRx::into_ring_buffered(&mut [u8]) -> RingBufferedUartRx` 或 `BufferedUartRx`；
  - 本项目原先在模拟板上用单字节 `read(&mut [u8; 1]).await` 驱动 SlipDecoder，在数字板持续以 230400 8N1 发送 SLIP 帧时，
    由于任务调度/中断间隙，DMA 在后台覆盖了 ring 中间段，造成 SetPoint 帧中部字节被“吃掉”。
- 可靠方案（已在本仓库验证）：
  - 在 analog 上，将 `UartRx<'static, Async>` 转为带环形缓冲的 `RingBufferedUartRx<'static>`：

    ```rust
    use embassy_stm32::usart::{RingBufferedUartRx, Uart, UartRx, UartTx, Config as UartConfig, ...};
    use static_cell::StaticCell;

    let uart = Uart::new(
        p.USART3, p.PC11, p.PC10, Irqs, p.DMA1_CH1, p.DMA1_CH2, uart_cfg,
    ).unwrap();

    let (mut uart_tx, uart_rx): (UartTx<'static, UartAsync>, UartRx<'static, UartAsync>) =
        uart.split();

    static UART_RX_DMA_BUF: StaticCell<[u8; 128]> = StaticCell::new();
    let uart_rx_ring: RingBufferedUartRx<'static> =
        uart_rx.into_ring_buffered(UART_RX_DMA_BUF.init([0; 128]));
    ```

  - `uart_setpoint_rx_task` 中使用较小的临时缓冲做分块读取，并逐字节喂给 `SlipDecoder`：

    ```rust
    let mut decoder: SlipDecoder<128> = SlipDecoder::new();
    let mut buf = [0u8; 32];

    loop {
        match uart_rx.read(&mut buf).await {
            Ok(n) if n > 0 => {
                for &b in &buf[..n] {
                    // 可选：前若干字节打印调试日志
                    match decoder.push(b) {
                        Ok(Some(frame)) => {
                            // 此时 frame.len() 恢复到完整 13 字节
                            let (_hdr, sp) = decode_set_point_frame(&frame)?;
                            // clamp 到 TARGET_I_MIN/MAX_MA 后写入 TARGET_I_LOCAL_MA
                        }
                        Ok(None) => {}
                        Err(_) => decoder.reset(),
                    }
                }
            }
            Ok(_) => {}
            Err(_) => decoder.reset(),
        }
    }
    ```

  - 在关闭 FAST_STATUS TX 的简化实验中，使用上面 ring-buffered RX 之后：
    - 模拟板能够完整看到数字板发送的 8 个 SetPoint SLIP 帧（每帧 13 字节 payload）；
    - `SetPoint received: target_i_ma=600 mA (prev=500 mA)` 日志按预期打印；
    - 证明协议、SLIP、CBOR 本身无结构性问题，原先的“帧被截断”纯粹是 RX 使用方式不当引入的丢字节。
- 实战建议：
  - 对于 MCU↔MCU 的高频单向流（300 kbps 级别、连续 SLIP 帧），在使用 Embassy STM32 + DMA 的场景下：
    - **严禁** 用裸 `UartRx<'_, Async>::read(&mut [u8; 1])` 逐字节喂协议解析器；
    - 必须使用 `RingBufferedUartRx` 或 `BufferedUartRx`，并在上层协议解析层使用 chunk 读取 + slip/CBOR 解码；
    - 如果出现“数字板看到完整帧、模拟板看到部分帧”的情况，优先怀疑 RX 缓冲/调度问题，而不是先动协议。
