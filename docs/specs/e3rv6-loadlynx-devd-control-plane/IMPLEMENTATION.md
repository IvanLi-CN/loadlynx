# LoadLynx devd control plane implementation

## Current State

The first implementation is complete and ready for PR review. It adds a local `loadlynx-devd` daemon, `loadlynx` CLI, Web devd/Firmware routes, firmware catalog tooling, digital identity/DNS-SD contract updates and mock-first Storybook coverage.

## Design Inputs

- `mains-aegis` `tools/mains-aegis-devd` provides the reference pattern for localhost HTTP/SSE, scan/list/bind/connect, Web USB leases, artifact selection, dry-run flash, reset, monitor, bounded session logs and PSK redaction.
- `mains-aegis` `docs/specs/p8k3d-mains-aegis-devd/SPEC.md` defines the daemon safety model.
- `mains-aegis` `docs/specs/ypfpu-web-management-ui/SPEC.md` defines Web devd, firmware page, USB lease and artifact mismatch behavior.
- LoadLynx `docs/specs/yy7th-mdns-and-lan-discovery/SPEC.md` already defines ESP32-S3 hostname and browser subnet scan behavior.
- LoadLynx `docs/interfaces/network-http-api.md` and `docs/interfaces/uart-link.md` define the current LAN API and dual-MCU control boundary.

## Implementation Notes

- Do not copy `mains-aegis-devd` without adapting the device model. LoadLynx needs separate digital and analog targets under one logical device.
- Treat `mcu-agentd` as a backend/fallback integration point for non-devd firmware workflows and analog/probe operations. Devd/Web ESP32-S3 digital firmware flashing uses devd's lease-gated direct `espflash` path with the approved `.esp32-port` target.
- Keep Web USB lease TTL short enough to recover from tab crashes while tolerating brief SSE/heartbeat jitter.
- Keep LAN discovery read-oriented until a separate LAN write-control safety design is accepted.
- `tools/loadlynx-devd/` exposes `loadlynx-devd serve` and `loadlynx` from one Rust package.
- devd scans native USB serial candidates, the cached digital `.esp32-port` USB path, LAN/mock candidates, but never writes selector cache files. When `.esp32-port` uses the mcu-agentd selector-record format, devd reads only the path line and ignores metadata lines such as `mac=...`.
- Compatibility endpoints require an explicit `device_id`/`lease_id` when there is no unique active lease.
- devd now manages USB CDC through lease-scoped per-port serial owners. Multiple Web/CLI clients may hold leases for the same device, but JSONL writes are serialized through the owner, request/response matching uses devd-generated unique `request_id` values, serial open/I/O failures are retried on later commands, and flash/reset reserve the port exclusively before invoking `espflash`.
- Serial owner cleanup is generation-checked so an exiting worker cannot unregister a replacement owner, leases retain the port snapshot needed for cleanup and active-owner checks even if the device record is removed, and flash/reset exclusive reservations use a drop guard so cancellation cannot leave a port permanently busy.
- Protocol-derived caches such as identity and USB PD accept both legacy fixed request IDs and generated `request_id` prefixes, so unique IDs do not break cache refresh behavior.
- The owner-facing CLI no longer exposes `--lease-id`. USB/devd commands create and release leases internally; long-running `monitor` keeps the lease alive and prints human-readable output by default, with `--format jsonl` for automation.
- CLI firmware flash/reset dry-runs do not create leases or touch USB serial state; real flash/reset operations create an internal lease before calling devd.
- Lease creation validates target port availability before marking a device connected, so rejected lease attempts do not leave misleading connected state.
- The devd USB compatibility surface includes PD sink reads and writes. Digital firmware exposes `get_pd` and `set_pd_policy`; devd maps them to `/api/v1/pd` and caches the last complete PD view from protocol frames so intermittent USB log noise cannot make Web lose a valid real-device PD snapshot.
- Firmware flash/reset paths default to dry-run and include target evidence. Real ESP32-S3 digital flash calls direct `espflash` through devd after artifact hash verification and a valid Web lease: ELF artifacts use `espflash flash`, and raw image artifacts require `flash_address` before using `espflash write-bin`. Analog flash/reset and reset-only paths continue to use existing backend guardrails.
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
- Real ESP32-S3 digital flash through devd direct `espflash` on `/dev/cu.usbmodem212101`; post-flash USB CDC `get_identity` matched the local digital firmware version.
- Real USB PD sink verification through devd `/api/v1/pd`: read attached 9V/500mA contract and PDO/APDO capabilities, applied fixed 5V/500mA, observed contract transition to 5V, restored fixed 9V/500mA, and observed contract return to 9V while load output stayed disabled.

`cargo +esp test --manifest-path firmware/digital/Cargo.toml mdns --no-run` is not a valid host-side unit path for this ESP target in the current toolchain; it fails inside xtensa test dependencies before reaching LoadLynx code. The firmware build path above is the accepted validation for the digital crate.
