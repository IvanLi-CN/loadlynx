---
name: loadlynx-usb-devd
description: "Operate LoadLynx Web/devd USB CDC CLI workflows without using mcu-agentd selectors."
---

# LoadLynx USB Devd

## Purpose

The default digital hardware USB port is a safety guardrail for Agent-driven development. Its only purpose is to prevent an Agent from guessing which ESP32-S3 USB CDC device to use or accidentally operating on another connected USB port.

## Rules

- Use this workflow for LoadLynx Web/devd USB CDC control-plane work.
- Do not use `mcu-agentd` or `mcu-agentd selector` for Web/devd USB CDC control-plane verification.
- When a devd/Web task includes ESP32-S3 digital firmware flashing, use devd's lease-gated flash operation. It must use the remembered `.esp32-port` target and direct `espflash` after artifact hash verification. ELF artifacts use `espflash flash`; raw image artifacts require `flash_address` and use `espflash write-bin`.
- Do not fall back to `just agentd flash digital` for devd/Web digital flashing. `mcu-agentd` remains the path for non-devd firmware workflows and analog/probe operations.
- Set the default digital USB port only with:

```bash
just loadlynx usb-port set digital <path>
```

- Run that command only after the owner explicitly approves the exact `<path>`.
- The CLI supports human interactive use with missing target or port, using arrow-key selection over espflash-style serial port candidates. Agents must not use interactive candidate selection to bypass owner approval of the exact port.
- Reuse `.esp32-port` as the sole memory file for this setting; do not introduce a replacement file or alternate memory scheme.
- Never change the remembered digital USB port without explicit owner approval for the new exact path.
- Vague owner messages such as “continue”, “retry”, “finish it”, “继续”, “再试”, or “你自己处理” are not approval to change USB ports.
- If the remembered digital port is missing, stale, unreadable, or does not match the owner-approved ESP32-S3 digital device, stop and ask the owner which USB port to use.
- Do not scan candidate ports and silently pick one as a replacement.
- Do not pass hardware port arguments when starting `loadlynx-devd`; start the daemon normally after the CLI memory has been set.

## Development Commands

Set the owner-approved default ESP32-S3 digital USB CDC port:

```bash
just loadlynx usb-port set digital /dev/cu.usbmodemXXXX
```

Start devd without hardware port arguments:

```bash
just devd-serve --bind 127.0.0.1:30180 --allow-dev-cors
```

Run CLI operations through the active devd:

```bash
just loadlynx discover --json
```

Run a devd firmware dry-run before any real flash:

```bash
just loadlynx flash digital --device <device-id> --artifact <artifact-id> --dry-run
```

## Verification

Real-device Web/devd verification requires protocol evidence from the remembered USB port, such as decoded JSONL frames or successful `hello`, `get_identity`, or `get_status` responses. Candidate discovery, serial-open-only checks, Web lease creation, mock data, or firmware dry-runs are not sufficient proof.
