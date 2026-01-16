# UI Components contracts — #0019

## `DashboardPdButtonLabel`（主界面 PD 按钮两行文案）

### Scope

- internal（digital on-device UI / dashboard main screen）

### Responsibilities

- 生成并渲染主界面 `PD` 按钮的两行文本（两行居中）：
  - `<line1>/<line2>`（`/` 表示换行）
  - 表达 Detach / PPS / PD Fixed 三态
- 复用既有颜色语义：
  - `pd_state` → 主强调色（上行 + 边框/按钮状态）
  - `target_available=false` → 下行灰显

### Geometry (pixel contract)

- Logical frame buffer: `320×240` (landscape, logical coordinate space).
- PD button rect: `(198,118)-(277,145)`（含边框），radius=`6 px`。
- Border thickness: `1 px`（内缩 1 像素绘制内层 fill）。
- Hit box: 与 rect 一致（点击该区域打开 USB‑PD settings）。

### Inputs (data contract)

- `display_mode`: `Detach | Fixed | Pps`
  - Detach：未连接到 PD Source（Type‑C: Unattached / no Attach）
  - Fixed：当前配置为 Fixed PDO
  - Pps：当前配置为 PPS APDO
- `target_mv`: `Option<u32>`
  - Fixed：**合同电压**（单位 mV；例如 `20000`）
  - PPS：**合同电压**（单位 mV；例如 `20000`）
  - Detach：为 `None`
- `pd_state`: `Standby | Negotiating | Active | Error`
- `target_available`: `bool`
  - `false` 表示目标档位在当前能力/范围内不可用（下行灰显）

### Outputs (render contract)

- `line1`: `PD` 或 `PPS`
- `line2`:
  - Detach：`Detach`
  - Fixed：`{V}V`（整数，例：`20V`）
  - PPS：`{V}V`（一位小数，例：`20.0V`）
- Missing/unknown：`N/A`
- `line1_color`: 由 `pd_state` 决定（复用现有 status accent）
- `line2_color`:
  - 电压值：`target_available ? line1_color : #555F75`
  - `N/A` / `Detach`：`#555F75`（避免与电压值混淆）

### Text layout (pixel contract)

- Font：`SmallFont`（8×12），字符间距 `0`。
- 两行水平方向居中：
  - `x = rect.left + (rect.width - text_width)/2`
  - `text_width = chars * 8`
- 垂直方向两行（与现状实现一致）：
  - `pad_top = 3`
  - `line1_y = rect.top + pad_top`
  - `line2_y = line1_y + 11`（轻微重叠以获得 2–3 px 视觉行距）

### Formatting rules (testable)

- Fixed：
  - `target_mv=Some(20000)` → `20V`
  - 小数部分不显示，不进行四舍五入（电压为整数档位时应直接整除）。
- PPS：
  - `target_mv=Some(20000)` → `20.0V`
  - 一位小数固定显示（包括 `.0`）。
- Unknown / missing:
  - `target_mv=None` 且非 Detach 时，`line2 = N/A`。

### Error / edge cases

- 当 `display_mode=Detach`：
  - 强制 `line1/line2 = PD/Detach`（不显示 `PPS/Detach`）。
- 当 `display_mode` 与 `target_mv` 不匹配（例如 Fixed 但 `target_mv=None`）：
  - `line2 = N/A`，不渲染误导性电压值。
- `target_available=false` 时：
  - 仅灰显下行，不改变上行状态色（上行仍表达 “当前 PD 状态”）。
