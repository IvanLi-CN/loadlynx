# Implementation

## Status

- Current status: 已完成
- Last updated: 2026-01-05
- Origin: migrated from legacy planning docs.

## Implementation Summary

The digital receive path uses UHCI DMA by default, with a polling fallback retained for controlled comparison. The analog receive path uses a ring-buffered DMA receiver for continuous SetPoint traffic. Stable troubleshooting and acceptance requirements remain in `SPEC.md`.

## Milestones

No explicit milestones were recorded in the legacy plan.

## Remaining Gaps

- Re-run bounded HIL whenever UART cadence, DMA buffering, display scheduling, or protocol frame sizes materially change.
- Keep firmware version evidence and bounded logs with each experiment; the existing evidence is historical and is not a substitute for validating a future implementation change.

## Current Implementation Evidence

- Digital display cadence: `DISPLAY_MIN_FRAME_INTERVAL_MS=33`, `DISPLAY_CHUNK_ROWS=16`, and `DISPLAY_CHUNK_YIELD_LOOPS=0` in `firmware/digital/src/main.rs`.
- Digital UART receive: `ENABLE_UART_UHCI_DMA=true`, `UART_DMA_BUF_LEN=1536`, FIFO threshold `120`, timeout `12`, and `uart_link_task_dma` in `firmware/digital/src/main.rs`.
- Protocol framing: `SlipDecoder<FAST_STATUS_SLIP_CAPACITY>` is used by the digital receive path; the shared decoder implementation lives in `libs/protocol/src/lib.rs`.
- Analog SetPoint receive: `SlipDecoder<128>` is fed by the ring-buffered USART3 receive path in `firmware/analog/src/main.rs`.
- Legacy async/polling observations remain historical comparison evidence; they do not define the default runtime path.

## Specification Companion Notes

`SPEC.md` owns the long-lived topic contract. Implementation progress, rollout records, documentation maintenance notes, and prior catalog state live in this companion document.

### Catalog Context
- Prior catalog status: 已完成
- Prior catalog timestamp: 2026-01-05
- Prior catalog origin: legacy planning taxonomy import.

### SPEC Metadata Context
- Lifecycle: active
- Status: 已完成
- Last: 2026-01-05

### 状态

- Status: 已完成
- Created: 2025-11-16
- Last: 2026-01-05
- Source: migrated from `uart-comm-troubleshooting.md` (removed)
