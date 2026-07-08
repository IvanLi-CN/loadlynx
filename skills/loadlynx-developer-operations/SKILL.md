---
name: loadlynx-developer-operations
description: "Operate LoadLynx developer and maintenance workflows as a superset of loadlynx-user-operations: inherit the released CLI-only user business workflows, USB-first/HTTP-fallback connection order, IPC-only CLI devd access, CLI device memory, Web Serial as the formal human browser path, first-flash gates, and command-availability gates, then add source checkout/clone, toolchain checks, Just recipes for loadlynx/devd/Web development, firmware and host-tool builds, GitHub Release asset maintenance, missing CLI/devd business capability implementation, calibration, reset/monitor/logs, and HIL verification."
---

# LoadLynx Developer Operations

Use this skill for engineering, maintenance, release, and hardware-debug work. This skill is a superset of `skills/loadlynx-user-operations/SKILL.md`: when the task includes ordinary LoadLynx hardware operation, first apply the user skill's CLI-only business workflows, USB-first/HTTP-fallback connection order, CLI device memory, command-availability gates, and safety checks. Then add the developer-only source checkout, Just, firmware, release, calibration, reset/monitor/logs, and HIL rules below. When this skill operates hardware, use `loadlynx` CLI + `loadlynx-devd`, not Web UI operation or external MCU daemons.

## Start Here

- Install this skill with:

```bash
npx skills add https://github.com/IvanLi-CN/loadlynx --skill loadlynx-developer-operations -g
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

- If the task is ordinary use of released programs on a user's machine and does not require source, use the user skill directly. If a developer task includes the same ordinary operation as setup, validation, or reproduction, inherit the user skill behavior instead of restating or bypassing it.

## Tooling Checks

- Source development expects `just`, Rust embedded targets, ESP Xtensa tooling for digital firmware, Node/npm or Bun for Web work as documented under `web/`, and Rust tooling for `tools/loadlynx-devd`.
- Host-tool and hardware development uses:

```bash
just devd-build
just devd-test
just devd-bridge-http --bind 127.0.0.1:<http-port> --allow-dev-cors
just loadlynx <args>
```

- Firmware development uses `just a-build` for STM32G431 analog and `just d-build` for ESP32-S3 digital.
- For release label decisions, no-release decisions, docs/skill release-impact checks, or
  release backfills, first use `skills/loadlynx-release-decision/SKILL.md`; this skill only
  owns the artifact maintenance and engineering execution side of releases.
- Release maintenance must keep GitHub Releases publishing the user-facing assets required by the user skill: installer scripts, platform `loadlynx-host-tools-<platform>.tar.gz` archives, firmware assets/catalogs when user CLI/Web flashing is advertised, web bundle, `SHA256SUMS` covering every release asset, and accurate release notes.
- If user docs require `loadlynx wifi ...`, first verify that the CLI, devd API, firmware protocol, persistence behavior, and release binaries implement it. If absent, implement and test it before presenting WiFi configuration as a user capability.
- If user docs require remembered devices, verify `loadlynx devices`, `loadlynx device list|add|use|remove`, `loadlynx status`, and `loadlynx status --device ...`. The registry must remain user-level config, not project checkout state.

## Business Capability Development

- Keep the released CLI as the user-facing control surface for agent-driven LoadLynx business operations. Web-only or raw-HTTP-only behavior is incomplete for skill-driven user operation. Web Serial remains a supported human browser path and must stay aligned with CLI/devd safety gates.
- Business capability coverage includes:
  - Identity/status/telemetry: firmware identity, uptime, network identity, link state, analog state, fault flags, voltage, current, power, temperature, and USB-PD attach/contract state.
  - Electronic-load control: output enable/disable, CC/CV/CP runtime setpoints, limits, preset edit/apply, and post-write status verification.
  - USB-PD control: Source capability readback, Fixed/PPS request/apply, Safe5V and extended-voltage gating, and failure-state reporting.
  - User lifecycle: released firmware catalog/assets, CLI dry-run/real flash, reset/reconnect evidence, and runtime WiFi configuration when implemented.
- When adding a business workflow, implement the full chain needed for a released user CLI command: firmware/protocol support if needed, devd API, `loadlynx` CLI surface, help text, tests, redaction for secrets, release packaging, and skill/spec documentation.
- Do not document a user business workflow as available until `loadlynx --help` and the released host-tools artifact expose it.

## Device Selection

- Never guess or silently switch hardware targets.
- For CLI/devd ESP32-S3 USB CDC work, use repo-local `.esp32-port` only after the owner explicitly authorizes the specific USB CDC port path.
- Explicit authorization can be natural language. Do not require the owner to answer with a fixed phrase or command string; the authorized action and target only need to be unambiguous.
- Set the approved digital USB CDC port only with:

```bash
just loadlynx usb-port set digital <path>
```

- Do not use interactive candidate selection as an Agent to bypass explicit owner authorization.
- Do not edit `.esp32-port`, `.stm32-port`, device registry files, or local `.loadlynx` files unless the owner explicitly authorizes the specific change.
- Before flash/reset/digital monitor/HIL, echo the saved device id, selected transport, approved digital USB CDC path or analog probe evidence, artifact id, and dry-run/real mode.

## devd, CLI, And USB CDC

- Use `loadlynx-devd` for CLI/devd USB CDC control-plane work; do not route that path through external MCU daemons.
- CLI/devd is native IPC-first. The CLI should auto-start a sibling `loadlynx-devd serve` on the default Unix socket / Windows named pipe when needed. `--ipc` is an endpoint override for explicit multi-instance or debugging scenarios, not part of normal user or agent commands. Do not reintroduce ordinary daemon-URL CLI workflows.
- `loadlynx-devd bridge-http` is the browser/debug bridge only, must bind loopback, and is the path used by local Web development or release/GitHub Pages browser bridge fallback.
- Run the CLI through Just during source development:

```bash
just loadlynx devices
just loadlynx device add
just loadlynx device list
just loadlynx device use <saved-id>
just loadlynx status
just loadlynx status --device <saved-id>
```

- For a deliberate alternate IPC endpoint, start the matching daemon and pass the override consistently. Do not use this form for normal flashing or user operation:

```bash
just devd-serve --endpoint /tmp/loadlynx-devd.sock
just loadlynx --ipc /tmp/loadlynx-devd.sock status --device <saved-id>
```

- Web development may point a local UI at `loadlynx-devd bridge-http` with `VITE_LOADLYNX_DEVD_URL=http://127.0.0.1:<http-port>`, but skill-driven hardware operations still use CLI commands.
- Use devd leases for USB writes. A scan result, serial-open check, mock identity, Web lease alone, or firmware dry-run alone is not enough to prove real-device coverage.
- Prove USB CDC coverage with decoded JSONL frames or successful `hello`, `get_identity`, `get_status`, or equivalent request/response evidence from the approved port.
- Redact WiFi PSK and equivalent secrets before traces or logs leave devd.

