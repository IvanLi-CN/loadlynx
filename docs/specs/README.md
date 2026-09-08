# 规格（Spec）Catalog

本目录维护 LoadLynx 的长期 topic-level specification。每个规格目录由 `SPEC.md`、`IMPLEMENTATION.md`、`HISTORY.md` 组成：`SPEC.md` 保留稳定需求、行为契约与验收口径，`IMPLEMENTATION.md` 记录实现覆盖与验证，`HISTORY.md` 记录关键演进原因。

> Canonical specs are the only planning-spec taxonomy; historical plan files have been removed after taxonomy consolidation.

## 快速新增一个规格

1. 为主题选择唯一、稳定的 lowercase kebab-case slug。
2. 新建目录：`docs/specs/<topic>/`。
3. 在该目录下创建 `SPEC.md`、`IMPLEMENTATION.md`、`HISTORY.md`。
4. 在下方 Index 表新增一行，并把 `Lifecycle` 设为当前生命周期。

## 生命周期（Lifecycle）说明

- `active`：当前有效规格。
- `superseded`：已被另一个规格取代；`Successor` 必须指向后继规格的 canonical `SPEC.md`。
- `archived`：仅保留历史记录，不再作为当前交付依据。

## Index

| Topic | Lifecycle | Implementation | Spec | Successor | Notes |
|-------|-----------|----------------|------|-----------|-------|
| `cc-load-switch-toggle` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `cc-load-switch-toggle/SPEC.md` | - | CC 负载开关（Load Switch）：设置值 / 生效值分离 |
| `cv-mode-presets` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `cv-mode-presets/SPEC.md` | - | CV 模式 + Preset |
| `ft6336u-touch-int` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `ft6336u-touch-int/SPEC.md` | - | FT6336U 触控（P024C128-CTP）驱动与 digital 集成 |
| `mdns-and-lan-discovery` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `mdns-and-lan-discovery/SPEC.md` | - | mDNS 与局域网发现 |
| `on-device-preset-ui` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `on-device-preset-ui/SPEC.md` | - | 本机 Preset UI（触屏 + 旋钮） |
| `preset-ui-protection-labels` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `preset-ui-protection-labels/SPEC.md` | - | Preset UI：UVLO / OCP / OPP 命名与三线约束 |
| `prompt-tone-manager` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `prompt-tone-manager/SPEC.md` | - | 提示音管理器（蜂鸣器 Prompt Tone） |
| `storybook-component-workshop` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `storybook-component-workshop/SPEC.md` | - | Web Storybook 组件工作台 |
| `uart-comm-troubleshooting` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `uart-comm-troubleshooting/SPEC.md` | - | MCU↔MCU 串口通信问题说明与排查方案（记录规范 & 实测数据） |
| `usb-pd-sink-toggle` | superseded | 早期 5V/20V 两态实现已由扩展电压开关与 Safe5V 门控契约取代。 | `usb-pd-sink-toggle/SPEC.md` | `dashboard-extended-voltage-toggle/SPEC.md` | USB‑PD Sink：5V/20V 两态切换（STM32G431 UCPD） |
| `web-layouts` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `web-layouts/SPEC.md` | - | Web UI Layout 规范化（Layouts 抽象） |
| `web-responsive-drawer-sidebar` | superseded | 旧 drawer / sidebar shell 已被顶部导航与设备工作面规格取代，保留历史 traceability。 | `web-responsive-drawer-sidebar/SPEC.md` | `web-top-nav-device-workspace/SPEC.md` | Web：Responsive Drawer Sidebar（ConsoleLayout 导航） |
| `usb-pd-pps-and-fixed-settings` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `usb-pd-pps-and-fixed-settings/SPEC.md` | - | USB‑PD 设置面板：Fixed PDO / PPS APDO（UI + UART 协议 + HTTP API） |
| `web-usb-pd-settings` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `web-usb-pd-settings/SPEC.md` | - | Web：USB‑PD 设置页（对接 /api/v1/pd） |
| `auto-screen-dim-off` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `auto-screen-dim-off/SPEC.md` | - | 数字板：屏幕自动调暗与熄屏 |
| `pd-settings-touch-value-editor` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `pd-settings-touch-value-editor/SPEC.md` | - | PD 设置：触屏友好的目标值编辑（PD Settings Touch Value Editor） |
| `cp-mode-ui-http-api` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `cp-mode-ui-http-api/SPEC.md` | - | CP 模式：本机屏幕界面 + HTTP API |
| `web-cp-mode-ui` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `web-cp-mode-ui/SPEC.md` | - | Web：CP 模式控制页 |
| `dashboard-pd-button-label` | superseded | 早期两行 PD 文案设计已由扩展电压开关与设置入口契约取代。 | `dashboard-pd-button-label/SPEC.md` | `dashboard-extended-voltage-toggle/SPEC.md` | 主界面：PD 按钮两行文案规范（Detach / PPS / Fixed） |
| `web-instrument-control-ui` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `web-instrument-control-ui/SPEC.md` | - | Web：仪器风格主界面（左监右控） |
| `touch-spring-load-switch-rgb-led` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `touch-spring-load-switch-rgb-led/SPEC.md` | - | 触摸弹簧（GPIO14）负载开关 + RGB 指示 + 语音播放（MAX98357A / I²S） |
| `storybook-hide-breakpoint-guides` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `storybook-hide-breakpoint-guides/SPEC.md` | - | Storybook：隐藏断点竖线标尺（768/1024） |
| `web-pages-deploy-lockfile` | active | Pages 只部署已验证的 release tarball，并作为 release 创建前置门槛。 | `web-pages-deploy-lockfile/SPEC.md` | - | GitHub Pages：release Web artifact 部署 |
| `web-version-github-link` | active | Pages footer、HTML shell 与 version.json 均来自同一 release artifact。 | `web-version-github-link/SPEC.md` | - | Web：版本号展示 + GitHub 分支跳转 |
| `service-ports-avoid-defaults` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `service-ports-avoid-defaults/SPEC.md` | - | 服务端口规范化 |
| `touch-power-button-standby-breathing-white` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `touch-power-button-standby-breathing-white/SPEC.md` | - | 触摸电源按键：睡眠待机白光低频呼吸 |
| `loadlynx-devd-control-plane` | active | 首版实现：devd/CLI、Web devd+Firmware、firmware catalog、digital identity/DNS-SD、Storybook/视觉证据 | `loadlynx-devd-control-plane/SPEC.md` | - | LoadLynx devd 本地设备控制面 |
| `loadlynx-operational-skills` | active | 新增 owner-facing 电脑安装/更新主指南；公开 skill 命令统一为 global install + official `skills update` | `loadlynx-operational-skills/SPEC.md` | - | LoadLynx operational skills packaging and workflow boundary |
| `boot-link-recovery` | active | 软件实现与构建验证完成；HIL 因当前 worktree 缺失 selector 阻断 | `boot-link-recovery/SPEC.md` | - | Dashboard Boot Link Recovery |
| `release-failure-telegram-alerts` | active | 统一监听 `Release (LoadLynx)`；普通 PR CI 失败不触发 Telegram，release failure 保留 smoke path | `release-failure-telegram-alerts/SPEC.md` | - | Release 失败 Telegram 告警接入 |
| `pr-label-release-flow` | active | PR label release contract、Label Gate、`main` PR-only + `0 approvals` | `pr-label-release-flow/SPEC.md` | - | PR Label Release Flow |
| `web-bundle-budget-gates` | active | 显式 app/Storybook bundle budget、Storybook framework runtime 单独验收、CI 门禁 | `web-bundle-budget-gates/SPEC.md` | - | Web Bundle Budget Gates |
| `buzzer-to-speaker-audio` | active | 实现完成；旧规划来源与实现记录保留在 companion docs。 | `buzzer-to-speaker-audio/SPEC.md` | - | 音频迁移：蜂鸣器 → 扬声器 |
| `usb-pd-epr-28v-sink` | active | PR #72；实现与构建已完成；owner-facing fixed PDO 语义已收敛为 live-only；HIL 仍待 EPR 线材补验 | `usb-pd-epr-28v-sink/SPEC.md` | - | USB-PD EPR 28V Sink Enablement |
| `dashboard-extended-voltage-toggle` | active | PR #70；实现已完成；HIL 可选 | `dashboard-extended-voltage-toggle/SPEC.md` | - | Dashboard 扩展电压开关与 PD 设置入口重构 |
| `calibration-mode-sync-stability` | active | 本地校准页稳定性修复、Storybook/E2E 回归与视觉证据已完成 | `calibration-mode-sync-stability/SPEC.md` | - | Calibration 页面模式同步与状态链路稳定化 |
| `display-psram-dma-pipeline` | active | PR #71；PSRAM 专用 framebuffer arena、多缓冲 present、真实 present-FPS、细粒度 dirty rect、pending 背压 | `display-psram-dma-pipeline/SPEC.md` | - | Digital Display PSRAM/DMA Pipeline |
| `loadlynx-backup-restore` | active | CLI/Web JSON backup restore；恢复前强制关闭负载；WiFi PSK 明文备份 | `loadlynx-backup-restore/SPEC.md` | - | LoadLynx Backup & Restore |
| `web-cyberpunk-redesign` | active | 全 Web Console 赛博朋克重做；移除 daisyUI/Iconify；新增 shadcn 风格组件与 i18n | `web-cyberpunk-redesign/SPEC.md` | - | Web Console Cyberpunk Redesign |
| `web-production-preview-smoke` | active | 修复 production-only 首屏崩溃；新增 dist preview smoke 并接入 web-check / web-pages | `web-production-preview-smoke/SPEC.md` | - | Web production preview smoke 与 chunk-cycle regression |
| `web-pwa-offline-updates` | active | PWA app shell、prompt-style 更新、离线 reload smoke、Storybook 更新提示证据 | `web-pwa-offline-updates/SPEC.md` | - | Web PWA Offline Shell and Update Prompt |
| `web-top-nav-device-workspace` | active | 顶部导航壳层、总览/仪表盘/系统 IA、设备 sheet/returnTo 切换与 about 页。 | `web-top-nav-device-workspace/SPEC.md` | - | Web Console 顶部导航与设备工作面 |
| `loadlynx-spec-conflicts` | active | 八项跨规格冲突已修复并由 Rust、Web 与 Storybook 回归覆盖。 | `loadlynx-spec-conflicts/SPEC.md` | - | LoadLynx 跨规格实现一致性 |
