# Web：Responsive Drawer Sidebar（ConsoleLayout 导航）

## 背景

当前 Web Console 的左侧导航在非 tool 布局下始终为固定宽度侧栏。中小屏（分屏/小笔记本/手机）会显著挤压内容区，缺少抽屉（drawer）与折叠（icon rail）形态，导致可用性与空间利用不佳。

本设计为 `ConsoleLayout` 的左侧菜单增加响应式行为：大屏固定展开、中屏默认图标栏可展开、小屏默认收起并通过汉堡按钮弹出抽屉。

## 目标

- **Large**：侧栏固定展开（图标 + 文案），布局与当前一致。
- **Medium**：侧栏默认 **icon rail（仅图标）**，支持按钮点击切换厚度（rail ↔ expanded）。
- **Small**：侧栏默认完全收起；点击汉堡按钮后以 **drawer（overlay）** 形式弹出。
- **图标**：导航项补齐图标，且 **必须使用 Iconify（非 CDN）**。
- **抽屉差异化**：drawer 内允许与大屏侧栏不同；要求在 drawer 底部提供 **切换设备**，默认跳转到 `/$deviceId/cc`。
- **Storybook 可验证**：在 Storybook 中提供“标尺/刻度”与固定视口 stories，快速确认断点与形态。

## 非目标

- 不调整路由结构与导航信息架构（菜单项名称/层级/路径保持不变）。
- 不在本次设计中强制启用 header 的“Current device selector”（当前 UI 可能为占位/禁用态）。
- 不引入依赖 CDN 的图标加载方式（包括 Iconify API 在线取图）。

## 断点（约定）

采用 Tailwind 默认断点作为三态切换基线：

- **Small**：`< md`（`< 768px`）
- **Medium**：`md <= width < lg`（`768px ~ 1023px`）
- **Large**：`>= lg`（`>= 1024px`）

> 断点必须在 Storybook 中通过“标尺/刻度”可视化展示，避免口头误差。

## 形态与行为规范

### 术语

- **Inline sidebar**：占据布局空间的常驻侧栏（非 overlay）。
- **Icon rail**：仅图标的窄侧栏（inline）。
- **Drawer**：overlay 抽屉（带遮罩层），从左侧滑入。
- **Tool layout**：用于“工具页/沉浸页”（现有机制），内容区优先全宽。

### 总体规则

1. **Large**
   - 渲染 inline sidebar，固定展开（图标 + 文案）。
   - 不显示 rail 折叠态。
2. **Medium**
   - 渲染 inline sidebar，但默认是 icon rail（仅图标）。
   - 提供“切换厚度”按钮：点击在 `rail ↔ expanded` 间切换（非 hover）。
3. **Small**
   - 默认不渲染 inline sidebar/rail（不占位）。
   - 在 header 提供汉堡按钮，点击打开 drawer。
   - drawer 打开时：点击遮罩、按 `Esc`、点击任一导航项 -> 关闭 drawer。
4. **路由切换**
   - 当用户通过 drawer 导航跳转路由后，drawer 必须自动关闭。
5. **尺寸切换（resize）**
   - 进入 **Small**：inline sidebar 关闭（不渲染），drawer 默认为关闭态。
   - 进入 **Medium/Large**：drawer 强制关闭；Medium 的 rail/expanded 状态保留为 UI 状态（可选持久化）。

### Tool layout 决策（本次由实现方拍板）

为兼顾沉浸与可导航性，约定：

- Tool layout 下 **不显示 inline sidebar/rail（Large/Medium 也隐藏）**，保持内容全宽。
- 但所有宽度下都允许通过 **header 汉堡按钮**打开 drawer 进行导航与切换设备（drawer 默认为关闭，不影响沉浸）。

## 导航内容规范

### 侧栏/抽屉的导航项

- 全局项：`Devices`
- 设备域项（需 `deviceId`）：
  - `CC Control`
  - `Status`
  - `Settings`
  - `Calibration`