## Firmware, Release, And HIL

- Build analog firmware with `just a-build`; build digital firmware with `just d-build`.
- For CLI/devd ESP32-S3 digital firmware flows, use devd's lease-gated direct `espflash` path against the approved `.esp32-port` target.
- CLI, devd bridge, and Web Serial real digital flashes must share the same first-flash/non-project gate: artifact/hash/target evidence, explicit owner confirmation, explicit non-project acknowledgement when applicable, and post-flash identity capture. Do not claim success from `espflash` exit status alone.
- Run a devd firmware dry-run before real flash:

```bash
just loadlynx flash digital --device <saved-id> --artifact <artifact-id>
```

- For real devd digital flash, use a saved USB device target (`--device <saved-id>` or saved default), require a valid lease, selected artifact, artifact hash verification, target evidence, explicit owner confirmation, and post-flash identity capture. Do not require a fixed typed phrase for this confirmation. ELF artifacts use `espflash flash`; raw image artifacts require `flash_address` and use `espflash write-bin`.
- Web Serial flash uses `esptool-js`, release firmware catalog/assets, browser-granted ports, and identity/profile memory only. It must not save OS port paths.
- Analog firmware flash/reset must also be exposed through `loadlynx` CLI + `loadlynx-devd`. Use `probe-rs` as an internal devd backend when needed. Analog RTT/defmt monitor/logs are a CLI/devd product gap until implemented; `loadlynx monitor analog` must reject explicitly rather than using digital USB monitor or any external MCU daemon.
- After flashing or reset, compare boot logs against `tmp/analog-fw-version.txt` or `tmp/digital-fw-version.txt` before claiming the board is running the local build.

## WiFi And Calibration

- Developer WiFi work may involve source configuration, firmware protocol changes, devd API changes, CLI support, release packaging, and secret redaction.
- Development digital firmware must not read repo-root `.env` WiFi credentials or `DIGITAL_WIFI_*` keys. Runtime WiFi credentials are written through USB/devd or Web Serial. Factory/release WiFi is allowed only for explicitly scoped `LOADLYNX_ENABLE_FACTORY_WIFI=1` builds with command-scoped `LOADLYNX_FACTORY_WIFI_*` values.
- Runtime user WiFi configuration must not be documented as available until the released CLI and firmware actually implement it.
- Treat calibration writes as maintenance operations. Read `docs/dev-notes/user-calibration.md` before changing calibration behavior or data.
- Keep calibration mode ownership single-writer, leave calibration mode `off` after maintenance, and collect before/after evidence when writing or committing calibration data.

## Validation

- Prefer targeted checks for the changed surface: `just devd-test`, affected `cargo test`, `just a-build`, `just d-build`, non-hardware Web checks, or release workflow linting.
- For release workflow changes, verify official and development Releases build required firmware/host-tools assets before creating a GitHub Release.
- HIL evidence must identify target, transport, lease/session where applicable, artifact/firmware identity, and observed protocol/log result.
- If a selector or saved device target is missing, stale, unreadable, or ambiguous, stop and ask the owner to identify the hardware target unambiguously.
