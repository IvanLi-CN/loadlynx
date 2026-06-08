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
- S3：Rust + esp-hal + Embassy；由 `mcu-agentd` 调用 espflash 完成烧录/复位/监视。

## 当前质量门

- README 保留“主入口 + 关键前置条件”；本文件保留“入口分层 + CI 对应关系 + 操作员说明”。
- 依赖准备：
  - `just deps-root`：根目录 Node.js 依赖（release-label / quality-gates / workflow hygiene）
  - `just deps-web`：`web/bun.lock` 对应的 Web 依赖
  - `just deps`：默认本地依赖准备
  - `just deps-web-browsers`：仅浏览器检查需要
- 格式化：
  - `just fmt`
  - `just fmt-check`
- 本地检查入口：
  - `just check`：默认快路径
  - `just check-full`：接近 CI 的重路径
  - `just check-root`
  - `just test-host`
  - `just lint-host`
  - `just lint-host-optional`
  - `just check-web`
  - `just check-web-full`
  - `just check-embedded`
  - `just lint-embedded`
- CI workflow 分工：
  - `Code Check`：根目录 policy + host Rust + embedded build checks
  - `Web Check`：Web lint / build / unit / storybook / e2e / bundle budgets
  - `Label Gate`：PR release-intent labels
  - `Release (LoadLynx)`：正式 release 产物组装
  - `Web Deploy (GitHub Pages)`：GitHub Pages bundle 发布
  - `Code Check` 内的 `digital-firmware` job 已覆盖 digital firmware 的格式检查与无硬件构建验证；不再单独维护重复的 `Digital Check` workflow

## 本地入口与 CI 对应关系

- `just check-root` ≈ `Code Check` 中的 release-label / repo policy / workflow hygiene 部分
- `just test-host` ≈ `Code Check` 中的 host Rust test 部分
- `just lint-host` ≈ `Code Check` 中的 host clippy + installer shell checks
- `just lint-host-optional`：本地可选 PowerShell 语法检查；CI 始终执行，缺 `pwsh` 的本地环境仅提示跳过
- `just check-embedded` ≈ `Code Check` 中的 analog / digital build 部分
- `just lint-embedded` ≈ `Code Check` 中的 analog clippy + digital clippy 部分
- `just check-web` + `just check-web-full` ≈ `Web Check`
- `just deps-root` / `just deps-web` / `just deps-web-browsers`：显式声明本地检查依赖准备步骤
- analog / embedded 本地检查还依赖 `third_party/embassy` 子模块；缺失时先执行 `git submodule update --init --recursive`
- digital / ESP32 本地入口还依赖 `cargo +esp` 与 `$HOME/export-esp.sh`；缺失时 `just fmt*`、`just d-clippy`、`just d-build`、`just check-embedded` 会先给出显式 setup hint
- `just d-build` 还要求 digital Wi-Fi 编译配置存在：默认读取仓库根 `.env` 的 `DIGITAL_WIFI_SSID` / `DIGITAL_WIFI_PSK`，也接受同名临时环境变量；缺失时先 fail-fast 提示 `.env.example`
- `just d-clippy` 会注入 dummy Wi-Fi 编译配置，所以不依赖仓库根 `.env`；它只验证 digital firmware 在默认 feature 集下的 lint gate
- `just check`：日常快速自检
- `just check-full`：尽量贴近 CI，但不触发 release / deploy / hardware side effects

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

## 文档真相源

- 仓库入口事实：`README.md`、`WORKFLOW.md`
- 长期功能/契约：`docs/specs/**`
- 可复用经验：`docs/solutions/**`
- Web 专项约束：`web/README.md`
- 接口与板级资料：`docs/interfaces/**`、`docs/boards/**`

## 长期演进主题
- 驱动层：NTC/温度、风扇 PWM、分流/跨阻采样链路
- 控制层：CC/CV/CP 模式，保护（OC/OV/OT/SCP），软启动
- 通信层：UART 帧协议、字段与容错、版本与校准同步
- UI 层：本地按键/旋钮 + Web UI（曲线/记录/标定）
