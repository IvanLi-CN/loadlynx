# LoadLynx Product Context

## Register

product

## Product Purpose

LoadLynx is a portable electronic load built around a split MCU architecture. The STM32G431 handles fast control loops and protection; the ESP32-S3 hosts the local UI, network bridge, USB/devd bridge, OTA and diagnostics. The Web Console is the operator-facing control and diagnostic surface for setup, monitoring, calibration, USB-PD configuration, firmware operations and simulated device workflows.

## Users

- Hardware and firmware developers bringing up the dual-MCU system on a desk with a laptop, USB serial access and test equipment nearby.
- Power electronics users running controlled load tests, checking thermal and protection status, and adjusting presets during bench work.
- Maintainers validating releases through mock devices, Storybook, Playwright and devd workflows before touching real hardware.

## Usage Context

The console is used beside real hardware under focused bench conditions: a developer or operator glances between laptop, device screen, PSU, meter and logs. The UI should feel like a high-trust instrument console, not a marketing dashboard.

## Strategic Principles

- Safety and clarity outrank decoration. Load state, protection, link health and write operations must be impossible to miss.
- Technical terms may remain English when translation would reduce precision: CC, CV, CP, USB-PD, PPS, PDO, APDO, Firmware, dry-run, lease, devd, UART.
- Mock-first review is part of the product. Storybook and simulation devices must expose believable states without requiring hardware.
- Dense information is acceptable when it helps bench work. The hierarchy must separate readback, setpoint, device state and destructive operations.
- Mobile support means complete task access on small screens, not feature reduction.

## Tone

Precise, instrument-like, calm under fault conditions. Chinese is the default UI language, with English available through i18n. Copy should be compact, direct and action-oriented.

## Anti-References

- Generic SaaS admin pages with pale cards and weak hierarchy.
- Decorative cyberpunk that makes forms, tables or safety states harder to read.
- Modal-first flows for routine operations.
- UI libraries imposing a generic look over the device-console identity.
