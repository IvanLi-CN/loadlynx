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

- 帧率上限：`DISPLAY_MIN_FRAME_INTERVAL_MS`（默认 33ms ≈ 30FPS）
  - `firmware/digital/src/main.rs:53`
- 分块参数：`DISPLAY_CHUNK_ROWS`（默认 32 行/块），`DISPLAY_CHUNK_YIELD_LOOPS`（默认 2）
  - `firmware/digital/src/main.rs:56, 57`
- UART RX 阈值/超时：`with_fifo_full_threshold(1)`，`with_timeout(1)`
  - `firmware/digital/src/main.rs:582, 583`
- SLIP 解码器容量：`SlipDecoder<512>`
  - 初始化：`firmware/digital/src/main.rs:379`（推入：`417`）

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
    - 记录本次日志文件路径（`tmp/agent-logs/...`），方便后续对照。
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
  - `scripts/agent_verify_digital.sh --no-log`（必要时 `--profile dev`）；analog 若也升级，先运行 `scripts/agent_verify_analog.sh --no-log`。
- 日志采集：
  - `scripts/agent_verify_digital.sh --timeout 25`（可加 `PORT=/dev/tty.*`）；只看日志时用 `scripts/agent_verify_digital.sh --no-flash --timeout 15`。  
  - 需要双板同时观察时，按指南运行 `scripts/agent_dual_monitor.sh --timeout 60`。
- 关注要点：
  - 先读取 `tmp/digital-fw-version.txt` / `tmp/analog-fw-version.txt`，与日志开头的 version 行比对，确认一致再评估。  
  - UART：`UART RX error: FifoOverflowed (total=...)`、DMA panic/nmi、`uhci_dma_rx_chunk...` 之类统计。  
  - 协议：`protocol decode error (...)` 或 `decode_errs` 的速率。  
  - 节奏：`fast_status ok (count=...)`、UI 帧日志、`DISPLAY dt_ms`。

## 近期实验记录（持续补充）

- **2025-11-16 11:33 digital async read + analog 30 Hz（本次）**
  - 版本：digital `digital 0.1.0 (profile release, 539f0f8-dirty, src 0xc0cce3b455259210)`；analog `analog 0.1.0 (profile release, 539f0f8-dirty, src 0x3fd51c3582d631c6)`
  - 配置：UART 230400；FAST_STATUS_HZ=30；DISPLAY_MIN_FRAME_INTERVAL_MS=25；DISPLAY_CHUNK_YIELD_LOOPS=0；SPI DMA on；UART await 异步读取。
  - 时长：~27 s（`agent_verify_digital --timeout 30`）
  - 指标：
    - `uart_rx_err_total`: 0→244（≈9 次/秒），类型全为 `Rx(FifoOverflowed)`；示例行：`22.658 [WARN ] UART RX error: Rx(FifoOverflowed) (total=206)`。
    - `PROTO_DECODE_ERRS`: 未见新增（日志未现 decode warn）。
    - `fast_status ok`: 32（8.6 s）、64（18.0 s），节奏约 30 Hz 保持递增。
  - UI：帧日志 dt_ms ~100–113 ms，卡顿不明显；显示刷新正常。
  - 结论：UART 溢出仍严重，需进一步调参（建议提升帧间隔/块间让出或提高 UART FIFO 阈值）。

- **实验计划：UART RX DMA（UHCI）回归**  
  - 目标：在 230400 波特率下，于“显示关闭”→“显示限速”→“正常显示”三个阶段验证 `Rx(FifoOverflowed)` ≈ 0，并确认 `decode_errs` 收敛。  
  - 条件：保留 SPI DMA、FAST_STATUS_HZ=30 起步，必要时测 60Hz；记录 DMA buffer 大小、阈值、环形读策略和日志路径。  
  - 状态：第一阶段（显示关闭）已在 2025-11-16 13:57 实验完成；下一步是恢复显示推屏与 60Hz 发送端并观察 decode_errs。
- **2025-11-16 12:00 digital UART UHCI DMA 尝试（结果：日志解码失败）**  
  - 版本：digital `digital 0.1.0 (profile release, 539f0f8-dirty, src 0xc0cce3b455259210)`；analog 未变。  
  - 配置：UART 230400；FAST_STATUS_HZ=30；DISPLAY_MIN_FRAME_INTERVAL_MS=25；DISPLAY_CHUNK_YIELD_LOOPS=0；SPI DMA on；UART RX via UHCI DMA（DMA_CH1，chunk_limit=4092，缓冲=~4KB）；LOGFMT=defmt。  
  - 结果：`espflash --monitor --log-format defmt` 启动后仅输出 ROM/boot 日志，随即 `Failed to decode defmt frame`，未见应用 defmt 行（含版本、stats 等）；日志文件 2 KB 左右（`tmp/agent-logs/digital-20251116-120030.log`）。  
  - 结论：UHCI DMA 版本首次运行未产生可解码 defmt 输出，需排查（可能 defmt 表不匹配/任务未启动/串口配置冲突）。暂未能量化 UART 溢出指标。
