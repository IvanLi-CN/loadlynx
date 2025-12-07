# 工作流概述（LoadLynx）

- 多目标（G431 + S3）统一在一个仓库内管理，固件分别存放于 `firmware/` 子目录。
- 控制回路与安全相关逻辑优先落在 G431；S3 侧专注于人机与联网。
- 构建/烧录通过 `scripts/` 与各自目录的工具链进行。

## 分支与工作区
- 建议使用 feature 分支进行新特性/模块开发。
- 若需在独立目录并行开发，可使用 `git worktree` 新建工作区；提交前对齐远端基线。

## 提交规范
- 使用 Conventional Commits（英文），例如：
  - `feat(analog): add ADC sampling skeleton`
  - `chore(digital): setup esp-hal display pipeline`

## 构建与验证
- G431：Rust + Embassy，目标 `thumbv7em-none-eabihf`，使用 probe-rs 调试与烧录。
- S3：Rust + esp-hal（可选集成 Embassy），使用 `cargo` + `espflash` 构建与烧录。

## MCU 端口/探针缓存与 Agentd

- 守护与 CLI：`tools/mcu-agentd` 提供单实例守护进程与 CLI（二进制名 `loadlynx-agentd`），推荐通过 Just 封装调用：
  - 启动/状态/停止：`just agentd-start` / `just agentd-status` / `just agentd-stop`。
- 缓存文件：
  - Digital（ESP32‑S3）：仓根 `./.esp32-port`。
  - Analog（STM32G431）：仓根 `./.stm32-port`（旧版 `./.stm32-probe` 仅在 `.stm32-port` 不存在时作为迁移来源，读取后写回 `.stm32-port` 并删除旧文件）。
- 设置与查看缓存（推荐流程）：
  - 设置：`just agentd set-port digital /dev/cu.usbserial-xxxx`；`just agentd set-port analog 0483:3748:SERIAL`。
  - 查看：`just agentd-get-port digital` / `just agentd-get-port analog`。
- 后续所有 `flash` / `reset` / `monitor` 子命令都会优先使用上述缓存值；仅当缓存缺失时才回退到 `scripts/ensure_esp32_port.sh` / `scripts/ensure_stm32_probe.sh` 的自动选择逻辑。

## 后续里程碑（建议）
- 驱动层：NTC/温度、风扇 PWM、分流/跨阻采样链路
- 控制层：CC/CV/CP 模式，保护（OC/OV/OT/SCP），软启动
- 通信层：UART 帧协议、字段与容错、版本与校准同步
- UI 层：本地按键/旋钮 + Web UI（曲线/记录/标定）
