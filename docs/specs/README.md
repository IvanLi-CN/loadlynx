# 规格（Spec）总览

本目录用于管理工作项的**规格与追踪**：记录范围、验收标准、生命周期、实现状态与关键演进原因，作为交付依据；实现与验证应以对应 `SPEC.md` 为准。

> Legacy compatibility: historical plan entries have canonical mirrors under `docs/specs/**`. Retained legacy plan files are marked `pending delete approval` until deletion is explicitly approved.

## 快速新增一个规格

1. 生成一个新的规格 `ID`（推荐 5 个字符的 nanoId 风格，降低并行建规格时的冲突概率）。
2. 新建目录：`docs/specs/<id>-<title>/`（`<title>` 用简短 slug，建议 kebab-case）。
3. 在该目录下创建 `SPEC.md`、`IMPLEMENTATION.md`、`HISTORY.md`。
4. 在下方 Index 表新增一行，并把 `Lifecycle` 设为 `active`，把 `Status` 设为当前规格状态。

## 生命周期（Lifecycle）说明

- `active`：当前有效规格。
- `superseded(#<id>)`：已被另一个规格取代。
- `archived`：仅保留历史记录，不再作为当前交付依据。

## 状态（Status）说明

状态用于追踪实现推进，可沿用 `待设计`、`待实现`、`跳过`、`部分完成（x/y）`、`已完成`、`作废`、`重新设计（#<id>）` 等项目既有值。

## Index

