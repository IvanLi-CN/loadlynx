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
