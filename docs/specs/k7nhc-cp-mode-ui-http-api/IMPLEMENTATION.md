# Implementation

## Status

- Current status: 已完成
- Last updated: 2026-01-18
- Origin: migrated from legacy planning docs.

## Implementation Summary

This canonical companion document was initialized from the legacy plan migration. Detailed implementation evidence remains in the migrated spec body and any referenced PR/HIL notes.

## Milestones

No explicit milestones were recorded in the legacy plan.

## Remaining Gaps

## Specification Companion Notes

`SPEC.md` owns the long-lived topic contract. Implementation progress, rollout records, documentation maintenance notes, and prior catalog state live in this companion document.

### Catalog Context
- Prior catalog status: 已完成
- Prior catalog timestamp: 2026-01-18
- Prior catalog origin: legacy planning taxonomy import.

### SPEC Metadata Context
- Spec ID: k7nhc
- Lifecycle: active
- Status: 已完成
- Last: 2026-01-18

### 状态

- Status: 已完成
- Created: 2026-01-15
- Last: 2026-01-18

### 文档更新（Docs to Update）

- `docs/interfaces/network-http-api.md`: 更新 `LoadMode`、`Preset`、`ControlView` 与相关端点描述，补齐 CP 语义与示例。
- `docs/interfaces/network-control.md`: 若该文档仍作为控制接口参考，需要同步加入 CP（至少在“将来可扩展”处升级为明确契约/或链接到本计划 contracts）。
- `docs/specs/README.md`: 新增本计划索引行（已完成）。

### 里程碑（Milestones）

- [x] M1: 冻结 CP 的 “数据模型 + 协议 + HTTP API” 契约（本计划 contracts + 开放问题闭环）
- [x] M2: `loadlynx-protocol` 支持 CP（枚举 + SetMode 字段 + FastStatus mode 映射）并通过单测
- [x] M3: analog：实现 CP 等效电流计算与限值 gating（含基本滤波/限速）
- [x] M4: digital：本机 UI 支持 CP（模式选择 + 目标功率编辑 + 安全关断规则）
- [x] M5: digital：HTTP API 支持 CP（/identity + /presets + /control）并与文档一致
- [x] M6: HIL 验证（不同电压下功率维持、限流/欠压/故障路径；并按本计划的“CP 编程精度/瞬态响应”口径复测）

#### M6：HIL 验证记录

> 说明：本节记录端到端链路验证与观测结果；**验收以本计划的“CP 编程精度 / CP 瞬态响应（内部自测口径，冻结）”为准**。其中编程精度的 `P` 以 `FastStatus.raw.calc_p_mw`（板上 ADC 计算值）为准。
>
> 备注：仍建议后续用示波器/外部仪表复测 `P(t)=V(t)*I(t)` 以对标商用规格，但不作为本计划的当前验收前置条件。

- 设备与链路
  - digital HTTP：`loadlynx-d68638.local`（mDNS），IP `192.168.31.216`，端口 `80`
  - `just agentd-get-port analog`：`0d28:0204:2BDC77EE006DCE9B589D7AD8F22BD989`
  - `just agentd-get-port digital`：`/dev/cu.usbmodem412101`
- 刷写与启动
  - build：`just a-build`、`just d-build`
  - flash：`just agentd flash analog`、`just agentd flash digital`
  - `GET /api/v1/identity`：`capabilities.cp_supported=true`
- CP 写入链路（HTTP → digital → UART → analog）
  - `PUT /api/v1/presets`：`mode="cp"` + `target_p_mw=10000` 写入成功
  - `POST /api/v1/presets/apply`：active preset 切换成功（mode=cp）
  - analog 日志可见 `SetMode received: ... mode=Cp target_p=10000mW ...` 与 ACK 往返
- 不同电压下恒功率达标（已覆盖两档：5V / 12V）
  - 通过 `PUT /api/v1/pd` 切换 PD contract（fixed PDO），再运行 CP=10W：
    - 5V：`object_pos=1`，`i_req_ma=2500`，`v_local_mv≈4.96V`；CP=10W：`avg_p≈9.949W`，`max_err≈0.102W`（PASS）
    - 12V：`object_pos=3`，`i_req_ma=1500`，`v_local_mv≈12.50V`；CP=10W：`avg_p≈10.060W`，`max_err≈0.108W`（PASS）
- 限流路径（CURRENT_LIMITED）
  - CP=10W、12V 下，将 `max_i_ma_total` 限到 `200mA`：`calc_p_mw≈2.6W`，`state_flags_decoded` 含 `CURRENT_LIMITED`（预期）
- 欠压锁存路径（UV_LATCHED）
  - 在输出已开启时，将 preset 的 `min_v_mv` 调高到高于当前 `v_local_mv`（例如 13_000mV）：FastStatus 进入 `UV_LATCHED`，且 `enable=false`（effective output=0）
  - 将 `min_v_mv` 恢复到 `0` 后，通过输出 `OFF → ON`（enable 上升沿）可清除 `UV_LATCHED`
- 瞬态自测（内部 `cp_perf`，用于本计划的验收）
  - 条件：PD `20V/5A`，CP 输出开启，步进 `10W ↔ 90W`；采样：`cp_perf` 以 100us 周期采样 `FastStatus.raw.calc_p_mw`（非示波器 `V(t)`/`I(t)`）
  - 控制环调度抖动（analog 日志 `control_loop dt_us`）：`avg≈100us`（10kHz）
  - 通过标准（内部自测口径）：
    - `t_10_90 <= 1000us` 且 `t_90_10 <= 1000us`（`cp_perf: quick_check pass`）
  - 建议增加多档位回归（减少“只在大步进下通过/只在特定区间通过”的盲区）：
    - 覆盖多个设定值，并包含一次大步进（例如 `90W → 10W`）用于验证大幅下降沿
    - 脚本：`scripts/cp-acceptance.sh`（短 dwell，避免长时间高功率）
  - 注意：为避免长时间高功率运行，建议每次步进验证控制在数十秒内完成，并在高功率段插入 OFF/降功率间隔。
