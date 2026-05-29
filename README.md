# LoadLynx — STM32G431 + ESP32‑S3 便携式电子负载

本仓库采用分体式架构：由 STM32G431 执行快速电流/功率闭环与保护，ESP32‑S3 负责 UI / Wi‑Fi / OTA / 记录与标定，以及与上位机工具的桥接。

- 核心回路（G431，Rust + Embassy）：`firmware/analog/`
- 宿主桥（S3，Rust + esp-hal + Embassy）：`firmware/digital/`
- 共享库与协议：`libs/`
- 文档与脚本：`docs/`, `scripts/`

## 目标与职责

- STM32G431（Cortex‑M4F）
  - 快速 CC/CV 闭环（ADC 采样 + PID）
  - 过流/过温/欠压/短路保护
  - 热传感上报（FET、散热片、远/近端电压电流）
  - 与 S3 通过 UART 帧通信（建议 CBOR/SLIP）
- ESP32‑S3
  - 本地 UI 与 Web UI，Wi‑Fi / OTA
  - 数据记录、标定流程与上位机桥接
  - 风扇 PWM / Tach 直接控制与温控策略
  - 与 G431 的可靠链路与升级/诊断工具

## 构建快速开始

本仓库仅提供最小可编译脚手架（占位代码）。实际硬件驱动、管脚、控制环参数需根据原理图与 PCB 定稿同步更新。

### 环境

- Rust nightly（embedded） + `thumbv7em-none-eabihf` 目标
- probe-rs（由 `mcu-agentd` 作为 STM32 后端调用）
- ESP32‑S3 Xtensa 工具链（`espup`）与 `espflash`（由 `mcu-agentd` 作为 ESP32 后端调用）

推荐用 `just` 作为统一入口：构建用 `just a-build` / `just d-build`；固件烧录/复位/监视通过 `mcu-agentd`（见下文 `MCU Agent`）。CLI/devd 的 USB CDC 验证不使用 `mcu-agentd` selector。

### G431（analog）

常用入口（仓库根目录执行）：

```sh
# 构建（默认 PROFILE=release）
just a-build

# 烧录（通过 mcu-agentd，需先接好调试 probe）
just agentd flash analog

# 监视（可选：复位后从头输出）
just agentd monitor analog --reset
```

备用：直接在子 crate 下构建：

```sh
(cd firmware/analog && cargo build --release)
```

### ESP32‑S3（digital）

常用入口（仓库根目录执行）：

```sh
# 构建（Rust + esp-hal，默认 PROFILE=release）
just d-build

# 烧录（通过 mcu-agentd）
just agentd flash digital

# 监视（可选：复位后从头输出）
just agentd monitor digital --reset
```

备用：直接在子 crate 下构建：

```sh
(cd firmware/digital && cargo +esp build --release)
```

### MCU Agent 守护进程

`mcu-agentd` 提供单实例守护与 CLI（外部仓库 `../mcu-agentd`），本仓库根目录 `Justfile` 封装了常见子命令。建议先执行一次安装/升级（会安装 `mcu-agentd`/`mcu-managerd` 到本机 cargo bin）：
`just agentd-init`（默认使用 `../mcu-agentd`，也可 `MCU_AGENTD_PATH=/path/to/mcu-agentd just agentd-init`）。

项目配置在 `mcu-agentd.toml`。常见子命令示例：

```sh
just agentd-start                       # 启动后台守护
just agentd-status                      # 查询状态
just agentd-stop                        # 停止

# 设置端口/探针缓存（写入仓根项目开发缓存）
just agentd selector set digital /dev/cu.usbserial-xxxx
just agentd selector set analog 0483:3748:SERIAL   # 例：ST-Link VID:PID:SER

# 查看当前缓存
just agentd-get-port digital
just agentd-get-port analog
```

### CLI + devd USB CDC 控制面

`loadlynx-devd` 是 CLI 访问 ESP32-S3 USB CDC JSONL 的本地守护。验证 CLI/devd 控制面时通过 `just loadlynx usb-port set digital <path>` 复用仓根项目开发端口缓存作为默认端口记忆，不要切换 `mcu-agentd selector`。CLI/devd 的 ESP32-S3 digital firmware flash 也走 devd：持有 lease/session、校验 artifact hash，并对批准的项目开发端口调用 direct `espflash`；ELF artifact 使用 `espflash flash`，raw image artifact 必须带 `flash_address` 并使用 `espflash write-bin`。不要退回 `just agentd flash digital`。如果项目开发端口缓存包含 selector metadata，CLI/devd 只使用端口路径行。

