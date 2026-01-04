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
    - 记录本次 `mcu-agentd` logs/monitor（可用 `just agentd logs all --tail 200 --sessions` 汇总），方便后续对照。
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
  - `just agentd flash digital`；analog：`just agentd flash analog`。
- 日志采集：
  - `just agentd monitor digital --reset`（停止时用 Ctrl+C）。
  - 只看已有日志：`just agentd monitor digital --from-start`。
  - 双板同时观察：开两个终端分别运行 `just agentd monitor analog --reset` 与 `just agentd monitor digital --reset`。
- 关注要点：
  - 先读取 `tmp/digital-fw-version.txt` / `tmp/analog-fw-version.txt`，与日志开头的 version 行比对，确认一致再评估。  
  - UART：`UART RX error: FifoOverflowed (total=...)`、DMA panic/nmi、`uhci_dma_rx_chunk...` 之类统计。  
  - 协议：`protocol decode error (...)` 或 `decode_errs` 的速率。  
  - 节奏：`fast_status ok (count=...)`、UI 帧日志、`DISPLAY dt_ms`。

## 近期实验记录（持续补充）

- **2025-11-16 11:33 digital async read + analog 30 Hz（本次）**
  - 版本：digital `digital 0.1.0 (profile release, 539f0f8-dirty, src 0xc0cce3b455259210)`；analog `analog 0.1.0 (profile release, 539f0f8-dirty, src 0x3fd51c3582d631c6)`
  - 配置：UART 230400；FAST_STATUS_HZ=30；DISPLAY_MIN_FRAME_INTERVAL_MS=25；DISPLAY_CHUNK_YIELD_LOOPS=0；SPI DMA on；UART await 异步读取。
  - 时长：~27 s（monitor 约 30 s）
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
- **后续计划：完整 UI + 渲染性能优化**  
  - 目标：在保持当前 UART UHCI DMA 稳定性的前提下，直接针对“现有完整 UI”做性能优化和局部刷新设计，避免依赖任何“简化界面”变体。  
  - 步骤：  
    1. 以当前完整界面（`ui::render`）和现有帧率配置为基线，短时运行观察栈守卫、`dt_ms` 分布和 SPI 事务情况，确认瓶颈集中在渲染而非 UART/SPI 带宽。  
    2. 若需要 profile，可引入仅用于调试的 feature（例如 `feature = "ui_profile"`），在不改变界面内容的前提下插桩记录每帧渲染耗时、格式化调用次数和绘制面积，用于指导后续优化。  
    3. 在数据层缓存“上一帧 UI 状态”，基于新旧数值对比驱动局部渲染（完整界面不变）：  
       - 在 `TelemetryModel` 中维护 `last_rendered: Option<UiSnapshot>`，提供 `diff_for_render() -> (UiSnapshot, UiChangeMask)` 接口，用数值阈值/字符串不等来决定哪些组件需要更新。  
       - `UiChangeMask` 按功能块拆分，例如：`main_metrics`（左侧 V/I/P 数码管）、`voltage_pair`、`current_pair`、`telemetry_lines`（底部 5 行）、`bars`（mirror bar/条形图等）。  
    4. 在视图层实现按组件的局部刷新，而不是每帧全屏重绘：  
       - 保留现有 `ui::render` 作为整帧重绘函数，仅用于首帧或调试。  
       - 新增 `ui::render_partial(frame, snapshot, mask)`，内部根据 `UiChangeMask` 只调用对应的 `draw_*` 函数，重绘受影响的矩形区域，背景和其它静态元素保持不动。  
       - 在 `display_task` 主循环中，先调用 `diff_for_render()` 获取 `(snapshot, mask)`，若 `mask` 全 false 则跳过本帧 UI 绘制；否则调用 `render_partial`。  
    5. 将“值驱动的局部渲染”与现有 SPI 行级 diff 结合：  
       - 每个 `draw_*` 函数（如 `draw_main_metric`、`draw_mirror_bar`、`draw_telemetry` 等）本身就有明确的屏幕矩形区域，可在 `render_partial` 内累积一个 `dirty_rows[DISPLAY_HEIGHT]` 数组，按这些区域的 `top/bottom` 落座标设置行标记。  
       - 后续的行合并与 `show_raw_data` 发送逻辑沿用当前实现，只是行脏信息直接来自组件级 diff，而不再依赖逐行 `memcmp(framebuffer, previous_framebuffer)`。  
    6. 在此基础上做全局级别的性能优化，而不改变 UI 设计：  
       - 将所有显示用字符串（主数值、电压/电流对、状态行等）的格式化逻辑，从 `ui::render` 挪到 `TelemetryModel::update_from_status`，只在数据变化时重算文案；`UiSnapshot` 持有这些预格式化的 `String`，渲染层只读。  
       - 将背景块、卡片、轴线等真正静态的元素在首帧渲染到 framebuffer 中，并在后续帧中仅更新数码管、条形图和文本区域。  
       - 在 `display_task` 中区分“采样频率”和“重绘频率”：保持 80ms 周期采样/更新快照，但只有当关键字段变化超过阈值（例如电压/电流/温度或运行时间秒数变动）时才触发重绘，否则降低重绘频率以平滑 CPU/栈压力。  
    7. 如需 A/B，对比方案应以“完整 UI + 不同渲染策略/阈值”为主，而不是切换到另一套简化界面；默认构建始终使用原设计界面，仅通过 feature 控制调试插桩与优化路径。
