# UI Components contracts（#0022）

## BreakpointRulerOverlay

- Kind: UI Component（internal）
- Consumers: Storybook `web/.storybook/preview.tsx` decorator

### Purpose

- 在 Storybook 中提供响应式验证辅助信息（当前 viewport 宽度、模式、md/lg 断点值）。

### Rendered elements（after change）

- MUST: 信息卡片（可拖拽）
  - 显示：`<width>px · <mode>`
  - 显示：`md: 768`、`lg: 1024`
  - 包含：小刻度条（在卡片内部，显示 md/lg 相对位置）
- MUST NOT: 全屏覆盖层的两条断点竖线（x=768/x=1024）与顶部数字标签

### Visibility（Storybook）

- 默认不渲染（由 `loadlynxShowBreakpointCard=false` 控制）。
- 当 `loadlynxShowBreakpointCard=true` 时渲染信息卡片。

### Change

- Before: 渲染全屏覆盖层（两条全高竖线 + 顶部数字标签），用于“画面内断点标尺”。
- After: 移除全屏覆盖层（两条竖线与顶部标签），仅保留信息卡片与卡片内的小刻度条用于断点提示。
