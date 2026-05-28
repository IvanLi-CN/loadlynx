---
name: loadlynx-user-operations
description: "Operate LoadLynx hardware from an end-user machine through the released loadlynx CLI only: install GitHub Release host tools, prefer USB/devd access before HTTP access, use CLI-saved hardware memory for previously connected devices, and perform user business workflows such as device identity/status/telemetry checks, electronic-load output control, presets or CC/CV/CP/PD controls when the installed CLI exposes them, released firmware flashing, and WiFi configuration only when released. Do not use Web UI, source checkouts, Just, Rust, mcu-agentd, probe tooling, or project-local developer caches for skill-driven hardware operation."
---

# LoadLynx User Operations

Use this skill on a normal user's computer. All skill-driven hardware operation must go through the released `loadlynx` CLI. Assume the user may have only network access, a USB cable, and released LoadLynx programs from GitHub Releases.

## Start Here

- Install this skill with:

```bash
npx skills add https://github.com/IvanLi-CN/loadlynx --skill loadlynx-user-operations
```

- Never require the user to clone the repository or install Rust, Bun, Just, `mcu-agentd`, `espflash`, or `probe-rs`.
- Use only released LoadLynx host tools and released firmware assets from `https://github.com/IvanLi-CN/loadlynx/releases`.
- Do not use the Web UI as the skill's hardware operation path. If the user wants hardware operated by an agent, use CLI commands only.
- Before giving any CLI workflow, verify the installed program supports it:

```bash
loadlynx --help
loadlynx-devd --help
```

- If the requested user workflow depends on a command that is absent, stop and report that the installed release does not support it. Do not invent commands, fall back to Web UI, or switch to source/developer instructions.

## Install Released Host Tools

- Download the latest stable Release unless the owner explicitly accepts a prerelease.
- Choose the platform archive:
  - Apple Silicon macOS: `loadlynx-host-tools-macos-aarch64.tar.gz`
  - Intel macOS: `loadlynx-host-tools-macos-x86_64.tar.gz`
  - Linux x86_64: `loadlynx-host-tools-linux-x86_64.tar.gz`
  - Windows x86_64: `loadlynx-host-tools-windows-x86_64.tar.gz`
- The archive contains:
  - `loadlynx-devd`: local USB CDC bridge daemon used behind CLI USB workflows.
  - `loadlynx`: released CLI for discovery, status, output, firmware flash, reset/monitor, and any user WiFi command that the current release actually implements.
- On macOS or Linux:

```bash
mkdir -p "$HOME/.local/bin"
tar -xzf "$HOME/Downloads/loadlynx-host-tools-<platform>.tar.gz" -C "$HOME/.local/bin"
chmod +x "$HOME/.local/bin/loadlynx-devd" "$HOME/.local/bin/loadlynx"
```

- Ensure `$HOME/.local/bin` is on `PATH`; ask before editing shell startup files.
- On Windows, extract to a user-owned folder such as `%LOCALAPPDATA%\LoadLynx\bin`, then add that folder to the user `Path`.
- Verify:

```bash
loadlynx-devd --help
loadlynx --help
```

## Connect Hardware

- Connection priority is USB first, HTTP second.
- For CLI-over-USB workflows, start the released local bridge before trying HTTP:

```bash
loadlynx-devd serve --bind 127.0.0.1:30180
```

- Keep the bridge running while using USB. Use `http://127.0.0.1:30180` as the devd URL.
- Use only the released CLI's user-facing selection flow for USB targets. Do not edit project-local developer port/probe caches or any selector file by hand.
- Use HTTP only when USB is unavailable, explicitly not desired, or the user chooses a saved HTTP device. HTTP targets may be explicit base URLs, IP addresses, or `loadlynx-<short-id>.local`.

## CLI Hardware Memory

- The CLI must be the source of remembered hardware. Do not rely on Web local storage, browser history, or project-local cache files for user workflows.
- Before scanning manually, check the installed CLI for saved-hardware commands that can list, select, connect, forget, or update devices.
- After a successful USB or HTTP connection, save or update that hardware through the CLI if the installed release exposes a saved-device command.
- Prefer a saved USB device over a saved HTTP endpoint for the same hardware. Use the saved HTTP endpoint only as fallback.
- If the installed CLI cannot remember previously connected hardware, report that the current release lacks required user hardware memory and escalate to developer work to implement/release it.

## LoadLynx Business Workflows

- Treat transport selection as preparation only. The user-facing job is operating the LoadLynx electronic-load and USB-PD device safely through released CLI commands.
- Identity and status:
  - Identify the selected hardware before writes.
  - Read firmware versions, uptime, network identity, link state, analog board state, fault flags, voltage, current, power, temperature, and USB-PD attach/contract state when the CLI/status payload exposes them.
- Electronic-load operation:
  - Enable or disable output only after confirming the hardware ID, intended mode, and target state.
  - Use released CLI commands for CC/CV/CP/preset editing or applying only when `loadlynx --help` exposes them.
  - If the installed CLI only supports output switching and status, do not present preset or CC/CV/CP editing as available.
