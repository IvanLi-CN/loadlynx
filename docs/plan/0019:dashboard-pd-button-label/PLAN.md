# 主界面：PD 按钮两行文案规范（Detach / PPS / Fixed）（#0019）

## 状态

- Status: 已完成
- Created: 2026-01-16
- Last: 2026-01-19

## 背景 / 问题陈述

- 主界面右侧 `PD` 按钮目前是两行文本，但仅在 `PD/5V` 与 `PD/20V` 间切换，信息密度不足：
  - 无法区分当前配置是否为 `PPS`（APDO）；
  - 未明确 “未连接/未挂载（Detach）” 状态；
  - Fixed 模式下也缺少“实际目标电压”表达（未来不止 5/20）。

## 目标 / 非目标

### Goals

- 规范主界面 `PD` 按钮两行显示的**口径与格式**，使其能表达三类配置状态：
  - Detach（未挂载）显示 `PD/Detach`
  - PPS 模式显示 `PPS/<V>V`
  - PD Fixed 模式显示 `PD/<V>V`
- 颜色语义保持与既有主界面一致（见下方“颜色语义”），不引入新的颜色含义。
- 文案规则可用于：固件渲染、效果图/标注、以及后续文档引用（可持续契约）。
  - 说明：Type‑C/PD 规范中常见状态命名为 `Attached/Unattached`；本 UI 使用 “Detach” 作为用户可读的 “Unattached（未连接到 PD Source / 无 Attach）” 表达。

### Non-goals

- 不修改 USB‑PD 协商策略、能力探测、或 Apply/保存逻辑。
- 不改变 `PD` 按钮的尺寸/圆角/布局（仅规范两行文本内容与格式）。
- 不新增新的 UI 入口或页面。

## 范围（Scope）

### In scope

- 主界面 `PD` 按钮两行文本的显示规则（含 “/” 代表换行的约定）。
- 值格式化（Fixed vs PPS 的精度差异）。
- 与既有颜色语义的对齐与复用（输出状态色、不可用灰显）。
- 必要时补齐/声明从哪获取“当前配置模式（Fixed/PPS）”与“目标电压（Vreq）”的来源（仅在文档契约中冻结；实现阶段再落地）。

### Out of scope

- 右侧 `PD Settings` 页面内的文案与布局（已由 #0013/#0016 管控）。
- Web 端显示（如需同步另开计划）。

## 需求（Requirements）

### MUST

- `PD` 按钮文本为两行（下文使用 `<line1>/<line2>` 表示，`/` 代表换行）：
  - **Detach**：`PD/Detach`
  - **PPS**：`PPS/<V>V`（`<V>` 显示一位小数，例如 `20.0V`）
  - **PD Fixed**：`PD/<V>V`（`<V>` 为整数，例如 `20V`）
- 两行均居中（沿用现有两行 layout，保持 2–3 px 视觉行距）。
- 颜色语义保持与既有主界面一致：
  - `pd_state` 决定按钮的主强调色（Standby/Negotiating/Active/Error）。
  - 当目标档位**不可用**时，下行 `<line2>` 灰显（不改变上行状态色）。
- 目标值来源：以**合同值（active contract）**为准渲染 `<V>`（而非 EEPROM “期望值”）。
- 目标值缺失时：下行显示 `N/A`（例如 `PD/N/A` 或 `PPS/N/A`；具体 `<line1>` 取决于当前配置模式）。
- 未连接（Detach）时：无论当前配置模式为何，均显示 `PD/Detach`（不显示 `PPS/Detach`）。

### SHOULD

- 当目标电压无法判定或缺失时，下行显示一个“明确的占位符”，避免误导（占位符细节见开放问题）。
- “不可用（target unavailable）” 的判断规则对 Fixed 与 PPS 都一致可解释（例如来自能力列表/合同信息/范围校验）。

### COULD

- 在 Error 状态下（`pd_state=Error`），下行可切换为更诊断向的短码（例如 `Err`），但必须保持两行布局不抖动（本计划默认不做，除非主人选择）。

## 颜色语义（与既有主界面保持一致）

> 颜色与含义的权威来源：`docs/interfaces/main-display-ui.md`（此处只做与本计划相关的最小复述）。

- `#4CC9F0`（theme / cyan）：主界面强调色（例如输出 ON、PD Active、进度条 fill、选中态）。
- `#FFB24A`（CV / amber）：电压相关的高可见强调（如大号 Voltage 数字、PD Negotiating）。
- `#FF5252`（CC / red）：电流相关的高可见强调 + 错误态（如大号 Current 数字、PD Error）。
- `#6EF58C`（power / green）：功率数字强调（Power digits）。
- `#555F75`（muted / gray）：不可用/禁用/Standby（例如未启用的按钮边框或灰显的目标档位）。
- 文本与背景/分割线等中性色：沿用既有 palette（不在本计划重复列全表）。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `DashboardPdButtonLabel` | UI Component | internal | Modify | `./contracts/ui-components.md` | digital UI | dashboard main screen | 规范两行文本的内容/格式/颜色语义 |

