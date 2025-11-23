# MCU Agent 服务设计（LoadLynx）

## 1. 目的与范围

- 提供单实例的本地守护与 CLI 客户端，统一管理 **STM32G431（analog）** 与 **ESP32‑S3（digital）** 的构建、烧录、复位、日志采集、端口/探针缓存。
- 面向自动化/Agent 场景，所有操作需返回本地时间戳、耗时与结果，避免日志缺口，方便后续脚本和 CI 采集。
- 与现有脚本（`scripts/agent_verify_*.sh`）兼容并逐步替换，复用现有版本文件与端口选择逻辑。

## 2. 设计概览

- 语言/框架：Rust（tokio + clap），独立二进制 crate，工程位置 `tools/mcu-agentd/`。
- 实例模型：单可执行兼具守护与客户端；Unix socket + 锁文件保证单实例；每 MCU 独立资源锁防止串口/探针争用。
- 端口/探针缓存：仓根 `.esp32-port`、`.stm32-probe`（兼容 `.stm32-port` 别名）；提供 set/get/list；默认沿用 `scripts/ensure_esp32_port.sh` 与 `scripts/ensure_stm32_probe.sh` 的筛选规则。
- 运行策略：
  - 构建 → 按需烧录 → 复位/日志分离；烧录默认不自动长时间运行，只在 monitor/attach 时启动目标。
  - Analog 端严格版本门控以保护 Flash；Digital 端依赖 `espflash --skip` 机制减少重复写入。
- 日志目录：元数据与会话日志写入 `logs/agentd/`（新）；兼容同步镜像到 `tmp/agent-logs/` 方便现有脚本。

## 3. 命令面（初版）

- `start | stop | status`：管理守护；返回 PID、锁状态、当前时间、活跃会话。
- `set-port --mcu {digital,analog} --path PATH` / `get-port --mcu ...` / `list-ports --mcu ...`：读写 `.esp32-port` / `.stm32-probe` 缓存；列举可用端口/探针。
- `flash --mcu digital --elf PATH [--after no-reset|hard-reset]`
- `flash --mcu analog --elf PATH`
- `reset --mcu {digital,analog}`：复位后写入元事件（必要时附短日志窗口）。
- `monitor --mcu digital [--elf PATH] [--timeout SEC]`：相当于 `make d-run`（flash+monitor），可指定日志时长。
- `attach --mcu analog [--elf PATH] [--timeout SEC]`：相当于 `make a-reset-attach`，受版本门控。
- `logs --mcu {digital,analog,all} [--since RFC3339] [--until RFC3339] [--tail N]`：按时间/数量筛选元数据或会话日志。
- 构建兜底：`--elf` 缺省时自动调用 `make d-build` 或 `make a-build` 生成默认 ELF；构建结果对齐 `tmp/{digital,analog}-fw-version.txt`。

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
  - 探针选择：沿用 `.stm32-probe` 缓存；若无缓存则按唯一 ST-Link/唯一探针自动选择，否则报错并提示运行 `scripts/select_stm32_probe.sh` 一次。

## 5. 单实例与资源锁

- 锁文件：`tmp/mcu-agentd.lock` 控制单实例；守护启动时获取，客户端通过 Unix socket 连接。
- Socket：`tmp/mcu-agentd.sock`，权限 600；客户端命令经 socket 发送，守护处理。
- MCU 级资源锁：`/tmp/loadlynx-mcu-agentd-{analog|digital}.lock`，确保串口/探针操作不交叉。
- 进程模型：守护持久化；客户端为短进程。守护异常退出时在心跳中标注，下一次启动尝试清理陈旧锁。

## 6. 端口与探针缓存

- 文件：
  - Digital：`./.esp32-port`
  - Analog：`./.stm32-probe`（兼容读取 `./.stm32-port` 如存在）
- `set-port` 会验证目标存在；`list-ports` 读取 `scripts/ensure_esp32_port.sh` / `probe-rs list` 输出并过滤唯一项。
- 客户端命令均可通过 `--port` / `--probe` 显式覆盖缓存。

## 7. 版本门控与构建兜底

- 版本文件：沿用现有生成文件 `tmp/analog-fw-version.txt`、`tmp/digital-fw-version.txt`；守护在构建后读取。
- Analog 烧录决策：比较 `tmp/analog-fw-version.txt` 与 `tmp/analog-fw-last-flashed.txt`，不同则执行 `flash`，成功后更新 last-flashed。
- Digital 烧录决策：直接执行 `espflash flash ...`，依赖其跳过未变分区；仍记录版本到元数据。
- 构建兜底：当命令缺省 `--elf` 时触发 `make a-build` / `make d-build`（PROFILE 默认 release，可由配置/命令覆盖）。

## 8. 日志与时间戳

- 时间：RFC3339 `ts` + 单调毫秒 `mono_ms`；守护启动时记录基准。
- 元数据（NDJSON）：`logs/agentd/{analog,digital}.meta.log`，按大小/天数滚动。字段示例：
  ```json
  {"ts":"2025-11-23T14:05:31.842-08:00","mono_ms":124422,"mcu":"digital","event":"flash","elf":".../digital/target/xtensa-esp32s3/release/digital","port":"/dev/cu.usbmodemXYZ","status":"ok","code":0,"duration_ms":8123,"op_id":"op-20251123-1405-digital-1"}
  ```
- 会话日志：`logs/agentd/{analog,digital}/YYYYMMDD_HHMMSS.session.log`；行前缀为 NDJSON 元数据 + 原始 defmt/串口行。按 `--timeout` 控制时长；默认同步一份到 `tmp/agent-logs/` 便于现有流程。
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
| 端口/探针缓存 | 读写 `.esp32-port`、`.stm32-probe` | set/get/list 正确读写并验证存在 | 待开发 | 兼容 `.stm32-port` 读取 |
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
