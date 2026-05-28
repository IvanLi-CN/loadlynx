---
name: loadlynx-developer-operations
description: "Operate LoadLynx developer and maintenance workflows from a source checkout: verify or clone the repository when a developer task requires it, install/check toolchains, build firmware and host tools from source, run Just recipes for loadlynx/devd/Web development, maintain GitHub Release firmware and host-tools assets, implement missing CLI business capabilities for identity/status/telemetry, electronic-load output/preset/CC/CV/CP control, USB-PD settings, WiFi, firmware flashing, and CLI hardware memory, and perform guarded hardware work through CLI/Just-controlled paths."
---

# LoadLynx Developer Operations

Use this skill for engineering, maintenance, release, and hardware-debug work. It complements `skills/loadlynx-user-operations/SKILL.md`; do not duplicate ordinary user tutorials. When this skill operates hardware, use CLI/Just-controlled paths, not Web UI operation.

## Start Here

- Install this skill with:

```bash
npx skills add https://github.com/IvanLi-CN/loadlynx --skill loadlynx-developer-operations
```

- Before any project command, prove the current directory is a LoadLynx checkout:

```bash
git rev-parse --show-toplevel
test -f Justfile
test -f tools/loadlynx-devd/Cargo.toml
test -d firmware/analog
test -d firmware/digital
```

- If no checkout is available and the task requires source work, clone the canonical repository, then repeat the checks:

```bash
git clone https://github.com/IvanLi-CN/loadlynx.git
cd loadlynx
```

- If the task is ordinary use of released programs on a user's machine, use the user skill instead of cloning.

## Tooling Checks

- Source development expects `just`, Rust embedded targets, ESP Xtensa tooling for digital firmware, Node/npm or Bun for Web work as documented under `web/`, and Rust tooling for `tools/loadlynx-devd`.
- Host-tool and hardware development uses:

```bash
just devd-build
just devd-test
just devd-serve --bind 127.0.0.1:<port> --allow-dev-cors
just loadlynx <args>
```

- Firmware development uses `just a-build` for STM32G431 analog and `just d-build` for ESP32-S3 digital.
- Release maintenance must keep GitHub Releases publishing the user-facing assets required by the user skill: platform `loadlynx-host-tools-<platform>.tar.gz` archives, firmware assets/catalogs when user CLI flashing is advertised, and accurate release notes.
- If user docs require `loadlynx wifi ...`, first verify that the CLI, devd API, firmware protocol, persistence behavior, and release binaries implement it. If absent, implement and test it before presenting WiFi configuration as a user capability.
- If user docs require remembered hardware, verify `loadlynx hardware available/recent/path/list/save/forget` and `loadlynx status --hardware ...`. The registry must remain user-level config, not project checkout state.

## Business Capability Development

- Keep the released CLI as the user-facing control surface for LoadLynx business operations. Web-only or raw-HTTP-only behavior is incomplete for skill-driven user operation.
- Business capability coverage includes:
  - Identity/status/telemetry: firmware identity, uptime, network identity, link state, analog state, fault flags, voltage, current, power, temperature, and USB-PD attach/contract state.
  - Electronic-load control: output enable/disable, CC/CV/CP runtime setpoints, limits, preset edit/apply, and post-write status verification.
  - USB-PD control: Source capability readback, Fixed/PPS request/apply, Safe5V and extended-voltage gating, and failure-state reporting.
  - User lifecycle: released firmware catalog/assets, CLI dry-run/real flash, reset/reconnect evidence, and runtime WiFi configuration when implemented.
- When adding a business workflow, implement the full chain needed for a released user CLI command: firmware/protocol support if needed, devd API, `loadlynx` CLI surface, help text, tests, redaction for secrets, release packaging, and skill/spec documentation.
- Do not document a user business workflow as available until `loadlynx --help` and the released host-tools artifact expose it.

## Device Selection

