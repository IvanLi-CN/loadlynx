# UI Components contracts — #0016

## `PdSettingsTargetValue`（目标值显示与编辑）

### Scope

- internal（digital on-device UI）

### Responsibilities

- 显示 `Vreq` / `Ireq` 的目标值（PPS：两行；Fixed：仅 `Ireq`）。
- 提供触屏进入编辑、切换编辑位；编码器增减当前编辑位。
- 在 UI 层做单位与格式化（`Ireq` 显示为 A，2 位小数），不改变内部存储与协议单位（可继续使用 mV / mA）。

### Visual contract

- 目标值区域为大点击面积（适配手指）。
- 数字使用与 Dashboard / Preset 目标值一致的等宽数字字体（setpoint font）。
- 选中/编辑位以“下划线/高亮”呈现（与 Preset Panel 同一语义）。
- 标签左侧两行：
  - 第 1 行：`Vreq` / `Ireq`
  - 第 2 行：步长文本（`20mV` / `50mA`）

### Formatting

- `Vreq`：显示 `x.xxV`（2 位小数）。步长 `0.02V`（`20mV`）。
- `Ireq`：显示 `xx.xxA`（2 位小数）。步长 `0.05A`（`50mA`）。

### Interaction contract (minimum, testable)

#### Inputs

- Touch:
  - `tap(x,y)`：点击命中目标值区域进入编辑/选位
  - `swipe(dx,dy)`：水平滑动切换编辑位（仅在编辑模式有效）
- Encoder:
  - `delta(steps)`：对当前编辑位做增减（`steps` 可为正/负）

#### State

- `focused_field`: `None | Vreq | Ireq`
- `focused_digit`: `None | Ones | Tenths | Hundredths`（具体集合待 #0016 冻结）

#### Outputs / Effects

- `focused_field` 与 `focused_digit` 变化应立即反映在 UI（外框高亮 + 编辑位下划线）。
- 旋钮增减会在允许范围内更新目标值（clamp 到 min/max），并保持与 PD 量化步长对齐：
  - `Vreq`：`20mV`
  - `Ireq`：`50mA`（显示为 `0.05A`）

#### Error / Edge cases

- 目标值越界：clamp，不允许 UI 显示超过 PDO/APDO 限制的值。
- 步长导致的“显示位不可逐 1 变化”：允许出现仅 `0/2/4/6/8` 或 `0/5` 的情况（具体由 #0016 的编辑位集合决策决定）。