普通用户需要操作硬件时，应从 GitHub Releases 下载对应平台的 `loadlynx-host-tools-*.tar.gz`，并通过 `loadlynx` CLI 操作硬件：USB/devd 优先，HTTP 其次。该发布包包含 `loadlynx-devd` 本地守护程序 / USB bridge，以及 `loadlynx` CLI 工具（当前源码可见命令包括 `discover`、`devices`、`status`、`output set`、`usb-port set`、`hardware`、`flash`、`reset`、`monitor`）。用户侧 CLI 用 `loadlynx hardware available/recent/path/list/save/forget` 和 `loadlynx status --hardware <id>` 记忆、查找、列出可连接设备、列出最近连接设备、列出已记住设备与遗忘设备；`status --device` 与 `status --url` 成功后会更新用户级记忆，后续优先找回 USB 设备，再 fallback 到 HTTP 设备。硬件记忆保存到用户配置目录：macOS `~/Library/Application Support/LoadLynx/devices.json`，Linux `${XDG_CONFIG_HOME:-~/.config}/loadlynx/devices.json`，Windows `%APPDATA%\LoadLynx\devices.json`，可用 `LOADLYNX_HOME` 覆盖目录。若安装版 CLI 不支持 WiFi 配置，不能退回 Web UI，需要进入开发/维护路径补齐并发布。用户侧固件烧录必须使用同一 Release 发布的 firmware catalog/assets，并先确认当前 `loadlynx flash --help` 支持所需流程。从源码构建、`just`、项目开发端口缓存、缺失 CLI 功能实现和 HIL 验证属于开发/维护路径。

常用本地入口：

```sh
# 设置 CLI/devd 默认 ESP32-S3 digital USB CDC 设备
just loadlynx usb-port set digital /dev/cu.usbmodemXXXX

# 人工交互选择端口（方向键选择，候选项按 espflash 默认串口枚举规则）
just loadlynx usb-port set digital

# 启动 devd
just devd-serve --bind 127.0.0.1:30180 --allow-dev-cors

# 启动 Web，并显式指向 devd
(cd web && VITE_LOADLYNX_DEVD_URL=http://127.0.0.1:30180 bun run dev)
```

真机验证应证明 devd 对指定串口完成 USB CDC JSONL 通信，例如收到 `hello` 或成功执行 `get_identity` / `get_status`。仅证明串口能打开、出现候选设备、创建 lease/session，或只完成 firmware dry-run，不足以说明 CLI/devd 真机链路可用。

## 目录结构

- `firmware/analog/` — G431 上运行的 Embassy 应用（控制环路 + 遥测流）
- `firmware/digital/` — S3 上运行的 Rust + esp‑hal 应用（本地 UI + UART 链路终端）
- `libs/` — 共享驱动与协议约定（当前包含无分配的 MCU↔MCU 协议 crate `loadlynx-protocol`）
- `docs/` — 控制环路 / 热设计 / 接口协议与板级文档
- `scripts/` — 开发辅助脚本

## 发布流程

LoadLynx 的正式发布意图由 PR labels 决定。每个 PR 必须恰好包含一个
`type:major|minor|patch|none` 和一个 `channel:stable|beta|dev`；可选
`component:firmware|web|host-tools|docs` 用于说明影响面。`Label Gate`
在合并前校验该契约。

合并到 `main` 后，`Release (LoadLynx)` 会读取源 PR 的 labels，计算下一版本，
并把该版本注入固件、Web 与 host-tools 发布包。Stable 发布使用 `vX.Y.Z` tag；
beta/dev 发布为 prerelease。发布成功后 workflow 会在源 PR 留下版本、release
链接、产物列表和 run 链接。Telegram 通知只覆盖 release workflow 失败，不覆盖普通
PR CI 失败。

## 片间通信建议

- 默认：UART + 帧编码（CBOR/SLIP），易调试、鲁棒、带宽足够
- 预留：SPI/I²C 可选（视硬件走线与带宽/时延需求）

## 致谢

- Embassy 项目（异步 HAL 与执行器）
- ESP‑IDF（ESP32 官方框架）
