# 规格（Spec）Catalog

本目录维护 LoadLynx 的长期 topic-level specification。每个规格目录由 `SPEC.md`、`IMPLEMENTATION.md`、`HISTORY.md` 组成：`SPEC.md` 保留稳定需求、行为契约与验收口径，`IMPLEMENTATION.md` 记录实现覆盖与验证，`HISTORY.md` 记录关键演进原因。

> Canonical specs are the only planning-spec taxonomy; historical plan files have been removed after taxonomy consolidation.

## 快速新增一个规格

1. 生成一个新的规格 `ID`（推荐 5 个字符的 nanoId 风格，必须以字母开头；不要使用纯数字编号）。
2. 新建目录：`docs/specs/<id>-<title>/`（`<title>` 用简短 slug，建议 kebab-case）。
3. 在该目录下创建 `SPEC.md`、`IMPLEMENTATION.md`、`HISTORY.md`。
4. 在下方 Index 表新增一行，并把 `Lifecycle` 设为当前生命周期。

## 生命周期（Lifecycle）说明

- `active`：当前有效规格。
- `superseded(#<id>)`：已被另一个规格取代。
- `archived`：仅保留历史记录，不再作为当前交付依据。

## Index

| ID | Title | Lifecycle | Spec | Implementation | Implementation Summary |
|---:|-------|-----------|------|----------------|------------------------|
| y5ztx | CC 负载开关（Load Switch）：设置值 / 生效值分离 | active | `y5ztx-cc-load-switch-toggle/SPEC.md` | `y5ztx-cc-load-switch-toggle/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| exkw2 | CV 模式 + Preset | active | `exkw2-cv-mode-presets/SPEC.md` | `exkw2-cv-mode-presets/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| c67hy | FT6336U 触控（P024C128-CTP）驱动与 digital 集成 | active | `c67hy-ft6336u-touch-int/SPEC.md` | `c67hy-ft6336u-touch-int/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| yy7th | mDNS 与局域网发现 | active | `yy7th-mdns-and-lan-discovery/SPEC.md` | `yy7th-mdns-and-lan-discovery/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| mq8ht | 本机 Preset UI（触屏 + 旋钮） | active | `mq8ht-on-device-preset-ui/SPEC.md` | `mq8ht-on-device-preset-ui/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| trrw7 | Preset UI：UVLO / OCP / OPP 命名与三线约束 | active | `trrw7-preset-ui-protection-labels/SPEC.md` | `trrw7-preset-ui-protection-labels/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| bd4vh | 提示音管理器（蜂鸣器 Prompt Tone） | active | `bd4vh-prompt-tone-manager/SPEC.md` | `bd4vh-prompt-tone-manager/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| hthpy | Web Storybook 组件工作台 | active | `hthpy-storybook-component-workshop/SPEC.md` | `hthpy-storybook-component-workshop/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| z5ey6 | MCU↔MCU 串口通信问题说明与排查方案（记录规范 & 实测数据） | active | `z5ey6-uart-comm-troubleshooting/SPEC.md` | `z5ey6-uart-comm-troubleshooting/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| h3gz5 | USB‑PD Sink：5V/20V 两态切换（STM32G431 UCPD） | active | `h3gz5-usb-pd-sink-toggle/SPEC.md` | `h3gz5-usb-pd-sink-toggle/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| cqu4e | Web UI Layout 规范化（Layouts 抽象） | active | `cqu4e-web-layouts/SPEC.md` | `cqu4e-web-layouts/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| t4zh9 | Web：Responsive Drawer Sidebar（ConsoleLayout 导航） | superseded(#m3n8p) | `t4zh9-web-responsive-drawer-sidebar/SPEC.md` | `t4zh9-web-responsive-drawer-sidebar/IMPLEMENTATION.md` | 旧 drawer / sidebar shell 已被顶部导航与设备工作面规格取代，保留历史 traceability。 |
| j24my | USB‑PD 设置面板：Fixed PDO / PPS APDO（UI + UART 协议 + HTTP API） | active | `j24my-usb-pd-pps-and-fixed-settings/SPEC.md` | `j24my-usb-pd-pps-and-fixed-settings/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| rbcuw | Web：USB‑PD 设置页（对接 /api/v1/pd） | active | `rbcuw-web-usb-pd-settings/SPEC.md` | `rbcuw-web-usb-pd-settings/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| guysf | 数字板：屏幕自动调暗与熄屏 | active | `guysf-auto-screen-dim-off/SPEC.md` | `guysf-auto-screen-dim-off/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| ye27x | PD 设置：触屏友好的目标值编辑（PD Settings Touch Value Editor） | active | `ye27x-pd-settings-touch-value-editor/SPEC.md` | `ye27x-pd-settings-touch-value-editor/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| k7nhc | CP 模式：本机屏幕界面 + HTTP API | active | `k7nhc-cp-mode-ui-http-api/SPEC.md` | `k7nhc-cp-mode-ui-http-api/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| ejgp8 | Web：CP 模式控制页 | active | `ejgp8-web-cp-mode-ui/SPEC.md` | `ejgp8-web-cp-mode-ui/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| wjhba | 主界面：PD 按钮两行文案规范（Detach / PPS / Fixed） | active | `wjhba-dashboard-pd-button-label/SPEC.md` | `wjhba-dashboard-pd-button-label/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| t5x4k | Web：仪器风格主界面（左监右控） | active | `t5x4k-web-instrument-control-ui/SPEC.md` | `t5x4k-web-instrument-control-ui/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| swzqu | 触摸弹簧（GPIO14）负载开关 + RGB 指示 + 语音播放（MAX98357A / I²S） | active | `swzqu-touch-spring-load-switch-rgb-led/SPEC.md` | `swzqu-touch-spring-load-switch-rgb-led/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| j6tmd | Storybook：隐藏断点竖线标尺（768/1024） | active | `j6tmd-storybook-hide-breakpoint-guides/SPEC.md` | `j6tmd-storybook-hide-breakpoint-guides/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| rjkcw | GitHub Pages：Web Deploy 失败修复（lockfile/CI） | active | `rjkcw-web-pages-deploy-lockfile/SPEC.md` | `rjkcw-web-pages-deploy-lockfile/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| yff7v | Web：版本号展示 + GitHub 分支跳转 | active | `yff7v-web-version-github-link/SPEC.md` | `yff7v-web-version-github-link/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| sjb5t | 服务端口规范化 | active | `sjb5t-service-ports-avoid-defaults/SPEC.md` | `sjb5t-service-ports-avoid-defaults/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| v3g2c | 触摸电源按键：睡眠待机白光低频呼吸 | active | `v3g2c-touch-power-button-standby-breathing-white/SPEC.md` | `v3g2c-touch-power-button-standby-breathing-white/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| e3rv6 | LoadLynx devd 本地设备控制面 | active | `e3rv6-loadlynx-devd-control-plane/SPEC.md` | `e3rv6-loadlynx-devd-control-plane/IMPLEMENTATION.md` | 首版实现：devd/CLI、Web devd+Firmware、firmware catalog、digital identity/DNS-SD、Storybook/视觉证据 |
| fhpfk | LoadLynx operational skills packaging and workflow boundary | active | `fhpfk-loadlynx-operational-skills/SPEC.md` | `fhpfk-loadlynx-operational-skills/IMPLEMENTATION.md` | 新增 owner-facing 电脑安装/更新主指南；公开 skill 命令统一为 global install + official `skills update` |
| fqmns | Dashboard Boot Link Recovery | active | `fqmns-boot-link-recovery/SPEC.md` | `fqmns-boot-link-recovery/IMPLEMENTATION.md` | 软件实现与构建验证完成；HIL 因当前 worktree 缺失 selector 阻断 |
| j979q | Release 失败 Telegram 告警接入 | active | `j979q-release-failure-telegram-alerts/SPEC.md` | `j979q-release-failure-telegram-alerts/IMPLEMENTATION.md` | 统一监听 `Release (LoadLynx)`；普通 PR CI 失败不触发 Telegram，release failure 保留 smoke path |
| dvfnn | PR Label Release Flow | active | `dvfnn-pr-label-release-flow/SPEC.md` | `dvfnn-pr-label-release-flow/IMPLEMENTATION.md` | PR label release contract、Label Gate、`main` PR-only + `0 approvals`、发布后 PR 自动评论 |
| m8k2v | Web Bundle Budget Gates | active | `m8k2v-web-bundle-budget-gates/SPEC.md` | `m8k2v-web-bundle-budget-gates/IMPLEMENTATION.md` | 显式 app/Storybook bundle budget、Storybook framework runtime 单独验收、CI 门禁 |
| shkmx | 音频迁移：蜂鸣器 → 扬声器 | active | `shkmx-buzzer-to-speaker-audio/SPEC.md` | `shkmx-buzzer-to-speaker-audio/IMPLEMENTATION.md` | 实现完成；旧规划来源与实现记录保留在 companion docs。 |
| t2f5j | USB-PD EPR 28V Sink Enablement | active | `t2f5j-usb-pd-epr-28v-sink/SPEC.md` | `t2f5j-usb-pd-epr-28v-sink/IMPLEMENTATION.md` | PR #72；实现与构建已完成；owner-facing fixed PDO 语义已收敛为 live-only；HIL 仍待 EPR 线材补验 |
| w4cpd | Dashboard 扩展电压开关与 PD 设置入口重构 | active | `w4cpd-dashboard-extended-voltage-toggle/SPEC.md` | `w4cpd-dashboard-extended-voltage-toggle/IMPLEMENTATION.md` | PR #70；实现已完成；HIL 可选 |
| wh4s9 | Calibration 页面模式同步与状态链路稳定化 | active | `wh4s9-calibration-mode-sync-stability/SPEC.md` | `wh4s9-calibration-mode-sync-stability/IMPLEMENTATION.md` | 本地校准页稳定性修复、Storybook/E2E 回归与视觉证据已完成 |
| y4sf4 | Digital Display PSRAM/DMA Pipeline | active | `y4sf4-display-psram-dma-pipeline/SPEC.md` | `y4sf4-display-psram-dma-pipeline/IMPLEMENTATION.md` | PR #71；PSRAM 专用 framebuffer arena、多缓冲 present、真实 present-FPS、细粒度 dirty rect、pending 背压 |
| br7kq | LoadLynx Backup & Restore | active | `br7kq-loadlynx-backup-restore/SPEC.md` | `br7kq-loadlynx-backup-restore/IMPLEMENTATION.md` | CLI/Web JSON backup restore；恢复前强制关闭负载；WiFi PSK 明文备份 |
| n9v2q | Web Console Cyberpunk Redesign | active | `n9v2q-web-cyberpunk-redesign/SPEC.md` | `n9v2q-web-cyberpunk-redesign/IMPLEMENTATION.md` | 全 Web Console 赛博朋克重做；移除 daisyUI/Iconify；新增 shadcn 风格组件与 i18n |
| n5nwv | Web production preview smoke 与 chunk-cycle regression | active | `n5nwv-web-production-preview-smoke/SPEC.md` | `n5nwv-web-production-preview-smoke/IMPLEMENTATION.md` | 修复 production-only 首屏崩溃；新增 dist preview smoke 并接入 web-check / web-pages |
| m3n8p | Web Console 顶部导航与设备工作面 | active | `m3n8p-web-top-nav-device-workspace/SPEC.md` | `m3n8p-web-top-nav-device-workspace/IMPLEMENTATION.md` | 顶部导航壳层、总览/仪表盘/系统 IA、设备 sheet/returnTo 切换与 about 页。 |