- **2025-11-17 00:56 digital UART UHCI DMA + 完整 UI 局部刷新实现（崩溃，未能评估性能）**  
  - 版本：digital `digital 0.1.0 (profile release, 39a1460-dirty, src 0x63d9331e91d947e0)`；analog `analog 0.1.0 (profile release, 5ccb582-dirty, src 0x8c5c13041e8576aa)`。  
  - 配置：UART 230400；FAST_STATUS≈60Hz（analog mock）；`DISPLAY_MIN_FRAME_INTERVAL_MS=80`；`DISPLAY_CHUNK_ROWS=8`；`DISPLAY_CHUNK_YIELD_LOOPS=6`；`ENABLE_DISPLAY_SPI_UPDATES=true`；`UART_RX_FIFO_FULL_THRESHOLD=120`；`UART_RX_TIMEOUT_SYMS=10`；`FAST_STATUS_SLIP_CAPACITY=1024`；`UART_DMA_BUF_LEN=512`；`ENABLE_UART_UHCI_DMA=true`。  
  - UI 实现：根据前文计划落地了完整 UI 的按需刷新路径：  
    - 数据层：在 `UiSnapshot` 中增加预格式化字符串字段（左侧大数值、右侧电压/电流、底部 5 行状态文本），由 `update_from_status` 统一更新；`TelemetryModel` 维护 `last_rendered: Option<UiSnapshot>` 并提供 `diff_for_render() -> (Option<UiSnapshot>, UiSnapshot, UiChangeMask)`。  
    - 视图层：保留 `ui::render` 作为首帧整屏渲染；新增 `UiChangeMask`（`main_metrics`/`voltage_pair`/`current_pair`/`telemetry_lines`/`bars`）与 `ui::render_partial(frame, prev, curr, mask)`，其中：  
      - 左侧大数字区域按字符 diff：对每位数码管/小数点做字符级比较，仅在字符变化时清除并重绘该 glyph 所在矩形（利用等宽字体与固定格式化宽度保证布局稳定）。  
      - 右侧数值及条形图按值整体刷新：对应掩码为真时重绘该功能块所在区域。  
      - 底部 5 行状态文本按行整体刷新。  
    - `display_task` 主循环中，先调用 `diff_for_render()` 获取 `(prev, curr, mask)`：若 `mask` 为空则跳过本帧 UI 绘制；首帧调用 `ui::render`，后续帧调用 `ui::render_partial`，再复用现有 framebuffer 行级 diff + `show_raw_data` SPI 传输逻辑。  
  - 结果：monitor 约 25 s 下，设备启动并输出版本与任务启动日志（含 `spawning display task` / `spawning uart link task (UHCI DMA)`），随后在 ~0.20 s 左右反复出现 `Exception occurred on ProCpu 'Cp0Disabled'` 与 `Breakpoint on ProCpu` 形式的 panic（由 `esp-backtrace` 打印），会话期间未见 `uart_rx_err_total`/`PROTO_DECODE_ERRS`/`fast_status ok`/`display: frame ...` 等常规运行日志。日志：`tmp/agent-logs/digital-20251117-005623.log`。
  - 结论：在当前 UHCI DMA 配置 + 完整 UI 局部刷新实现下，数字板在早期初始化阶段即发生 ProCpu 异常，尚无法采集 UART 溢出或 UI 帧/脏行统计；该 crash 疑似与现有 DMA/任务栈问题同源，需要单独排查（可考虑以 `debug=2` 重新构建并使用 `xtensa-esp32s3-elf-addr2line` 精确定位 PC）。  
