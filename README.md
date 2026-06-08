# LoadLynx — STM32G431 + ESP32‑S3 便携式电子负载

本仓库采用分体式架构：由 STM32G431 执行快速电流/功率闭环与保护，ESP32‑S3 负责本地 UI、USB/Wi‑Fi 控制面、标定与 preset 持久化，以及与 host tools / Web 的桥接。

- 核心回路（G431，Rust + Embassy）：`firmware/analog/`
- 数字控制板（S3，Rust + esp-hal + Embassy）：`firmware/digital/`
- 共享库与协议：`libs/`
- 文档与脚本：`docs/`, `scripts/`

## 目标与职责

- STM32G431（Cortex‑M4F）
  - 快速 CC/CV 闭环（ADC 采样 + PID）
  - 过流/过温/欠压/短路保护
  - 热传感上报（FET、散热片、远/近端电压电流）
  - 与 S3 通过 UART 帧通信（建议 CBOR/SLIP）
- ESP32‑S3
  - 本地 UI、HTTP/Web 控制面、USB CDC/host-tools bridge
  - EEPROM-backed calibration / presets / PD policy 持久化
  - 与 G431 的可靠 UART 控制链（SoftReset、CalWrite、SetEnable、LimitProfile、SetMode、PD request）
  - 本地风扇 PWM 控制（`FAN_TACH` 与跨 MCU `thermal_derate` 联动仍在后续阶段）
  - Wi‑Fi、mDNS、release Web / CLI / devd 控制入口

## 构建快速开始

本仓库不是“最小脚手架”。当前仓库同时承载可构建的 analog / digital firmware、共享协议与校准库、`loadlynx-devd` / `loadlynx` host tools，以及 Web Console / Storybook / Playwright 验证入口。仍会持续演进的部分主要是硬件定型后的参数、校准数据与真机联调细节，而不是空壳占位代码。

### 环境

- Rust 1.90+（host tooling / analog checks）与 `thumbv7em-none-eabihf` 目标
- probe-rs（由 `mcu-agentd` 作为 STM32 后端调用）
- ESP32‑S3 `esp` Rust toolchain（`espup`）与 `espflash`
- Bun 1.3.14（见仓库根 `.bun-version`；用于 Web UI、Storybook、Playwright 与 bundle budget checks）
- Node.js 20（见仓库根 `.node-version`；用于根目录 workflow / release-label / quality-gate tooling）

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
(cd firmware/analog && cargo build --release --target thumbv7em-none-eabihf)
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
. "$HOME/export-esp.sh"
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

普通用户需要操作硬件时，应从 GitHub Releases 使用 `install-loadlynx-host.sh` / `install-loadlynx-host.ps1` 安装 host tools；安装器会下载对应平台的 `loadlynx-host-tools-*.tar.gz`，用 release `SHA256SUMS` 校验后安装到用户目录，并只打印 PATH 提示，不自动修改 shell/profile。也可以手动下载 archive，但必须先用 `SHA256SUMS` 校验。发布包包含 `loadlynx-devd` 本地守护程序 / USB bridge，以及 `loadlynx` CLI 工具。CLI/devd 本地控制为 IPC-first：`loadlynx` 通过本地 IPC endpoint 与 sibling `loadlynx-devd serve` 通信，并可按需 auto-start；macOS/Linux 默认使用 Unix socket，Windows 默认使用 named pipe，`--ipc` / `--endpoint` 仅在需要覆盖默认 endpoint 时使用。旧的普通 `--devd http://...` CLI 路径不再作为用户操作入口。`loadlynx-devd bridge-http` 仅用于浏览器/Web/debug bridge，必须绑定 loopback。用户侧通过 `loadlynx` CLI 操作硬件：USB/devd IPC 优先，HTTP 作为已保存 transport fallback。CLI 用 `loadlynx hardware available/path/list/bind/default/use/forget` 维护以稳定 `identity.device_id` 为主键的用户级硬件 registry；USB 与 HTTP 可绑定到同一个硬件，`last_transport` 记录上次使用的连接方式，`loadlynx status` 使用已保存 default，`loadlynx status --hardware <id>` 指定硬件。临时 USB candidate ID 只能用于 `hardware bind usb --candidate <id>`，不得直接用于控制、诊断、烧录或监控；USB bind/control 会校验固件 identity，旧固件若仍返回 `digital-esp32s3` 这类非稳定 ID 必须先升级。硬件记忆保存到用户配置目录：macOS `~/Library/Application Support/LoadLynx/devices.json`，Linux `${XDG_CONFIG_HOME:-~/.config}/loadlynx/devices.json`，Windows `%APPDATA%\LoadLynx\devices.json`，可用 `LOADLYNX_HOME` 覆盖目录。若安装版 CLI 不支持 WiFi 配置，不能退回 raw HTTP，需要进入开发/维护路径补齐并发布。用户侧固件烧录必须使用同一 Release 发布的 firmware catalog/assets，并先确认当前 `loadlynx flash --help` 支持所需流程；真实 ESP32-S3 flash 需要 artifact/hash/target evidence、`yes` 确认、非项目固件风险确认（如适用）和 post-flash identity capture。GitHub Pages 与 release Web bundle 也是正式 Web Serial 人类操作入口；Web Serial 仅保存 identity/profile，不保存 OS 端口路径。不做桌面壳。从源码构建、`just`、项目开发端口缓存、缺失 CLI 功能实现和 HIL 验证属于开发/维护路径。

