# MCU Agent 服务设计（LoadLynx）

## 1. 目的与范围

- 提供单实例的本地守护与 CLI 客户端，统一管理 **STM32G431（analog）** 与 **ESP32‑S3（digital）** 的构建、烧录、复位、日志采集、端口/探针缓存。
- 面向自动化/Agent 场景，所有操作需返回本地时间戳、耗时与结果，避免日志缺口，方便后续脚本和 CI 采集。
- 与现有脚本（`scripts/agent_verify_*.sh`）兼容并逐步替换，复用现有版本文件与端口选择逻辑。

## 2. 设计概览

- 语言/框架：Rust（tokio + clap），独立二进制 crate，工程位置 `tools/mcu-agentd/`，二进制名 `loadlynx-agentd`（通过 Just 配方 `just agentd ...` 调用）。
- 实例模型：单可执行兼具守护与客户端；Unix socket + 锁文件保证单实例；每 MCU 独立资源锁防止串口/探针争用。
- 端口/探针缓存：仓根 `.esp32-port`（digital）、`.stm32-port`（analog）。旧版遗留的 `.stm32-probe` 若存在，仅在 `.stm32-port` 缺失时作为迁移来源：读取其值、写入 `.stm32-port` 后删除旧文件。提供 set/get/list；默认沿用 `scripts/ensure_esp32_port.sh` 与 `scripts/ensure_stm32_probe.sh` 的筛选规则（后者内部已切换到 `.stm32-port`）。
- 运行策略：
  - 构建 → 按需烧录 → 复位/日志分离；烧录默认不自动长时间运行，只在 monitor/attach 时启动目标。
  - Analog 端严格版本门控以保护 Flash；Digital 端依赖 `espflash --skip` 机制减少重复写入。
- 日志目录：元数据与会话日志写入 `logs/agentd/`；`tmp/agent-logs/` 仅作为兼容占位目录保留（当前实现不再主动同步镜像）。

## 3. 命令面（当前）

- `start | stop | status`：管理守护；返回 PID、socket 路径以及当前时间。通常通过 `just agentd start|stop|status` 或 `just agentd-start/agentd-stop/agentd-status` 调用。
- `set-port <mcu> [PATH]` / `get-port <mcu>` / `list-ports <mcu>`：`mcu ∈ {digital,analog}`。读写 `.esp32-port` / `.stm32-port` 缓存；列举可用端口/探针。`set-port` 在 PATH 省略时会进入交互式选择。
- `flash <mcu> [ELF] [--after {no-reset|hard-reset}]`：烧录固件但不自动长时间运行；`mcu ∈ {digital,analog}`。digital 支持 `--after` 控制 espflash 的 reset 策略（analog 忽略该参数）。`ELF` 省略时使用默认 release ELF 路径；若该文件不存在则报错，提示先构建或显式提供 `ELF`。
- `reset <mcu>`：仅复位 MCU，不烧录；复位结果写入 meta 日志（必要时附短日志窗口），Analog 端在部分 “interfaces are claimed” 情况下会视为软成功并继续重启 monitor。
- `monitor <mcu> [ELF] [--reset] [--duration DUR] [--lines N]`：拉起/附着到一段日志会话。`--reset` 会在监视前触发一次 `reset` 并等待新的会话文件出现；`--duration` 使用 humansize 时长（如 `30s`/`2m`，0 表示不按时间截断），`--lines` 控制从会话中输出的最大行数（0 表示不限）。目前 `ELF` 仅作为预留位置，实际监视基于守护端已配置的默认 ELF。
- `logs <mcu|all> [--since RFC3339] [--until RFC3339] [--tail N] [--sessions]`：按时间/数量筛选元数据（meta 日志）；`--sessions` 为 true 时附带按会话聚合的日志片段（每个会话单独 tail N 行）。`--tail` 省略时使用守护配置中的默认值（当前为 200）。
- 构建兜底（规划中）：当前实现不会自动调用 `make a-build` / `make d-build`，而是直接报错并要求用户先构建或提供显式 `ELF`；未来如需要可在守护层引入可选的自动构建逻辑。

## 4. 底层命令选择

- **ESP32‑S3（digital）**
  - 烧录：`espflash flash <elf> --chip esp32s3 --port <cache> --after no-reset --ignore_app_descriptor --non-interactive --skip-update-check`
  - 监视：`espflash monitor --chip esp32s3 --port <cache> --no-reset --non-interactive --elf <elf> --log-format defmt`
  - 复位：`espflash reset --chip esp32s3 --port <cache> --after hard-reset`
  - 端口获取：兼容现有 `scripts/ensure_esp32_port.sh` 逻辑；优先显式 `PORT` → `.esp32-port` → 唯一可用端口。

- **STM32G431（analog）**
  - 烧录：`probe-rs download --chip STM32G431CB --probe <cache> <elf>`（不复位）
  - 监视：`probe-rs run --chip STM32G431CB --probe <cache> --log-format defmt <elf>`（或 `reset-attach` 变体）
  - 复位：`probe-rs reset --chip STM32G431CB --probe <cache>`
  - 探针选择：沿用 `.stm32-port` 缓存；若仅存在旧版 `.stm32-probe`，在无 `.stm32-port` 时作为一次性迁移来源。若仍无缓存则按唯一 ST-Link/唯一探针自动选择，否则报错并提示运行 `scripts/select_stm32_probe.sh` 一次。

## 5. 单实例与资源锁

