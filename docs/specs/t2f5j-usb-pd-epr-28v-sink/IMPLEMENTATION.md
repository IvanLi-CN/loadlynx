# Implementation Notes

## Current behavior

- Analog `PD_STATUS.fixed_pdos` remains the only source of truth for real fixed PDO capabilities.
- Digital owner-facing surfaces now treat `fixed_pdos` as a live capability snapshot:
  - `/api/v1/pd`
  - on-device PD settings
  - Web PD/settings summaries
- Detached state, missing `PD_STATUS`, or SPR-only capabilities with `epr_capable=true` no longer inject a synthetic 28V fixed row into owner-facing capability lists.

## Request-path boundary

- The sink request path still keeps the existing EPR fixed 28V helper logic so a persisted 28V target can be interpreted internally during EPR entry.
- Owner-facing UI and HTTP validation no longer use that helper to pretend 28V is present in `fixed_pdos`.
- `PUT /api/v1/pd` accepts only real current capabilities for fixed/PPS selection; missing fixed PDOs now stay on the existing `selected PDO not present in capabilities` error path.

## UI behavior

- Fixed capability lists show only real fixed PDO rows.
- If the saved fixed selection is absent from the current live list, the UI no longer shows a missing 28V summary row; it falls back to “select PDO” / no visible fixed selection and keeps `Apply` disabled until the owner chooses a real row.
- Web summaries only mention `PDO #N` for fixed mode when that PDO is currently visible in the live capability list.

## Specification Companion Notes

`SPEC.md` owns the long-lived topic contract. Implementation progress, rollout records, documentation maintenance notes, and prior catalog state live in this companion document.

### Catalog Context
- Prior catalog status: 部分完成（3/4）
- Prior catalog timestamp: 2026-04-25
- Prior catalog implementation note: PR #72；实现与构建已完成；owner-facing fixed PDO 语义已收敛为 live-only；HIL 仍待 EPR 线材补验

### 状态

- Status: 已完成（软件链路已验，28V 成功合同待补验）
- Created: 2026-03-19
- Last: 2026-04-25
- Notes: PR #72；实现与构建已完成；模拟板启动已恢复；HIL 已观察到 Safe5V -> EnterEprMode；当前 100W 线材返回 `CableNotEprCapable`；主人接受按“软件链路已可用、28V 成功合同待 EPR 线材补验”收口；owner-facing fixed PDO 语义已收敛为 live-only

### 实现前置条件（Definition of Ready / Preconditions）

- 已确认依赖升级目标固定为 `usbpd 2.0.0` 与 `usbpd-traits 2.0.0`。
- 已确认本轮只交付 EPR fixed 28V 的用户路径，AVS 保持只读。
- HIL 需要真实 28V-capable EPR source、合规 5A 线材与现有板卡。

### 文档更新（Docs to Update）

- `docs/interfaces/uart-link.md`
- `docs/interfaces/network-http-api.md`
- `docs/interfaces/main-display-ui.md`

### 实现里程碑（Milestones / Delivery checklist）

- [x] M1: 新 spec / index 落盘，并冻结 EPR fixed 28V 的范围与验收口径。
- [x] M2: 模拟板升级到 `usbpd 2.0.0` 并完成 EPR fixed 28V 协商路径接入。
- [x] M3: 共享协议与数字侧模型/UI/API 支持 28V fixed + EPR 只读状态。
- [x] M4: 构建、测试完成；HIL 已证明软件可进入 EPR mode entry，28V 成功合同待后续 EPR 线材补验。
