# 计划（Plan）总览

本目录用于管理“先计划、后实现”的工作项：每个计划在这里冻结范围与验收标准，进入实现前先把口径对齐，避免边做边改导致失控。

## 快速新增一个计划

1. 分配一个新的四位编号 `ID`（查看下方 Index，取未使用的最小或递增编号）。
2. 新建目录：`docs/plan/<id>:<title>/`（`<title>` 用简短 slug，建议 kebab-case）。
3. 在该目录下创建 `PLAN.md`（模板见下方“PLAN.md 写法（简要）”）。
4. 在下方 Index 表新增一行，并把 `Status` 设为 `待设计` 或 `待实现`（取决于是否已冻结验收标准），并填入 `Last`（通常为当天）。

## 目录与命名规则

- 每个计划一个目录：`docs/plan/<id>:<title>/`
- `<id>`：四位数字（`0001`–`9999`），一经分配不要变更。
- `<title>`：短标题 slug（建议 kebab-case，避免空格与特殊字符）；目录名尽量稳定。
- 人类可读标题写在 Index 的 `Title` 列；标题变更优先改 `Title`，不强制改目录名。

## 状态（Status）说明

仅允许使用以下状态值：

- `待设计`：范围/约束/验收标准尚未冻结，仍在补齐信息与决策。
- `待实现`：计划已冻结，允许进入实现阶段（或进入 PM/DEV 交付流程）。
- `部分完成（x/y）`：实现进行中；`y` 为该计划里定义的里程碑数，`x` 为已完成里程碑数（见该计划 `PLAN.md` 的 Milestones）。
- `已完成`：该计划已完成（实现已落地或将随某个 PR 落地）；如需关联 PR 号，写在 Index 的 `Notes`（例如 `PR #123`）。
- `作废`：不再推进（取消/价值不足/外部条件变化）。
- `重新设计（#<id>）`：该计划被另一个计划取代；`#<id>` 指向新的计划编号。

## `Last` 字段约定（推进时间）

- `Last` 表示该计划**上一次“推进进度/口径”**的日期，用于快速发现长期未推进的计划。
- 仅在以下情况更新 `Last`（不要因为改措辞/排版就更新）：
  - `Status` 变化（例如 `部分完成（2/3）` → `已完成`）
  - `Notes` 中写入/更新 PR 号（例如 `PR #123`）
  - `PLAN.md` 的里程碑勾选变化
  - 范围/验收标准冻结或发生实质变更

## PLAN.md 写法（简要）

每个计划的 `PLAN.md` 至少应包含：

- 背景/问题陈述（为什么要做）
- 目标 / 非目标（做什么、不做什么）
- 范围（in/out）
- 需求列表（MUST/SHOULD/COULD）
- 验收标准（Given/When/Then + 边界/异常）
- 非功能性验收/质量门槛（测试策略、质量检查、Storybook/视觉回归等按仓库已有约定）
- 文档更新（需要同步更新的项目设计文档/架构说明/README/ADR）
- 里程碑（Milestones，用于驱动 `部分完成（x/y）`）
- 风险与开放问题（需要决策的点）

## Index（固定表格）

