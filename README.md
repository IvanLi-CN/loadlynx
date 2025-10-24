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
  - 散热与风扇曲线控制
  - 与 S3 通过 UART 帧通信（建议 CBOR/SLIP）
- ESP32‑S3
  - 本地 UI 与 Web UI，Wi‑Fi / OTA
  - 数据记录、标定流程与上位机桥接
  - 与 G431 的可靠链路与升级/诊断工具

## 构建快速开始

本仓库仅提供最小可编译脚手架（占位代码）。实际硬件驱动、管脚、控制环参数需根据原理图与 PCB 定稿同步更新。

### 环境
- Rust nightly（embedded） + `thumbv7em-none-eabihf` 目标
- probe-rs 工具链（调试/烧录）
- ESP‑IDF (v5+) 与 `idf.py`

### G431（analog）
- 目标三元组：`thumbv7em-none-eabihf`
- 建议 Runner：`probe-rs run --chip STM32G431CB`（具体后缀依 probe-rs 芯片库而定）
- 本仓库默认在 `.cargo/config.toml` 中设置目标与 runner，你也可以通过脚本执行：

```sh
# 构建
(cd firmware/analog && cargo build)

# 烧录（使用 probe-rs 直接调用）
probe-rs run --chip STM32G431CB --protocol swd --speed 4000 \
  --firmware target/thumbv7em-none-eabihf/debug/analog
```

### ESP32‑S3（digital）
- 需要正确安装 `espup`/Xtensa 工具链与 `espflash`（Rust 生态）。

```sh
# 构建（Rust + esp-hal）
(cd firmware/digital && cargo build)

# 烧录与监视（自动探测串口）
(cd firmware/digital && cargo run --release)

# 或指定端口
loadlynx/scripts/flash_s3.sh --release --port /dev/tty.usbserial-xxxx
```

## 目录结构
- `firmware/analog/` — G431 最小 Embassy 应用（定时与日志占位）
- `firmware/digital/` — S3 最小 ESP‑IDF 应用（UART/任务占位）
- `libs/` — 共享驱动与协议约定（占位）
- `docs/` — 控制环路/热设计/接口协议笔记（占位）
- `scripts/` — 烧录与构建脚本

## 片间通信建议
- 默认：UART + 帧编码（CBOR/SLIP），易调试、鲁棒、带宽足够
- 预留：SPI/I²C 可选（视硬件走线与带宽/时延需求）

## 致谢
- Embassy 项目（异步 HAL 与执行器）
- ESP‑IDF（ESP32 官方框架）
