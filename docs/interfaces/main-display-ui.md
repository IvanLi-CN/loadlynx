# LoadLynx Main Display UI

![Main display mock](../assets/main-display/main-display-mock.png)

> Mock is rendered at 320×240 px, matching the landscape frame buffer of the P024C128-CTP module (`docs/other-datasheets/p024c128-ctp.md`).

## Panel Constraints

- Module: Shenzhen P&O P024C128-CTP, 2.4 in, RGB vertical stripe, 4-wire SPI.
- Native grid: 320 px (X) × 240 px (Y) when mounted landscape in the enclosure.
- Active area: 48.96 mm × 36.72 mm; enforce ≥4 px safe margins and align objects to an 8 px baseline wherever possible.

## Layout Breakdown

| Zone | Pixel bounds (x1,y1)-(x2,y2) | Notes |
| --- | --- | --- |
| Left primary block | (0,0)-(189,239) | Holds voltage, current, power. Each card is 72 px tall with 6 px gutters and tinted slab backgrounds to reinforce grouping. |
| Right status block | (190,0)-(319,239) | Remote/local voltage、CH1/CH2 currents、runtime、temperature、energy，依次垂直排列。 |

### Left block element map

| Element | Style | Content | Font | Placement (px) |
| --- | --- | --- | --- | --- |
| Voltage label | `#9AB0D8` caption | VOLTAGE | SmallFont (8×12) | (16,10) |
| Voltage digits | `#FFB347` | 24.50 (decimal dot rendered manually) | SevenSegNumFont (32×50) | 左区 80 px 高度段 #1：数字/单位右对齐至 x=170，绘制区域 `(24,28)-(170,72)` |
| Voltage unit | `#9AB0D8` | V | SmallFont | 基线 y=72，与数字紧贴 |
| Current label | `#9AB0D8` | CURRENT | SmallFont | (16,90) |
| Current digits | `#FF5252` | 12.00 | SevenSegNumFont | 左区 80 px 高度段 #2：区域 `(24,108)-(170,152)`，右对齐至 x=170 |
| Current unit | `#9AB0D8` | A | SmallFont | 基线 y=152，与数字相连 |
| Power label | `#9AB0D8` | POWER | SmallFont | (16,170) |
| Power digits | `#6EF58C` | 294.0 | SevenSegNumFont | 左区 80 px 高度段 #3：区域 `(24,188)-(170,232)`，右对齐至 x=170，支持 0.1 W 精度 |
| Power unit | `#9AB0D8` | W | SmallFont | 基线 y=232，与数字紧贴并仍留 8 px 底边距 |

### 数字精度规范

| 指标 | 显示形式 | 备注 |
| --- | --- | --- |
| 左列电压/电流/功率 | 固定 4 位数（含小数点）；最小分辨率 0.001 → 例如 `24.50`, `12.00`, `294.0` | 若数值不足四位，用前导空格/零补齐；若超限，则采用科学计数或滚动提示而非截断。 |
| 右列远/近端电压、通道电流 | 同样 4 位数 + 单位，分辨率与传感器一致（最高千分位） | 条形长度仍按真实比例渲染。 |
| 温度 | 0 或 1 位小数（`37°` 或 `37.8°`），根据传感器噪声门限自动选择 | 单位符号与数值之间保留 1 空格。 |
| 运行时间、能量 | 现有格式 (`HH:MM:SS`, `125.4Wh`) | 如需更多精度，在右列列表中扩展即可。 |

### Right block element map（对称双值布局）

| Pair | Payload | Font | Color | Placement |
| --- | --- | --- | --- | --- |
| Voltage pair | 左列 REMOTE `24.52 V`，右列 LOCAL `24.47 V` | 标签 SmallFont；数值 SmallFont（字符间距 0，强制 4 位数格式） | 文本 `#DFE7FF`、标签 `#6D7FA4` | 左列起点 (198,8)，右列起点 (258,8) |
| Voltage mirror bar | 中心 0 V，左右各 55 px 行程（上限 40 V） | — | 轨道 `#1C2638`，填充与两侧条统一使用 `#4CC9F0`，中心刻度 `#6D7FA4` | 长条 `(198,44)-(314,50)`，中心 x=256 |
| Current pair | 左列 CH1 `4.20 A`，右列 CH2 `3.50 A` | 标签 SmallFont；数值 SmallFont（字符间距 0，强制 4 位数格式） | 同上 | 左列起点 (198,96)，右列起点 (258,96) |
| Current mirror bar | 0 A 居中，上限 5 A/通道 | — | 轨道 `#1C2638`，填充 `#4CC9F0` | 长条 `(198,132)-(314,138)`，中心 x=256 |
| Run status line | “RUN 01:32:10” | SmallFont | `#DFE7FF` | Baseline at `(198,200)` |
| Temperature line | “TEMP 37.8°C” | SmallFont | `#DFE7FF` | Baseline at `(198,214)` |
| Energy line | “ENERGY 125.4Wh” | SmallFont | `#DFE7FF` | Baseline at `(198,228)` |

> Mirror bars：中心刻度标注 `0`，左半对应该对数据中的左值，右半对应右值。左/右填充长度 = `min(value / limit, 1.0) * half_width`，其中电压上限 40 V，电流上限 5 A。

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
| Status values | `arial_bold` (16×16) | UTFT bitmap submitted by MBWK; stored at `docs/assets/fonts/arial_bold.c`. |

All fonts were downloaded from http://rinkydinkelectronics.com/r_fonts.php (Public Domain) and rendered pixel-by-pixel to ensure firmware/layout parity.

## Data Binding & Refresh

1. **Left metrics**
   - Sample at 1 kHz, low-pass (α = 0.3), refresh UI at 20 Hz.
   - When a limit is exceeded, flash a 2 px strip along the top edge of the affected card using `#FF5252` (over) or `#6EF58C` (under).
   - Decimal dot: draw a filled 6×6 square at `y = glyph_baseline − 8` so it lines up with the mock.
2. **Right status**
   - Voltage bars = `clamp(V_measured / V_range)` with default `V_range = 30 V`.
   - Channel bars = `clamp(I_actual / I_rating)` (defaults: CH1 12 A, CH2 6 A).
   - Runtime + energy update at 2 Hz, temperature at 5 Hz. Keep color semantics fixed for muscle memory.

## Interaction Hooks

- (Optional CTP) tap on any metric tile to open its detail drawer（对远/近端电压可切换 sense 设置，对通道可切换 CC/CV/CP）。
- Drag horizontally on the voltage/current bars to trim setpoints in ±10 mV / ±10 mA steps；长按可锁定通道。
- 提供 320×240 RGB565 framebuffer dump，方便产线校验像素级界面。

## Assets

- `docs/assets/main-display/main-display-mock.png` — pixel-level mock (rendered with UTFT fonts).
- `docs/assets/fonts/*.c` — raw UTFT fonts bundled for reproducible rendering.