- **2025-11-17 00:59 digital async no-DMA + 完整 UI 局部刷新实现（同样崩溃）**  
  - 版本：digital `digital 0.1.0 (profile release, 39a1460-dirty, src 0x9d186a2b6b3337fb)`；analog 同上。  
  - 配置：与上一实验基本一致，但编译期配置为 `ENABLE_UART_UHCI_DMA=false`，运行路径为 `UART link task starting (await read, no DMA)`（无 UHCI DMA，仅基于 `AsyncRead::read` 抽干 FIFO）。  
  - 结果：monitor 约 25 s 下，设备在 ~0.21 s 左右同样出现 `Exception occurred on ProCpu 'Cp0Disabled'` / `Breakpoint on ProCpu` 交替的 panic，日志模式与上一实验高度一致；同样未能看到 `fast_status ok` 或 UI 帧日志。日志：`tmp/agent-logs/digital-20251117-005935.log`。
  - 结论：崩溃在关闭 UART UHCI DMA 后依然存在，说明问题并非仅限 DMA 代码路径，更可能与最近引入的 UI/Telemetry 变更、任务栈或其它共享资源有关。当前仍缺乏足够调试信息来确认根因，后续需要在不启用 `espflash --log-format defmt` 或提高调试信息等级的前提下进一步缩小问题范围；在 crash 未解决前，尚无法在实际硬件上量化“按需更新”对 UART 溢出和 UI 性能的改善程度。
- **2025-11-17 02:38 digital UART UHCI DMA + 手写浮点格式化 + 区域级局部刷新（稳定，可量化性能）**  
  - 版本：digital `digital 0.1.0 (profile release, 39a1460-dirty, src 0x3a421e9ff094b829)`；analog `analog 0.1.0 (profile release, 5ccb582-dirty, src 0x8c5c13041e8576aa)`。  
  - 配置：UART 230400；FAST_STATUS≈60Hz；`DISPLAY_MIN_FRAME_INTERVAL_MS=80`；`DISPLAY_CHUNK_ROWS=8`；`DISPLAY_CHUNK_YIELD_LOOPS=6`；`ENABLE_DISPLAY_SPI_UPDATES=true`；`UART_RX_FIFO_FULL_THRESHOLD=120`；`UART_RX_TIMEOUT_SYMS=10`；`FAST_STATUS_SLIP_CAPACITY=1024`；`UART_DMA_BUF_LEN=512`；`ENABLE_UART_UHCI_DMA=true`。  
  - UI 实现（修正前述崩溃原因）：  
    - 移除所有基于 `core::fmt::write` 的浮点格式化（`{:5.2}` / `{:04.1}` / `{:05.1}C` 等），改为手写的定点整数格式化：  
      - `format_value` / `format_four_digits` / `compute_status_lines` 统一使用 `scaled = (value * scale + 0.5) as u32` + 十进制整型输出，仅依赖基本 `f32` 乘加，不再触发 `core::num::bignum::Big32x40::*` 路径，从而避免 ProCpu 上的 `Cp0Disabled`。  
    - `TelemetryModel::diff_for_render()` 改为只返回 `(UiSnapshot, UiChangeMask)`，内部仅在 UI 线程（`display_task`）中调用 `snapshot.update_strings()` 并在 `last_rendered: Option<UiSnapshot>` 上做字符串级比较；不再把完整 `UiSnapshot` 的克隆返回给调用者，避免在栈上同时持有多份大对象。  
    - `ui::render_partial` 降级为“区域级”局部刷新：  
      - 左侧三块大数码管在 `mask.main_metrics` 为真时通过重复调用 `draw_main_metric(...)` 整块重绘，而不是 per-char diff；  
      - 右侧电压/电流对与条形图在对应掩码为真时按组件整体重绘；  
      - 底部 5 行状态文本按行整体重绘。  
    - `display_task` 在首帧（`frame_idx==1`）仍调用 `ui::render` 整屏绘制布局，后续帧根据 `UiChangeMask` 决定是否调用 `render_partial` 或跳过绘制；最终的 SPI 推屏仍由逐行 `memcmp(framebuffer, previous_framebuffer)` + 行合并逻辑驱动。  
  - 结果（monitor 约 25 s，日志：`tmp/agent-logs/digital-20251117-023808.log`）：
    - 稳定性：全程未再出现 `Detected a write to the stack guard value` 或 `Exception occurred on ProCpu 'Cp0Disabled'`；系统在单次 run 内连续输出到 ~22 s。  
    - UART：`uart_rx_err_total=0`，`UART RX error` 未见；`fast_status ok` 自 0.192 s 左右开始，每约 1 s 增加 ~51–52 次，与预期 60Hz mock 接近（典型：`20.184 [INFO ] fast_status ok (count=992, display_running=true)`，`21.401 ... (count=1056, ...)`）。  
    - 协议：`decode_errs` 在完整 22 s 窗口内从 0 增至 ~40（约 1.8 次/s），明显低于先前无显示/简化 UI 场景下的 20+ 次/s；尚未做更细致的错误类型统计。  
    - 显示帧率与脏行统计：  
      - 帧间隔：`display: rendering frame N (dt_ms=80)` 基本稳定在 80 ms 附近，少数首帧为 ~138 ms（初始化阶段）；  
      - 前几帧典型日志：  
        - `display: frame 1 push complete (dirty_rows=320 dirty_spans=1)`（完整首帧）；  
        - `display: frame 2 push complete (dirty_rows=108 dirty_spans=5)`；  
        - `display: frame 3 push complete (dirty_rows=62 dirty_spans=5)`；  
        - 后续多数帧 `dirty_rows` 在 10–50 行之间，`dirty_spans` 1–3（区域级局部刷新 + 行级 diff 联合作用）。  
      - 在 20–22 s 的窗口末尾，当数据持续变化且局部刷新触发频繁时，观察到部分帧退化为 `dirty_rows=320 dirty_spans=1` 的整屏推送（例如 `frame 243–269`），说明在高 churn 场景下当前区域/掩码策略仍需进一步调优才能长期维持行级局部更新的优势。  
  - 初步结论：  
    - 手写浮点格式化成功规避了之前由 `core::fmt` 引入的 ProCpu `Cp0Disabled` 异常，UI + UART + UHCI DMA 组合在 230400 波特率 + 60Hz FAST_STATUS 下可以稳定运行 ≥20 s；  
    - 在数值变化“普通”的阶段，区域级局部刷新明显降低了每帧 `dirty_rows`，但在高变化、接近“全屏都变”的场景下仍会退化为整屏推送；后续可以在 `UiChangeMask` 的粒度和行合并策略上继续优化，以便在更多负载模式下保持 SPI 事务稀疏。  
