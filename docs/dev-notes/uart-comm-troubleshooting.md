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

### 推荐优化路径（按优先级刷新）

1) **显示让出优先**  
   - 目标：降低 UI 对执行器的占用，让 UART task 及时运行。  
   - 建议起始参数：`DISPLAY_MIN_FRAME_INTERVAL_MS=40–50`、`DISPLAY_CHUNK_ROWS=8–12`、`DISPLAY_CHUNK_YIELD_LOOPS=2–4`，保持 SPI DMA on。  
   - 观察：`uart_rx_err_total` 增长率、`fast_status ok` 节奏、画面是否可接受。

2) **UART 阈值/超时调参**（可与 1 并行）  
   - 目标：降低中断风暴、单次搬运更多数据。  
   - 建议起始：`UART_RX_FIFO_FULL_THRESHOLD=16 或 32`，`UART_RX_TIMEOUT_SYMS=2 或 3`。  
   - 观察：`Rx(FifoOverflowed)` 速率变化。

3) **A/B：波特率或显示帧率回退**  
   - 临时降波特率到 `115200`，或将显示帧率降到 ≈20FPS，对比溢出速率；验证出“真正卡口”后再回到 230400。

4) **A/B：SPI 事务形态**  
   - 关闭 SPI DMA（保留分块 + 让出），看溢出是否下降；若无改善再回 DMA。

5) **接收缓冲冗余与观测**  
   - SLIP 解码容量调大（例：`SlipDecoder<1024>`）；若协议有 seq，记录缺口估丢帧。

6) **更强方案（如仍不达标）**  
   - UART UHCI DMA 环形接收；改动较大，但已列为必测项（见“待执行实验”）；在上述手段无效时立即推进。

## 记录规范（每次实验务必填写）

- **基础信息**
  - `build version`（analog/digital 各自的 fw version 日志行；确保与本地 tmp/{analog|digital}-fw-version.txt 一致）
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

- `uart_rx_err_total` 增长率 < 1 次/秒（理想为 0）。
- `decode_errs` 不增长或仅偶发（≪ 0.1 次/秒）。
- `fast_status ok` 节奏符合目标帧率，UI 正常刷新。

## 复现实验与日志观测

- 构建（release）：
  - `make d-build`
- 烧录 + 监听：
  - `make d-run PORT=/dev/tty.*`
- 关注日志：
  - UART：`UART RX error: FifoOverflowed (total=...)` 的增长速率。
  - 协议：`protocol decode error (...)` 的出现频次与类型。
  - 节奏：`fast_status ok (count=...)` 的稳定性。

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

- **待执行实验：UART RX DMA（UHCI）基线**  
  - 目标：验证在相同显示负载与 230400 波特率下，使用 esp-hal UHCI DMA 环形缓冲能否把 `Rx(FifoOverflowed)` 降到 <1 次/分钟。  
  - 条件：保留 SPI DMA on、FAST_STATUS_HZ=30/60 两档各测 1 次；记录 DMA buffer 大小、触发阈值、环形读策略。  
  - 状态：尚未实施；需实现 DMA 版本 `uart_link_task` 并纳入记录模板。
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
  - 结论：UHCI DMA 在关闭显示且简化解码输出的情况下稳定运行且无溢出，但协议解码错误计数较高，需继续分析错误来源或调节 chunk/处理方式；下一步可逐步恢复显示负载并监控溢出与解码错误。
- **2025-11-16 13:34 digital UART UHCI DMA 再次尝试（崩溃）**  
  - 版本：digital `digital 0.1.0 (profile release, 539f0f8-dirty, src 0xcc6dc52a46acf4d1)`；analog 未变。  
  - 配置：UART 230400；FAST_STATUS_HZ=30；DISPLAY_MIN_FRAME_INTERVAL_MS=50；DISPLAY_CHUNK_ROWS=12；DISPLAY_CHUNK_YIELD_LOOPS=3；SPI DMA on；UART RX via UHCI DMA（DMA_CH1，buf=2048，chunk_limit=2048）；UART RX 阈值=32，超时=2。  
  - 结果：~10.9s 时出现多次 panic（`Breakpoint on ProCpu / Cp0Disabled`），任务崩溃；未记录到 UART 溢出或 fast_status 行（崩溃前主要是显示日志）。日志：`tmp/agent-logs/digital-20251116-133427.log`。  
  - 结论：当前 UHCI DMA 实施不稳定（可能 chunk_limit/配置或 DMA 使用方式触发异常）；需最小化示例重测或进一步调试 UHCI 配置。

## 附：当前实现的关键点

- UART 接收路径存在两种实现形态：
  - 早期版本：使用 `read_async()`，在本项目负载下会频繁出现 `FifoOverflowed` 与协议解码错误；
  - 当前版本：使用轮询式 `uart.read()` 抽干 FIFO，在 `230400` 波特率 + 60Hz FAST_STATUS 下已验证稳定。
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
