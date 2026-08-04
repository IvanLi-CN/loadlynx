# LoadLynx 跨规格实现一致性

> 当前有效规范以本文为准；实现覆盖与当前状态见 `./IMPLEMENTATION.md`，关键演进原因见 `./HISTORY.md`。

## 背景 / 问题陈述

LoadLynx 的控制面、Web、PWA 与模拟板固件分别由多个 topic-level Specs 约束。现有实现仍有若干路径偏离这些稳定契约，单独查看任一组件时不易识别跨边界冲突。本规格提供聚合验收边界和可追踪入口，但不复制或取代被引用 Specs 的领域规范。

## 目标 / 非目标

### Goals

- 统一追踪已经由 canonical Specs 明确定义、但实现尚未满足的跨组件行为。
- 为安全写入、状态协调、网络发现、Storybook、USB-PD、PWA 与 Web UI 提供共同的完成判据。
- 保证修复期间不改变已确认的 200 W CP 满量程和 EPR 28 V persistence/application 决策。

### Non-goals

- 不成为被引用 Specs 的替代品，也不重新定义其领域接口。
- 不把 36 V 或 48 V EPR capability 变成可持久化或可应用的 target。
- 不以本文授权 HIL、设备选择缓存变更、flash 或 reset。
- 不规定具体的 Issue、分支、PR 或交付拓扑。

## 范围（Scope）

### In scope

- CLI/devd 的 LAN WiFi 写入边界。
- Calibration mode 的单一协调入口。
- Web LAN `/24` 扫描来源约束。
- Storybook 应用初始化的存储副作用。
- 模拟板对 live PDO/APDO 请求的 ACK/NACK 校验。
- PWA migration-only service-worker takeover 与刷新行为。
- 中文 Web UI 可见文案及 daisyUI token 清理。

### Out of scope

- 既有 Specs 未涉及的新功能或架构重构。
- USB-PD capability 发现协议和 28 V EPR 以外的 target policy 扩展。
- 与上述冲突无关的视觉重做、文案重写或网络扫描能力扩展。

## 需求（Requirements）

### MUST

- WiFi `set|clear` 以及 restore 中的 WiFi 写入必须经 USB/devd 独立控制路径；LAN HTTP 和任何 insecure override 必须 fail closed。
- Calibration mode 的设备写入必须由单一 coordinator 所有，route、layout 与 action 不得绕过协调入口直接竞争写入。
- LAN 扫描只能从浏览器当前网络事实派生当前 `/24`；不得接受任意用户输入作为跨网段扫描 seed。
- Storybook 加载应用模块时不得由应用代码读写 `localStorage` 或触发真实网络副作用。
- 模拟板只有在 object position 对应当前 live capability、PDO/APDO 类型和目标范围均有效时才 ACK；无效请求必须 NACK，不得 ACK 后静默回退 Safe5V。
- `skipWaiting` 与 `clientsClaim` 只能用于明确的 migration worker 路径；operator refresh action 在没有 waiting worker 时也必须完成实际刷新。
- 默认中文界面不得出现未经翻译的非领域术语可见文案，live source 不得保留 daisyUI semantic tokens。
- CP 满量程保持 200 W；EPR 只有 28 V target 可持久化和应用，36 V/48 V 最多作为只读 capability 展示。

### SHOULD

- 每项修复应在最接近契约所有者的层级加入回归测试。
- UI 相关修复应复用稳定 Storybook states，并保留可复查的 mock-only 视觉证据。

### COULD

- 独立且可回滚的静态文案和 class-token 修复可以与同一 Web hygiene 变更一起交付。

## 功能与行为规格（Functional/Behavior Spec）

### Core flows

- 写入安全：所有可能改变 WiFi 凭据的入口先解析 transport；非 USB/devd transport 在任何 override 组合下均拒绝写入。
- 状态协调：页面 mount、设备切换、route exit、tab/action 对齐均通过同一 calibration coordinator 串行化。
- PD 请求：UART 请求在 ACK 前对照最近一次 live Source Capabilities；校验失败返回明确 NACK，现有安全合同保持不变。
- PWA 更新：普通 release 保持 operator-driven 更新；只有迁移构建启用 takeover，刷新命令不依赖 waiting worker 是否存在。

