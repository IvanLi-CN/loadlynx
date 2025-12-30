# LoadLynx Main Display UI

![Main display mock (CC)](../assets/main-display/main-display-mock-cc.png)

![Main display mock (CV)](../assets/main-display/main-display-mock-cv.png)

> Mock is rendered at 320×240 px, matching the landscape frame buffer of the P024C128-CTP module (`docs/other-datasheets/p024c128-ctp.md`).

## 需求说明

- 左列 `CURRENT`：将两路电流镜像条（CH1/CH2）移动到 `CURRENT` 标签右侧，仅显示条形图（不显示 `CH1/CH2` 文本标签）；条形图宽度自适应剩余空间，不得溢出当前卡片。
- 右列：删除原 `CH1/CH2` 电流区块（标签与条形图均不显示）。
- 除以上要点外，其余布局与信息保持不变（control row、左列三大读数、右列 status lines 等）。

## Panel Constraints

- Module: Shenzhen P&O P024C128-CTP, 2.4 in, RGB vertical stripe, 4-wire SPI.
- Native grid: 320 px (X) × 240 px (Y) when mounted landscape in the enclosure.
- Active area: 48.96 mm × 36.72 mm; enforce ≥4 px safe margins and align objects to an 8 px baseline wherever possible.

## Layout Breakdown

| Zone | Pixel bounds (x1,y1)-(x2,y2) | Notes |
| --- | --- | --- |
| Left primary block | (0,0)-(189,239) | Holds voltage, current, power. Each card is 72 px tall with 6 px gutters and tinted slab backgrounds to reinforce grouping. |
| Right status block | (190,0)-(319,239) | control row、Remote/local voltage、status lines，依次垂直排列。 |

### Left block element map

| Element | Style | Content | Font | Placement (px) |
| --- | --- | --- | --- | --- |
| Voltage label | `#9AB0D8` caption | VOLTAGE | SmallFont (8×12) | (16,10) |
| Voltage digits | `#FFB347` | 24.50 (decimal dot rendered manually) | SevenSegNumFont (32×50) | 左区 80 px 高度段 #1：数字/单位右对齐至 x=170，绘制区域 `(24,28)-(170,72)` |
| Voltage unit | `#9AB0D8` | V | SmallFont | 基线 y=72，与数字紧贴 |
| Current label | `#9AB0D8` | CURRENT | SmallFont | (16,90) |
| Current mirror bar | 轨道 `#1C2638`、填充 `#4CC9F0`、中心刻度 `#6D7FA4` | Mirror bar（CH1 left / CH2 right，无标签） | — | `CURRENT` 标签右侧，动态宽度：`x=(label_end+4)..(CARD_BG_RIGHT-2)`（限制在当前卡片背景内，不得溢出到右侧区块）；默认 `CURRENT` 文本宽 56 px → `x=76..180`；`y=92..99` |
| Current digits | `#FF5252` | 12.00 | SevenSegNumFont | 左区 80 px 高度段 #2：区域 `(24,108)-(170,152)`，右对齐至 x=170 |
| Current unit | `#9AB0D8` | A | SmallFont | 基线 y=152，与数字相连 |
| Power label | `#9AB0D8` | POWER | SmallFont | (16,170) |
| Power digits | `#6EF58C` | 294.0 | SevenSegNumFont | 左区 80 px 高度段 #3：区域 `(24,188)-(170,232)`，右对齐至 x=170，支持 0.1 W 精度 |
| Power unit | `#9AB0D8` | W | SmallFont | 基线 y=232，与数字紧贴并仍留 8 px 底边距 |

### 数字精度规范（布局稳定：固定总位数）

| 指标 | 显示形式 | 备注 |
| --- | --- | --- |
| 左列电压/电流 | 固定 `DD.dd`（4 个数字 + 1 个小数点），四舍五入到 0.01 → 例如 `24.50`, `03.20` | 固定宽度用于布局稳定；两位整数不足时左侧补零。异常/超出显示能力时显示 `99.99`。 |
| 左列功率 | 固定 `DDD.d`（4 个数字 + 1 个小数点），四舍五入到 0.1 → 例如 `294.0`, `001.1` | 固定宽度用于布局稳定；三位整数不足时左侧补零。异常/超出显示能力时显示 `999.9`。 |
| 右列远/近端电压 | 固定 `DD.dd` + 单位（例如 `24.52V`），四舍五入到 0.01 | 保持与左侧主读数一致的总位数策略以避免“空间忽大忽小”的观感。 |
| 电流镜像条 | 仅显示两路镜像条形图（无 `CH1/CH2` 文本标签，不显示单通道数值） | 条形图位于左列 `CURRENT` 标签右侧；右列不再显示 `CH1/CH2` 区块。 |
| 温度 | 0 或 1 位小数（`37°` 或 `37.8°`），根据传感器噪声门限自动选择 | 单位符号与数值之间保留 1 空格。 |
| 运行时间、能量 | 现有格式 (`HH:MM:SS`, `125.4Wh`) | 如需更多精度，在右列列表中扩展即可。 |

