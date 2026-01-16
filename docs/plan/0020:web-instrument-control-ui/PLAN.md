# Web：仪器风格主界面（左监右控）（#0020）

## 状态

- Status: 待实现
- Created: 2026-01-16
- Last: 2026-01-16

## 背景 / 问题陈述

- 现有 Web 的“CC Control”页面同时承载监视与控制，但布局偏功能堆叠，缺乏“大屏仪器感”和清晰的视觉层级。
- 目标用户希望在同一界面完成“监视 + 操作”，且风格接近专业仪器的前面板体验。
- 当前监视重点（电压/温度视觉权重）与用户期待存在差异，需要在设计上对主读数进行明确优先级定义。

## 目标 / 非目标

### Goals

- 形成“左监右控”的单屏仪器风格布局：监视与操作同屏且层级清晰。
- 建立 Set vs Readback 的清楚对照，避免设定值与读数混淆。
- 保留现有功能能力（模式切换、输出开关、预设管理、主显示屏等），但重新组织信息结构与视觉层级。
- 保持现有 HTTP API 与数据流不变，仅做 UI 结构与交互优化。
- `device-status` 作为技术信息诊断页保留，避免影响工程排查流程。

### Non-goals

- 不改动固件/协议/HTTP API 结构与语义。
- 不新增功能能力（例如新增模式或新测试类型）。
- 不在本阶段处理移动端的彻底重构（仅保证基本可用）。

## 范围（Scope）

### In scope

- `web/src/routes/device-cc.tsx` 布局重构为左监右控。
- 抽取可复用 UI 组件（监视区卡片、控制区模块）。
- 统一视觉风格为“仪器大屏”风格（暗色面板、发光读数、仪表层级）。
- 修正文案与信息层级（模式、输出、故障/保护等）。
- `device-status` 页面保留诊断定位，入口弱化但可达。

### Out of scope

- 新的 API 或设备能力扩展。
- 现有 `device-status` 页面完全废弃或合并到主界面。
- 对设计系统/主题进行全站级重构。

## 需求（Requirements）

### MUST

- 左侧为监视区（读数/状态/趋势/故障/PD 摘要），右侧为控制区（模式/输出/设定/限制/预设）。
- 监视区至少包含：V/A/W/R 读回、主显示屏（含趋势线）、热/故障摘要、链路/模拟状态。
- 控制区必须包含：模式切换、输出开关、设定值（Set）与读回（Readback）并列、保护限制、预设应用。
- Set 与 Readback 视觉上明确区分，且不会混淆数值单位。
- 不改变现有 API 调用与轮询策略的语义；写操作仍需避免与轮询冲突。
- 温度与故障信息常驻显示（先出一版，后续再评估是否降权）。

### SHOULD

- 监视区与控制区之间可形成“仪器感”的对照（读数集中、控制模块化）。
- 故障/保护状态在顶栏或监视区有清晰信号（颜色/标签）。
- 主显示区支持替换主读数（由模式决定优先项）。

### COULD

- 高级功能入口（瞬态/列表/电池/触发）保持折叠摘要形式。
- 提供轻量趋势线（不做重型图表库）。

## UI 设计规范与约束（Instrument UI Guidelines）

- 信息层级：主读数 > 次级读数 > 说明文字；主读数字号至少为次级的 1.5×。
- 左监右控固定布局（>=1280px）：监视区宽度 60–65%，控制区 35–40%；两列同高对齐。
- Readback vs Setpoint：必须明确标注（Read/Set），并采用不同色阶与字重；单位始终显示。
- 数值格式：V/A 保留 3 位小数、W 保留 2 位小数、R 保留 2 位小数；无数据用 `—` 占位。
- 状态提示：OK/WARN/DANGER 三态颜色 + 文本标签双通道提示，不能只靠颜色。
- 仪器风格：暗色面板 + 细边框 + 内阴影 + 轻发光读数；避免扁平白底风格。
- 交互反馈：按钮/模式切换有清晰 active 状态；输出开关与 Apply 的风险提示常驻可见。
- 动效约束：仅使用轻量过渡（<200ms），避免高频动画干扰读数辨识。
- 响应式：<1024px 时控制区下移，但功能完整；监视区优先呈现。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| InstrumentStatusBar | UI Component | internal | New | ./contracts/ui-components.md | Web | DeviceCcRoute | 顶部状态条 |
| MonitorReadouts | UI Component | internal | New | ./contracts/ui-components.md | Web | DeviceCcRoute | V/A/W/R 读数 |
| MainDisplayPanel | UI Component | internal | New | ./contracts/ui-components.md | Web | DeviceCcRoute | 主显示区 |
| ThermalPanel | UI Component | internal | New | ./contracts/ui-components.md | Web | DeviceCcRoute | 温度与故障 |
| HealthTiles | UI Component | internal | New | ./contracts/ui-components.md | Web | DeviceCcRoute | 状态小卡 |
| PdSummaryPanel | UI Component | internal | New | ./contracts/ui-components.md | Web | DeviceCcRoute | PD 摘要 |
| ControlModePanel | UI Component | internal | New | ./contracts/ui-components.md | Web | DeviceCcRoute | 模式/输出 |
| SetpointsPanel | UI Component | internal | New | ./contracts/ui-components.md | Web | DeviceCcRoute | 设定值 |
| LimitsPanel | UI Component | internal | New | ./contracts/ui-components.md | Web | DeviceCcRoute | 保护限制 |
| PresetsPanel | UI Component | internal | New | ./contracts/ui-components.md | Web | DeviceCcRoute | 预设 |
| AdvancedPanel | UI Component | internal | New | ./contracts/ui-components.md | Web | DeviceCcRoute | 高级入口 |

