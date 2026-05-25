# 工作流概述（LoadLynx）

- 多目标（G431 + S3）统一在一个仓库内管理，固件分别存放于 `firmware/` 子目录。
- 控制回路与安全相关逻辑优先落在 G431；S3 侧专注于人机与联网。
- 构建在各自 crate 内完成；固件烧录/复位/监视通过 `mcu-agentd`（仓根 `mcu-agentd.toml`）。
- Web 控制台与 `loadlynx-devd` 的 USB CDC 控制面验证直接使用 `loadlynx-devd`，不使用 `mcu-agentd` selector。

## 分支与工作区
- 建议使用 feature 分支进行新特性/模块开发。
- 若需在独立目录并行开发，可使用 `git worktree` 新建工作区；提交前对齐远端基线。

## 提交规范
- 使用 Conventional Commits（英文），例如：
  - `feat(analog): add ADC sampling skeleton`
  - `chore(digital): setup esp-hal display pipeline`

## 构建与验证
- G431：Rust + Embassy，目标 `thumbv7em-none-eabihf`；由 `mcu-agentd` 调用 probe-rs 完成烧录/复位/监视。
- S3：Rust + esp-hal（可选集成 Embassy）；由 `mcu-agentd` 调用 espflash 完成烧录/复位/监视。

## MCU 端口/探针缓存与 Agentd

- 守护与 CLI：使用外部 `mcu-agentd`（配置见仓根 `mcu-agentd.toml`）。推荐先执行 `just agentd-init` 安装/升级二进制，再通过 Just 封装调用：
  - 启动/状态/停止：`just agentd-start` / `just agentd-status` / `just agentd-stop`。
- 缓存文件：
  - Digital（ESP32‑S3）：仓根 `./.esp32-port`。
  - Analog（STM32G431）：仓根 `./.stm32-port`（旧版 `./.stm32-probe` 仅在 `.stm32-port` 不存在时作为迁移来源，读取后写回 `.stm32-port` 并删除旧文件）。
- 设置与查看缓存（推荐流程）：
  - 设置：`just agentd selector set digital /dev/cu.usbserial-xxxx`；`just agentd selector set analog 0483:3748:SERIAL`。
  - 查看：`just agentd-get-port digital` / `just agentd-get-port analog`。
- 后续所有 `flash` / `reset` / `monitor` 子命令都会优先使用上述缓存值；缓存缺失时可用 `just agentd selector list <mcu>` 查看候选，或用 `just agentd selector set <mcu> --auto`（仅当候选唯一时成功）。

## Web/devd USB CDC 控制面验证

- `loadlynx-devd` 负责 Web 控制台到 ESP32-S3 USB CDC JSONL 的本地桥接，协议见 `docs/interfaces/usb-cdc-jsonl-bridge.md`。
- 使用 `just loadlynx usb-port set digital <path>` 设置默认 ESP32-S3 digital USB CDC 端口；后续 CLI/devd 操作读取该项目本地记忆并使用该端口。
- `.esp32-port` 可以保留 mcu-agentd 兼容的 metadata 行（例如 `mac=...`）；CLI/devd 只把端口路径行作为默认 USB 端口。
- 人工开发时可用 `just loadlynx usb-port set` 或 `just loadlynx usb-port set digital` 进入方向键交互选择；候选项按 `espflash` 默认串口枚举规则展示。Agent 不得用交互候选选择绕过 owner 对 exact path 的批准。
- Web 启动时通过 `VITE_LOADLYNX_DEVD_URL=<devd-url>` 指向当前 devd。
- 真机验证必须证明 devd 与设备完成 JSONL 协议通信，例如收到 `hello` 或成功执行 `get_identity` / `get_status`。串口打开、候选扫描、Web lease 或 firmware dry-run 只能作为辅助证据。
- 该流程复用 `.esp32-port` 作为 ESP32-S3 digital USB CDC 默认端口记忆，但不得读取、修改或依赖 `.stm32-port`，也不得调用 `just agentd selector set`。devd/Web ESP32-S3 digital firmware flash 继续留在 devd 路径：持有 Web lease、校验 artifact hash，并对批准端口调用 direct `espflash`；ELF artifact 使用 `espflash flash`，raw image artifact 必须带 `flash_address` 并使用 `espflash write-bin`。非 devd 固件烧录/复位/monitor 和 analog/probe 操作才进入 `mcu-agentd` 流程。

## 后续里程碑（建议）
- 驱动层：NTC/温度、风扇 PWM、分流/跨阻采样链路
- 控制层：CC/CV/CP 模式，保护（OC/OV/OT/SCP），软启动
- 通信层：UART 帧协议、字段与容错、版本与校准同步
- UI 层：本地按键/旋钮 + Web UI（曲线/记录/标定）
