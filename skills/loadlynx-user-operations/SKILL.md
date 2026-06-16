---
name: loadlynx-user-operations
description: "Operate LoadLynx hardware from an end-user machine through released host tools and official Web paths: install GitHub Release host tools with SHA256SUMS verification, use the released loadlynx/loadlynx-devd CLI surface as shipped, prefer saved devices and released CLI control flows, keep Web Serial as a formal human browser UI path, and do not fall back to source checkouts, raw local devd HTTP, or developer-only tooling."
---

# LoadLynx User Operations

Use this skill on a normal user's computer. All skill-driven hardware operation must go through released `loadlynx` and `loadlynx-devd` binaries from GitHub Releases.

## Start Here

- Install this skill with:

```bash
npx skills add https://github.com/IvanLi-CN/loadlynx --skill loadlynx-user-operations
```

- Never require the user to clone the repository or install Rust, Bun, Just, `mcu-agentd`, `espflash`, or `probe-rs`.
- Use only released LoadLynx host tools and released firmware assets from `https://github.com/IvanLi-CN/loadlynx/releases`.
- Do not use the Web UI as the skill's agent-operated hardware path. Web Serial is a supported human browser path, not the default agent control path.
- Before giving any CLI workflow, verify the installed program supports it:

```bash
loadlynx --help
loadlynx-devd --help
loadlynx -v
```

- The current stable CLI surface should expose top-level commands such as `devices`, `device`, `status`, `cc`, `cv`, `cp`, `pd`, `wifi`, `control`, `preset`, and `flash`.
- `loadlynx-devd --help` must expose both `serve` and `bridge-http`.
- If the requested workflow depends on a command that is absent, stop and report that the installed release does not support it. Do not invent commands, fall back to Web UI, or switch to source/developer instructions.

## Install Released Host Tools

- Download the latest stable Release unless the owner explicitly accepts a prerelease.
- Prefer the release installer script:
  - macOS/Linux: `install-loadlynx-host.sh`
  - Windows: `install-loadlynx-host.ps1`
- The installer downloads the platform archive, downloads `SHA256SUMS`, verifies the selected archive hash, installs into a user-owned directory, validates the binaries, and prints PATH guidance. It must not edit shell startup files or PATH automatically.
- If the installer fails in transport, switch to manual release-asset installation instead of silently continuing with an older binary.
- If installing manually, choose the platform archive:
  - Apple Silicon macOS: `loadlynx-host-tools-macos-aarch64.tar.gz`
  - Intel macOS: `loadlynx-host-tools-macos-x86_64.tar.gz`
  - Linux x86_64: `loadlynx-host-tools-linux-x86_64.tar.gz`
  - Windows x86_64: `loadlynx-host-tools-windows-x86_64.tar.gz`
- Manual installs must verify the archive against the release `SHA256SUMS` before extraction.
- On macOS or Linux:

```bash
mkdir -p "$HOME/.local/bin"
tar -xzf "$HOME/Downloads/loadlynx-host-tools-<platform>.tar.gz" -C "$HOME/.local/bin"
chmod +x "$HOME/.local/bin/loadlynx" "$HOME/.local/bin/loadlynx-devd"
```

- Verify after installation:

```bash
loadlynx --help
loadlynx-devd --help
loadlynx -v
```

## Connect Hardware

- Connection priority is saved device first, then direct USB/LAN discovery.
- Start the local daemon only when needed for long-running local USB sessions:

```bash
loadlynx-devd serve
```

- The current `loadlynx` CLI auto-manages its local control path. Do not try to pass legacy daemon URL flags unless a future released CLI explicitly documents them again.
- `loadlynx-devd bridge-http` is only for browser/Web/debug paths and must bind loopback only:

```bash
loadlynx-devd bridge-http --bind 127.0.0.1:30180
```

- Keep the bridge running while the browser uses it. Do not expose it on non-loopback interfaces.
- Do not edit project-local caches or developer selector files by hand.

## CLI Device Memory

- The CLI must be the source of remembered hardware. Do not rely on Web local storage, browser history, or project-local cache files for user workflows.
- Use the current released device commands:

```bash
loadlynx devices
loadlynx device list
loadlynx device add --name <name>
loadlynx device add --url http://<device-host-or-ip> --name <name>
loadlynx device use <id>
loadlynx device use --global <id>
loadlynx device use --clear
loadlynx device remove <id>
```

- `loadlynx devices --json` and `loadlynx device list --json` expose the persisted device list plus the current global default.
- `loadlynx device add` without extra selectors is interactive; in non-interactive automation it fails by design. Do not fake around that with manual JSON edits.
- Prefer using a saved `--device <id>` target instead of repeatedly retyping hostnames or URLs.

## LoadLynx Business Workflows

- Treat transport selection as preparation only. The user-facing job is operating the LoadLynx electronic load and PD sink safely through released CLI commands.
- Identity and status:

```bash
loadlynx devices
loadlynx status --device <id>
loadlynx control get --device <id>
```

