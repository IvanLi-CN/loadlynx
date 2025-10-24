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
  - `chore(digital): setup ESP-IDF cmake`

## 构建与验证
- G431：Rust + Embassy，目标 `thumbv7em-none-eabihf`，使用 probe-rs 调试与烧录。
- S3：Rust + esp-hal（可选集成 Embassy），使用 `cargo` + `espflash` 构建与烧录。

## 后续里程碑（建议）
- 驱动层：NTC/温度、风扇 PWM、分流/跨阻采样链路
- 控制层：CC/CV/CP 模式，保护（OC/OV/OT/SCP），软启动
- 通信层：UART 帧协议、字段与容错、版本与校准同步
- UI 层：本地按键/旋钮 + Web UI（曲线/记录/标定）
