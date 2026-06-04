# LoadLynx devd control plane implementation

## Current State

The control plane is implemented across the local daemon, CLI, Web routes and ESP32-S3 USB JSONL bridge. The current implementation covers devd leases, per-port serial ownership, firmware artifact flows, PD/output control and the expanded compatibility surface for control, presets, calibration, WiFi status, soft reset and diagnostics. User WiFi credentials persist in the digital board EEPROM and the WiFi task reloads those credentials at runtime, falling back to factory `.env` credentials when no user blob is present.

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
- `tools/loadlynx-devd/` exposes `loadlynx-devd serve`, `loadlynx-devd bridge-http`, and `loadlynx` from one Rust package.
- `loadlynx-devd serve` is the IPC daemon for CLI workflows. It binds a native JSONL IPC endpoint, uses an internal dispatcher for the existing API surface, and exits after an idle timeout. macOS/Linux use Unix sockets and Windows uses named pipes. `loadlynx` exposes `--ipc` / `--no-auto-start`, auto-starts a sibling devd when needed, and no longer exposes the old ordinary `--devd http://...` CLI path.
- `loadlynx-devd bridge-http` is the browser/debug bridge and rejects non-loopback binds. Local Web development points `VITE_LOADLYNX_DEVD_URL` at this bridge; skill-driven hardware operations still use CLI IPC.
- Saved USB hardware records from earlier CLI releases that still contain an HTTP devd endpoint continue to work as an upgrade-compatibility path. The CLI detects `http://` and `https://` endpoint values at request time and routes those calls through HTTP instead of treating them as IPC socket or named-pipe paths.
- Unix IPC startup only removes confirmed stale socket files. If another daemon is still listening on the configured socket path, startup fails with an in-use error instead of unlinking the live endpoint.
- devd scans native USB serial candidates, the cached digital `.esp32-port` USB path, LAN/mock candidates, but never writes selector cache files. When `.esp32-port` uses the mcu-agentd selector-record format, devd reads only the path line and ignores metadata lines such as `mac=...`.
- Compatibility endpoints require an explicit `device_id`/`lease_id` when there is no unique active lease.
- devd now manages USB CDC through lease-scoped per-port serial owners. Multiple Web/CLI clients may hold leases for the same device, but JSONL writes are serialized through the owner, request/response matching uses devd-generated unique `request_id` values, serial open/I/O failures are retried on later commands, and flash/reset reserve the port exclusively before invoking `espflash`.
- Serial owner cleanup is generation-checked so an exiting worker cannot unregister a replacement owner, leases retain the port snapshot needed for cleanup, conflict detection and active-owner checks even if the device record is removed, and flash/reset exclusive reservations use a drop guard plus registry-level owner creation checks so cancellation cannot leave a port permanently busy and JSONL owners cannot open during vendor-tool exclusivity. Pre-lease identity rejection also closes any owner opened for the probe.
- Protocol-derived caches such as identity and USB PD accept both legacy fixed request IDs and generated `request_id` prefixes, so unique IDs do not break cache refresh behavior.
- USB PD GET preserves the previous cached PD view when a serial owner command times out or receives only mismatched response IDs. Other serial failures, such as open errors or exclusive reservations, still return explicit errors instead of hiding the failure behind stale cache data.
- The owner-facing CLI no longer exposes `--lease-id`. USB/devd commands create and release leases internally, and long-running `monitor` keeps the lease alive while printing human-readable output by default, with `--format jsonl` for automation.
- CLI hardware memory uses registry schema 2. The registry key is the stable `identity.device_id`, records may contain both USB and HTTP transport locators, `last_transport` records the selected transport, and `default_hardware_id` powers selector-free `loadlynx status`. Legacy flat USB/HTTP records are migrated when read.
- `hardware bind usb --candidate ...` is the only user path from a temporary USB/devd candidate into saved hardware. The bind flow creates a short bind-probe lease, reads identity, and rejects firmware whose `identity.device_id` is missing or is not a stable `loadlynx-<short-id>` ID. Saved USB operations on a fresh devd retry `device_not_found` by scanning before creating an identity-confirming lease.
- `status --device` and other direct temporary candidate operations are blocked with a bind-first error. `status --url` remains an explicit one-off LAN read path, but it no longer silently creates or updates saved hardware.
- CLI firmware flash/reset dry-runs do not create leases or touch USB serial state; real flash/reset operations create an internal lease before calling devd.
- Lease creation validates target port availability before marking a device connected, so rejected lease attempts do not leave misleading connected state. For real non-mock USB CDC ports, lease creation requires the selected port to match the approved default digital USB port memory; missing or unreadable `.esp32-port` state returns `target_selector_not_cached` rather than probing hardware.
- A bind-probe lease is exempt from the project `.esp32-port` cache because the selected candidate and stable identity confirmation are the binding evidence. Ordinary saved USB operations pass the expected hardware ID to devd so the daemon rejects identity drift before granting the lease.
- The devd USB compatibility surface includes PD sink reads and writes. Digital firmware exposes `get_pd` and `set_pd_policy`; devd maps them to `/api/v1/pd` and caches the last complete PD view from protocol frames so intermittent USB log noise cannot make Web lose a valid real-device PD snapshot.
- The devd USB compatibility surface now includes `/api/v1/control`, `/api/v1/presets`, `/api/v1/calibration/*`, `/api/v1/wifi`, `/api/v1/soft-reset` and diagnostics export. devd maps those HTTP-compatible calls onto compact USB JSONL ops and requires the same lease/selection rules as PD and CC writes.
- USB calibration profile reads use a 4096-byte JSONL output frame and a compact `cal_profile_v1` firmware payload for curve arrays. devd expands that compact payload to the HTTP/Web profile shape before returning it to callers, so valid 24-point curves do not fail as oversized single-frame responses.
- Sensitive diagnostic/trace payloads use recursive redaction for `psk`, `password`, `passphrase`, `secret` and `token` fields instead of relying on one frame shape.
- Digital firmware stores user WiFi credentials in a dedicated EEPROM blob. `set_wifi_config` writes the blob and may wait for connection state, `clear_wifi_config` invalidates it, and the WiFi task reconnects when the EEPROM credential source changes.
- CLI USB workflows cover PD policy writes and output control without exposing lease IDs. `output set` requires exactly one of `--enable` or `--disable`; `--target-i-ma` applies the active firmware preset as CC mode before enabling output.
- CLI business workflows cover WiFi show/set/clear, control get/set, preset list/set/apply, calibration profile/mode/apply/commit/reset, soft-reset and diagnostics export. Control writes require explicit `--enable` or `--disable`. LAN WiFi writes require `--allow-insecure-lan-wifi`; USB/devd writes create short-lived leases internally. CLI output is human-readable by default, with `--json` preserving structured automation output.
- Web Settings exposes devd-backed WiFi status/config and diagnostics export. LAN WiFi writes require a confirmation dialog; USB/devd writes proceed as local physical-access operations.
- ESP32-S3 USB Serial/JTAG can interleave binary logs with JSONL response text. devd keeps an enlarged per-command non-protocol buffer, prefers complete matching `request_id` responses, and uses operation-scoped recovery for identity, status, output-control, control, presets, calibration profile and WiFi status responses. Recovery is limited to frames/text observed in the matching command window and shaped like the requested operation; unrelated response IDs or stale monitor frames are never success.
- Firmware flash/reset paths default to dry-run and include target evidence. Real ESP32-S3 digital flash calls direct `espflash` through devd after artifact hash verification and a valid Web lease: ELF artifacts use `espflash flash`, and raw image artifacts require `flash_address` before using `espflash write-bin`. Real digital flash also requires explicit `yes` confirmation, non-project firmware acknowledgement when applicable, and post-flash identity capture before success is reported. Analog flash/reset and reset-only paths continue to use existing backend guardrails.
- Web Serial is implemented as a formal browser path for GitHub Pages and release Web bundles. The Firmware route accepts release catalog JSON and a matching firmware file, verifies SHA-256 in-browser, gates real ESP32-S3 flash with the same phrase/risk/identity requirements, flashes through `esptool-js`, and stores only identity/profile metadata instead of OS port paths.
- Release packaging includes host-tools installers, platform host-tools archives, firmware artifacts, a firmware catalog, the Web bundle, and `SHA256SUMS` covering all release assets. The installers verify `SHA256SUMS`, install to user-owned directories, validate `loadlynx` and `loadlynx-devd`, and only print PATH guidance.
- Web Storybook coverage uses canvas stories for Devices devd lease creation and Firmware dry-run/session states.

