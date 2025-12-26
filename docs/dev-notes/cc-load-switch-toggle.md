# CC 负载开关（Load Switch）：设置值 / 生效值分离

本文档定义数字板（ESP32‑S3）“负载开关”交互的预期行为：将“设置值（setpoint）”与“生效值（effective）”分离，并让旋钮单击（按下）作为负载开关使用；同时同步 Web Console 与 HTTP API（`/api/v1/cc`）的语义，确保本地与远程控制一致。

## 1. 背景与动机

当前数字板旋钮单击行为为“清零”，在需要反复进行负载开/关测试时不够高效：用户希望通过一次单击在 `0` 与“当前设置值”之间快速切换，从而更便捷地验证电流环、功耗与保护逻辑。

## 2. 目标

- 将旋钮单击从“清零”改为“负载开关（ON/OFF）”。
- 分离“设置值”与“生效值”：
  - 设置值用于 UI 展示与用户意图；
  - 生效值用于实际下发到模拟板的控制量（`SetPoint.target_i_ma`）。
- 默认状态安全：上电后负载开关为 `OFF`，且当设置值变为 `0` 时强制 `OFF`。
- Web Console 与 HTTP API 采用与本地旋钮一致的语义与状态模型。

## 3. 范围与非目标

### 3.1 范围（In scope）

- 数字板固件：
  - 旋钮交互逻辑（旋转/单击）。
  - `SetPoint` 下发目标值计算（使用生效值）。
  - `/api/v1/cc` 视图与更新语义调整。
  - 身份能力字段中的 `api_version`（升级到 `2.0.0` 以反映语义变更）。
- Web Console：
  - CC 控制页的 enable/target 语义与文案对齐新模型。
  - `CcControlView` / `CcUpdateRequest` 类型与接口消费逻辑调整（必要时新增字段）。

### 3.2 非目标（Out of scope）

- 不改动模拟板（STM32G431）固件的控制闭环与协议。
- 不使用协议层的 `SetEnable` 来实现本次“负载开关”（避免触碰硬件供电/驱动链路）。
- 不保留旧的“旋钮单击清零”行为，也不新增长按/双击等替代动作。
- 不引入新的工作模式（CV/CP/CR 等）或新的保护策略；限值与保护字段保持现状。

## 4. 统一控制模型（核心定义）

在数字板固件与 HTTP API 中引入一致的控制状态：

- `set_i_ma`（设置值 / setpoint）：用户意图的目标电流（mA）。
- `load_enabled`（负载开关 / load switch）：是否将设置值应用到输出（默认 `false`）。
- `effective_i_ma`（生效值 / effective）：实际下发给模拟板的目标电流（mA）：

  - `effective_i_ma = load_enabled ? set_i_ma : 0`
  - 并在下发前进行范围夹紧（clamp），与现有 `TARGET_I_MIN_MA..TARGET_I_MAX_MA` 一致。

### 4.1 强制安全规则（A 规则）

当 `set_i_ma` 被更新为 `0`（无论来自旋钮还是 HTTP）时，必须执行：

- `load_enabled = false`

目的：确保从 `0` 旋转到非零不会“意外上负载”，必须显式开启负载开关。

## 5. 本地旋钮交互（数字板 UI）

### 5.1 旋钮旋转

- 仅更新 `set_i_ma`（不自动改变 `load_enabled`）。
- 若旋转使 `set_i_ma` 变为 `0`，触发 “A 规则” 强制 `load_enabled=false`。
- UI 展示（屏幕 `SET I`）始终显示设置值 `set_i_ma`，不额外标记 ON/OFF，不显示生效值。

### 5.2 旋钮单击（按下）

- 若 `set_i_ma == 0`：不触发任何动作。
- 若 `set_i_ma > 0`：切换 `load_enabled`（`OFF ↔ ON`）。
  - `OFF`：生效值变为 `0`
  - `ON`：生效值恢复为 `set_i_ma`

## 6. 模拟板下发（协议与链路约束）

- 本次“负载开关”仅影响数字板计算出的 `effective_i_ma`，并通过现有 `SetPoint { target_i_ma }` 下发。
- 不使用 `SetEnable` 来实现开关（不触碰硬件驱动电源/供电链路）。

## 7. HTTP API：`/api/v1/cc` 语义变更

目标：远程控制与本地旋钮语义一致。

### 7.1 `GET /api/v1/cc`（控制视图）

返回字段语义调整为：

