# LoadLynx devd control plane history

## Initial Design

The specification was created after reviewing `mains-aegis` devd, Web management UI specs and LoadLynx's existing mDNS/LAN discovery plan. The main design decision is to reuse the local daemon + explicit lease model while changing the domain model from a single ESP32 UPS to a dual-target LoadLynx device with separate ESP32-S3 and STM32G431 flash/control paths.

## First Implementation

The first implementation keeps the daemon and CLI in one Rust crate with two binaries. Web support starts from the existing Devices route and adds a Firmware route instead of introducing a separate fleet app shell. Firmware changes are limited to stable identity/DNS-SD contract fields and build metadata; the analog board remains represented by artifact/probe provenance until a direct identity read path is designed.

## Serial Owner Concurrency

The USB CDC bridge moved from per-request serial opens to lease-scoped per-port ownership. This keeps devd as the only process inside the control plane that touches the serial port, lets multiple CLI/Web clients share the same device through internal leases, and separates authorization from physical access ordering. Flash/reset remain exclusive because vendor tools need the OS port directly, so devd closes the serial owner before invoking them and rejects same-port JSONL work while the exclusive operation is active.

Review convergence kept two safety-compatible legacy behaviors intact: PD GET falls back to the cached PD view only for missing or mismatched serial responses, and real USB lease creation refuses to touch hardware unless the selected port matches the approved `.esp32-port` default.

Real CLI HIL verification exposed that owner-facing commands must cover the full safe-control lifecycle, not just enable. The CLI now includes PD writes, CC/CV/CP targets and explicit output disable, while devd handles USB Serial/JTAG log-noise fragmentation without treating stale or mismatched request IDs as success.

## Control Plane Completion

The bridge contract was expanded from identity/status/PD/output operations to the full owner-facing control surface: control, presets, calibration, WiFi status/config requests, soft reset and diagnostics. The accepted model keeps USB as a compact dedicated protocol while devd maps Web/CLI/LAN-compatible HTTP calls to firmware ops. Firmware remains the final authority for validation and safety, and devd only prevalidates selection, lease ownership, request matching and sensitive-field redaction.

Calibration profile reads use a compact firmware response instead of the verbose HTTP shape on USB. This keeps full 24-point curve profiles inside the single JSONL frame budget while preserving the public HTTP/Web shape through devd expansion.

Lease semantics were clarified: multiple clients may hold leases for the same device/port, ordinary JSONL writes are queued through the per-port owner, and flash/reset are exclusive windows that fail same-port JSONL work fast instead of letting it wait behind long vendor-tool operations.

Real completion HIL showed that ESP32-S3 USB Serial/JTAG log noise can corrupt not only status/output-control responses but also identity, control, presets, compact calibration profile and WiFi status responses. The implementation now keeps a larger non-protocol buffer and performs operation-scoped recovery from post-transmit frames or text fragments. Diagnostics already returned a complete matched response. Preset list recovery requires the full five-preset set: devd retries and merges recovered fragments, and reports an incomplete retryable response instead of returning a partial list as success.

Final preset-list HIL showed a repeatable USB Serial/JTAG failure mode where the list response yielded M2-M5 as standalone preset fragments while M1 was dropped by log interleaving. The daemon now keeps the completeness gate, accepts only real preset-shaped fragments, and may use a real `get_control` preset response with bounded retries to fill the active preset. This preserves correctness because partial lists remain errors unless the daemon proves all five preset records from device responses.

Saved-device USB realtime reads exposed a narrower regression later: `loadlynx status --device ... --json` and `loadlynx control get --device ... --json` could still hit the full host-side operation timeout even when device discovery and LAN status were healthy. The accepted fix kept the owner-facing CLI unchanged and tightened both ends of the compat read path: devd now treats serial response gaps for those reads as bounded retries with operation-scoped recovery, and firmware keeps `get_status` / `get_control` on compact USB response shapes instead of depending on the broader HTTP body renderer.

## IPC-first host tools and Web Serial release path

The host tools boundary changed to make released CLI/devd safer for ordinary users. `loadlynx-devd serve` is now an IPC daemon used by the CLI, while `loadlynx-devd bridge-http` is the loopback-only browser/debug bridge. This is a minor breaking change because ordinary CLI workflows no longer expose the legacy daemon-URL flag; the CLI uses `--ipc` and can auto-start a sibling devd process.

Web Serial moved from follow-up idea to official browser path for GitHub Pages and release Web bundles. The browser flash path uses release firmware catalog/assets, in-browser SHA-256 verification, typed confirmation, optional identity match, non-project firmware acknowledgement and post-flash identity capture. Identity/profile memory is allowed, but OS serial port paths are not persisted.

## Binding-first device registry

Real CLI usage showed that a saved device ID could still depend on a devd process's temporary device table. The CLI registry now keys devices by stable firmware `identity.device_id`, stores USB and HTTP transport locators under that device, remembers `last_transport`, and exposes a global default for selector-free automation. Temporary USB candidate IDs are discovery outputs only; they must be bound before use, and saved USB operations confirm runtime identity before control.

The digital firmware USB identity now derives the same `loadlynx-<short-id>` device ID as LAN/mDNS identity. Older USB firmware that reports a generic `digital-esp32s3` identity cannot be bound or controlled through saved devices because it cannot prove which physical device is attached.

HTTP identity was tightened so a configured Wi-Fi hostname cannot replace the MAC-derived `identity.device_id`. This prevents two devices with the same human-facing hostname from merging into one saved device record.

## USB identity recovery and legacy flash migration

Real upgrade testing showed that generic-identity firmware could produce an oversized USB identity response that timed out before devd could bind the device, even after flashing a fixed host tool. The firmware identity response is kept compact and repeats the MAC-derived stable identity in a small `stable_identity` object so devd can recover `loadlynx-<short-id>` from fragmented post-transmit frames.

The generic `digital-esp32s3` identity remains rejected for bind and saved control. A separate migration path exists only for real digital flash when the CLI explicitly declares `expected_identity_device_id=digital-esp32s3`; devd may tolerate an identity timeout for the preflash lease, but the operation still requires the approved port, artifact hash evidence, explicit confirmation and post-flash stable identity capture.

## Documentation Model

`SPEC.md` is the active topic contract. Historical rationale, evolution notes, and records moved out of the topic contract are kept here.

## Formal HIL telemetry follow-up

The Mains Aegis 12V formal HIL run exposed a host-tooling gap rather than a control-plane contract gap: owner-facing `status --device` was correct for spot reads, but formal multi-device telemetry needed a stable single-lease status poller that would not reopen the USB port for every sample. The accepted response was to add a hidden `loadlynx status-stream` command for internal/test use, not a new public control abstraction.

The same HIL run also showed that the saved-device USB recovery path was still wasting time after the response was already recoverable from fragments or text. The implementation therefore moved the recoverable-response check earlier in the serial read loop and shortened the warmup drain window. The design intent stayed the same: bounded request-window recovery only, no stale-frame success, but less dead time during sustained telemetry polling.