### Edge cases / errors

- 缺少 USB/devd、live capabilities、浏览器当前网段或 waiting worker 时必须得到确定且可测试的拒绝/降级行为，不能静默成功。
- Storybook 与 mock 环境不得通过测试 shim 掩盖生产模块初始化时的真实副作用。

## 接口契约（Interfaces & Contracts）

None。本规格聚合既有契约；接口所有权仍属于下列 References 中的 canonical Specs。

## 验收标准（Acceptance Criteria）

- Given 设备仅能通过 LAN HTTP 访问
  When 请求 WiFi set、clear 或含 WiFi 的 restore
  Then 操作被拒绝，且不存在 override 可绕过 USB/devd 要求。
- Given calibration 页面发生 mount、切换设备、切换 tab 或离开 route
  When 需要改变 calibration mode
  Then 所有写入均由同一 coordinator 发出。
- Given 用户提供任意 IPv4 地址
  When 触发 LAN scan
  Then 扫描范围不会因此脱离浏览器当前 `/24`。
- Given UART 请求引用缺失或类型不匹配的 live PDO/APDO
  When 模拟板处理请求
  Then 返回 NACK，且不会以 Safe5V fallback 返回成功 ACK。
- Given 普通 production build 或没有 waiting worker 的 version refresh
  When service worker 更新流程执行
  Then 普通构建不启用 migration takeover，refresh action 仍刷新页面。
- Given 默认 locale 为 `zh-CN`
  When 渲染 CC route 和版本链接
  Then 非领域术语均已本地化，且 source 不含 daisyUI semantic token。

## 验收清单（Acceptance checklist）

- [x] 八项实现冲突均有对应回归测试并通过。
- [x] 相关 canonical Specs 的实现状态与历史已同步。
- [x] Rust/Web/Storybook 的目标构建、lint 与测试通过。
- [x] UI 相关变更具有稳定 Storybook 入口和 owner-facing 视觉证据。
- [x] 200 W CP 与 EPR 28 V 决策未发生漂移。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Rust host/unit tests 覆盖 CLI/devd transport 与 PD request validation。
- Web unit/integration tests 覆盖 coordinator、scan derivation、i18n initialization 与 PWA lifecycle。
- Storybook interaction tests 覆盖受影响的稳定 UI states。

### UI / Storybook

- 更新受影响的既有 route/component stories，不创建依赖真实后端的证据面。
- 对行为关键状态补充 `play` 覆盖，并按仓库视觉证据合同验证桌面与移动视口。

### Quality checks

- `cargo fmt --all`
- 受影响 Rust crates 的 tests/checks。
- `web` lint、typecheck、unit tests、Storybook CI 与 production build。

## Visual Evidence

PR: none

- `./assets/cc-route-chinese.jpg`：默认中文 CC route 的趋势区、状态与控制文案。
- `./assets/app-version-link.jpg`：使用本地样式且不含 daisyUI semantic token 的版本链接。

## Related PRs

- [PR #124](https://github.com/IvanLi-CN/loadlynx/pull/124) establishes the canonical slug-only Specs used by this umbrella boundary.

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：跨组件修复若缺少 exact-SHA integration validation，可能在单项测试通过后重新引入契约漂移。
- 假设：各领域行为仍由其 canonical Spec 所有；本文只负责聚合一致性验收。

## 参考（References）

- `../loadlynx-backup-restore/SPEC.md`
- `../loadlynx-devd-control-plane/SPEC.md`
- `../loadlynx-operational-skills/SPEC.md`
- `../calibration-mode-sync-stability/SPEC.md`
- `../mdns-and-lan-discovery/SPEC.md`
- `../storybook-component-workshop/SPEC.md`
- `../usb-pd-pps-and-fixed-settings/SPEC.md`
- `../web-pwa-offline-updates/SPEC.md`
- `../web-cyberpunk-redesign/SPEC.md`
- `../cp-mode-ui-http-api/SPEC.md`
- `../usb-pd-epr-28v-sink/SPEC.md`