- USB-PD operation:
  - Read Source capabilities, current contract, Fixed/PPS selection, Safe5V, and extended-voltage gate state only through released CLI commands when present.
  - Do not use Web UI or raw HTTP to apply PD settings if the CLI has not shipped that workflow.
- Firmware and network lifecycle:
  - Firmware flash is a user workflow only when the Release provides firmware catalog/assets and the installed CLI can select and verify them.
  - WiFi configuration is a user workflow only when the installed CLI exposes a real WiFi command.
- If a requested business workflow is absent from the installed CLI, stop and escalate to the developer skill to implement, test, package, and release that CLI capability.

## Download Released Firmware

- Download firmware only from the chosen GitHub Release.
- Prefer a release-provided firmware catalog, commonly named `loadlynx-firmware-catalog.json`, plus the firmware files referenced by that catalog.
- If the Release does not include a firmware catalog or the CLI/devd cannot select a downloaded catalog, stop and report that user-side GitHub firmware flashing is not supported by that release.
- Keep downloaded firmware and catalog files together in one user-owned folder so relative catalog paths can resolve.

## User Workflows

- CLI discovery and status:

```bash
loadlynx hardware available
loadlynx hardware recent
loadlynx hardware list
loadlynx --devd http://127.0.0.1:30180 devices
loadlynx --devd http://127.0.0.1:30180 status --device <device-id>
loadlynx status --url http://<device-host-or-ip>
loadlynx status --hardware <saved-hardware-id>
```

- Hardware memory:

```bash
loadlynx hardware path
loadlynx hardware available --scan
loadlynx hardware recent
loadlynx hardware list
loadlynx hardware save --id <name> --transport usb --device <device-id> --devd http://127.0.0.1:30180
loadlynx hardware save --id <name> --transport http --url http://<device-host-or-ip>
loadlynx hardware forget <saved-hardware-id>
```

- Use `hardware available` to see currently visible USB/devd devices plus saved HTTP fallback entries; add `--scan` when the bridge should refresh device visibility first. If devd is not running, use the reported USB error and saved HTTP fallback to decide whether to start the bridge or use HTTP.
- Use `hardware recent` to list remembered hardware by most recent successful connection or save time.
- `loadlynx status --device ...` and `loadlynx status --url ...` best-effort update the CLI hardware memory after a successful connection; a memory write failure must not hide a successful status result.
- The memory file lives in the user's OS config directory: macOS `~/Library/Application Support/LoadLynx/devices.json`, Linux `${XDG_CONFIG_HOME:-~/.config}/loadlynx/devices.json`, Windows `%APPDATA%\LoadLynx\devices.json`; `LOADLYNX_HOME` overrides the directory.
- List saved hardware before scanning, then use `--hardware <saved-hardware-id>` instead of retyping device IDs or URLs.
- CLI output control:
  - Confirm `loadlynx output --help` and `loadlynx output set --help` expose the needed command.
  - Require the user to confirm the saved hardware ID or target base URL and intended output state before changing output.
  - Verify the result with `loadlynx status --hardware <saved-hardware-id>`, `loadlynx status --url <base-url>`, or `loadlynx --devd <devd-url> status --device <device-id>`.
- CLI firmware flash:
  - Confirm `loadlynx flash --help` supports the needed artifact/catalog options.
  - Use dry-run first whenever the CLI exposes it.
  - Require the user to confirm the device id, target board, firmware artifact, and whether the command is dry-run or real flash.
  - Do not flash if target evidence, artifact hash verification, or lease/session requirements are missing.
- CLI WiFi configuration:
  - Confirm `loadlynx --help` exposes an implemented WiFi command before giving steps.
  - Never echo PSKs or secrets in chat, logs, screenshots, traces, shell history, or PR text.
  - If no released `loadlynx wifi ...` command exists, say the current user release cannot configure WiFi by CLI and escalate to developer work to implement or release that capability.

## Escalate Out

- Switch to `skills/loadlynx-developer-operations/SKILL.md` for source checkout, cloning, project builds, `just`, local devd builds, release workflow changes, `mcu-agentd`, probe/selector maintenance, calibration writes, HIL/debug sessions, or implementing missing CLI features.
- If a hardware operation is only available through Web UI and not through `loadlynx`, stop and escalate to developer work to add/release a CLI command.
- If hardware memory is only available in Web UI or project-local files and not through `loadlynx`, stop and escalate to developer work to add/release CLI hardware memory.
- Do not use raw HTTP writes or source-tree commands to bypass missing released user functionality.
- Do not continue hardware-changing operations when identity, artifact, target, lease, or command availability is ambiguous.

## Safety Checks

- Before enabling output or flashing firmware, verify the selected device identity and target board.
- Treat `LINK_DOWN`, `ANALOG_FAULTED`, `ANALOG_NOT_READY`, `LIMIT_VIOLATION`, PD failure states, command absence, and hash/target mismatch as stop conditions.
- After changing settings, flashing, or configuring WiFi, verify status/telemetry/reconnect behavior before claiming success.
