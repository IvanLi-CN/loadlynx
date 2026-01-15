# #0016 PD 设置：触屏友好的目标值编辑（PD Settings Touch Value Editor）

## 1) 问题陈述

当前 `REQ=PD` 的 PD 设置界面在电压/电流目标值处使用 `- / +` 小按钮进行调整，触屏点击命中率低；同时右栏布局空间紧张，标签（如 `Vreq•20mV`）占用较多垂直高度，导致右侧内容容易溢出、观感不统一。

## 2) 目标 / 非目标

### 目标（Goals）

- 将目标值调整改为“**只显示目标值**”的触屏交互：点击目标值进入编辑，编辑位可通过点击/滑动切换；旋钮（编码器）用于增减当前位。
- 目标值显示使用与 Dashboard / Preset 的目标值一致的 **等宽数字字体**与高亮语义（选中/编辑位高亮）。
- `Ireq` 目标值显示单位改为 **安培 A**，显示 **2 位小数**（内部仍可保留 mA）。
- 右栏布局确保**不溢出**；左右边距必须相等；右栏内容区宽度与分栏比例满足下方像素契约。
- 标签与步长文本改为左侧两行显示：第一行 `Vreq`/`Ireq`，第二行显示步长（如 `20mV`/`50mA`）。

### 非目标（Non-goals）

- 不改变 USB‑PD 协议/能力协商逻辑、Apply 的生效语义、以及现有 Fixed/PPS 列表内容。
- 不在本计划阶段确定最终的手势细节（例如滑动阈值/加速度/长按行为），仅冻结“可实现、可测试”的最小交互契约。

## 3) 用户与场景

- 主要用户：使用触摸屏操作设备的用户（手指触控）。
- 次要输入：旋钮（编码器）调整值；触屏用于进入编辑与选择编辑位。

## 4) 需求列表（MUST / SHOULD / COULD）

### MUST

- 移除 `- / +` 小按钮交互；目标值区域必须是可触控的大点击区域。
- 点击 `Vreq/Ireq` 目标值后进入“编辑模式”，并能够切换当前编辑位。
- 旋钮用于对“当前编辑位”做增减；增减步进遵守 PD 的量化步长（见“约束与风险”）。
- `Ireq` 显示格式：`xx.xxA`（2 位小数；支持前导 0 的一致宽度策略，见开放问题）。
- 标签两行：左侧 `Vreq` / `Ireq`（第一行）+ 步长文本（第二行）。
- 布局控制：不得出现文本/控件溢出裁切；右栏左右边距相等。

### SHOULD

- 目标值区域的视觉样式与 Dashboard/Preset 的 setpoint 行保持一致（等宽数字、选中高亮、编辑位高亮）。
- 触屏滑动切换编辑位：水平滑动选择不同数字位；点击可直接选择某一位。

### COULD

- 支持长按目标值快速回到默认编辑位（例如 `Tenths`）。
- 在编辑模式下提供轻量的提示（如闪烁/下划线）以强调当前编辑位。

## 5) 接口清单与契约（Interface Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `PdSettingsTargetValue` | UI Component | internal | Modify | `./contracts/ui-components.md` | digital UI | `pd_settings` screen | 目标值显示/编辑（触屏+编码器） |

## 6) 约束与风险

- **步长 ≠ 显示精度**：
  - `Vreq` 步长为 `20mV`（`0.02V`），而显示为 `x.xxV`。
  - 因此在选择编辑位时，`Hundredths` 位只能按 `2` 为步长变化（0、2、4、6、8）。
  - 需要在交互与验收中明确“允许选择哪些位”。
- `Ireq` 步长为 `50mA`（`0.05A`），显示 `x.xxA`；同样存在 hundredths 位只能按 5 为步长变化的问题。
- 屏幕空间有限：右栏必须在固定宽度下容纳两行标签与大号等宽数字，存在挤压风险；必须用明确像素契约约束布局。

## 7) 验收标准（Acceptance Criteria）

### 7.1 布局像素契约（以 320×240 逻辑坐标为准）

> 说明：此处契约用于保证与主人的标注图一致的分栏比例。右栏宽度的定义为“**不含四周边距的内容区宽度**”；左右边距必须相等。