- **2025-11-16 13:10 digital async no-DMA + 放宽显示/阈值（本次）**  
  - 版本：digital `digital 0.1.0 (profile release, 539f0f8-dirty, src 0x0acb654ca47d57f6)`；analog 未变。  
  - 配置：UART 230400；FAST_STATUS_HZ=30；DISPLAY_MIN_FRAME_INTERVAL_MS=50；DISPLAY_CHUNK_ROWS=12；DISPLAY_CHUNK_YIELD_LOOPS=3；SPI DMA on；UART async no-DMA；UART_RX_FIFO_FULL_THRESHOLD=32；UART_RX_TIMEOUT_SYMS=2。  
  - 时长：~20 s（`agent_verify_digital --timeout 25`，日志至 20.58s 有数据）。  
  - 指标：`uart_rx_err_total` 2.07s=13 → 20.58s=184，约 9.2 次/秒，溢出仍严重；`fast_status ok` 未见输出（显示抢占导致日志全被 display 占用，需延长/调整打印间隔）。  
  - UI：帧间隔日志仍在 100–140 ms 范围，几乎整屏推送（dirty_rows=320），说明即使升至 50 ms 帧间隔 + 12 行分块 + 3 次让出，显示占用仍重。  
  - 结论：当前调优未遏制溢出，需进一步降低显示占用（更大帧间隔/更多让出/更小分块或关闭 SPI 更新）或推行 UART DMA。
- **2025-11-16 13:57 digital UART UHCI DMA 稳定性基线（关闭显示推屏）**  
  - 版本：digital `digital 0.1.0 (profile release, 539f0f8-dirty, src 0x1611f8cfdd16e5c0)`；analog 未变。  
  - 配置：UART 230400；FAST_STATUS_HZ=30；DISPLAY_MIN_FRAME_INTERVAL_MS=80；DISPLAY_CHUNK_ROWS=8；DISPLAY_CHUNK_YIELD_LOOPS=6；`ENABLE_DISPLAY_SPI_UPDATES=false`；UART RX via UHCI DMA（DMA_CH1，buf=1024，chunk_limit=1024）；UART 阈值=32，超时=2；stats 每 1s；解码仅计数（不打印）。  
  - 时长：~22 s（`agent_verify_digital --timeout 25`，日志：`tmp/agent-logs/digital-20251116-135727.log`）。  
  - 指标：`uart_rx_err_total=0` 全程无溢出；`fast_status_ok` 0→113（~31s 对应 113，约 6/s）；`decode_errs` 0→478（约 21.4/s），显示未推屏。  
  - 结论：UHCI DMA 在关闭显示且简化解码输出的情况下稳定运行且无溢出，但协议解码错误计数较高；判断更多是 SLIP 层/发送端噪声而不是 DMA 溢出。下一步：
    - 将 `SlipDecoder` 扩大到 1024 并记录 `decoder_bytes`、`drop_bytes`；
    - 在 analog 端打出 seq gap（或记录 FAST_STATUS 序号）以确认 decode_errs 是否真实丢帧；
    - 维持 `FAST_STATUS_HZ=30`，逐步恢复显示负载前先验证低帧率（10Hz）下 decode_errs 是否下降，若不降则集中排查 SLIP/协议解析。
- **2025-11-16 13:34 digital UART UHCI DMA 再次尝试（崩溃）**  
  - 版本：digital `digital 0.1.0 (profile release, 539f0f8-dirty, src 0xcc6dc52a46acf4d1)`；analog 未变。  
  - 配置：UART 230400；FAST_STATUS_HZ=30；DISPLAY_MIN_FRAME_INTERVAL_MS=50；DISPLAY_CHUNK_ROWS=12；DISPLAY_CHUNK_YIELD_LOOPS=3；SPI DMA on；UART RX via UHCI DMA（DMA_CH1，buf=2048，chunk_limit=2048）；UART RX 阈值=32，超时=2。  
  - 结果：~10.9s 时出现多次 panic（`Breakpoint on ProCpu / Cp0Disabled`），任务崩溃；未记录到 UART 溢出或 fast_status 行（崩溃前主要是显示日志）。日志：`tmp/agent-logs/digital-20251116-133427.log`。  
  - 结论：当前 UHCI DMA 实施不稳定（可能 chunk_limit/配置或 DMA 使用方式触发异常）；需最小化示例重测或进一步调试 UHCI 配置。

  - 追踪：崩溃前 DMA chunk 列表已经跑满（`chunk_limit=buf_len=2048`），怀疑是 ISR/任务共享的缓冲在高负载下未及时 recycle；下一次实验计划将 `chunk_limit` 降到 1024 并加上故障计数。

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
