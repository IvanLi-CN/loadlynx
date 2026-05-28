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

The reusable pattern is a project-specific local daemon (`devd`) that owns USB/probe sessions and exposes a localhost HTTP/SSE API to Web, CLI and future desktop tools.

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
- CLI hardware-changing commands print target evidence before acting.
- Firmware artifacts are selected through a catalog and verified by SHA-256 before flashing.
- Runtime identity must match `build_id`, profile, features and target chip before log decode can be trusted.
- LAN records and USB records merge by `identity.device_id`, not by URL, port path or display name.
- Sensitive frame fields such as WiFi PSK are redacted at trace ingestion, before logs leave the daemon.
- Web evidence should come from mock-first Storybook canvas/docs states so localhost hardware daemons are not required for UI review.

## Guardrails / Reuse Notes

- A devd should be project-specific when the hardware domain has custom targets, safety semantics or firmware identities.
- Keep `mcu-agentd` or vendor tools as backend executors where useful, but do not expose their raw selector mutation commands through a Web UI.
- Do not persist browser leases in localStorage. A refresh should require a new lease.
- Do not let compatibility endpoints return arbitrary data when multiple devices are active; require `device_id` or `lease_id`.
- Treat mDNS/DNS-SD as convenience discovery. Always keep manual IP/hostname entry and bounded scan fallback.
- For dual-MCU devices, represent board targets explicitly instead of flattening them into one generic "serial device".
- Keep firmware catalog generation outside the daemon. devd should verify manifests and hashes, not invent release metadata.

## References

- `docs/specs/e3rv6-loadlynx-devd-control-plane/SPEC.md`
- `docs/specs/0004-mdns-and-lan-discovery/SPEC.md`
- `docs/interfaces/network-http-api.md`
- `docs/interfaces/uart-link.md`
- `/Users/ivan/Projects/Ivan/mains-aegis/tools/mains-aegis-devd/src/main.rs`
- `/Users/ivan/Projects/Ivan/mains-aegis/docs/specs/p8k3d-mains-aegis-devd/SPEC.md`
- `/Users/ivan/Projects/Ivan/mains-aegis/docs/specs/ypfpu-web-management-ui/SPEC.md`
