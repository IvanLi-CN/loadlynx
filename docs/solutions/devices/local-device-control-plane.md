---
title: Local device control plane for Web, CLI and firmware operations
module: devices
problem_type: local_hardware_control
component: devd
tags: [devd, usb-http-bridge, firmware-flash, mdns, cli, hardware-safety]
status: active
related_specs: [e3rv6]
---

# Local device control plane for Web, CLI and firmware operations

## Context

Embedded projects often grow separate hardware entrypoints: browser LAN HTTP, USB serial scripts, firmware flash commands, log monitors and development daemons. Once a Web UI also needs USB writes or firmware flashing, letting multiple processes compete for the same serial port creates unreliable behavior and unsafe device selection.

The reusable pattern is a project-specific local daemon (`devd`) that owns USB/probe sessions, exposes an IPC API to CLI tools, and exposes a loopback-only HTTP bridge only for browser/debug paths.

## Symptoms

- Web UI, monitor and flash commands contend for one USB CDC or probe.
- Multiple connected boards make "first port wins" selection unsafe.
- Browser state outlives a tab or refresh and leaves a serial port occupied.
- Firmware log decoding uses an ELF that may not match the running image.
- LAN discovery and USB control produce duplicate records for the same physical device.

## Root Cause

The system lacks one authoritative owner for local hardware sessions. Direct browser serial access, command-line flashing and ad hoc scripts all have partial device knowledge, but none can enforce exclusive ownership, artifact matching, bounded logs, redaction and explicit target evidence across the whole workflow.

## Resolution

Use a local-first control plane:

- `scan` only discovers candidates; it never connects or picks a device.
- The owner explicitly selects a candidate before `bind`, `connect`, `lease`, `flash` or `reset`.
- USB/Web writes require a per-device lease with heartbeat and TTL cleanup.
- CLI-to-devd traffic should be IPC-first when the CLI is a released local tool. Keep HTTP bridge URLs out of ordinary CLI help, auto-start a sibling daemon when reasonable, and reserve local HTTP for browser/debug paths.
- Any HTTP bridge that can write hardware must bind loopback only unless a separate authenticated remote-access design exists.
- Treat lease and physical serialization as separate layers: leases authorize clients and identify ownership, while a per-port owner or queue serializes the actual USB/probe operations.
- CLI hardware-changing commands print target evidence before acting.
- Firmware artifacts are selected through a catalog and verified by SHA-256 before flashing.
- Real first-flash or non-project-firmware flows need a stronger gate than "tool exited 0": artifact/hash/target evidence, explicit `yes` confirmation, explicit risk acknowledgement when applicable, and post-flash identity capture.
- Runtime identity must match `build_id`, profile, features and target chip before log decode can be trusted.
- LAN records and USB records merge by `identity.device_id`, not by URL, port path or display name.
- `identity.device_id` must be device-unique. Do not use configurable hostnames or aliases as the registry key; keep them as display or network locator metadata.
- A local CLI should treat discovery IDs as temporary candidate IDs. Persisted devices should be keyed by stable firmware identity, and ordinary operations should use a saved device ID or a saved default instead of reusing a daemon's transient device table.
- Saved devices can contain multiple transport locators for the same physical device. Remember the last selected transport per device entity, and verify runtime identity before using a saved local USB transport so a stale port or candidate ID cannot silently control a different device.
- Sensitive frame fields such as WiFi PSK are redacted at trace ingestion, before logs leave the daemon.
- Keep device-local transports compact and purpose-built. When USB/serial frame budgets are tight, firmware may return a compact operation-specific payload while the daemon expands it back to the public HTTP/Web shape for CLI and browser callers.
- Treat safe-control CLIs as mode-first entrypoints. `cc`, `cv` and `cp` should each own their target unit, preset/edit/apply path and explicit disable path; do not keep a user-facing `output set` toggle as the primary surface once the mode-specific commands exist.
- Separate local physical-access writes from LAN writes in the user interface. LAN credential writes should require an explicit unsafe-network confirmation or flag, while USB/devd writes can rely on lease and selected-port evidence.
- Web evidence should come from mock-first Storybook canvas/docs states so localhost hardware daemons are not required for UI review.
- Web Serial can be a formal browser path when users need static GitHub Pages or release bundle operation, but it should save identity/profile metadata only and reconnect through browser-granted ports rather than OS serial paths.
- If an owner-facing backup/export workflow explicitly needs secrets, expose that as a narrow read operation with a sensitive artifact contract. Keep ordinary status, diagnostics, traces and logs redacted; do not broaden generic observability paths just to make backups convenient.

## Guardrails / Reuse Notes