- 锁文件：`logs/agentd/agentd.lock` 控制单实例；守护启动时获取，客户端通过 Unix socket 连接。
- Socket：`logs/agentd/agentd.sock`，权限 600；客户端命令经 socket 发送，守护处理。
- MCU 级资源锁：`/tmp/loadlynx-mcu-agentd-{analog|digital}.lock`，确保串口/探针操作不交叉。
- 进程模型：守护持久化；客户端为短进程。守护异常退出时在心跳中标注，下一次启动尝试清理陈旧锁。

## 6. 端口与探针缓存

- 文件：
  - Digital：`./.esp32-port`
  - Analog：`./.stm32-port`（`.stm32-probe` 仅作为旧版本遗留缓存的迁移来源：当 `.stm32-port` 不存在时读取其值写回新文件并删除旧文件）
- `set-port` 会验证目标存在；`list-ports` 读取 `scripts/ensure_esp32_port.sh` / `probe-rs list` 输出并过滤唯一项。
- 当前 agentd CLI 不再暴露 `--port` / `--probe` 覆盖选项，端口/探针选择完全由缓存文件与 helper 脚本负责。

## 7. 版本门控与构建兜底

- 版本文件：沿用现有生成文件 `tmp/analog-fw-version.txt`、`tmp/digital-fw-version.txt`，供日志与后续版本门控使用。
- Analog 烧录决策（规划中）：设计上预留了 `tmp/analog-fw-last-flashed.txt` 等文件用于比较版本并避免重复烧录，但当前实现尚未启用该门控逻辑。
- Digital 烧录决策：直接执行 `espflash flash ...`，依赖其自身的 “未变分区跳过” 机制；仍记录版本到元数据。
- 构建兜底（规划中）：目前当命令缺省 `ELF` 时仅尝试使用默认 release ELF 路径；如文件不存在则报错并提示用户自行运行 `make a-build` / `make d-build`（或等价的 `just` 配方），未来可根据需要在守护层加入自动构建能力。

## 8. 日志与时间戳

- 时间：RFC3339 `ts` + 单调毫秒 `mono_ms`；守护启动时记录基准。
- 元数据（NDJSON）：`logs/agentd/{analog,digital}.meta.log`，按大小/天数滚动。字段示例：
  ```json
  {"ts":"2025-11-23T14:05:31.842-08:00","mono_ms":124422,"mcu":"digital","event":"flash","elf":".../digital/target/xtensa-esp32s3/release/digital","port":"/dev/cu.usbmodemXYZ","status":"ok","code":0,"duration_ms":8123,"op_id":"op-20251123-1405-digital-1"}
  ```
- 会话日志：`logs/agentd/{analog,digital}/YYYYMMDD_HHMMSS.session.log`；行前缀为 NDJSON 元数据 + 原始 defmt/串口行。按 monitor 子命令传入的 `--duration` / `--lines` 控制截取窗口；默认同步一份到 `tmp/agent-logs/` 便于现有流程。
- 心跳：守护每 60s 写入 `{event:"heartbeat", active_sessions:[...]}` 到元日志。

## 9. 配置与环境

- 默认配置文件：`configs/mcu-agentd.toml`（新），可覆盖：构建 profile、日志滚动策略、默认超时、芯片型号字符串、底层命令附加参数。
- 环境变量优先级：命令行 > 环境 > 配置文件 > 内置默认。

## 10. 路径与与现有脚本的关系

- 新建 crate：`tools/mcu-agentd/`（与 ups120 项目保持一致）。
- 兼容层：
  - 守护提供与 `scripts/agent_verify_{analog,digital}.sh` 等价的操作，通过子命令包装现有 Make 目标；短期可直接复用这些脚本（由守护调用）以降低首版风险。
  - 逐步将脚本逻辑迁移到 Rust 实现后，脚本保留为 thin wrapper 以供手动使用。
- 依赖：沿用现有构建工具链（probe-rs、espflash、cargo +esp）。

## 11. 验收与进展追踪

| 功能 | 描述 | 验收标准 | 状态 | 备注 |
| --- | --- | --- | --- | --- |
| 单实例守护 | 锁文件+Unix socket；start/stop/status 可用 | 第二实例返回 already running；status 显示 PID/活跃会话 | 待开发 |  |
| 端口/探针缓存 | 读写 `.esp32-port`、`.stm32-port` | set/get/list 正确读写并验证存在 | 待开发 | `.stm32-probe` 仅作为旧缓存迁移来源 |
| 烧录 ESP32 | `espflash flash` 默认 `--after no-reset` | 返回 ts/耗时；未变区域被跳过 | 待开发 |  |
| 烧录 STM32 | `probe-rs download` 不复位 | 返回 ts/耗时；版本门控生效 | 待开发 |  |
| 复位控制 | per MCU 调用 espflash/probe-rs reset | 元事件记录 ts/code；可选短日志 | 待开发 |  |
| 日志采集 | monitor/attach 捕获输出 + 会话/元日志 | logs 子命令可按 MCU/时间过滤；tail N 生效 | 待开发 |  |
| 构建兜底 | 缺 ELF 自动 make a-build / d-build | 自动生成后继续原指令 | 待开发 |  |
| 配置文件 | `configs/mcu-agentd.toml` 覆盖默认 | 配置生效，env/CLI 可覆盖 | 待开发 |  |

## 12. 后续迭代建议

- 增加 HTTP/gRPC 接口以便 CI/外部系统调用。
- 自动检测端口热插拔并刷新缓存。
- 安全模式：烧录前自动备份旧固件（ESP32 读 flash，STM32 读整片）。
- 可选 SQLite 索引：为 meta 日志提供 `ts,mcu,event` 索引，保持文件格式不变。