- 分栏 divider：2px 宽（像素列精确坐标以标注图为准）。
- 右栏内容区（不含 padding）：宽度必须与标注图一致。
- 右栏 padding：左右边距必须相等（像素级一致）。

当前缺口：主人用于标注比例的参考图（例如 `codex-clipboard-Wyjbq8.png`）未在工作区出现，无法反推最终像素坐标；在拿到标注图后，将把 divider/content/padding 坐标补齐并冻结到此处。

### 7.2 目标值区域效果图（Mocks）

- **阶段 0（已冻结）**：先去掉旧的 `Vreq/Ireq` 行与 `+/-` 小按钮，并用“空白卡片”标出后续可用区域，避免旧控件干扰布局讨论。
  - 适用图片：`pd-settings-pps-touch-available-area.png`
  - 允许保留：右栏的 `Active contract` / `Selected` 信息卡片，以及底部 `Back` / `Apply` 按钮。
  - （像素契约）清空区域：`x=190..319`，`y=130..205`（覆盖旧的 `Vreq/Ireq` 行与 `+/-` 控件；不得裁切 `Selected` 卡片与底部按钮）。
  - 禁止像素污染：mock 图中不得出现调试标注残留（例如红色像素/红框）。

- **阶段 1（待冻结）**：触屏目标值编辑的 3 张效果图应体现“标签两行 + 大点击目标值 + 焦点/编辑位高亮”的最小口径。
  - 适用图片：`pd-settings-pps-touch-vreq.png` / `pd-settings-pps-touch-ireq.png` / `pd-settings-fixed-touch-ireq.png`
  - 必须移除 `+/-` 小按钮；目标值以大点击区域呈现（等宽数字）。
  - `Ireq` 显示为 `xx.xxA`（2 位小数；内部单位仍可保持 mA）。
  - 焦点语义：当前字段用外框高亮；当前编辑位用下划线（或等价的可视化标记，最终由开放问题 #4 冻结）。

### 7.3 触屏与编码器交互（可测试最小口径）

- Given 当前在 PD 设置页，When 点击目标值区域，Then 进入编辑模式并高亮当前编辑位。
- Given 编辑模式，When 左右滑动，Then 编辑位在允许集合内切换（见开放问题/最终决策）。
- Given 编辑模式，When 旋钮顺/逆时针旋转，Then 目标值按 PD 步长变化，且不会越界（min/max 由所选 PDO/APDO 决定）。

## 8) 开放问题（需要主人决策）

1. `Vreq` 编辑位允许集合是否固定为 `{Ones, Tenths, Hundredths}`（不允许 `Thousandths`）？
2. `Ireq` 在 `0.05A` 步长下，是否也允许选择 `Hundredths` 位（但只能 0/5 变化），还是只允许 `{Ones, Tenths}`？
3. 等宽目标值是否需要固定宽度（例如总是显示前导 0：`09.00V`、`02.00A`）？
4. 目标值区域的“高亮”形态：仅外框高亮，还是同时对当前 digit 做下划线/反色？
5. 请提供/放入工作区“右栏宽度标注图”（例如 `codex-clipboard-Wyjbq8.png`）或直接给出：divider 的 `x` 坐标与右栏内容区宽度（不含边距）。我会据此冻结 7.1 的像素契约并重绘效果图。

## 9) 假设（Assumptions；待主人确认）

- 假设本计划仅涉及 on-device UI（digital 固件渲染），不涉及 web UI。
- 假设 `Ireq` 内部仍使用 mA 存储与协议传输，仅 UI 层做 `A` 的格式化展示。

## Milestones

- [ ] 冻结交互口径（编辑位集合/手势规则/前导 0 策略）
- [ ] 冻结布局像素契约（divider/右栏内容宽度/边距）
- [ ] 产出并确认 3 张效果图（PPS Vreq focused / PPS Ireq focused / Fixed Ireq focused）

## 附：效果图位置

- 资产目录：`docs/assets/usb-pd-settings-panel/`
  - `pd-settings-pps-touch-available-area.png`（空白卡片：标出目标值编辑的可用区域）
  - `pd-settings-pps-touch-vreq.png`
  - `pd-settings-pps-touch-ireq.png`
  - `pd-settings-fixed-touch-ireq.png`
- 预览页：`docs/plan/0016:pd-settings-touch-value-editor/preview.html`（用 Playwright 打开）
