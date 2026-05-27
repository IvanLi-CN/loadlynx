# History

- 2026-04-25: Created after observing a live-FPS Dashboard with frozen demo-like telemetry after full-system power-on; reset of either MCU recovers link, pointing to cold-start MCU UART recovery rather than display task freeze.
- 2026-04-25: Implemented digital-side offline production snapshot, seq/baseline SoftReset ACK matching, and a rate-limited SetMode TX boot-link recovery watchdog.
- 2026-05-01: Extended recovery to treat all-zero cold-start measurement as untrusted until a non-zero FastStatus signal arrives, and preserved non-zero analog raw telemetry from being reported as exactly zero after calibration mapping.