- Never guess or silently switch hardware targets.
- For CLI/devd ESP32-S3 USB CDC work, use repo-local `.esp32-port` only after the owner approves the exact port path.
- Set the approved digital USB CDC port only with:

```bash
just loadlynx usb-port set digital <path>
```

- Do not use interactive candidate selection as an Agent to bypass exact owner approval.
- Do not call `just agentd selector set ...` or edit `.esp32-port` / `.stm32-port` unless the owner explicitly approves the exact change.
- Before flash/reset/monitor/HIL, echo the target from `just agentd-get-port digital`, `just agentd-get-port analog`, or the approved `.esp32-port` path for devd digital work.

## devd, CLI, And USB CDC

- Use `loadlynx-devd` for CLI/devd USB CDC control-plane work; do not route that path through `mcu-agentd` selectors.
- In source checkout mode, start devd through Just:

```bash
just devd-serve --bind 127.0.0.1:<port> --allow-dev-cors
```

- Run the CLI through Just during source development:

```bash
just loadlynx --devd http://127.0.0.1:<port> devices
just loadlynx hardware available
just loadlynx hardware recent
just loadlynx hardware list
just loadlynx --devd http://127.0.0.1:<port> status --device <device-id>
just loadlynx status --hardware <saved-hardware-id>
```

- Web development may point a local UI at devd with `VITE_LOADLYNX_DEVD_URL=http://127.0.0.1:<port>`, but skill-driven hardware operations still use CLI commands.
- Use devd leases for USB writes. A scan result, serial-open check, mock identity, Web lease alone, or firmware dry-run alone is not enough to prove real-device coverage.
- Prove USB CDC coverage with decoded JSONL frames or successful `hello`, `get_identity`, `get_status`, or equivalent request/response evidence from the approved port.
- Redact WiFi PSK and equivalent secrets before traces or logs leave devd.

## Firmware, Release, And HIL

- Build analog firmware with `just a-build`; build digital firmware with `just d-build`.
- For CLI/devd ESP32-S3 digital firmware flows, use devd's lease-gated direct `espflash` path against the approved `.esp32-port` target.
- Run a devd firmware dry-run before real flash:

```bash
just loadlynx flash digital --device <device-id> --artifact <artifact-id>
```

- For real devd digital flash, require a valid lease, selected artifact, artifact hash verification, and target evidence. ELF artifacts use `espflash flash`; raw image artifacts require `flash_address` and use `espflash write-bin`.
- Do not fall back to `just agentd flash digital` for CLI/devd digital flashing.
- For analog firmware and non-devd firmware workflows, use `mcu-agentd` through `just agentd ...` and preserve selector guardrails.
- After flashing or reset, compare boot logs against `tmp/analog-fw-version.txt` or `tmp/digital-fw-version.txt` before claiming the board is running the local build.

## WiFi And Calibration

- Developer WiFi work may involve source configuration, firmware protocol changes, devd API changes, CLI support, release packaging, and secret redaction.
- Build-time digital WiFi credentials must come from repo-defined `.env` / `DIGITAL_WIFI_*` sources; do not override them ad hoc unless the owner explicitly approves it.
- Runtime user WiFi configuration must not be documented as available until the released CLI and firmware actually implement it.
- Treat calibration writes as maintenance operations. Read `docs/dev-notes/user-calibration.md` before changing calibration behavior or data.
- Keep calibration mode ownership single-writer, leave calibration mode `off` after maintenance, and collect before/after evidence when writing or committing calibration data.

## Validation

- Prefer targeted checks for the changed surface: `just devd-test`, affected `cargo test`, `just a-build`, `just d-build`, non-hardware Web checks, or release workflow linting.
- For release workflow changes, verify official and development Releases build required firmware/host-tools assets before creating a GitHub Release.
- HIL evidence must identify target, transport, lease/session where applicable, artifact/firmware identity, and observed protocol/log result.
- If a selector is missing, stale, unreadable, or ambiguous, stop and ask the owner for the exact hardware target.
