# Digital Display PSRAM/DMA Pipeline（#y4sf4）

## 状态

- Status: 已完成
- Created: 2026-03-18
- Last: 2026-03-18

## 背景 / 问题陈述

- 当前 digital 固件的显示路径仍以单个 render 任务串行完成 UI 绘制与 SPI 推屏。
- 默认 `net_http` 构建为了节省内部 RAM，会退化成单 framebuffer + 整帧分块推送，这让默认构建无法享受已有的局部刷新收益。
- 现有 SPI DMA 只使用较小的内部 RAM staging buffer，render/push 与 UART/Wi‑Fi 更容易争用同一颗 ESP32-S3 的内部资源。
- 这次工作要在不改 UI 视觉与业务语义的前提下，把显示链路重构成“PSRAM 专用多缓冲 + DMA present 管线”，降低 CPU 占用和整帧推送概率。

## 目标 / 非目标

### Goals

- 为显示链路单独启用 PSRAM，并把三份 32B 对齐 framebuffer 放入 PSRAM 专用 arena。
- 把 render 与 present 拆成两段管线，present 只消费最新待显示帧。
- 删除默认 `net_http` 构建的单缓冲退化路径，统一默认/无 `net_http` 构建的显示缓冲行为。
- 用 dirty-span flush 取代当前按行 `memcmp(framebuffer, previous_framebuffer)` 的全帧 diff。
- 补齐 render/clone/present/drop counters，并用 baseline/candidate HIL 证据验证收益与回归。

### Non-goals

- 不改变 dashboard、PD settings、audio menu 的视觉设计与交互语义。
- 不引入全局 PSRAM heap，不把 Atomics、Mutex、任务状态、DMA descriptors 或通用 allocator 对象放进 PSRAM。
- 不切换显示控制器、显示总线类型或 PSRAM/Flash 频率档位。
- 不修改 analog 固件协议、发送频率或 HIL selector 配置。

## 范围（Scope）

### In scope

- `firmware/digital/src/main.rs` 的显示资源初始化、PSRAM arena carve-out、多缓冲状态机与 render/present 任务拆分。
- `firmware/digital/src/ui/mod.rs` 的 dirty region/dirty span 产出。
- 默认 release 与无 `net_http` release 构建的编译验证。
- digital baseline/candidate 的 HIL flash、monitor 和日志对比。
- 本 spec 与 `docs/specs/README.md` 的进度同步。

### Out of scope

- `firmware/analog/**`
- 设备选择器缓存（`.esp32-port` / `.stm32-port`）切换
- UI 视觉稿、主界面布局或文案
- Web/API 语义改动

## 需求（Requirements）

### MUST

- 默认 `net_http` 构建必须恢复多缓冲，不再保留“单 framebuffer + 整帧 chunked push”退化路径。
- 三个 framebuffer 必须来自 PSRAM 专用 arena，且保持 32B 对齐。
- SPI DMA descriptors 与 staging buffer 必须保留在内部 RAM。
- render 必须基于最新 `composed` buffer 复制到 `free` buffer 再绘制，不能直接就地覆写 displayed/presenting buffer。
- present 必须只消费最新 pending frame；旧 pending frame 允许被覆盖并计入 drop counter。
- full-screen 视图、wake、view 切换、首帧仍允许强制整帧 present。

### SHOULD

- 默认 staging chunk 以 `8192B / 16 rows` 为首选，只有真实 HIL 回归时才回退到 `4096B / 8 rows`。
- render 层应直接产出 dirty rows/spans，而不是再做全帧逐行 `memcmp`。
- baseline/candidate HIL 对比应直接记录 `fast_status ok`、`PROTO_DECODE_ERRS`、`uart_rx_err_total` 与新的 display counters。

### COULD

- 额外记录 `display_present_full_count` 与 `display_pending_drops` 的秒级 stats，方便 PR 里做趋势对比。

## 功能与行为规格（Functional/Behavior Spec）

### Core flows

- 上电初始化：
  - `hal::Config` 显式启用 ESP32-S3 PSRAM 配置（`PsramSize::AutoDetect` + `SpiRamFreq::Freq40m`）。
  - 初始化显示专用 PSRAM arena，切出 3 份 framebuffer。
  - 初始化内部 RAM DMA staging buffer 和 descriptors。
