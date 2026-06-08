# Implementation Notes

## Current behavior

- Digital production telemetry starts from `UiSnapshot::offline()` instead of `UiSnapshot::demo()`.
- Demo telemetry remains available for tests and mock rendering, but real firmware no longer shows demo voltage/current/power before the first UART frame.
- Digital SetMode TX owns boot-link recovery because it already serializes SoftReset, CalWrite, SetEnable, LimitProfile and SetMode writes onto UHCI TX.
- Cold-start recovery covers both:
  - `never-seen-frame`: `LAST_GOOD_FRAME_MS == 0` after boot grace;
  - `stale-link-down`: at least one frame was seen, but `LINK_UP=false` persists past stale grace.
- Cold-start measurement qualification is separate from link qualification. A new analog `HELLO` resets measurement trust; the digital side only clears the `MEAS` state after a `FastStatus` contains a non-zero voltage, current or power signal.
- If frames are present but `FastStatus` never arrives, or measurement remains all-zero past the measurement grace window, SetMode TX runs the same serialized recovery handshake with reason `no-fast-status` or `zero-measurement`.
- While measurement is not trusted, the Dashboard shows unavailable readouts rather than treating `0V/0A/0W` as real telemetry.
- Analog FastStatus reporting preserves the uncalibrated non-zero reading when an active calibration curve would otherwise collapse a non-zero raw sample to exactly zero; this affects reported telemetry only and does not change protection/control calculations.
- Each recovery attempt is rate-limited and only runs when there is no pending SetMode or PD request ACK.
- Recovery sends SoftReset, waits a quiet gap, sends all calibration curves, sends SetEnable(true), sends LimitProfile, then forces the next SetMode snapshot.
- SoftReset ACK matching uses `SOFT_RESET_ACK_TOTAL` plus `SOFT_RESET_LAST_ACK_SEQ`; a stale ACK boolean can no longer short-circuit a new handshake.

## Safety boundaries

- Output-on SetMode commands still obey existing link/fault/offline gates.
- The measurement qualification path does not restore prior output state and does not alter analog-side safety gates.
- The recovery watchdog does not change UART pins, baud rate, protocol message IDs, CBOR payloads, or HTTP API shape.
- Hardware selector caches are not modified by this implementation.

## Validation notes

- `cargo test --manifest-path libs/protocol/Cargo.toml` passed.
- `cargo test --manifest-path libs/calibration-format/Cargo.toml` passed.
- `cargo fmt --manifest-path firmware/digital/Cargo.toml -- --check` passed.
- `just d-build` passed after creating a temporary ignored `.env` from `.env.example` for compile-time Wi-Fi variables; the placeholder `.env` was removed afterwards and was not committed.
- HIL flash/monitor was not run because both `digital` and `analog` selectors are missing in this worktree, and `mcu-agentd config validate` also reported the analog artifact missing before analog build.
- `just a-build` was attempted for preflight but is blocked by missing `third_party/embassy/embassy-embedded-hal` in this worktree; analog firmware was not modified.

## Specification Companion Notes

`SPEC.md` owns the long-lived topic contract. Implementation progress, rollout records, documentation maintenance notes, and prior catalog state live in this companion document.

### Catalog Context
- Prior catalog status: 已完成
- Prior catalog timestamp: 2026-04-25
- Prior catalog implementation note: 软件实现与构建验证完成；HIL 因当前 worktree 缺失 selector 阻断

### 状态

- Status: 已完成（冷上电测量可信性补强）
- Created: 2026-04-25
- Last: 2026-05-01
- Notes: 冷启动 recovery 进一步覆盖“有帧但测量仍为 0”的假正常状态。

### 实现前置条件（Definition of Ready / Preconditions）

- 已锁定快车道 `merge-ready`；HIL 仅允许使用当前缓存/已确认 selector。

### 文档更新（Docs to Update）

- `docs/dev-notes/software.md`
- `docs/specs/README.md`
- 如形成可复用经验，新增或刷新 `docs/solutions/firmware/**`。

### 实现里程碑（Milestones）

- [x] M1: 生产 UI 初始状态改为 offline/unknown，demo 仅保留给 mock/test。
- [x] M2: SoftReset ACK 改为本次 seq/baseline 绑定。
- [x] M3: SetMode TX 增加冷启动/持续 down 的限频恢复握手。
- [x] M4: 构建与可用 HIL 验证完成，文档同步（HIL selector 缺失，记录为阻断证据）。
- [x] M5: 增加测量可信性判定，覆盖有帧但全零测量的冷上电假正常状态。