常用控制命令：

```sh
loadlynx cc 2000 --hardware <id>
loadlynx cv 24500 --hardware <id>
loadlynx cp 60000 --hardware <id>
loadlynx cc 2000 --hardware <id> --disable
```

常用本地入口：

```sh
# 设置 CLI/devd 默认 ESP32-S3 digital USB CDC 设备
just loadlynx usb-port set digital /dev/cu.usbmodemXXXX

# 人工交互选择端口（方向键选择，候选项按 espflash 默认串口枚举规则）
just loadlynx usb-port set digital

# 启动 devd IPC（CLI 路径）
just devd-serve --endpoint /tmp/loadlynx-devd.sock

# 启动 devd HTTP bridge（浏览器/Web 路径，loopback only）
just devd-bridge-http --bind 127.0.0.1:30180 --allow-dev-cors

# 启动 Web，并显式指向 HTTP bridge
(cd web && VITE_LOADLYNX_DEVD_URL=http://127.0.0.1:30180 bun run dev)
```

真机验证应证明 devd 对指定串口完成 USB CDC JSONL 通信，例如收到 `hello` 或成功执行 `get_identity` / `get_status`。仅证明串口能打开、出现候选设备、创建 lease/session，或只完成 firmware dry-run，不足以说明 CLI/devd 真机链路可用。

## 质量门与日常验证

- 推荐顺序：

```sh
just deps
git submodule update --init --recursive   # 若需要 analog / embedded 检查
just deps-web-browsers   # 仅当需要 Playwright / Storybook 浏览器检查
just check               # 日常快速自检
just check-full          # 更接近 CI 的全量检查
```

常用入口：

- `just deps`：安装根目录 Node.js 依赖与 `web/bun.lock` 对应的 Web 依赖。
- `just deps-web-browsers`：只为 `just check-web-full` / Playwright / Storybook 浏览器检查安装浏览器二进制。
- `just fmt` / `just fmt-check`：统一的 Rust + Web 格式化入口。
- `just check`：本地快速质量门，覆盖格式检查、release-label / workflow 契约、host-side tests 和 Web 快速检查。
- `just check-full`：在 `just check` 基础上增加 host static checks、embedded clippy/build、Storybook / Playwright 浏览器检查，更接近 CI，但仍不触发 release、deploy 或硬件 HIL。

关键前置条件：

- analog / embedded 相关检查依赖 `third_party/embassy` 子模块；若 worktree 里只有空目录，先执行 `git submodule update --init --recursive`。
- digital / ESP32 相关入口（`just fmt*` 中的 digital 格式检查、`just d-clippy`、`just d-build`、`just check-embedded`）依赖 `cargo +esp` 与 `$HOME/export-esp.sh`；缺失时命令会直接提示先完成 `espup` 安装，而不是把错误留给底层构建链。
- `just d-build` 还要求 digital Wi-Fi 编译配置可用：优先从仓库根 `.env` 读取 `DIGITAL_WIFI_SSID` / `DIGITAL_WIFI_PSK`，也可在命令前临时注入同名环境变量；缺失时会先给出 `.env.example` 提示再退出。
- `just d-clippy` 使用临时 dummy Wi-Fi 编译配置，因此不要求仓库根 `.env` 存在；它的目标是把 digital lint gate 对齐到 CI，而不是验证真实网络配置。
- `just check-root` / `just check-web*` 在缺少依赖时会直接报出对应的 `deps-*` 提示，而不是把错误延迟到下层工具。
- `just fmt` 现在会在任一 crate / Web 格式化失败时直接报错，不再吞掉失败。

更细的入口分层、CI 对应关系和操作员视角说明放在 `WORKFLOW.md`；Web 专项矩阵继续看 `web/README.md`。

## 目录结构

- `firmware/analog/` — G431 上运行的 Embassy 应用（控制环路 + 遥测流）
- `firmware/digital/` — S3 上运行的 Rust + esp‑hal 应用（本地 UI + UART 链路终端）
- `libs/` — 共享驱动与协议约定（当前包含无分配的 MCU↔MCU 协议 crate `loadlynx-protocol`）
- `docs/` — 控制环路 / 热设计 / 接口协议与板级文档
- `scripts/` — 开发辅助脚本

规范与当前实现状态优先查看：

- 项目入口与操作约定：`README.md`、`WORKFLOW.md`
- 设计与产品边界：`DESIGN.md`、`PRODUCT.md`
- 可复用结论：`docs/solutions/**`
- 长期 topic-level specs：`docs/specs/**`

## 发布流程

LoadLynx 的正式发布意图由 PR labels 决定。每个 PR 必须恰好包含一个
`type:major|minor|patch|none` 和一个 `channel:stable|beta|dev`；可选
`component:firmware|web|host-tools|docs` 用于说明影响面。`Label Gate`
在合并前校验该契约。

`main` 必须保持 PR-only：禁止直接 push，管理员同样受保护约束；但 PR 不要求
人工 approval，`required_approvals` 固定为 `0`，只要求通过声明的 required check
与签名提交约束。

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