| ID   | Title | Status | Plan | Last | Notes |
|-----:|-------|--------|------|------|-------|
| 0001 | CC 负载开关（Load Switch）：设置值 / 生效值分离 | 已完成 | `0001:cc-load-switch-toggle/PLAN.md` | 2025-12-26 | - |
| 0002 | CV 模式 + Preset 需求与概要设计 | 已完成 | `0002:cv-mode-presets/PLAN.md` | 2026-01-03 | - |
| 0003 | FT6336U 触控（P024C128-CTP）驱动与 digital 集成设计（草案） | 已完成 | `0003:ft6336u-touch-int/PLAN.md` | 2025-12-25 | - |
| 0004 | mDNS 与局域网发现设计草案 | 已完成 | `0004:mdns-and-lan-discovery/PLAN.md` | 2025-12-11 | - |
| 0005 | 本机 Preset UI（触屏 + 旋钮）需求与概要设计 | 已完成 | `0005:on-device-preset-ui/PLAN.md` | 2026-01-07 | - |
| 0006 | Preset UI：UVLO / OCP / OPP 命名与三线约束（需求与概要设计） | 已完成 | `0006:preset-ui-protection-labels/PLAN.md` | 2026-01-07 | - |
| 0007 | 提示音管理器（蜂鸣器 Prompt Tone）设计 | 已完成 | `0007:prompt-tone-manager/PLAN.md` | 2026-01-07 | - |
| 0008 | Web Storybook 组件工作台：需求分析与概要设计 | 已完成 | `0008:storybook-component-workshop/PLAN.md` | 2025-12-23 | - |
| 0009 | MCU↔MCU 串口通信问题说明与排查方案（记录规范 & 实测数据） | 已完成 | `0009:uart-comm-troubleshooting/PLAN.md` | 2026-01-05 | - |
| 0010 | USB‑PD Sink：5V/20V 两态切换（STM32G431 UCPD） | 已完成 | `0010:usb-pd-sink-toggle/PLAN.md` | 2026-01-07 | - |
| 0011 | Web UI Layout 规范化（Layouts 抽象） | 已完成 | `0011:web-layouts/PLAN.md` | 2025-12-26 | - |
| 0012 | Web：Responsive Drawer Sidebar（ConsoleLayout 导航） | 已完成 | `0012:web-responsive-drawer-sidebar/PLAN.md` | 2025-12-26 | - |
| 0013 | USB‑PD 设置面板：Fixed PDO / PPS APDO（UI + UART 协议 + HTTP API） | 已完成 | `0013:usb-pd-pps-and-fixed-settings/PLAN.md` | 2026-01-12 | HIL: IP6557 + e‑marker |
| 0014 | Web：USB‑PD 设置页（对接 /api/v1/pd） | 已完成 | `0014:web-usb-pd-settings/PLAN.md` | 2026-01-13 | HIL: PPS OK (9V@500mA) |
| 0015 | 数字板：屏幕自动调暗与熄屏（背光省电） | 已完成 | `0015:auto-screen-dim-off/PLAN.md` | 2026-01-13 | HIL: verify 2min/5min + wake |
| 0016 | PD 设置：触屏友好的目标值编辑（无 +/-；点击/滑动选位；旋钮调节） | 已完成 | `0016:pd-settings-touch-value-editor/PLAN.md` | 2026-01-16 | PR #46 |
| 0017 | CP 模式：本机屏幕界面 + HTTP API | 已完成 | `0017:cp-mode-ui-http-api/PLAN.md` | 2026-01-18 | HIL (internal): CP multi-level + large-step: cp_perf t10_90/t90_10<=1ms AND peak_err<=max(0.1*Δ, tol) within 1ms; script: `scripts/cp-acceptance.sh` |
| 0018 | Web：CP 模式控制页 | 已完成 | `0018:web-cp-mode-ui/PLAN.md` | 2026-01-19 | Entry: /$deviceId/cc |
| 0019 | 主界面：PD 按钮两行文案规范（Detach / PPS / Fixed） | 已完成 | `0019:dashboard-pd-button-label/PLAN.md` | 2026-01-19 | HIL: pd_button Active from contract |
| 0020 | Web：仪器风格主界面（左监右控） | 已完成 | `0020:web-instrument-control-ui/PLAN.md` | 2026-01-20 | - |
| 0021 | 触摸弹簧（GPIO14）负载开关 + RGB 指示 + 语音播放（MAX98357A/I²S） | 已完成 | `0021:touch-spring-load-switch-rgb-led/PLAN.md` | 2026-02-03 | HIL: touch+RGB OK; speaker boot diag + WAV clips (8kHz); PR #67 |
| 0022 | Storybook：隐藏断点竖线标尺（768/1024） | 已完成 | `0022:storybook-hide-breakpoint-guides/PLAN.md` | 2026-01-20 | - |
| 0023 | GitHub Pages：Web Deploy 失败修复（lockfile/CI） | 已完成 | `0023:web-pages-deploy-lockfile/PLAN.md` | 2026-01-21 | Actions: web-pages#16 / web-check#105 |
| 0024 | Web：版本号展示 + GitHub 分支跳转 | 已完成 | `0024:web-version-github-link/PLAN.md` | 2026-01-21 | - |
| 0025 | 服务端口规范化（高位端口 + 禁止自动换端口） | 已完成 | `0025:service-ports-avoid-defaults/PLAN.md` | 2026-01-21 | PR #62 |