- **2025-11-17 12:20 digital UART UHCI DMA + 区域级局部刷新 + 16ms 帧间隔（UI 更流畅，但 decode_errs 偏高）**  
  - 版本：digital `digital 0.1.0 (profile release, 9ed3fce-dirty, src 0x40a3cd3b26db9ab5)`；analog `analog 0.1.0 (profile release, 5ccb582-dirty, src 0x8c5c13041e8576aa)`（来自 `tmp/analog-fw-version.txt`）。  
  - 配置：UART 230400；FAST_STATUS_PERIOD_MS≈1000/60（当次实验使用的模拟板配置，现已改为 1000/30）；`DISPLAY_MIN_FRAME_INTERVAL_MS=16`；`DISPLAY_CHUNK_ROWS=8`；`DISPLAY_CHUNK_YIELD_LOOPS=6`；`ENABLE_DISPLAY_SPI_UPDATES=true`；`UART_RX_FIFO_FULL_THRESHOLD=120`；`UART_RX_TIMEOUT_SYMS=10`；`FAST_STATUS_SLIP_CAPACITY=1024`；`UART_DMA_BUF_LEN=512`；`ENABLE_UART_UHCI_DMA=true`。  
  - 结果（monitor 约 20 s，日志：`tmp/agent-logs/digital-20251117-122018.log`）：
    - 稳定性：整个 ~18 s 窗口内未见 `UART RX error`、`Detected a write to the stack guard value` 或 `Cp0Disabled` 类 panic。  
    - UART / 协议：`uart_rx_err_total=0`；`PROTO_DECODE_ERRS` 从 0 增至 ~544（约 30 次/s），`fast_status ok` 从 ~1.25 s 时的 17 增至 ~17.6 s 时的 329，折算约 18–20 帧/s，有效 FAST_STATUS 解码率明显低于发送端 ~60Hz。  
    - 显示：  
      - 帧间隔：采样帧中的 `dt_ms` 在稳定阶段多为 16ms 或 64–70ms；在 UI 无变更时大量帧被 `mask.is_empty()` 跳过绘制，仅保留行级 diff 与 SPI 推屏；  
      - UI 层左上角 FPS 叠加基于 `dt_ms` 估算，理论上稳定阶段接近 60FPS，主观观感相较 80ms 配置明显更顺滑；  
      - `display: frame N push complete` 中 `dirty_rows` 在 UI 平稳时经常为 0 或几十行，说明区域级局部刷新 + 行级 diff 仍然在起作用。  
  - 初步结论：16ms 帧间隔配置在 UI 与 DMA/SPI 侧表现稳定，也未引入新的 UART 硬件溢出，但在当前 UART 配置下显著抬高了协议层 `decode_errs`，有效 `fast_status ok` 帧率远低于发送端。若后续需要“显示帧率≈60Hz 且有效状态帧率也接近 60Hz”，需要在 UART 参数 / SLIP 解码 / 帧率上进一步折中和优化。  
