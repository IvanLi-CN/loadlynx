# Storybook：隐藏断点竖线标尺（768/1024）（#0022）

## 状态

- Status: 已完成
- Created: 2026-01-20
- Last: 2026-01-20

## 背景 / 问题陈述

- 当前 Storybook 默认 decorator 会挂载 `BreakpointRulerOverlay`，在画面上绘制 768/1024 两条全高竖线与顶部标签。
- 这些竖线会干扰组件/页面的视觉检查与截图（尤其是仪器风格大屏界面），希望在 Storybook 中去掉。

## 目标 / 非目标

### Goals

- Storybook 中不再显示 768/1024 的两条全高竖线（以及对应的顶部数字标签）。
- 默认不显示左上角断点信息卡片，但提供开关可在 Storybook 中随时切换显示/隐藏。

### Non-goals

- 不修改 Tailwind 断点定义（`md=768`、`lg=1024` 等）。
- 不变更 Storybook 的 viewport 选项集合与各预设视口尺寸。
- 不在本计划内引入新的 Storybook addon 或新的视觉回归工具。

## 范围（Scope）

### In scope

- 调整 `BreakpointRulerOverlay` 在 Storybook 中的呈现：移除（或默认隐藏）全高断点竖线与顶部标签。
- 必要时调整 Storybook 的 `preview` decorator 以匹配新的默认呈现（仅限 Storybook 环境）。

### Out of scope

- Web App（非 Storybook）运行时的 UI/布局变更。
- 对 `BreakpointRulerOverlay` 做“多模式复杂配置化”（除非主人明确要求）。

## 需求（Requirements）

### MUST

- 在 `localhost:6006/iframe.html?...&viewMode=story` 中打开任意 story 时：
  - **不显示**位于 x=768 与 x=1024 的两条全高竖线；
  - **不显示**与这两条竖线对应的顶部数字标签（768/1024）。
- 默认（未开启开关）不渲染 `BreakpointRulerOverlay` 的任何 UI（包括左上角卡片）。
- 提供 Storybook 工具栏开关，用于切换 `BreakpointRulerOverlay` 左上角信息卡片显示/隐藏：
  - 开启时：显示信息卡片（宽度/模式 + `md: 768` / `lg: 1024` + 卡片内小刻度条）。
  - 关闭时：不显示任何 overlay。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| BreakpointRulerOverlay | UI Component | internal | Modify | ./contracts/ui-components.md | Web | Storybook `preview` decorator | 移除全高竖线与顶部标签 |
| loadlynxShowBreakpointCard | Config | internal | New | ./contracts/config.md | Web | Storybook toolbar | 开关：是否显示左上角信息卡片 |

### 契约文档（按 Kind 拆分）

- [contracts/ui-components.md](./contracts/ui-components.md)
- [contracts/config.md](./contracts/config.md)

## 验收标准（Acceptance Criteria）

- Given 打开任意 Storybook story，When 页面渲染完成，Then 画面中不存在两条全高竖线（768/1024）及其顶部数字标签。
- Given 未开启工具栏开关，When 打开任意 story，Then 左上角不显示断点信息卡片且无任何 overlay。
- Given 开启工具栏开关，When 打开任意 story，Then 显示左上角断点信息卡片（宽度/模式/md/lg），且画面中仍不存在两条全高竖线与顶部数字标签。

## 非功能性验收 / 质量门槛（Quality Gates）

- 不新增依赖与 addon；只通过现有 Storybook 与现有组件代码完成。
- Storybook 启动与构建流程不应受影响（实现阶段按仓库既有脚本验证）。

## 里程碑（Milestones）

- [x] 默认不显示任何 overlay；工具栏开关可切换左上角信息卡片
- [x] 移除全高竖线与顶部数字标签，并确保 story 画面无遮挡

## 已确认口径（Decisions）

- 默认不显示 overlay（统一对所有 stories 生效）。
- 提供 Storybook 工具栏开关，可切换是否显示左上角信息卡片。
- 全高竖线与顶部数字标签统一移除（不做按 story 区分）。