## Verification Plan

- Unit-test daemon and CLI logic without hardware first.
- Use mock devices for Web visual evidence and Storybook coverage.
- Use `dry_run=true` for artifact/target validation before any real flash.
- Run HIL only with approved cached selectors and echo target selection before flash/reset.

## Verification Results

- `cargo check` in `tools/loadlynx-devd`
- `cargo test` in `tools/loadlynx-devd`
- `bun run check` in `web`
- `bun run build` in `web`
- `tools/loadlynx-devd/install/install-loadlynx-host.sh --dry-run`
- `bun run build-storybook --quiet` in `web`
- Storybook mock screenshot for `Routes/Settings` with PSK leak assertion

Digital firmware `cargo check` reached the firmware build script and stopped before Rust compilation because the current worktree has no repo-root `.env` with `DIGITAL_WIFI_SSID` and `DIGITAL_WIFI_PSK`.
- `PROFILE=release just a-build`
- Real ESP32-S3 digital flash through devd direct `espflash` on `/dev/cu.usbmodem212101`; post-flash USB CDC `get_identity` matched the local digital firmware version.
- Real USB PD sink verification through devd `/api/v1/pd`: read attached 9V/500mA contract and PDO/APDO capabilities, applied fixed 5V/500mA, observed contract transition to 5V, restored fixed 9V/500mA, and observed contract return to 9V while load output stayed disabled.
- Real CLI/devd control verification on `/dev/cu.usbmodem212101`: flashed the current ESP32-S3 digital artifact through `loadlynx flash digital`, set fixed PD to 12V/2A through `loadlynx pd set`, enabled a 2A CC load through `loadlynx output set --target-i-ma 2000 --enable`, verified CLI monitor could run concurrently with output control, then stopped the load with `loadlynx output set --disable` and confirmed `status` reported `output_enabled=false`, `enable=false` and near-zero measured current.
- Real CLI/devd completion HIL on `/dev/cu.usbmodem212101`: after direct devd flash, `status` reported `link_up=true`, `hello_seen=true`, `analog_state=ready` and output disabled; `control get` recovered the active CP preset; `calibration profile` expanded the user-calibrated compact profile; `wifi show` recovered `state=connected` and IP `192.168.31.216`; `diagnostics export` returned redacted firmware diagnostics with `link_up=true`. `preset list` treats partial recovered lists as incomplete, retries and merges recovered preset fragments, uses a real `get_control` preset response to fill the active-preset gap when USB log noise drops that preset from the list response, and returns an explicit retryable incomplete-response error if the full five-preset set cannot be proven.
- Final real-device CLI/devd completion HIL on `/dev/cu.usbmodem212101` against `digital 0.1.0 (profile release, v0.1.1-6-g3fd3751, src 0xa4147fae8fda149b)`: `preset list` returned all five presets M1-M5, `status` reported `link_up=true`, `hello_seen=true`, `analog_state=ready` and output disabled, `control get` returned active preset M1, `calibration profile` returned the expanded user-calibrated profile, `wifi show` returned `state=connected` and IP `192.168.31.216`, and `diagnostics export` returned redacted firmware diagnostics with `log_decode.status=verified`.
- Current verification: `cargo test --manifest-path tools/loadlynx-devd/Cargo.toml` passed, `just d-build` passed for digital release firmware, and formatting was run for `tools/loadlynx-devd` plus `firmware/digital`.
- Current registry v2 verification: `cargo test --manifest-path tools/loadlynx-devd/Cargo.toml --locked` passed and covers hardware bind/default/use command parsing, schema migration, bind-probe leases, JSON error classification and fresh devd scan-before-lease retry. Digital firmware was also built with `cargo +esp build --release --no-default-features` to prove USB identity does not depend on the `net_http`/mDNS module.
- Review convergence verification: `cargo fmt --manifest-path tools/loadlynx-devd/Cargo.toml --all`, `cargo test --manifest-path tools/loadlynx-devd/Cargo.toml --locked`, `git diff --check`, and `tools/loadlynx-devd/install/install-loadlynx-host.sh --dry-run` passed. The devd test suite includes regression cases proving legacy HTTP endpoint strings are still accepted by the CLI request path, analog real flashes do not prompt for digital confirmation, and Unix IPC startup refuses live sockets while removing stale sockets.

`cargo +esp test --manifest-path firmware/digital/Cargo.toml mdns --no-run` is not a valid host-side unit path for this ESP target in the current toolchain; it fails inside xtensa test dependencies before reaching LoadLynx code. The firmware build path above is the accepted validation for the digital crate.
