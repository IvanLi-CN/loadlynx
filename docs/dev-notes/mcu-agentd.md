# MCU Agent（mcu-agentd）集成（LoadLynx）

LoadLynx 的硬件在环（HIL）工作流使用外部升级版 `mcu-agentd`（默认 sibling checkout：`../mcu-agentd`），不再依赖仓库内置的 `tools/mcu-agentd/`。

根目录 `Justfile` 提供 `just agentd-init` 安装/升级（默认 `../mcu-agentd`，可用 `MCU_AGENTD_PATH` 覆盖）；安装后 `just agentd ...` 直接调用 `mcu-agentd`。若二进制未安装，则回退到 `cargo run --manifest-path $MCU_AGENTD_MANIFEST ...`。

## 1) 配置与落盘目录

- 配置文件：仓根 `mcu-agentd.toml`（提交到仓库，定义 MCU 目标、后端、芯片、ELF 路径等）。
- 运行态目录：仓根 `/.mcu-agentd/`（socket/lock/logs/sessions/monitor/meta，已在 `.gitignore`）。
- selector 缓存（兼容旧工作流）：在 `mcu-agentd.toml` 中显式指向仓根缓存文件：
  - Digital（ESP32‑S3）：`./.esp32-port`
  - Analog（STM32G431）：`./.stm32-port`

## 2) LoadLynx 的 MCU 定义

见 `mcu-agentd.toml`：

- `digital`：`backend="espflash"`，`chip="esp32s3"`，ELF `firmware/digital/target/xtensa-esp32s3-none-elf/release/digital`
- `analog`：`backend="probe-rs"`，`chip="STM32G431CB"`，ELF `firmware/analog/target/thumbv7em-none-eabihf/release/analog`

## 3) 常用命令（Just 包装）

- daemon：`just agentd-start` / `just agentd-status` / `just agentd-stop`
- selector：`just agentd selector set|get|list ...`（也可用 `just agentd-set-port ...` / `just agentd-get-port ...`）
- 操作：`just agentd flash|reset|monitor|logs ...`

## 4) 参考（外部仓库文档）

- 使用指南：`../mcu-agentd/docs/usage/mcu-agentd.md`
- 配置规范：`../mcu-agentd/docs/design/config.md`
