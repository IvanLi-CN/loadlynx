# 规格（Spec）总览

本目录用于管理工作项的**规格与追踪**：记录范围、验收标准、生命周期、实现状态与关键演进原因，作为交付依据；实现与验证应以对应 `SPEC.md` 为准。

> Canonical specs are the only planning-spec taxonomy; legacy plan files have been removed after migration.

## 快速新增一个规格

1. 生成一个新的规格 `ID`（推荐 5 个字符的 nanoId 风格，必须以字母开头；不要使用纯数字编号）。
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
| y5ztx | CC 负载开关（Load Switch）：设置值 / 生效值分离 | 已完成 | active | `y5ztx-cc-load-switch-toggle/SPEC.md` | `y5ztx-cc-load-switch-toggle/IMPLEMENTATION.md` | 2025-12-26 | migrated |
| exkw2 | CV 模式 + Preset 需求与概要设计 | 已完成 | active | `exkw2-cv-mode-presets/SPEC.md` | `exkw2-cv-mode-presets/IMPLEMENTATION.md` | 2026-01-03 | migrated |
| c67hy | FT6336U 触控（P024C128-CTP）驱动与 digital 集成设计（草案） | 已完成 | active | `c67hy-ft6336u-touch-int/SPEC.md` | `c67hy-ft6336u-touch-int/IMPLEMENTATION.md` | 2025-12-25 | migrated |
| yy7th | mDNS 与局域网发现设计草案 | 已完成 | active | `yy7th-mdns-and-lan-discovery/SPEC.md` | `yy7th-mdns-and-lan-discovery/IMPLEMENTATION.md` | 2025-12-11 | migrated |
| mq8ht | 本机 Preset UI（触屏 + 旋钮）需求与概要设计 | 已完成 | active | `mq8ht-on-device-preset-ui/SPEC.md` | `mq8ht-on-device-preset-ui/IMPLEMENTATION.md` | 2026-01-07 | migrated |
| trrw7 | Preset UI：UVLO / OCP / OPP 命名与三线约束（需求与概要设计） | 已完成 | active | `trrw7-preset-ui-protection-labels/SPEC.md` | `trrw7-preset-ui-protection-labels/IMPLEMENTATION.md` | 2026-01-07 | migrated |
| bd4vh | 提示音管理器（蜂鸣器 Prompt Tone）设计 | 已完成 | active | `bd4vh-prompt-tone-manager/SPEC.md` | `bd4vh-prompt-tone-manager/IMPLEMENTATION.md` | 2026-01-07 | migrated |
| hthpy | Web Storybook 组件工作台：需求分析与概要设计 | 已完成 | active | `hthpy-storybook-component-workshop/SPEC.md` | `hthpy-storybook-component-workshop/IMPLEMENTATION.md` | 2025-12-23 | migrated |
| z5ey6 | MCU↔MCU 串口通信问题说明与排查方案（记录规范 & 实测数据） | 已完成 | active | `z5ey6-uart-comm-troubleshooting/SPEC.md` | `z5ey6-uart-comm-troubleshooting/IMPLEMENTATION.md` | 2026-01-05 | migrated |
| h3gz5 | USB‑PD Sink：5V/20V 两态切换（STM32G431 UCPD） | 已完成 | active | `h3gz5-usb-pd-sink-toggle/SPEC.md` | `h3gz5-usb-pd-sink-toggle/IMPLEMENTATION.md` | 2026-01-07 | migrated |
| cqu4e | Web UI Layout 规范化（Layouts 抽象） | 已完成 | active | `cqu4e-web-layouts/SPEC.md` | `cqu4e-web-layouts/IMPLEMENTATION.md` | 2025-12-26 | migrated |
| t4zh9 | Web：Responsive Drawer Sidebar（ConsoleLayout 导航） | 已完成 | active | `t4zh9-web-responsive-drawer-sidebar/SPEC.md` | `t4zh9-web-responsive-drawer-sidebar/IMPLEMENTATION.md` | 2025-12-26 | migrated |
| j24my | USB‑PD 设置面板：Fixed PDO / PPS APDO（UI + UART 协议 + HTTP API） | 已完成 | active | `j24my-usb-pd-pps-and-fixed-settings/SPEC.md` | `j24my-usb-pd-pps-and-fixed-settings/IMPLEMENTATION.md` | 2026-01-12 | migrated |
| rbcuw | Web：USB‑PD 设置页（对接 /api/v1/pd） | 已完成 | active | `rbcuw-web-usb-pd-settings/SPEC.md` | `rbcuw-web-usb-pd-settings/IMPLEMENTATION.md` | 2026-01-13 | migrated |
| guysf | 数字板：屏幕自动调暗与熄屏 | 已完成 | active | `guysf-auto-screen-dim-off/SPEC.md` | `guysf-auto-screen-dim-off/IMPLEMENTATION.md` | 2026-01-13 | migrated |
| ye27x | PD 设置：触屏友好的目标值编辑（PD Settings Touch Value Editor） | 已完成 | active | `ye27x-pd-settings-touch-value-editor/SPEC.md` | `ye27x-pd-settings-touch-value-editor/IMPLEMENTATION.md` | 2026-01-15 | migrated |
| k7nhc | CP 模式：本机屏幕界面 + HTTP API | 已完成 | active | `k7nhc-cp-mode-ui-http-api/SPEC.md` | `k7nhc-cp-mode-ui-http-api/IMPLEMENTATION.md` | 2026-01-18 | migrated |
| ejgp8 | Web：CP 模式控制页 | 已完成 | active | `ejgp8-web-cp-mode-ui/SPEC.md` | `ejgp8-web-cp-mode-ui/IMPLEMENTATION.md` | 2026-01-19 | migrated |
| wjhba | 主界面：PD 按钮两行文案规范（Detach / PPS / Fixed） | 已完成 | active | `wjhba-dashboard-pd-button-label/SPEC.md` | `wjhba-dashboard-pd-button-label/IMPLEMENTATION.md` | 2026-01-19 | migrated |
| t5x4k | Web：仪器风格主界面（左监右控） | 已完成 | active | `t5x4k-web-instrument-control-ui/SPEC.md` | `t5x4k-web-instrument-control-ui/IMPLEMENTATION.md` | 2026-01-20 | migrated |
| swzqu | 触摸弹簧（GPIO14）负载开关 + RGB 指示 + 语音播放（MAX98357A / I²S） | 已完成 | active | `swzqu-touch-spring-load-switch-rgb-led/SPEC.md` | `swzqu-touch-spring-load-switch-rgb-led/IMPLEMENTATION.md` | 2026-02-03 | migrated |
| j6tmd | Storybook：隐藏断点竖线标尺（768/1024） | 已完成 | active | `j6tmd-storybook-hide-breakpoint-guides/SPEC.md` | `j6tmd-storybook-hide-breakpoint-guides/IMPLEMENTATION.md` | 2026-01-20 | migrated |
| rjkcw | GitHub Pages：Web Deploy 失败修复（lockfile/CI） | 已完成 | active | `rjkcw-web-pages-deploy-lockfile/SPEC.md` | `rjkcw-web-pages-deploy-lockfile/IMPLEMENTATION.md` | 2026-01-21 | migrated |
| yff7v | Web：版本号展示 + GitHub 分支跳转 | 已完成 | active | `yff7v-web-version-github-link/SPEC.md` | `yff7v-web-version-github-link/IMPLEMENTATION.md` | 2026-01-21 | migrated |
| sjb5t | 服务端口规范化 | 已完成 | active | `sjb5t-service-ports-avoid-defaults/SPEC.md` | `sjb5t-service-ports-avoid-defaults/IMPLEMENTATION.md` | 2026-01-21 | migrated |
| v3g2c | 触摸电源按键：睡眠待机白光低频呼吸 | 已完成 | active | `v3g2c-touch-power-button-standby-breathing-white/SPEC.md` | `v3g2c-touch-power-button-standby-breathing-white/IMPLEMENTATION.md` | 2026-02-03 | migrated |
| e3rv6 | LoadLynx devd 本地设备控制面 | 已完成 | active | `e3rv6-loadlynx-devd-control-plane/SPEC.md` | `e3rv6-loadlynx-devd-control-plane/IMPLEMENTATION.md` | 2026-05-24 | 首版实现：devd/CLI、Web devd+Firmware、firmware catalog、digital identity/DNS-SD、Storybook/视觉证据 |
| fhpfk | LoadLynx operational skills packaging and workflow boundary | 已更新 | active | `fhpfk-loadlynx-operational-skills/SPEC.md` | `fhpfk-loadlynx-operational-skills/IMPLEMENTATION.md` | 2026-05-28 | PR #78；用户/开发者 skill 场景拆分、CLI-only 硬件操作、USB 优先、硬件记忆门禁、vercel-labs/skills 安装验证 |
| fqmns | Dashboard Boot Link Recovery | 已完成 | active | `fqmns-boot-link-recovery/SPEC.md` | `fqmns-boot-link-recovery/IMPLEMENTATION.md` | 2026-04-25 | 软件实现与构建验证完成；HIL 因当前 worktree 缺失 selector 阻断 |
| j979q | Release 失败 Telegram 告警接入 | 已更新 | active | `j979q-release-failure-telegram-alerts/SPEC.md` | `j979q-release-failure-telegram-alerts/IMPLEMENTATION.md` | 2026-05-29 | 统一监听 `Release (LoadLynx)`；普通 PR CI 失败不触发 Telegram，release failure 保留 smoke path |
| dvfnn | PR Label Release Flow | 已更新 | active | `dvfnn-pr-label-release-flow/SPEC.md` | `dvfnn-pr-label-release-flow/IMPLEMENTATION.md` | 2026-06-07 | PR label release contract、Label Gate、`main` PR-only + `0 approvals`、发布后 PR 自动评论 |
| m8k2v | Web Bundle Budget Gates | 已完成 | active | `m8k2v-web-bundle-budget-gates/SPEC.md` | `m8k2v-web-bundle-budget-gates/IMPLEMENTATION.md` | 2026-06-07 | 显式 app/Storybook bundle budget、Storybook framework runtime 单独验收、CI 门禁 |
| shkmx | 音频迁移：蜂鸣器 → 扬声器 | 已完成 | active | `shkmx-buzzer-to-speaker-audio/SPEC.md` | `shkmx-buzzer-to-speaker-audio/IMPLEMENTATION.md` | 2026-02-05 | migrated |
| t2f5j | USB-PD EPR 28V Sink Enablement | 部分完成（3/4） | active | `t2f5j-usb-pd-epr-28v-sink/SPEC.md` | `t2f5j-usb-pd-epr-28v-sink/IMPLEMENTATION.md` | 2026-04-25 | PR #72；实现与构建已完成；owner-facing fixed PDO 语义已收敛为 live-only；HIL 仍待 EPR 线材补验 |
| w4cpd | Dashboard 扩展电压开关与 PD 设置入口重构 | 已完成 | active | `w4cpd-dashboard-extended-voltage-toggle/SPEC.md` | `w4cpd-dashboard-extended-voltage-toggle/IMPLEMENTATION.md` | 2026-03-10 | PR #70；实现已完成；HIL 可选 |
| wh4s9 | Calibration 页面模式同步与状态链路稳定化 | 已完成 | active | `wh4s9-calibration-mode-sync-stability/SPEC.md` | `wh4s9-calibration-mode-sync-stability/IMPLEMENTATION.md` | 2026-04-16 | 本地校准页稳定性修复、Storybook/E2E 回归与视觉证据已完成 |
| y4sf4 | Digital Display PSRAM/DMA Pipeline | 已完成 | active | `y4sf4-display-psram-dma-pipeline/SPEC.md` | `y4sf4-display-psram-dma-pipeline/IMPLEMENTATION.md` | 2026-03-19 | PR #71；PSRAM 专用 framebuffer arena、多缓冲 present、真实 present-FPS、细粒度 dirty rect、pending 背压 |
| br7kq | LoadLynx Backup & Restore | 已完成 | active | `br7kq-loadlynx-backup-restore/SPEC.md` | `br7kq-loadlynx-backup-restore/IMPLEMENTATION.md` | 2026-05-31 | CLI/Web JSON backup restore；恢复前强制关闭负载；WiFi PSK 明文备份 |
| n9v2q | Web Console Cyberpunk Redesign | implemented | active | `n9v2q-web-cyberpunk-redesign/SPEC.md` | `n9v2q-web-cyberpunk-redesign/IMPLEMENTATION.md` | 2026-06-01 | 全 Web Console 赛博朋克重做；移除 daisyUI/Iconify；新增 shadcn 风格组件与 i18n |