- **2025-11-17 12:40 digital UART UHCI DMA + 区域级局部刷新 + analog 30Hz（新默认，负载减轻）**  
  - 版本：digital `digital 0.1.0 (profile release, 9ed3fce-dirty, src 0x40a3cd3b26db9ab5)`；analog `analog 0.1.0 (profile release, 1105f33-dirty, src 0x5f542375efe5ff0f)`（来自最新 `tmp/{digital,analog}-fw-version.txt`）。  
  - 配置：UART 230400；FAST_STATUS_PERIOD_MS≈1000/30；`DISPLAY_MIN_FRAME_INTERVAL_MS=16`；`DISPLAY_CHUNK_ROWS=8`；`DISPLAY_CHUNK_YIELD_LOOPS=6`；`ENABLE_DISPLAY_SPI_UPDATES=true`；`UART_RX_FIFO_FULL_THRESHOLD=120`；`UART_RX_TIMEOUT_SYMS=10`；`FAST_STATUS_SLIP_CAPACITY=1024`；`UART_DMA_BUF_LEN=512`；`ENABLE_UART_UHCI_DMA=true`。  
  - 结果（analog/digital 各 monitor 约 20 s，日志：`tmp/agent-logs/analog-20251117-123013.log`、`tmp/agent-logs/digital-20251117-124025.log`）：
    - analog：版本行打印为 `LoadLynx analog alive; streaming mock FAST_STATUS frames`，新构建版本 `analog 0.1.0 (profile release, 1105f33-dirty, src 0x5f542375efe5ff0f)` 成功刷写并运行；脚本未观察到异常或重启。  
    - digital UART：`uart_rx_err_total=0`；`fast_status ok` 在 ~18 s 窗口内从 13 增至 361，折算约 18–20 帧/s；`decode_errs` 从 0 增至 ~144（约 8 次/s），明显低于前一轮 analog≈60Hz 时约 30 次/s 的水平。  
    - 显示：`display: rendering frame N (dt_ms=16/63–74)` 持续出现，说明显示任务仍以 16ms 最小帧间隔运行；局部刷新生效，`dirty_rows` 多数在 90–150 行之间，部分帧在无 UI 变化时 `dirty_rows=0`。  
  - 初步结论：将模拟板 FAST_STATUS 周期从 1000/60 调整为 1000/30 后，整体链路的数据压力明显下降，`decode_errs` 速率从约 30 次/s 降至约 8 次/s，`fast_status ok` 保持在 18–20 帧/s 量级；显示任务仍以 16ms 间隔高帧率运行，UI 流畅度未受影响。后续若需要进一步提升有效状态帧率，可在保持 30Hz 发送端的前提下继续针对 UART/SLIP 解码与日志采样策略做优化。  