- A devd should be project-specific when the hardware domain has custom targets, safety semantics or firmware identities.
- Keep vendor tools as internal backend executors where useful, but do not expose their raw selector mutation commands through a Web UI or bypass the product CLI/devd control plane.
- Do not persist browser leases in localStorage. A refresh should require a new lease.
- Do not let compatibility endpoints return arbitrary data when multiple devices are active; require `device_id` or `lease_id`.
- Do not expose lease IDs as ordinary user CLI parameters. CLI tools should create, heartbeat and release leases internally, keeping the user workflow stable while still preserving daemon-side authorization.
- Do not open a USB CDC port per HTTP request once multiple clients can talk to the daemon. Keep one daemon-owned reader/writer per physical port, generate unique request IDs, and reject mismatched responses instead of treating the latest frame as success.
- For firmware transports that mix binary logs and JSONL on one USB channel, response recovery must be operation-scoped. Recover only from payloads observed after the matching transmit frame and only when they have the expected operation shape.
- For saved-device USB compat read paths, classify serial response gaps explicitly. Treat timeouts, mismatched request IDs, missing responses and invalid operation shapes as bounded retryable failures for the current command window; do not fall back to stale data or wait the whole host timeout when the request window already proved unhealthy.
- For sustained lab telemetry on top of the same USB/devd control plane, prefer an internal single-lease stream command over repeated spot-read commands that reopen or re-lease every sample. Keep that stream hidden/internal unless there is a clear owner-facing product need for it.
- When a command window already contains a response that is recoverable from fragments or text, return as soon as that proof exists. Do not keep burning the full read timeout after the current request has already succeeded by conservative recovery rules.
- Preserve enough non-protocol text for one noisy command window. Some valid responses may be too corrupted to parse as JSON but still contain operation-specific compact payloads, such as calibration curves or WiFi state; recover those conservatively and mark them as recovered instead of silently synthesizing missing fields.
- Keep firmware-side USB read responses compact when the owner-facing public shape is larger than the transport budget. Let firmware emit the smallest stable operation payload that still proves the requested state, and let the daemon expand or normalize it back to the public CLI/Web shape.
- For list-style responses recovered from noisy serial frames, require a completeness gate before returning success. Merge only records that have the requested operation shape, and use a separate real device response to fill gaps only when that response proves the missing record; otherwise return an explicit incomplete-response error.
- Safe-control CLIs must expose both sides of a state transition. If a command can enable a load, it must also provide an explicit disable command; absence of an enable flag must not silently mean disable.
- Flash/reset tools that need the OS serial port directly must temporarily close or pause the daemon serial owner and return explicit busy/in-progress errors to concurrent same-port commands.
- Do not fake unsupported compatibility endpoints. If firmware does not expose an operation on the current build, return a clear unsupported-operation error rather than advertising the operation or synthesizing success in the daemon.
- Long-running firmware-side waits need matching daemon-side serial timeouts for that operation only. Keep ordinary request timeouts short, then widen timeout windows for explicit wait semantics such as WiFi connection waits.
- Treat mDNS/DNS-SD as convenience discovery. Always keep manual IP/hostname entry and bounded scan fallback.
- For dual-MCU devices, represent board targets explicitly instead of flattening them into one generic "serial device".
- If older firmware exposes only a generic USB identity, block binding and control until firmware provides a stable device ID. Do not synthesize a persistent ID from OS port paths or daemon candidate IDs.
- Treat bind-probe leases as a narrow identity-binding capability. They may bypass a project-local approved-port cache only to read identity for an explicitly selected candidate, and must be rejected by ordinary operation checks. Saved USB operation leases must return the expected stable identity; a missing identity field is a failed confirmation.
- Keep firmware catalog generation outside the daemon. devd should verify manifests and hashes, not invent release metadata.
- Release installers should verify a `SHA256SUMS` file that covers every release asset, install into a user-owned directory, validate installed binaries, and print PATH guidance without editing profiles automatically.

## References

- `docs/specs/e3rv6-loadlynx-devd-control-plane/SPEC.md`
- `docs/specs/yy7th-mdns-and-lan-discovery/SPEC.md`
- `docs/interfaces/network-http-api.md`
- `docs/interfaces/uart-link.md`
- `/Users/ivan/Projects/Ivan/mains-aegis/tools/mains-aegis-devd/src/main.rs`
- `/Users/ivan/Projects/Ivan/mains-aegis/docs/specs/p8k3d-mains-aegis-devd/SPEC.md`
- `/Users/ivan/Projects/Ivan/mains-aegis/docs/specs/ypfpu-web-management-ui/SPEC.md`