### Right block element map（对称双值布局）

| Pair | Payload | Font | Color | Placement |
| --- | --- | --- | --- | --- |
| Control row | 模式切换（CC/CV）+ 当前 preset 目标值（单位随模式变更） | SmallFont | 背景 `#1C2638`；CC 文本 `#FF5252`；CV 文本 `#FFB347`；数值 `#DFE7FF`；选中位高亮背景 `#4CC9F0`（高亮位字符改为深色） | 行背景：`y=10..38`；MODE pill `(198,10)-(252,38)`；VALUE pill `(256,10)-(314,38)`；默认选中十分位（0.1），不显示“步长”文本 |
| Voltage pair | 左列 REMOTE `24.52 V`，右列 LOCAL `24.47 V` | 标签 SmallFont；数值 SmallFont（字符间距 0，强制 4 位数格式） | 文本 `#DFE7FF`、标签 `#6D7FA4` | 左列起点 (198,50)，右列起点 (258,50) |
| Voltage mirror bar | 中心 0 V，左右各 55 px 行程（上限 40 V） | — | 轨道 `#1C2638`，填充与两侧条统一使用 `#4CC9F0`，中心刻度 `#6D7FA4` | 长条 `(198,84)-(314,91)`，中心 x=256 |
| Status lines (5) | 运行时间 + 温度 + 状态行（例如 `RUN 01:32:10`、`CORE 42.3C`、`SINK 38.1C`、`MCU 35.0C`、`RDY` / `CAL` / `OFF` / `FLT 0x12345678`） | SmallFont | `#DFE7FF` | Right block 底部对齐：Top-left at `(198,172)` 起，每行 +12px，底边距约 12px（**每行最多 15 字符**，避免右侧被裁切） |

### Status line #5：状态文案（禁止 debug 噪声）

- 正常就绪：`RDY`
- 模拟板离线：`OFF`
- 校准缺失：`CAL`
- 故障：`FLT`（仅状态）或 `FLT 0x12345678`（fault_flags 非 0）
- **禁止显示**：`P1..P5`、`CC/CV`、`OUT0/OUT1`、`UV0/UV1` 等调试位域（易误读且会被裁切）

> Mirror bars：中心刻度标注 `0`，左半对应该对数据中的左值，右半对应右值。左/右填充长度 = `min(value / limit, 1.0) * half_width`，其中电压上限 40 V，电流上限 5 A。

### Control row：选中位背景高亮（固定宽度字体，禁止“视觉居中”偏移）

- 目标值文本固定为 6 字符：`DD.ddU`（例如 `12.00A`、`24.50V`），使用 `SmallFont`（8×12）绘制，字符间距为 0（固定宽度字形单元格）。
- 目标值文本在 VALUE pill 内 **右对齐**，右内边距 **4 px**；因此该 6 字符串宽度固定 48 px，用于保持布局稳定。
- 选中位高亮背景（颜色 `#4CC9F0`）以 `SmallFont` 数字字形的稳定 bbox 为基准绘制：
  - 数字字形 bbox：`x=0..4`、`y=2..9`（5×8）
  - 高亮背景内边距：左/右各 **1 px**，上/下各 **2 px**（得到 7×12 的高亮块；避免 padding 溢出到相邻字符）
- 选中位字符本身改为深色（推荐 `#080F19`），其余字符保持 `#DFE7FF`。
- 选中位索引（按 `DD.ddU` 的字符索引）：`ones=1`、`tenths=3`、`hundredths=4`。

## Color Palette

