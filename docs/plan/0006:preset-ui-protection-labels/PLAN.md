# Preset UI：UVLO / OCP / OPP 命名与三线约束（需求与概要设计）（#0006）

## 状态

- Status: 已完成
- Created: 2026-01-03
- Last: 2026-01-07
- Source: migrated from `preset-ui-protection-labels.md` (removed)

## 背景

当前本机屏幕的 Preset 界面使用 `V-LIM / I-LIM / P-LIM` 作为 limit 字段标签，并且在 `CC` 下不显示电流上限字段（仅显示 `TARGET / V-LIM / P-LIM`）。这会带来两类困惑：

- `CV` 下出现 `V-LIM` 容易被误解为“电压上限”，但其语义实际是欠压锁存阈值（`min_v_mv`）。
- `CC` 下电流上限存在于数据模型中但 UI 不可见，用户难以理解“为什么目标被限制/为什么行为和直觉不一致”。

本设计将 Preset 的“三条安全线”在 `CC/CV` 下统一呈现，并将 UI 标签从 `*-LIM` 改为更贴近行业习惯的保护语义缩写。

## 目标

- `CC` 与 `CV` 的 Preset UI 都显示三条安全线（电压/电流/功率），并保持字段顺序一致。
- UI 标签不再使用 `LIM`，改为 `UVLO / OCP / OPP`。
- 交互上禁止出现“目标值突破安全上限”的状态：
  - **安全线（UVLO/OCP/OPP）永远不自动调整**；
  - 当用户调整安全线导致当前目标越界时，允许自动调整目标以恢复不变式（A2）；
  - 当用户直接调整目标时，目标值必须被安全线钳制（不允许越界）。

## 非目标

- 不改变 Preset 底层字段集合与持久化结构（仍使用 `min_v_mv / max_i_ma_total / max_p_mw` 等字段）。
- 不引入新工作模式（如 CP/CR）。
- 不在本文中规定具体控制算法参数（PI、滤波、周期等）。

## 术语与字段映射（UI 标签 ⇔ 内部字段）

> 说明：下列 `UVLO/OCP/OPP` 为 UI 命名；固件/协议字段名保持不变。

| UI 标签 | 中文含义 | 内部字段 | 单位 | 关键语义 |
|---|---|---|---|---|
| `UVLO` | 欠压锁存阈值 | `min_v_mv` | V | 当 `output_enabled=true` 且 `V_main ≤ UVLO` 时触发 `uv_latched`，退流到 0，并仅在用户“关→开”后解除。`UVLO=0` 表示禁用该阈值。 |
| `OCP` | 总电流上限（预设保护阈值） | `max_i_ma_total` | A | 作为“预设保护阈值”参与运行期安全策略：正常情况下用于限制总电流目标（叠加系统硬上限）；若运行期间检测到**实测电流**明显超过该阈值，则必须触发保护停机并在 UI 中以 `OCP` 作为“Protection Trip”原因提示，等待用户本地确认后才消音/清除提示（见 `docs/plan/0005:on-device-preset-ui/PLAN.md`）。注意：此处 `OCP` 不等同于硬件/系统 `fault_flags` 的过流故障。 |
| `OPP` | 总功率上限（预设保护阈值） | `max_p_mw` | W | 作为“预设保护阈值”参与运行期安全策略：正常情况下基于 `V_main` 推导允许的最大电流并限制，确保功率不超过上限；若运行期间检测到**实测功率**明显超过该阈值，则必须触发保护停机并在 UI 中以 `OPP` 作为“Protection Trip”原因提示，等待用户本地确认后才消音/清除提示（见 `docs/plan/0005:on-device-preset-ui/PLAN.md`）。注意：此处 `OPP` 不等同于系统 `fault_flags` 的故障。 |

## UI 字段集合与顺序（冻结）

两种模式均使用相同字段集合与顺序：

1. `TARGET`
2. `UVLO`
3. `OCP`
4. `OPP`

其中 `TARGET` 的语义随 mode 改变：

- `mode=CC`：`TARGET` 为电流目标（A）
- `mode=CV`：`TARGET` 为电压目标（V）

## 交互与不变式（A2：自动联动，禁止突破）

### 1) CC：`TARGET_I ≤ OCP`（冻结）

- 当用户编辑 `TARGET(I)` 时：`TARGET` 不能超过 `OCP`（越界即钳制到 `OCP`）。
- 当用户编辑 `OCP` 且将其调到 `< 当前 TARGET` 时：系统必须自动执行 `TARGET := OCP`，以保持 `TARGET ≤ OCP`。

### 2) CV：`UVLO ≤ TARGET_V`（冻结）

为避免“设置后立即触发欠压锁存”的反直觉情况，冻结以下不变式：

- `UVLO` 不得高于 `TARGET(V)`。
- 当用户编辑 `TARGET(V)` 且将其调到 `< 当前 UVLO`：`TARGET` 被钳制为 `UVLO`（`UVLO` 不变）。
- 当用户编辑 `UVLO` 且将其调到 `> 当前 TARGET`：系统必须自动执行 `TARGET := UVLO`（目标跟随上调；`UVLO` 不回退）。

### 3) OPP 与目标的关系

`OPP` 为运行时限功率线，依赖 `V_main` 推导有效电流上限；因此不对 `TARGET` 建立静态不变式（目标不因 `OPP` 变化而被 UI 静态拒绝），但运行时必须真实限功率。

## 兼容性与迁移

- Preset 数据结构与持久化字段不变：仅 UI 标签、字段显示与编辑联动规则发生变化。
- 网络/HTTP API 与 MCU 间协议字段名保持不变（仍为 `min_v_mv / max_i_ma_total / max_p_mw` 等）。
- 旧 EEPROM 中的 Preset 数据无需迁移：显示时按新标签映射呈现。

## 验收标准（Given/When/Then）

### 显示一致性

- Given：进入 Preset 设置面板，`mode=CC`  
  When：渲染字段  
  Then：按顺序显示 `TARGET(A) / UVLO(V) / OCP(A) / OPP(W)`。

- Given：进入 Preset 设置面板，`mode=CV`  
  When：渲染字段  
  Then：按顺序显示 `TARGET(V) / UVLO(V) / OCP(A) / OPP(W)`。

### A2 联动（CC）

- Given：`mode=CC`，`TARGET=3.000A`，`OCP=5.000A`  
  When：用户将 `OCP` 下调至 `2.500A`  
  Then：`TARGET` 自动变为 `2.500A`，且系统不出现 `TARGET>OCP` 的状态。

- Given：`mode=CC`，`OCP=2.500A`  
  When：用户尝试将 `TARGET` 上调到 `>2.500A`  
  Then：`TARGET` 被钳制为 `2.500A`。

### A2 联动（CV）

- Given：`mode=CV`，`TARGET=5.000V`，`UVLO=1.000V`  
  When：用户将 `UVLO` 上调至 `6.000V`  
  Then：`UVLO=6.000V` 且 `TARGET` 自动变为 `6.000V`（安全线不回退）。

- Given：`mode=CV`，`TARGET=5.000V`，`UVLO=4.500V`  
  When：用户尝试将 `TARGET` 下调到 `4.000V`  
  Then：`TARGET` 被钳制为 `4.500V`（`UVLO` 不变）。

### 欠压锁存（UVLO）

- Given：`output_enabled=true` 且 `UVLO=X>0`  
  When：`V_main ≤ X`  
  Then：触发欠压锁存并退流到 0；仅在用户“关→开”后允许恢复出力。

- Given：`UVLO=0`  
  When：电压跌落  
  Then：不得因为 UVLO 阈值触发欠压锁存（等价禁用）。