- render task：
  - 维持 `DISPLAY_MIN_FRAME_INTERVAL_MS=33` 的 cadence。
  - 读取 `TelemetryModel` / `ControlState`，生成最新 `UiSnapshot` 与 `UiChangeMask`。
  - 以 `composed` buffer 为基底复制到 `free` buffer，在其上调用 `ui::render` / `ui::render_partial`。
  - 生成 dirty rows/spans；若是 full-screen 场景，则直接标记整帧。
  - 提交最新 pending frame；若旧 pending 尚未 present，则覆盖并增加 drop counter。
- present task：
  - 独占 `lcd_async::Display`、背光与 sleep/wake 流程。
  - 等待 pending frame 后，按 dirty spans + chunk staging 把数据送到 ST7789。
  - 完成后更新 `displayed_idx`，释放旧 displayed buffer。
  - 记录 `display_present_ms`、dirty rows、full present 次数。

### Edge cases / errors

- 若当前没有 free buffer，render task 不得覆写正在 displaying/presenting 的 buffer；该次更新应被丢弃并计数。
- 若 dirty span 数量超过内部上限或 full-screen force flag 为真，present 退回整帧 flush。
- 若 HIL 观察到 `uart_rx_err_total > 0`，或 `PROTO_DECODE_ERRS / fast_status ok` 相比 baseline 出现持续性恶化，唯一允许的默认回退是 staging chunk 从 `16 rows` 降到 `8 rows` 后重测。
- 任意构建或实机运行中，出现 `Illegal cache access`、stack guard、DMA alignment panic、PSRAM access exception 都视为阻断。

## 接口契约（Interfaces & Contracts）

- None

## 验收标准（Acceptance Criteria）

- Given 默认 release 构建
  When 编译 digital 固件
  Then `just d-build` 通过，且默认构建不再依赖 `previous_framebuffer + memcmp` 的旧路径。
- Given 无 `net_http` release 构建
  When 执行 release build
  Then 编译通过，且显示管线仍使用同一套 PSRAM 多缓冲实现。
- Given 同一块板、同一 cached selector、同一 analog 30Hz 条件
  When 分别运行 baseline 与 candidate monitor
  Then `uart_rx_err_total=0`，且 `PROTO_DECODE_ERRS` / `fast_status ok` 不得持续劣化。
- Given steady dashboard update
  When candidate 运行
  Then steady-state flush 主要走 dirty-span present，`display_present_full_count` 只允许在首帧、wake、view 切换、full-screen view 增长。
- Given candidate 运行
  When 查看版本行与运行日志
  Then digital 版本行与 `tmp/digital-fw-version.txt` 一致，且运行中没有 `Illegal cache access`、`Cp0Disabled`、stack guard、DMA alignment panic、PSRAM exception。

## 实现前置条件（Definition of Ready / Preconditions）

- 已冻结“PSRAM 只承载原始 framebuffer 字节”的边界。
- 已冻结“三缓冲 + latest-wins pending slot + dirty-span present”的实现方向。
- 已冻结默认 cadence 为 33ms、首选 staging chunk 为 16 rows、回退 chunk 为 8 rows。
- 已冻结 HIL 对比口径：同板、同 selector、同 analog 30Hz。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Build:
  - `just d-build`
  - `cd firmware/digital && cargo +esp build --release --no-default-features --features "enable_analog_5v_on_boot,esp32s3,audio_menu"`
- HIL:
  - `just agentd flash digital`
  - `just agentd monitor digital`
  - 必要时只读监测 analog 版本与运行状态

### Quality checks

- `cargo fmt --all`
- 与改动直接相关的 release build / HIL monitor 日志核验

## 文档更新（Docs to Update）

- `docs/specs/README.md`: 记录状态、Last 与 PR 号
- `docs/specs/y4sf4-display-psram-dma-pipeline/SPEC.md`: 跟踪 milestone、HIL 证据和 review 修复

## 计划资产（Plan assets）