- **2025-11-17 13:06 digital UART UHCI DMA + 区域级局部刷新 + analog/digital 均 30Hz 目标（DISPLAY_MIN_FRAME_INTERVAL_MS=33）**  
  - 版本：digital `digital 0.1.0 (profile release, 9ed3fce-dirty, src 0x40a3cd3b26db9ab5)`；analog `analog 0.1.0 (profile release, 1105f33-dirty, src 0x5f542375efe5ff0f)`。  
  - 配置：UART 230400；FAST_STATUS_PERIOD_MS≈1000/30；`DISPLAY_MIN_FRAME_INTERVAL_MS=33`；`DISPLAY_CHUNK_ROWS=8`；`DISPLAY_CHUNK_YIELD_LOOPS=6`；`ENABLE_DISPLAY_SPI_UPDATES=true`；`UART_RX_FIFO_FULL_THRESHOLD=120`；`UART_RX_TIMEOUT_SYMS=10`；`FAST_STATUS_SLIP_CAPACITY=1024`；`UART_DMA_BUF_LEN=512`；`ENABLE_UART_UHCI_DMA=true`。  
  - FPS 统计实现：在 `display_task` 中维护 500ms 以上的时间窗口，窗口内按帧计数，结束时计算 `fps = frames * 1000 / window_ms`，并：  
    - 将得到的 `last_fps` 传给 `ui::render_fps_overlay`，在左上角覆盖显示 `FPS <整数>`；  
    - 通过日志周期性打印：`display: fps window_ms=... frames=... fps=...`，用于对照 UI 观感与实际帧率。  
  - 结果（monitor 约 20 s，日志：`tmp/agent-logs/digital-20251117-130608.log`）：
    - `uart_rx_err_total=0`；`fast_status ok` 在 18s 左右窗口内从 24 增至约 218（`stats` 行），折算约 10–12 帧/s；  
    - `display: fps ...` 日志中的 FPS 多数在 17–20 之间（典型窗口：`window_ms≈500–560ms, frames≈9–11`），短期内 UI 实际帧率略低于理论 30FPS 上限；  
    - `decode_errs` 继续累计（约 10 次/s 量级），说明在当前 UART/SLIP 配置与完整 UI 负载下，30Hz FAST_STATUS + 33ms 显示目标仍然偏紧，后续需要结合 UART 阈值、日志采样和 SLIP 解码策略进一步调优。  
  - 初步结论：analog 与 digital 均以 30Hz 作为设计目标（发送端 1000/30ms，显示端 33ms 间隔）后，UI 侧 FPS 统计与日志已统一，但在实际硬件上受到任务调度与协议开销影响，稳定 FPS 仍停留在 17–20 区间；这验证了新的 FPS 统计路径正确工作，同时暴露出后续需要针对“30Hz link + 完整 UI”的系统级调优空间。  
- **2025-11-16 12:00 digital UART UHCI DMA 尝试（结果：日志解码失败）**  
  - 版本：digital `digital 0.1.0 (profile release, 539f0f8-dirty, src 0xc0cce3b455259210)`；analog 未变。  
  - 配置：UART 230400；FAST_STATUS_HZ=30；DISPLAY_MIN_FRAME_INTERVAL_MS=25；DISPLAY_CHUNK_YIELD_LOOPS=0；SPI DMA on；UART RX via UHCI DMA（DMA_CH1，chunk_limit=4092，缓冲=~4KB）；LOGFMT=defmt。  
  - 结果：monitor 启动后仅输出 ROM/boot 日志，随即 `Failed to decode defmt frame`，未见应用 defmt 行（含版本、stats 等）；日志文件 2 KB 左右（`tmp/agent-logs/digital-20251116-120030.log`）。
  - 结论：UHCI DMA 版本首次运行未产生可解码 defmt 输出，需排查（可能 defmt 表不匹配/任务未启动/串口配置冲突）。暂未能量化 UART 溢出指标。
- **2025-11-16 13:10 digital async no-DMA + 放宽显示/阈值（本次）**  
  - 版本：digital `digital 0.1.0 (profile release, 539f0f8-dirty, src 0x0acb654ca47d57f6)`；analog 未变。  
  - 配置：UART 230400；FAST_STATUS_HZ=30；DISPLAY_MIN_FRAME_INTERVAL_MS=50；DISPLAY_CHUNK_ROWS=12；DISPLAY_CHUNK_YIELD_LOOPS=3；SPI DMA on；UART async no-DMA；UART_RX_FIFO_FULL_THRESHOLD=32；UART_RX_TIMEOUT_SYMS=2。  
  - 时长：~20 s（monitor 约 20 s，日志至 20.58s 有数据）。
  - 指标：`uart_rx_err_total` 2.07s=13 → 20.58s=184，约 9.2 次/秒，溢出仍严重；`fast_status ok` 未见输出（显示抢占导致日志全被 display 占用，需延长/调整打印间隔）。  
  - UI：帧间隔日志仍在 100–140 ms 范围，几乎整屏推送（dirty_rows=320），说明即使升至 50 ms 帧间隔 + 12 行分块 + 3 次让出，显示占用仍重。  
  - 结论：当前调优未遏制溢出，需进一步降低显示占用（更大帧间隔/更多让出/更小分块或关闭 SPI 更新）或推行 UART DMA。