### 契约文档（按 Kind 拆分）

- [contracts/ui-components.md](./contracts/ui-components.md)

## UI 设计约束（Design constraints; frozen）

本计划的“像素/排版/颜色/文案”约束以两处为准（后续实现必须逐条满足）：

- 全局主界面布局文档：`docs/interfaces/main-display-ui.md`（PD button 小节）
- 组件契约：`docs/plan/0019:dashboard-pd-button-label/contracts/ui-components.md`

## 验收标准（Acceptance Criteria）

- Given `Detach`（未挂载），When 渲染 `PD` 按钮，Then 显示 `PD/Detach`，并保持两行居中与既有行距。
- Given `PPS` 模式且目标电压为 20000 mV，When 渲染 `PD` 按钮，Then 显示 `PPS/20.0V`（一位小数）。
- Given `PD Fixed` 模式且目标电压为 20000 mV，When 渲染 `PD` 按钮，Then 显示 `PD/20V`（整数）。
- Given `pd_state=Standby|Negotiating|Active|Error`，When 渲染，Then 上行状态色与按钮边框状态色保持一致且不引入新的颜色含义。
- Given 目标档位不可用（例如所选目标不在当前能力列表内），When 渲染，Then 下行灰显（`#555F75`），上行不灰显。
- Given 目标电压缺失/无法解析，When 渲染，Then 下行使用约定的占位符且不崩溃、不显示误导性电压值。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Unit tests（如可在 host 侧抽出纯格式化逻辑）：覆盖三种模式 + 缺失值 + 不可用灰显判定。
- On-device smoke：确认 `PD` 按钮文本不会因为刷新抖动（宽度变化）造成肉眼可见的“跳动”。

### Quality checks

- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -D warnings`（如仓库/目标允许；不新增工具）

## 文档更新（Docs to Update）

- `docs/interfaces/main-display-ui.md`: 补充/修订 `PD` 按钮两行文本规则（从 “PD/5V|20V” 更新为 Detach/PPS/Fixed 规则）。

## 里程碑（Milestones）

- [x] M1: 冻结三态文案映射（Detach/PPS/Fixed）与触发条件
- [x] M2: 冻结数值格式化与占位符规则（Fixed 整数 / PPS 一位小数 / `N/A`）
- [x] M3: 冻结“不可用灰显”判定来源（完整 caps/范围校验）并补齐文档更新点

## 方案概述（Approach, high-level）

- 复用现有 `PD` 按钮两行居中布局：仅替换 `line1/line2` 的生成规则。
- 将 “显示模式（Detach/PPS/Fixed）” 与 “目标电压（mV）” 作为渲染输入（来自 EEPROM 配置与/或最近一次 PD status/合同），并在契约中冻结字段口径。
- 将格式化逻辑尽量收敛为纯函数（便于 host 单测），渲染层只负责排版与上色。

## 风险与开放问题（Risks & Open Questions）

（已冻结；无开放问题）

## 参考（References）

- `docs/interfaces/main-display-ui.md`
- `firmware/digital/src/ui/mod.rs`（dashboard `PD` button 现状）
- 相关计划：`docs/plan/0013:usb-pd-pps-and-fixed-settings/PLAN.md`、`docs/plan/0016:pd-settings-touch-value-editor/PLAN.md`

## 效果图（Mocks）

> 说明：本组效果图基于 `docs/assets/main-display/main-display-mock-cc.png`，并按本计划的契约重绘 `PD` 按钮区域（含 SmallFont 像素字体），用于主人确认“实际显示效果”。

- 你需要关注的点：
  - 两行文本是否在按钮内“垂直居中且视觉行距稳定”（不挤、不飘）。
  - `PPS/20.0V` 的 `.0` 是否清晰可读（不糊成一团）。
  - `PD/Detach` 与 `*/N/A` 是否在 8×12 像素字体下仍可辨识。
  - “灰显仅影响第二行”的语义是否符合直觉（第一行仍表达状态色）。

- 组合预览：`docs/assets/main-display/pd-button/dashboard-pd-button-states.png`
- 单张（320×240）：
  - Detach：`docs/assets/main-display/pd-button/dashboard-detach.png`
  - Fixed：`docs/assets/main-display/pd-button/dashboard-fixed-20v.png`
  - PPS：`docs/assets/main-display/pd-button/dashboard-pps-20.0v.png`
  - Fixed（unavailable 灰显）：`docs/assets/main-display/pd-button/dashboard-fixed-unavail.png`
  - PPS（缺失值 N/A）：`docs/assets/main-display/pd-button/dashboard-pps-na.png`

## Change log

- 2026-01-19: Dashboard PD button uses active PD contract for voltage display, and pd_state derives from protocol/contract presence (no v_local-based inference; fixes false Error/red when the contract is established).
