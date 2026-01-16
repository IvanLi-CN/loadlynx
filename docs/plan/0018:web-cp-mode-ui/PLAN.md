# Web：CP 模式控制页（#0018）

## 状态

- Status: 待实现
- Created: 2026-01-15
- Last: 2026-01-16

## 背景 / 问题陈述

- 在 #0017 提供 CP 模式 + HTTP API 后，需要在 `web/` 增加对应的控制页面，便于在桌面/移动端进行 CP 设定与诊断。
- 现有 Web 已具备设备选择、Status、CC control/presets 等基础能力，但尚未提供 CP 的交互与展示。

## 目标 / 非目标

### Goals

- 在现有 “Device control” 页面（`/$deviceId/cc`）增加 CP 能力，用于：
  - 查看当前 active preset 的模式与关键遥测（尤其是功率相关）；
  - 当设备 `cp_supported=true` 时，允许将 preset 设为 CP 并编辑目标功率（以及必要的限值字段）；
  - 对 `LINK_DOWN` / `ANALOG_NOT_READY` / `LIMIT_VIOLATION` 等错误给出可理解提示。
- 复用仓库既有 Web 技术栈与交互模式（TanStack Router/Query、`HttpApiError`、`mock://` 设备）。

### Non-goals

- 不在本计划内实现固件侧 CP（见 `docs/plan/0017:cp-mode-ui-http-api/PLAN.md`）。
- 不重构全站 Layout 与设备管理信息架构（沿用既有）。
- 不实现 CR/脚本化功率曲线。

## 范围（Scope）

### In scope

- Web route + 页面：
  - 扩展现有 `/$deviceId/cc` 页面：增加 CP 模式的展示与编辑（仅调整 label/单位与必要字段，不要求高保真 mock）。
- API client：
  - 增加 CP 相关类型与请求封装（消费 #0017 的 HTTP API 契约）。
- Mock backend：
  - `mock://` 设备补齐 CP 相关字段与错误路径，支撑 Storybook/E2E。
- 测试与质量：
  - Storybook：关键状态 stories（支持/不支持、链路掉线、越界错误、限流提示）。
  - E2E：至少 1 条用例覆盖“编辑 CP 目标功率 → 保存/应用 → 看到状态更新”。

### Out of scope

- 不改固件端接口形状；如发现接口不足，必须回到 #0017 的 contracts 增量演进再继续 Web。

## 需求（Requirements）

### MUST

- 设备能力 gating：
  - `identity.capabilities.cp_supported=false` 时，页面显示 “固件不支持 CP” 且不展示写入控件（或禁用）。
- 读路径：
  - 能读取并展示 active preset（含 mode、目标功率、限值）与关键遥测（功率/电压/电流摘要）。
- 写路径：
  - 能将某个 preset 设为 `mode="cp"` 并设置 `target_p_mw`；
  - 能触发 apply（若 UI 设计选择通过 presets/apply 方式）。
- 越界与错误：
  - UI 阻止明显越界输入（例如 `target_p_mw > max_p_mw`）；
  - 服务端错误以统一 `HttpApiError` 形式展示，且包含可理解 hint。

### SHOULD

- 状态解释：
  - 当 CP 受限（例如电流限流导致功率达不到）时，UI 有明确提示（若 #0017 提供可诊断字段则优先使用）。
- 刷新策略：
  - 低频轮询/按需刷新，避免与 `FastStatus` SSE 争抢连接资源。

### COULD

- 在 Status 页增加 CP 摘要入口（例如当前模式/目标功率/实际功率），并链接到 CP 控制页。

## 接口契约（Interfaces & Contracts）

（本计划不新增/修改固件侧 HTTP API；消费的外部契约以 #0017 为准。此处只冻结 Web 内部“路由与组件接口”。）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `/:deviceId/cc` | UI Component | internal | Modify | ./contracts/ui-components.md | web | web | 在 Device control 页面扩展 CP |
| `CpPresetEditor` / `CpStatusSummary` | UI Component | internal | New | ./contracts/ui-components.md | web | web | 内部子组件（可选拆分） |

### 契约文档（按 Kind 拆分）

- [contracts/README.md](./contracts/README.md)
- [contracts/ui-components.md](./contracts/ui-components.md)

## 验收标准（Acceptance Criteria）

- Given 设备 `cp_supported=false`，
  When 打开 CP 页面，
  Then 页面展示“不支持”说明且不存在可提交的写入动作。

- Given 设备 `cp_supported=true` 且链路健康，
  When 用户设置 `mode="cp"` 与 `target_p_mw` 并保存/应用，
  Then 页面能看到返回的 `preset` 更新，并在下一次状态刷新中看到功率相关摘要同步变化。

- Given 用户输入越界（例如 `target_p_mw > max_p_mw`），
  When 尝试提交，
  Then UI 阻止提交或服务端返回 `422 LIMIT_VIOLATION` 并被友好展示（不出现无提示失败）。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Storybook：
  - CP 支持/不支持；
  - link down / analog not ready；
  - limit violation（越界）；
  - 受限提示（若有）。
- E2E：
  - 至少 1 条用例覆盖 CP 编辑/保存/应用主路径。

### Quality checks

- Lint / typecheck / formatting: 复用仓库既有（Biome / TypeScript / 现有脚本），不引入新工具。

## 文档更新（Docs to Update）

- `docs/plan/README.md`: 新增本计划索引行（已完成）。
- Web 相关开发文档（如存在）：补充 CP 页入口与 mock 说明。

## 里程碑（Milestones）

- [ ] M1: 冻结 Web 信息架构（入口/路由/页面布局）与组件拆分
- [ ] M2: API client + mock:// 支持 CP 数据与错误路径
- [ ] M3: 页面实现 + Storybook stories
- [ ] M4: E2E 覆盖主路径与关键错误态

## 方案概述（Approach, high-level）

- 以 #0017 的 HTTP API contracts 作为唯一外部依赖口径；Web 不做历史 schema 兼容兜底（接口不够用就先改 contracts）。
- 复用现有页面模式：明确 “Unsupported / Link down / Ready” 三态渲染，避免在一个页面里堆条件分支难以维护。

## 风险与开放问题（Risks & Open Questions）

### 风险

- 若 #0017 在实现时未提供足够的可诊断字段，Web 侧可能难以解释 “功率达不到” 等状态（需要在 contracts 阶段补齐）。

### 已冻结的决策（Freeze）

- CP 控制入口：放在现有 `/$deviceId/cc`（Device control）页面中，与 CC/CV 同位置管理。
- UI 规格：不要求高保真 mock，优先复用现有页面结构，仅调整 label 与单位并补齐 CP 所需字段。
- 写路径：沿用 “编辑 preset + apply”。

## 假设（Assumptions）

- Web 侧不引入新的独立路由；CP 入口与交互全部收敛在 `/$deviceId/cc` 内。