- **2025-11-16 13:57 digital UART UHCI DMA 稳定性基线（关闭显示推屏）**  
  - 版本：digital `digital 0.1.0 (profile release, 539f0f8-dirty, src 0x1611f8cfdd16e5c0)`；analog 未变。  
  - 配置：UART 230400；FAST_STATUS_HZ=30；DISPLAY_MIN_FRAME_INTERVAL_MS=80；DISPLAY_CHUNK_ROWS=8；DISPLAY_CHUNK_YIELD_LOOPS=6；`ENABLE_DISPLAY_SPI_UPDATES=false`；UART RX via UHCI DMA（DMA_CH1，buf=1024，chunk_limit=1024）；UART 阈值=32，超时=2；stats 每 1s；解码仅计数（不打印）。  
  - 时长：~22 s（monitor 约 22 s，日志：`tmp/agent-logs/digital-20251116-135727.log`）。
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

- **2025-11-16 20:39 digital UART UHCI DMA 基线（display task 关闭，仅串口验证）**  
  - 版本：digital `digital 0.1.0 (profile release, 5cd4e45-dirty, src 0x83890fa979237bbd)`；analog `analog 0.1.0 (profile release, 5ccb582-dirty, src 0x8c5c13041e8576aa)`。  
  - 配置：UART 230400；FAST_STATUS_PERIOD_MS≈1000/60（当次实验使用的模拟板配置，现已改为 1000/30）；DISPLAY_MIN_FRAME_INTERVAL_MS=80；`ENABLE_DISPLAY_TASK=false`；`ENABLE_DISPLAY_SPI_UPDATES=false`；UART RX via UHCI DMA（DMA_CH1，buf=1024，chunk_limit=1024）；UART 阈值=32，超时=2。  
  - 时长：~22 s（monitor 约 22 s，日志：`tmp/agent-logs/digital-20251116-203944.log`）。
  - 指标：`uart_rx_err_total=0` 全程无溢出；`fast_status_ok` 0→1156（约 52 次/秒，对应 analog mock ~60Hz）；`decode_errs=0`，显示任务完全关闭。  
  - 结论：在关闭显示任务的前提下，UHCI DMA 路径在 230400 波特率 + 60Hz FAST_STATUS 下已长期稳定运行，串口本身不再是性能瓶颈；后续问题集中在显示任务与 Core0 栈/调度。

- **2025-11-16 21:37 digital UART UHCI DMA + 简化 UI（SPI 推屏开启）**  
  - 版本：digital `digital 0.1.0 (profile release, 9accc66-dirty, src 0x????????????????)`；analog 同上。  
  - 配置：UART 230400；FAST_STATUS_PERIOD_MS≈1000/60（当次实验使用的模拟板配置，现已改为 1000/30）；DISPLAY_MIN_FRAME_INTERVAL_MS=80；DISPLAY_CHUNK_ROWS=8；DISPLAY_CHUNK_YIELD_LOOPS=6；`ENABLE_DISPLAY_TASK=true`；`SIMPLE_DISPLAY_MODE=true`（简化条形图 UI）；`ENABLE_DISPLAY_SPI_UPDATES=true`；UART RX via UHCI DMA（DMA_CH1，buf=1024，chunk_limit=1024）；UART 阈值=32，超时=2。  
  - 时长：~22 s（monitor 约 22 s，日志：`tmp/agent-logs/digital-20251116-213713.log`）。
  - 指标：`uart_rx_err_total=0`；`fast_status_ok` 0→1134（约 51 次/秒）持续线性增长；`decode_errs` 初始有少量（6 次）后停止增加；display 帧间隔稳定在 ~80ms，dirty_rows 多数为小块，偶尔整帧 fallback。  
  - 结论：在启用“简化 UI + SPI DMA 推屏”的前提下，数字板可以同时维持稳定的显示刷新和 UART UHCI DMA 链路；串口不再因显示负载而溢出或大量解码错误。现有冲突集中在“完整 UI + 栈守卫”这一组合上，而非 UART/SPI 本身的带宽或配置。

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
