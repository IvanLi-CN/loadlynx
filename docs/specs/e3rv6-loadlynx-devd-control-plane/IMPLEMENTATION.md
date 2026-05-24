# LoadLynx devd control plane implementation

## Current State

The first implementation is complete and ready for PR review. It adds a local `loadlynx-devd` daemon, `loadlynxctl` CLI, Web devd/Firmware routes, firmware catalog tooling, digital identity/DNS-SD contract updates and mock-first Storybook coverage.

## Design Inputs

- `mains-aegis` `tools/mains-aegis-devd` provides the reference pattern for localhost HTTP/SSE, scan/list/bind/connect, Web USB leases, artifact selection, dry-run flash, reset, monitor, bounded session logs and PSK redaction.
- `mains-aegis` `docs/specs/p8k3d-mains-aegis-devd/SPEC.md` defines the daemon safety model.
- `mains-aegis` `docs/specs/ypfpu-web-management-ui/SPEC.md` defines Web devd, firmware page, USB lease and artifact mismatch behavior.
- LoadLynx `docs/plan/0004:mdns-and-lan-discovery/PLAN.md` already defines ESP32-S3 hostname and browser subnet scan behavior.
- LoadLynx `docs/interfaces/network-http-api.md` and `docs/interfaces/uart-link.md` define the current LAN API and dual-MCU control boundary.

## Implementation Notes

- Do not copy `mains-aegis-devd` without adapting the device model. LoadLynx needs separate digital and analog targets under one logical device.
- Treat `mcu-agentd` as a backend/fallback integration point. Do not change cached selectors from devd or CLI unless the owner explicitly approves the exact selector command.
- Keep Web USB lease TTL short enough to recover from tab crashes while tolerating brief SSE/heartbeat jitter.
- Keep LAN discovery read-oriented until a separate LAN write-control safety design is accepted.
- `tools/loadlynx-devd/` exposes `loadlynx-devd serve` and `loadlynxctl` from one Rust package.
- devd scans native USB serial candidates, cached `.esp32-port`/`.stm32-port` selectors, LAN/mock candidates, but never writes selector cache files.
- Compatibility endpoints require an explicit `device_id`/`lease_id` when there is no unique active lease.
- Firmware flash/reset paths default to dry-run and include target evidence; real operations call `just agentd flash|reset <digital|analog>`.
- Web Storybook coverage uses canvas stories for Devices devd lease creation and Firmware dry-run/session states.

## Verification Plan

- Unit-test daemon and CLI logic without hardware first.
- Use mock devices for Web visual evidence and Storybook coverage.
- Use `dry_run=true` for artifact/target validation before any real flash.
- Run HIL only with approved cached selectors and echo target selection before flash/reset.

## Verification Results

- `cargo test --manifest-path tools/loadlynx-devd/Cargo.toml`
- `bun run check`
- `bun run build`
- `bun run build-storybook --quiet`
- `bun run test:storybook:ci`
- `PROFILE=release just d-build`
- `PROFILE=release just a-build`

`cargo +esp test --manifest-path firmware/digital/Cargo.toml mdns --no-run` is not a valid host-side unit path for this ESP target in the current toolchain; it fails inside xtensa test dependencies before reaching LoadLynx code. The firmware build path above is the accepted validation for the digital crate.
