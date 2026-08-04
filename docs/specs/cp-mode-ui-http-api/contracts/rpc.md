# RPC / Protocol（digital ↔ analog）

本文件描述本计划对 `loadlynx-protocol` 的增量契约，用于保证 digital/analog 在实现阶段有一致口径。

## `LoadMode`（SetMode.mode / FastStatus.mode）

- 范围（Scope）: internal
- 变更（Change）: Modify

### 数值映射（u8）

- `1` → `CC`
- `2` → `CV`
- `3` → `CP`（新增）

### 兼容性与迁移（Compatibility / migration）

- `LoadMode` 作为 u8 传输：未知值必须按 “保留/Reserved” 处理，避免解码失败导致链路异常。

## `MSG_SET_MODE` / `SetMode` payload

- 范围（Scope）: internal
- 变更（Change）: Modify

### 变化点（旧 → 新）

- 旧：`SetMode` 仅支持 `CC/CV`，且无 `target_p_mw`。
- 新：`SetMode` 支持 `CP` 模式，并增加字段 `target_p_mw`（mW）。

### Schema（CBOR map keys）

在现有 `SetMode` 的基础上新增：

- `target_p_mw`：`u32`，CBOR key `8`（固定为 `8`，单位 mW）。

CP 模式下的语义：

- `mode=CP` 时，以 `target_p_mw` 作为目标功率；`target_i_ma`/`target_v_mv` 保留但不参与 CP 的目标（可用于未来扩展或保持字段稳定）。
- `max_p_mw` 仍表示功率上限（OPP），并要求 `target_p_mw <= max_p_mw`。

### 兼容性与迁移（Compatibility / migration）

- CBOR map 增量字段对解码的影响：
  - 新固件可解码旧 payload（缺省 `target_p_mw` 视为 `0` 或按实现默认）。
  - 旧固件遇到未知 key 时应忽略（若现状无法忽略，需要在实现中优先修正解码策略）。