- `loadlynx status --device <id> --json` is the primary released read path for live telemetry, fault flags, link state, temperatures, voltage, current, power, and enable state.
- `loadlynx control get --device <id> --json` reports the current mode, output-enabled state, active preset, and control snapshot.
- Electronic-load control:

```bash
loadlynx cc <target_i_ma> --device <id> [--min-v-mv <mv>] [--max-i-ma-total <ma>] [--max-p-mw <mw>]
loadlynx cv <target_v_mv> --device <id> [--min-v-mv <mv>] [--max-i-ma-total <ma>] [--max-p-mw <mw>]
loadlynx cp <target_p_mw> --device <id> [--min-v-mv <mv>] [--max-i-ma-total <ma>] [--max-p-mw <mw>]
loadlynx cc <target_i_ma> --device <id> --disable
loadlynx cv <target_v_mv> --device <id> --disable
loadlynx cp <target_p_mw> --device <id> --disable
loadlynx control set --device <id> --enable
loadlynx control set --device <id> --disable
```

- Use `--max-i-ma-total` and `--max-p-mw` as protection rails when running power tests.
- USB-PD operation:

```bash
loadlynx pd set --device <id> --mode fixed --object-pos <n>
loadlynx pd set --device <id> --mode pps --target-mv <mv> --i-req-ma <ma>
```

- Use `--allow-extended-voltage true|false` only when the user explicitly wants to cross the extended-voltage gate.
- Presets:

```bash
loadlynx preset list --device <id>
loadlynx preset set --device <id> --file <file>
loadlynx preset apply --device <id> <preset_id>
```

- Wi-Fi:

```bash
loadlynx wifi show --device <id>
loadlynx wifi set --device <id> --ssid <ssid> --psk <psk> [--wait] [--allow-insecure-lan-wifi]
loadlynx wifi clear --device <id> [--allow-insecure-lan-wifi]
```

- Never echo PSKs or secrets in chat, logs, screenshots, traces, or shell history.
- Firmware:

```bash
loadlynx flash digital --device <id> --artifact <artifact>
loadlynx flash analog --device <id> --artifact <artifact>
```

- The default flash path is dry-run first; `--no-dry-run` is the real flash gate.
- Real flash also requires `--confirm <target>` and `--acknowledge-non-project-firmware` when applicable. A successful flash command alone is not enough to claim the device is usable; verify identity and status afterward.

## External USB-C Source Validation

- Use LoadLynx as a generic validation sink for external USB-C source devices without embedding another project's workflow into this skill.
- Use a saved device target and prefer the saved USB/devd transport for operations that change LoadLynx state:

```bash
loadlynx status --device <id> --json
loadlynx pd set --device <id> --mode fixed --object-pos <n>
loadlynx pd set --device <id> --mode pps --target-mv <mv> --i-req-ma <ma>
loadlynx cv <target_v_mv> --device <id> --max-i-ma-total <ma> --max-p-mw <mw>
loadlynx cv <target_v_mv> --device <id> --disable
```

- `loadlynx pd set` is the PD sink stimulus path. `loadlynx cv <target_v_mv>` is the voltage-clamp load stimulus path for pushing a source device toward its current-limit or constant-current behavior.
- For external current-limit validation, treat the external DUT's own diagnostics as the primary verdict. LoadLynx terminal voltage, current, power, and PD contract state are auxiliary cross-checks that help explain the stimulus and observed operating point.
- A sink-side voltage drop during CV loading is not by itself a failure. If the external DUT reports that it is current-limiting as configured and has no relevant fault latch, the voltage drop can be expected evidence that the source entered constant-current regulation.
- Always disable the LoadLynx output after the run and verify status before claiming the setup is restored.

## Download Released Firmware

- Download firmware only from the chosen GitHub Release.
- Prefer a release-provided firmware catalog such as `loadlynx-firmware-catalog-v0.5.1.json`, plus the firmware files referenced by that catalog.
- Keep downloaded firmware and catalog files together in one user-owned folder so relative catalog paths can resolve.

## Web Serial

- GitHub Pages and release Web bundles are supported human browser paths for identity, status, control, Wi-Fi, diagnostics, and flash only when the browser exposes Web Serial.
- Web Serial saves identity/profile only and reconnects through browser-granted ports. It must not save OS port paths.
- Unsupported browsers must guide the user to Chrome/Edge or the released CLI/devd tools.

## Escalate Out

- Switch to `skills/loadlynx-developer-operations/SKILL.md` for source checkout, cloning, project builds, local devd builds, release workflow changes, probe or selector maintenance, HIL/debug sessions, or implementing missing CLI features.
- If a hardware operation is only available through Web UI and not through released `loadlynx`, stop and escalate to developer work to add and release the CLI capability.
- Do not use raw HTTP writes or source-tree commands to bypass missing released user functionality.

## Safety Checks

- Before enabling output, changing sink mode, or flashing firmware, verify the selected device identity and target.
- Treat command absence, link-down states, analog fault states, target mismatch, and hash mismatch as stop conditions.
- After changing settings, flashing, or configuring Wi-Fi, verify status and reconnect behavior before claiming success.