| Token | Hex | Usage |
| --- | --- | --- |
| canvas | `#05070D` | Root background. |
| left-base | `#101829` | Base fill for the left column. |
| card tints | `#171F33` / `#141D2F` / `#111828` | Voltage/current/power slabs. |
| voltage-accent | `#FFB347` | High-visibility voltage digits. |
| current-accent | `#FF5252` | High-visibility current digits. |
| power-accent | `#6EF58C` | High-visibility power digits. |
| caption | `#9AB0D8` | All labels and units. |
| right-label | `#6D7FA4` | Secondary text in the right block. |
| right-value | `#DFE7FF` | Status numerics. |
| bar-track | `#1C2638` | Neutral progress-bar background. |
| bar-fill | `#4CC9F0` | Remote/local voltage + channel load percentage. |
| divider | `#1C2A3F` | Column split. |

## Typography (UTFT bitmap fonts)

| Usage | Font | Notes |
| --- | --- | --- |
| Large numerics | `SevenSegNumFont` (32×50) | Numeric-only font from rinkydink; decimal dot drawn as a 6×6 block aligned 8 px above the baseline. Stored at `docs/assets/fonts/SevenSegNumFont.c`. |
| Labels & units | `SmallFont` (8×12) | Default UTFT font. Stored at `docs/assets/fonts/SmallFont.c`. |
| Status values | `SmallFont` (8×12) | Current firmware UI uses UTFT `SmallFont` for all right-column text (labels + values) for predictable spacing. |

All fonts were downloaded from http://rinkydinkelectronics.com/r_fonts.php (Public Domain) and rendered pixel-by-pixel to ensure firmware/layout parity.

## Data Binding & Refresh

1. **Left metrics**
   - Sample at 1 kHz, low-pass (α = 0.3), refresh UI at 20 Hz.
   - When a limit is exceeded, flash a 2 px strip along the top edge of the affected card using `#FF5252` (over) or `#6EF58C` (under).
   - Decimal dot: draw a filled 6×6 square at `y = glyph_baseline − 8` so it lines up with the mock.
   - 左侧 `CURRENT` 仅显示合计电流主读数（数值 + 单位）；两路通道电流以 `CURRENT` 标签右侧镜像条形图表达（无文字标签，不显示单通道数值文本）。
2. **Right status**
   - Voltage bars = `clamp(V_measured / V_range)` with default `V_range = 40 V`.
   - **Control row** replaces the legacy “SET I” line:
     - Shows the **current preset** `mode` (`CC`/`CV`) and the **current preset target** (unit follows the active mode).
     - Target value is sourced from the digital preset model (not measured values), right-aligned to x=314.
     - The selected adjustment digit MUST be shown with a special background highlight (do not rely on underline).
     - Highlight padding: **1 px** left/right, **2 px** top/bottom around the selected `SmallFont` digit bbox (`x=0..4`, `y=2..9`).
   - Runtime + energy update at 2 Hz, temperature at 5 Hz. Keep color semantics fixed for muscle memory.

## Interaction Hooks

### Control row touch + encoder

#### Tap targets (logical pixel bounds)

- MODE pill background: `(198,10)-(252,38)` (rounded rectangle, no border)
  - **CC button**: `(200,12)-(224,36)`
  - **CV button**: `(226,12)-(250,36)`
- VALUE pill background: `(256,10)-(314,38)` (rounded rectangle, no border)
  - Tap the **ones / tenths / hundredths** digit to select the active adjustment digit (highlighted by a special background color; do not rely on underline).

#### Behavior

- Tap **CC** / **CV**: updates the **current preset** `mode` immediately (no extra “apply” step), and MUST force `output_enabled=false` for safety.
- Rotate encoder: adjusts the selected target for the **current preset**:
  - CC: `target_i_ma` (A)
  - CV: `target_v_mv` (V)
  - Adjustment digit options: ones / tenths / hundredths; default is tenths (0.1).
  - The selected digit MUST be remembered (do not show step text in the UI).
- Encoder push button:
  - Short press: toggle `output_enabled` for the current preset.
  - Long press (~800ms): cycle `active_preset_id` (P1..P5), and force `output_enabled=false`.

### Operator quick guide

- Tap the **CC** or **CV** text inside the right-side rounded pill to switch the current preset mode (output is forced OFF for safety).
- Tap a digit in the right-side value pill to select the adjustment digit (the selected digit is highlighted with a special background color; default is tenths = 0.1).
- Rotate the encoder to change the current preset target:
  - CC → current target (A)
  - CV → voltage target (V)

## Assets

- `docs/assets/main-display/main-display-mock-cc.png` — pixel-level mock (CC active).
- `docs/assets/main-display/main-display-mock-cv.png` — pixel-level mock (CV active).
- `docs/assets/fonts/*.c` — raw UTFT fonts bundled for reproducible rendering.
