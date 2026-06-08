# History

## Origin

- Migrated from legacy planning docs into the canonical specs taxonomy.

## Key Decisions

- Preserve the legacy spec ID `0009` and slug `uart-comm-troubleshooting` for traceability.
- Keep the original planning scope traceable while assigning long-lived requirements to `SPEC.md` and implementation/history records to companion documents.

## Documentation Model

`SPEC.md` is the active topic contract. Historical rationale, evolution notes, and records moved out of the topic contract are kept here.

### 近期实验记录（持续补充）

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