- `enable`：映射为 `load_enabled`（负载开关）
- `target_i_ma`：映射为 `set_i_ma`（设置值）
- `effective_i_ma`：新增字段，返回生效值（`enable ? target_i_ma : 0`）

示例：

```json
{
  "enable": false,
  "target_i_ma": 1500,
  "effective_i_ma": 0
}
```

> 其余 `limit_profile`、`protection`、派生测量字段（`i_total_ma`/`v_main_mv`/`p_main_mw`）保持原结构不变。

### 7.2 `PUT/POST /api/v1/cc`（更新控制）

写入字段语义调整为：

- `enable`：更新 `load_enabled`
- `target_i_ma`：更新 `set_i_ma`
- 若 `target_i_ma == 0`：强制 `enable=false`（A 规则）
- 若请求包含 `enable=true` 且 `target_i_ma==0`：服务器应将其纠正为 `enable=false` 并返回更新后的视图（不建议报错）。

返回更新后的完整视图（同 `GET /api/v1/cc`），包含 `effective_i_ma`。

### 7.3 兼容性：`api_version=2.0.0`（必需）

本改动保持字段名不变（仍为 `enable`/`target_i_ma`），但语义发生破坏性变化（从“生效控制”变为“负载开关 + 设置值”模型），因此必须将 `GET /api/v1/identity` 的 `capabilities.api_version` 升级为 **`2.0.0`**，以便客户端按版本适配。

## 8. Web Console：CC 控制页预期

- “Enable output” 的语义改为“负载开关”（控制 `enable/load_enabled`）。
- “Target current” 滑条/输入改为“设置值”（控制 `target_i_ma/set_i_ma`），即使负载开关 `OFF` 也应可编辑。
- 页面主要展示与硬件屏一致：`SET I` 展示设置值，不额外标记 ON/OFF；负载是否开启由开关本身体现。
- 若需要更强的可观测性，可使用 API 提供的 `effective_i_ma` 在调试信息区域显示（可选，不作为硬性要求）。

## 9. 兼容性与迁移

- 旧语义：`enable=false` 等价于把目标电流直接写成 `0`（会丢失用户设置）。
- 新语义：`enable=false` 仅关闭负载，不修改 `target_i_ma`（仍保留设置值）。
- Web Console 与任何外部客户端需要按新语义更新：
  - 不要再通过“写 `target_i_ma=0` 来实现关闭且覆盖设置”的假设；
  - 使用 `enable` 作为负载开关，使用 `target_i_ma` 作为设置值，读取 `effective_i_ma` 获取生效输出目标。

## 10. 测试计划（实现阶段）

### 10.1 数字板本地交互

- 上电：默认 `enable=false`，`effective_i_ma=0`。
- 旋转到 `target_i_ma>0`：保持 `enable=false`，输出仍为 0。
- 单击：`enable` 在 `OFF↔ON` 切换，输出在 `0↔target` 切换。
- 旋转回 `target_i_ma=0`：强制 `enable=false`（A 规则）。

### 10.2 HTTP 与 Web

- `PUT /api/v1/cc` 设置 `target_i_ma=1500, enable=false`：回读应保留设置值且 `effective_i_ma=0`。
- 再写 `enable=true`（保持 `target_i_ma=1500`）：`effective_i_ma=1500`。
- 写入 `target_i_ma=0`：回读应显示 `enable=false`。

## 11. 风险与注意事项

- 并发一致性：旋钮任务与 HTTP handler 可能同时更新 `enable/target`，需要确保状态更新原子化或具备明确优先级策略（至少保证 A 规则不被破坏）。
- API 语义变更：属于破坏性变更，需要通过 `api_version` 或文档明确提示，避免旧客户端误用。

## 12. 并发与仲裁（建议实现策略）

为避免“设置值/开关”在本地与远程交互中出现短暂不一致，建议采用以下规则：

- 单一事实来源：数字板内部维护一份 `{enable, target_i_ma}` 控制状态；`effective_i_ma` 永远由二者计算派生，不作为可写状态。
- 原子更新：无论来自旋钮还是 HTTP，请在一次更新中同时完成：
  - 写入 `target_i_ma`（若请求/旋钮涉及）；
  - 写入 `enable`（若请求/按键涉及）；
  - 应用 A 规则（`target_i_ma==0 → enable=false`）。
- 冲突策略：字段级“最后写入生效”即可，但 A 规则具有最高优先级（任何路径把 `target_i_ma` 设为 0 都必须关断 enable）。
