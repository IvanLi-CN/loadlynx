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

推荐用 `just` 作为统一入口：构建用 `just a-build` / `just d-build`；**烧录/复位/监视一律通过 `mcu-agentd`**（见下文 `MCU Agent`）。

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

# 设置端口/探针缓存（写入仓根 .esp32-port / .stm32-port）
just agentd selector set digital /dev/cu.usbserial-xxxx
just agentd selector set analog 0483:3748:SERIAL   # 例：ST-Link VID:PID:SER

# 查看当前缓存
just agentd-get-port digital
just agentd-get-port analog
```

## 目录结构

- `firmware/analog/` — G431 上运行的 Embassy 应用（控制环路 + 遥测流）
- `firmware/digital/` — S3 上运行的 Rust + esp‑hal 应用（本地 UI + UART 链路终端）
- `libs/` — 共享驱动与协议约定（当前包含无分配的 MCU↔MCU 协议 crate `loadlynx-protocol`）
- `docs/` — 控制环路 / 热设计 / 接口协议与板级文档
- `scripts/` — 开发辅助脚本

## 片间通信建议

- 默认：UART + 帧编码（CBOR/SLIP），易调试、鲁棒、带宽足够
- 预留：SPI/I²C 可选（视硬件走线与带宽/时延需求）

## 致谢

- Embassy 项目（异步 HAL 与执行器）
- ESP‑IDF（ESP32 官方框架）
