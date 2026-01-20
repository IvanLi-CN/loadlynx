# Config contracts（#0022）

## `loadlynxShowBreakpointCard`（Storybook globals）

- Kind: Config（internal）
- Surface: Storybook toolbar + `preview` decorator

### Purpose

- 提供一个全局开关，控制是否渲染 `BreakpointRulerOverlay` 的左上角信息卡片。

### Shape

- Key: `loadlynxShowBreakpointCard`
- Type: boolean
- Default: `false`

### Behavior

- When `false`: `preview` decorator 不渲染 `BreakpointRulerOverlay`（无任何 overlay）。
- When `true`: `preview` decorator 渲染 `BreakpointRulerOverlay`（仅左上角信息卡片；不包含全高竖线与顶部数字标签）。

### UI (toolbar)

- Control: toggle（on/off）
- Label: `Breakpoint card`（或等价简短文案；实现阶段统一口径）

### Compatibility

- 仅影响 Storybook 环境；Web App 运行态不依赖此配置。
