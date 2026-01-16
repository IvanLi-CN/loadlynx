# UI Component Contracts

> Scope: internal React components for the instrument-style, single-screen control & monitor layout.

## Inventory

| Component | Change | Purpose |
| --- | --- | --- |
| InstrumentStatusBar | New | 顶部状态条：设备信息、模式、链路、输出与保护摘要 |
| MonitorReadouts | New | 四象限读数：V/A/W/R（Readback） |
| MainDisplayPanel | New | 主显示区：大号读数 + 小状态 + 趋势线 |
| ThermalPanel | New | 热状态与故障摘要 |
| HealthTiles | New | Online/Fault/Latency 简表 |
| PdSummaryPanel | New | USB‑PD 摘要卡片（如可用） |
| ControlModePanel | New | 模式切换 + 输出开关 |
| SetpointsPanel | New | 设定值与读回并列显示 |
| LimitsPanel | New | 保护阈值与锁存状态 |
| PresetsPanel | New | 预设列表 + Apply/Save |
| AdvancedPanel | New | 高级功能入口（折叠或摘要） |

## Contracts

### InstrumentStatusBar

**Props**

- `deviceName: string` — 设备名
- `deviceIp: string | null` — IP（未知时为 null）
- `firmwareVersion: string | null` — 固件版本（未知时为 null）
- `modeLabel: "CC" | "CV" | "CP" | "CR" | "UNKNOWN"`
- `linkState: "up" | "down" | "unknown"`
- `outputState: { enabled: boolean; setpointLabel: string | null }`
- `protectionState: { summary: string; level: "ok" | "warn" | "danger" }`
- `faultSummary: string | null` — 无故障时可为 null

**Notes**

- `setpointLabel` 示例：`"1.500 A"` / `"12.000 V"`，用于与模式一致的设定值摘要。

---

### MonitorReadouts

**Props**

- `voltage: { read: number; local?: number; remote?: number }` (unit: V)
- `current: { read: number; local?: number; remote?: number }` (unit: A)
- `power: { read: number; ripplePct?: number }` (unit: W)
- `resistance: { read: number | null }` (unit: Ω)

**Behavior**

- `resistance.read === null` 时显示 `—`，不显示衍生说明。

---

### MainDisplayPanel

**Props**

- `headline: { value: number; unit: "V" | "A" | "W" }`
- `modeLabel: "CC" | "CV" | "CP" | "CR"`
- `setpointLabel: string`
- `uptimeLabel: string` — 例如 `"01:24:18"`
- `trend: { points: number[]; min: number; max: number }`

**Behavior**

- `trend.points` 为空时显示空线与 `No data` 说明。

---

### ThermalPanel

**Props**

- `sinkCoreC: number | null`
- `sinkExhaustC: number | null`
- `mcuC: number | null`
- `faults: string[]` — 为空表示无故障

---

### HealthTiles

**Props**

- `analogState: string`
- `faultLabel: string`
- `linkLatencyMs: number | null`

---

### PdSummaryPanel

**Props**

- `visible: boolean`
- `contractText: string | null`
- `ppsText: string | null`
- `savedText: string | null`

---

### ControlModePanel

**Props**

- `availableModes: Array<"CC" | "CV" | "CP" | "CR">`
- `activeMode: "CC" | "CV" | "CP" | "CR"`
- `onModeChange: (mode) => void`
- `outputEnabled: boolean`
- `onOutputToggle: (nextEnabled) => void`
- `outputHint: string | null`

---

### SetpointsPanel

**Props**

- `setpoints: Array<{ label: string; value: string; readback: string | null }>`

**Notes**

- `readback` 为 `null` 时隐藏 readback 行。

---

### LimitsPanel

**Props**

- `limits: Array<{ label: string; value: string; tone?: "ok" | "warn" | "danger" }>`

---

### PresetsPanel

**Props**

- `presets: Array<{ id: number; label: string; active: boolean }>`
- `onPresetSelect: (id) => void`
- `onApply: () => void`
- `onSave: () => void`
- `applyDisabled: boolean`
- `saveDisabled: boolean`

---

### AdvancedPanel

**Props**

- `summary: string`
- `collapsed: boolean`
- `onToggle: (collapsed) => void`