无 `deviceId` 时，设备域项需以禁用态展示（仍显示图标；icon-only 形态必须有可理解的无障碍标签）。

### Drawer 的差异化内容

drawer 允许增加额外区域，至少包含：

- **底部固定区域：Device Switcher**
  - 用于切换当前设备（从本地 devices registry 选择）。
  - 选择后执行路由跳转（见下一节）。

## Device Switcher（drawer 底部）

### 目标行为

- 在 drawer 底部提供设备选择控件（select 或列表均可）。
- 当用户选择目标设备 `B`：
  - 若当前路径形如 `/$deviceId/<tab>`（例如 `/A/status`）：跳到 `/B/<tab>`。
  - 否则（例如在 `/devices` 或其它非设备域页面）：跳到 **`/B/cc`**（默认）。
- 跳转完成后关闭 drawer。

### 边界情况

- 设备列表为空：控件显示空态并禁用。
- 目标设备不存在（极端情况，缓存过期）：不跳转，提示或保持原样（实现阶段定）。

## 图标方案（Iconify，禁止 CDN）

### 强制约束

- 必须使用 Iconify 的 React 组件并通过 **本地依赖打包**图标。
- 禁止使用在线 API/字符串 icon 名触发的按需网络加载。

### 推荐实现形态（结构示意）

```tsx
import { Icon } from "@iconify/react";
import home from "@iconify-icons/mdi-light/home";

<Icon icon={home} />
```

> 关键点：`icon` 传入 **icon data object**，而不是 `"mdi-light:home"` 这类字符串。

## Storybook 验证设计

### 目标

Storybook 需要成为断点验证标尺：

- 在画面内叠加显示：当前 viewport 宽度、当前模式（Small/Medium/Large）、以及 768/1024 刻度。
- 提供 3 个固定视口 stories（Small/Medium/Large）用于快速回归。

### 预期检查点

- Large story：存在 inline sidebar（图标+文案），无 drawer。
- Medium story：默认 icon rail；点击 toggle 后展开显示文案。
- Small story：无 inline sidebar；点击汉堡按钮后出现 drawer；点击遮罩关闭。
- Tool layout story：无 inline sidebar；汉堡按钮可打开 drawer。

## 模块边界（概要）

建议将“导航渲染”与“布局外壳”分离，减少重复：

- `ConsoleLayout`：负责 header + 主内容区 + 根据模式选择 Inline/Ddrawer。
- `SidebarNav`（可提取组件）：负责渲染导航项列表（同一份数据驱动 Large/Medium/Drawer）。
- `DeviceSwitcher`（可提取组件）：drawer 底部的设备切换控件与跳转逻辑。
- `useResponsiveNavMode`（可提取 hook）：负责根据 viewport 与 tool layout 计算 nav mode，并维护 rail/drawer 状态。

## 可访问性要求（最低线）

- icon-only 按钮/链接必须有 `aria-label` 或可见 tooltip（至少其一）。
- drawer 支持 `Esc` 关闭、点击遮罩关闭。
- drawer 打开后焦点可达并可关闭（是否做 focus trap 在实现阶段评估）。

## 兼容性与迁移

- URL 与路由结构不变（仅 UI 布局与交互变化）。
- 现有 `ConsoleLayout` 的 tool layout 机制保留（本设计仅扩展其导航策略）。

## 风险点与待确认（实现阶段再定）

- Medium 的 rail/expanded 状态是否需要持久化（localStorage）；
- drawer 中设备切换控件形态（select vs 列表）与空态呈现；
- tooltip 与可访问性实现细节（DaisyUI 组件/自研实现取舍）。

## 测试计划（实现阶段）

- Storybook：新增/更新 stories + play assertions（断点形态与交互）。
- 手工：桌面/分屏/手机三种尺寸下导航与设备切换路径回归。
- E2E（可选）：覆盖 drawer 打开/关闭与 device switch 跳转逻辑。