### 契约文档（按 Kind 拆分）

- [contracts/README.md](./contracts/README.md)
- [contracts/ui-components.md](./contracts/ui-components.md)

## 验收标准（Acceptance Criteria）

- Given 设备页面处于 CC 模式，When 打开控制页，Then 单屏呈现“左监右控”布局，且不需要跳页即可完成监视与操作。
- Given 输出已启用，When 应用预设导致输出关闭，Then 控制区明确提示需要重新开启输出。
- Given 主读数显示 Set 与 Readback，When 任意一个缺失，Then 以 `—` 显示并保持单位占位。
- Given 链路断开或错误，When 状态更新失败，Then 顶部或监视区出现明显告警，且控制区仍可显示上次已知状态（标注为 stale）。
- Given 控制区切换模式，When 模式变更成功，Then 监视区主读数同步反映新的主指标（如 CC 优先电流）。
- Given 浏览器宽度 >= 1280px，When 进入页面，Then 左监右控两列同屏且不出现横向滚动。
- Given 浏览器宽度 < 1024px，When 进入页面，Then 监视区优先显示，控制区可下移但功能完整可用。
- Given 用户需要诊断信息，When 进入 `device-status` 页面，Then 仍可查看技术信息与原始状态数据。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Unit tests: 若新增组件存在纯计算逻辑，放入 `web/src` 对应测试目录。
- Integration tests: 更新/新增 `web/tests` 中与控制页相关的流程（模式切换、输出开关、预设应用）。
- E2E tests: 覆盖“同屏监视+控制”核心路径（进入页 → 切模式 → Apply → Output on）。

### UI / Storybook (if applicable)

- Stories to add/update: 为新组件（MonitorReadouts/MainDisplayPanel/ControlModePanel）补充基础 stories（若当前 web 启用 Storybook）。
- Visual regression baseline changes (if any): 如已有基线，更新本页面快照。

### Quality checks

- Lint / typecheck / formatting: `bun lint` / `bun typecheck`（按 web 现有约定）

## 文档更新（Docs to Update）

- `web/README.md`: 更新页面结构与入口说明（如果已有 UI 导航文档）。
- `docs/plan/README.md`: 新增本计划索引行。

## 里程碑（Milestones）

- [ ] M1: 需求冻结（读数优先级与主读数策略确认）
- [ ] M2: 组件与布局方案确定（UI 结构与信息层级）
- [ ] M3: 验收标准冻结（含异常与响应式）

## 方案概述（Approach, high-level）

- 以 `device-cc` 为唯一单屏入口，重排为左监右控，强调仪器面板感。
- 将监视区拆为模块化组件（读数、主显示、热/故障、摘要卡），保证可测试与可复用。
- 控制区统一为“模式/输出/设定/限制/预设/高级”模块化布局。
- 不改动 API 与数据流，保持现有轮询与写操作冲突规避逻辑。

## 风险与开放问题（Risks & Open Questions）

- 风险：信息密度过高导致可读性下降，需要精确的层级与字号策略。
- 风险：监视区持续刷新与控制区写操作并存时的状态一致性。
- 需要决策的问题：None。

## 开放问题（需要主人回答）

- None.

## 假设（Assumptions）

- 假设 CC 模式下主读数优先为电流 + 功率，电压作为次级读数。
- 假设 PD 摘要保留在监视区底部。

## 参考（References）

- `web/src/routes/device-cc.tsx`
- `web/src/routes/device-status.tsx`
- `docs/assets/plan-0020-web-instrument-control-ui/instrument-dashboard-mock.png`