- Directory: `docs/specs/y4sf4-display-psram-dma-pipeline/assets/`
- PR visual evidence source: 本次如需截图，只能放在该目录并从 `## Visual Evidence (PR)` 引用

## Visual Evidence (PR)

- Baseline monitor: `.mcu-agentd/monitor/digital/20260318_103504.mon.ndjson`
  - 默认构建仍走旧的单任务整帧 chunked push，日志为 `display: frame ... push complete (dirty_rows=320 dirty_spans=80)`。
  - 稳态 `fast_status ok` 大约落在 `7-10/s`，`uart_rx_err_total=0`。
- Candidate monitor（16 行 chunk，已回退前）: `.mcu-agentd/monitor/digital/20260318_110028.mon.ndjson`
  - `display_present_ms` 多次落在 `141-151ms`，`display_pending_drops` 快速攀升。
  - 依照规格回退到 `4096B / 8 rows`。
- Candidate monitor（最终 8 行 chunk）: `.mcu-agentd/monitor/digital/20260318_112042.mon.ndjson`
  - 启动版本与 `tmp/digital-fw-version.txt` 一致：`src 0x556f8d3862056bfd`。
  - 首帧 `display: frame 1 rendered ... clone_ms=0 render_ms=134 dirty_rows=320 full=true`，对应 `present_ms=69`。
  - 稳态样本：`display_clone_ms=20-22`、`display_present_ms=4-23`（偶发 `130-149` 峰值）、`uart_rx_err_total=0`、`fast_status_ok=325 @ 22.757s`。
  - 与回退前版本相比，clone 成本明显下降，`fast_status ok` 速率提升，`framing_drops` 仍存在但未触发 UART 错误。

## 资产晋升（Asset promotion）

None

## 实现里程碑（Milestones / Delivery checklist）

- [x] M1: 建立新 spec，并完成 baseline build/HIL 证据采集。
- [x] M2: 启用 PSRAM arena 与三 framebuffer 池，删除默认构建的单缓冲退化。
- [x] M3: 拆分 render/present 任务，接入 dirty-span flush 与 display counters。
- [x] M4: 完成 baseline/candidate 对比、spec sync、review 修复与 reviewable PR。

## 方案概述（Approach, high-level）

- 保留现有 `lcd_async + ST7789 + SPI DMA` 组合，不替换控制器初始化与 DCS 流程。
- 通过 PSRAM arena 缓解内部 RAM 压力，把“高容量、低同步要求”的像素存储放到 PSRAM，把“高同步、DMA 敏感”的描述符/状态留在内部 RAM。
- 用 render/present 两段管线替代当前单任务串行路径，让 SPI flush 不再阻塞下一帧的 UI 组合。
- 用 dirty-span flush 替代 framebuffer 全帧比较，减少 steady-state 整帧 present。

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：PSRAM 可用于 DMA，但对齐要求更严；若 carve-out 或 chunk 对齐处理错误，运行期会直接 panic。
- 风险：present 落后于 render 时，三缓冲可能出现 free buffer 紧缺；需要显式记录 drop 行为而不是静默覆写。
- 假设：当前硬件按仓库文档为 `ESP32-S3FH4R2`，片内 2MB PSRAM 可用。
- 假设：cached selector 可继续用于本轮 digital/analog HIL，不需要切换设备。

## 变更记录（Change log）

- 2026-03-18: 创建 spec，冻结 PSRAM 专用 arena、三缓冲 render/present 管线、dirty-span flush、33ms cadence 与 baseline/candidate HIL 验收口径。
- 2026-03-18: 实现 PSRAM 三缓冲 + render/present 双任务，默认 chunk 按规格回退到 `4096B / 8 rows`，并补齐 baseline/candidate HIL 证据。
- 2026-03-18: 创建 reviewable PR #71，spec 状态切换为已完成。

## 参考（References）

- `firmware/digital/src/main.rs`
- `firmware/digital/src/ui/mod.rs`
- `docs/interfaces/pinmaps/esp32-s3.md`
- `docs/other-datasheets/esp32-s3.md`
- `docs/plan/0009:uart-comm-troubleshooting/PLAN.md`