| ID | Title | Status | Lifecycle | Spec | Implementation | Last | Notes |
|---:|-------|--------|-----------|------|----------------|------|-------|
| 0001 | CC 负载开关（Load Switch）：设置值 / 生效值分离 | 已完成 | active | `0001-cc-load-switch-toggle/SPEC.md` | `0001-cc-load-switch-toggle/IMPLEMENTATION.md` | 2025-12-26 | legacy pending delete approval |
| 0002 | CV 模式 + Preset 需求与概要设计 | 已完成 | active | `0002-cv-mode-presets/SPEC.md` | `0002-cv-mode-presets/IMPLEMENTATION.md` | 2026-01-03 | legacy pending delete approval |
| 0003 | FT6336U 触控（P024C128-CTP）驱动与 digital 集成设计（草案） | 已完成 | active | `0003-ft6336u-touch-int/SPEC.md` | `0003-ft6336u-touch-int/IMPLEMENTATION.md` | 2025-12-25 | legacy pending delete approval |
| 0004 | mDNS 与局域网发现设计草案 | 已完成 | active | `0004-mdns-and-lan-discovery/SPEC.md` | `0004-mdns-and-lan-discovery/IMPLEMENTATION.md` | 2025-12-11 | legacy pending delete approval |
| 0005 | 本机 Preset UI（触屏 + 旋钮）需求与概要设计 | 已完成 | active | `0005-on-device-preset-ui/SPEC.md` | `0005-on-device-preset-ui/IMPLEMENTATION.md` | 2026-01-07 | legacy pending delete approval |
| 0006 | Preset UI：UVLO / OCP / OPP 命名与三线约束（需求与概要设计） | 已完成 | active | `0006-preset-ui-protection-labels/SPEC.md` | `0006-preset-ui-protection-labels/IMPLEMENTATION.md` | 2026-01-07 | legacy pending delete approval |
| 0007 | 提示音管理器（蜂鸣器 Prompt Tone）设计 | 已完成 | active | `0007-prompt-tone-manager/SPEC.md` | `0007-prompt-tone-manager/IMPLEMENTATION.md` | 2026-01-07 | legacy pending delete approval |
| 0008 | Web Storybook 组件工作台：需求分析与概要设计 | 已完成 | active | `0008-storybook-component-workshop/SPEC.md` | `0008-storybook-component-workshop/IMPLEMENTATION.md` | 2025-12-23 | legacy pending delete approval |
| 0009 | MCU↔MCU 串口通信问题说明与排查方案（记录规范 & 实测数据） | 已完成 | active | `0009-uart-comm-troubleshooting/SPEC.md` | `0009-uart-comm-troubleshooting/IMPLEMENTATION.md` | 2026-01-05 | legacy pending delete approval |
| 0010 | USB‑PD Sink：5V/20V 两态切换（STM32G431 UCPD） | 已完成 | active | `0010-usb-pd-sink-toggle/SPEC.md` | `0010-usb-pd-sink-toggle/IMPLEMENTATION.md` | 2026-01-07 | legacy pending delete approval |
| 0011 | Web UI Layout 规范化（Layouts 抽象） | 已完成 | active | `0011-web-layouts/SPEC.md` | `0011-web-layouts/IMPLEMENTATION.md` | 2025-12-26 | legacy pending delete approval |
| 0012 | Web：Responsive Drawer Sidebar（ConsoleLayout 导航） | 已完成 | active | `0012-web-responsive-drawer-sidebar/SPEC.md` | `0012-web-responsive-drawer-sidebar/IMPLEMENTATION.md` | 2025-12-26 | legacy pending delete approval |
| 0013 | USB‑PD 设置面板：Fixed PDO / PPS APDO（UI + UART 协议 + HTTP API） | 已完成 | active | `0013-usb-pd-pps-and-fixed-settings/SPEC.md` | `0013-usb-pd-pps-and-fixed-settings/IMPLEMENTATION.md` | 2026-01-12 | legacy pending delete approval |
| 0014 | Web：USB‑PD 设置页（对接 /api/v1/pd） | 已完成 | active | `0014-web-usb-pd-settings/SPEC.md` | `0014-web-usb-pd-settings/IMPLEMENTATION.md` | 2026-01-13 | legacy pending delete approval |
| 0015 | 数字板：屏幕自动调暗与熄屏 | 已完成 | active | `0015-auto-screen-dim-off/SPEC.md` | `0015-auto-screen-dim-off/IMPLEMENTATION.md` | 2026-01-13 | legacy pending delete approval |
| 0016 | PD 设置：触屏友好的目标值编辑（PD Settings Touch Value Editor） | 已完成 | active | `0016-pd-settings-touch-value-editor/SPEC.md` | `0016-pd-settings-touch-value-editor/IMPLEMENTATION.md` | 2026-01-15 | legacy pending delete approval |
| 0017 | CP 模式：本机屏幕界面 + HTTP API | 已完成 | active | `0017-cp-mode-ui-http-api/SPEC.md` | `0017-cp-mode-ui-http-api/IMPLEMENTATION.md` | 2026-01-18 | legacy pending delete approval |
| 0018 | Web：CP 模式控制页 | 已完成 | active | `0018-web-cp-mode-ui/SPEC.md` | `0018-web-cp-mode-ui/IMPLEMENTATION.md` | 2026-01-19 | legacy pending delete approval |
| 0019 | 主界面：PD 按钮两行文案规范（Detach / PPS / Fixed） | 已完成 | active | `0019-dashboard-pd-button-label/SPEC.md` | `0019-dashboard-pd-button-label/IMPLEMENTATION.md` | 2026-01-19 | legacy pending delete approval |
| 0020 | Web：仪器风格主界面（左监右控） | 已完成 | active | `0020-web-instrument-control-ui/SPEC.md` | `0020-web-instrument-control-ui/IMPLEMENTATION.md` | 2026-01-20 | legacy pending delete approval |
| 0021 | 触摸弹簧（GPIO14）负载开关 + RGB 指示 + 语音播放（MAX98357A / I²S） | 已完成 | active | `0021-touch-spring-load-switch-rgb-led/SPEC.md` | `0021-touch-spring-load-switch-rgb-led/IMPLEMENTATION.md` | 2026-02-03 | legacy pending delete approval |
| 0022 | Storybook：隐藏断点竖线标尺（768/1024） | 已完成 | active | `0022-storybook-hide-breakpoint-guides/SPEC.md` | `0022-storybook-hide-breakpoint-guides/IMPLEMENTATION.md` | 2026-01-20 | legacy pending delete approval |
| 0023 | GitHub Pages：Web Deploy 失败修复（lockfile/CI） | 已完成 | active | `0023-web-pages-deploy-lockfile/SPEC.md` | `0023-web-pages-deploy-lockfile/IMPLEMENTATION.md` | 2026-01-21 | legacy pending delete approval |
| 0024 | Web：版本号展示 + GitHub 分支跳转 | 已完成 | active | `0024-web-version-github-link/SPEC.md` | `0024-web-version-github-link/IMPLEMENTATION.md` | 2026-01-21 | legacy pending delete approval |
| 0025 | 服务端口规范化 | 已完成 | active | `0025-service-ports-avoid-defaults/SPEC.md` | `0025-service-ports-avoid-defaults/IMPLEMENTATION.md` | 2026-01-21 | legacy pending delete approval |
| 6mre7 | 触摸电源按键：睡眠待机白光低频呼吸 | 已完成 | active | `6mre7-touch-power-button-standby-breathing-white/SPEC.md` | `6mre7-touch-power-button-standby-breathing-white/IMPLEMENTATION.md` | 2026-02-03 | legacy pending delete approval |
| e3rv6 | LoadLynx devd 本地设备控制面 | 已完成 | active | `e3rv6-loadlynx-devd-control-plane/SPEC.md` | `e3rv6-loadlynx-devd-control-plane/IMPLEMENTATION.md` | 2026-05-24 | 首版实现：devd/CLI、Web devd+Firmware、firmware catalog、digital identity/DNS-SD、Storybook/视觉证据 |
| fhpfk | LoadLynx operational skills packaging and workflow boundary | 已更新 | active | `fhpfk-loadlynx-operational-skills/SPEC.md` | `fhpfk-loadlynx-operational-skills/IMPLEMENTATION.md` | 2026-05-28 | PR #78；用户/开发者 skill 场景拆分、CLI-only 硬件操作、USB 优先、硬件记忆门禁、vercel-labs/skills 安装验证 |
| fqmns | Dashboard Boot Link Recovery | 已完成 | active | `fqmns-boot-link-recovery/SPEC.md` | `fqmns-boot-link-recovery/IMPLEMENTATION.md` | 2026-04-25 | 软件实现与构建验证完成；HIL 因当前 worktree 缺失 selector 阻断 |
| j979q | Release 失败 Telegram 告警接入 | 待实现 | active | `j979q-release-failure-telegram-alerts/SPEC.md` | `j979q-release-failure-telegram-alerts/IMPLEMENTATION.md` | 2026-04-12 | Add a repo-local notifier for the analog release and development release workflows with a manual Telegram smoke path |
| shkmx | 音频迁移：蜂鸣器 → 扬声器 | 已完成 | active | `shkmx-buzzer-to-speaker-audio/SPEC.md` | `shkmx-buzzer-to-speaker-audio/IMPLEMENTATION.md` | 2026-02-05 | legacy pending delete approval |
| t2f5j | USB-PD EPR 28V Sink Enablement | 部分完成（3/4） | active | `t2f5j-usb-pd-epr-28v-sink/SPEC.md` | `t2f5j-usb-pd-epr-28v-sink/IMPLEMENTATION.md` | 2026-04-25 | PR #72；实现与构建已完成；owner-facing fixed PDO 语义已收敛为 live-only；HIL 仍待 EPR 线材补验 |
| w4cpd | Dashboard 扩展电压开关与 PD 设置入口重构 | 已完成 | active | `w4cpd-dashboard-extended-voltage-toggle/SPEC.md` | `w4cpd-dashboard-extended-voltage-toggle/IMPLEMENTATION.md` | 2026-03-10 | PR #70；实现已完成；HIL 可选 |
| wh4s9 | Calibration 页面模式同步与状态链路稳定化 | 已完成 | active | `wh4s9-calibration-mode-sync-stability/SPEC.md` | `wh4s9-calibration-mode-sync-stability/IMPLEMENTATION.md` | 2026-04-16 | 本地校准页稳定性修复、Storybook/E2E 回归与视觉证据已完成 |
| y4sf4 | Digital Display PSRAM/DMA Pipeline | 已完成 | active | `y4sf4-display-psram-dma-pipeline/SPEC.md` | `y4sf4-display-psram-dma-pipeline/IMPLEMENTATION.md` | 2026-03-19 | PR #71；PSRAM 专用 framebuffer arena、多缓冲 present、真实 present-FPS、细粒度 dirty rect、pending 背压 |
