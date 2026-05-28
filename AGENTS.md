# Repository Guidelines

## Project Structure & Module Organization

- `firmware/analog/` — STM32G431 (Rust + Embassy) analog board firmware: control loop and telemetry reporting.
- `firmware/digital/` — ESP32‑S3 (Rust + esp‑hal) digital host firmware: local display, bridging, and UART link endpoint.
- `libs/` — shared crates; currently includes `libs/protocol` (`loadlynx-protocol`, MCU↔MCU frame format/CBOR/SLIP/CRC).
- `docs/` — hardware/firmware design notes (boards, interfaces, components, thermal, power, dev-notes, other-datasheets, etc.).
- `scripts/` — developer helpers (format hooks, test helpers, etc.).

## Build, Test, and Development Commands

- Analog firmware (STM32G431) — build: `just a-build` or `(cd firmware/analog && cargo build --release --target thumbv7em-none-eabihf)` (defaults to `PROFILE=release`).
- Analog firmware (STM32G431) — flash: `just agentd flash analog` (build first).
- Digital firmware (ESP32‑S3) — build: `just d-build` or `(cd firmware/digital && cargo +esp build --release)` (defaults to `PROFILE=release`).
- Digital firmware (ESP32‑S3) — flash: `just agentd flash digital` (build first).

- Format: `just fmt` or `cargo fmt --all`.

Prerequisites: Rust (embedded), `thumbv7em-none-eabihf` target, `probe-rs`; for ESP32‑S3, `espup`/Xtensa toolchain and `espflash` (both invoked by `mcu-agentd`).

## Coding Style & Naming Conventions

- Rust 2024 edition, 4‑space indent, no tabs. Run `cargo fmt --all` before commits.
- Naming: modules `snake_case`, types `PascalCase`, functions `snake_case`, constants `SCREAMING_SNAKE_CASE`.
- Prefer `no_std` and async Embassy patterns; avoid heap unless justified.
- Optional lint: `cargo clippy --all-targets --all-features -D warnings` (where applicable).

## Testing Guidelines

- MCU crates are `no_std`; host unit tests are limited. Add testable logic to `libs/` and use `cargo test` there.
- Firmware verification relies on on‑device logs: look for `info!("LoadLynx analog alive; ...")` on G431, `info!("LoadLynx digital alive; ...")` on S3, and periodic `info!("fast_status ok (count=...)")` once the UART link is running.
- Provide a short test plan in PRs (build, flash, logs, basic behavior observed).

## Firmware Flash/Reset/Monitor Workflow

Firmware flash/reset/monitor operations are driven by the external `mcu-agentd` daemon + CLI (sibling checkout at `../mcu-agentd`), with per-project config `mcu-agentd.toml` and repo-local development selector caches. Prefer the `just` wrappers below for those firmware operations only.

CLI/devd USB CDC verification is separate from `mcu-agentd`. It must use `tools/loadlynx-devd` directly against the intended ESP32-S3 digital USB CDC device. Set the default digital hardware USB port through `loadlynx usb-port set digital <path>`, which reuses the repo-local development digital port cache; do not call or change `mcu-agentd selector` state for CLI/devd verification. When the current task is the CLI/devd firmware flow, real ESP32-S3 digital flash must also go through devd's lease-gated direct `espflash` path using the approved repo-local development digital port target, not `just agentd flash digital`.

### Build-time versioning and boot logs

- Both firmwares still write version files during build: `tmp/analog-fw-version.txt` and `tmp/digital-fw-version.txt`, formatted as `"<crate> <semver> (profile <profile>, <git describe|unknown>, src 0xXXXXXXXXXXXXXXX)"`.
- On boot the first log lines include the version:
  - Analog: `LoadLynx analog firmware version: ...`
  - Digital: `LoadLynx digital firmware version: ...`
- When validating on hardware, compare these lines against the local version files to confirm the board is running the latest build.

### MCU Agentd (recommended)

- Install/upgrade (recommended): `just agentd-init` (installs `mcu-agentd`/`mcu-managerd` from `../mcu-agentd` into your cargo bin).
- Daemon control: `just agentd-start` / `just agentd-status` / `just agentd-stop` (equivalent to `mcu-agentd {start|status|stop}`; falls back to `cargo run --manifest-path $MCU_AGENTD_MANIFEST ...` if the binary isn't installed). Runtime state (socket/lock/logs) lives under `.mcu-agentd/`.
- Selector cache (ports/probes):
  - Set: `just agentd selector set digital /dev/tty.usbserial-xxxx`; `just agentd selector set analog 0483:3748:SERIAL`（无连字符版本）。在修改端口/探针前必须征得用户明确批准，严禁擅自切换连接设备。
  - Get: `just agentd-get-port digital` / `analog` (wrapper for `just agentd selector get ...`).
- Flash: `just agentd flash digital` or `just agentd flash analog` (uses `artifact_elf` from `mcu-agentd.toml`; build first).
- Reset: `just agentd reset digital|analog` (reset only, no flash).
- Monitor/attach: `just agentd monitor digital` or `just agentd monitor analog`; use `--from-start` and/or `--reset` as needed.
- Log query: `just agentd logs all --tail 200 --sessions` aggregates meta + recent sessions. Logs live under `.mcu-agentd/` (see `../mcu-agentd/docs/design/config.md` for the layout).

### Notes

- Non-devd firmware flash/reset/monitor must go through `mcu-agentd` (`just agentd ...`).
- CLI/devd control-plane verification must not go through `mcu-agentd`; use the `loadlynx-devd` HTTP API plus the USB CDC JSONL protocol documented in `docs/interfaces/usb-cdc-jsonl-bridge.md`.

### Expectations

- Prefer `mcu-agentd` for firmware flash/reset/log capture; do not ask the user to run scripts manually. If a port/probe is unknown,在获得用户明确同意后再使用 `just agentd selector set ...` 或 `just agentd selector list ...`，不得擅自调用或更换连接设备。
- `agentd` will first use its cached port/probe or auto-select a likely candidate; only if it cannot resolve a usable port/probe and returns an error should the user be asked to confirm or provide one.
- When analyzing logs, always cross-check `LOADLYNX_FW_VERSION` against local `tmp/{analog|digital}-fw-version.txt`.
- Treat probe-rs/espflash failures as repository engineering issues first; adjust config or command arguments before escalating.

### CLI/Devd USB CDC Verification

- Follow `skills/loadlynx-user-operations/SKILL.md` for released owner-facing hardware operation on a user machine: CLI-only operation, USB/devd first, HTTP fallback second, CLI-saved hardware memory, GitHub Release host-tools installation, released firmware download, and released CLI workflows that the installed `loadlynx --help` actually supports.
- Follow `skills/loadlynx-developer-operations/SKILL.md` for source checkout/clone, Just-based devd/CLI workflows, local firmware builds, release maintenance, missing CLI feature implementation, calibration writes, reset/monitor, and HIL verification.
- User-facing CLI hardware memory is managed with `loadlynx hardware available/recent/path/list/save/forget` and `loadlynx status --hardware <id>`. It is stored in the user's OS config directory, not in the repository checkout or developer port/probe caches.
- The default digital USB port exists only as an Agent safety guardrail: it prevents CLI/devd work from guessing or touching the wrong ESP32-S3 USB CDC device.
- During development, set the default digital USB port only through `just loadlynx usb-port set digital <path>` after the owner explicitly approves the exact ESP32-S3 digital USB CDC path. This writes the repo-local development digital port memory used by subsequent CLI/devd operations.
- The CLI also supports human interactive use (`just loadlynx usb-port set` or `just loadlynx usb-port set digital`) with arrow-key selection over espflash-style serial port candidates, but an Agent must not use interactive candidate selection to bypass owner approval of the exact port.
- Reuse the existing repo-local development digital port memory for this setting; do not introduce a replacement file or alternate memory scheme.
- The repo-local development digital port memory may retain mcu-agentd-compatible metadata lines; CLI/devd must treat only the approved port path line as the default USB port.
- Never change the repo-local development digital port memory or rerun `just loadlynx usb-port set digital ...` without explicit owner approval for the new exact path. Vague instructions like “继续”, “再试”, or “你自己处理” are not approval to change USB ports.
- If the repo-local development digital port memory is missing, stale, unreadable, or does not match the ESP32-S3 digital device the owner approved, stop and ask the owner which USB port to use. Do not scan candidates and silently pick one.
- After the default digital USB port is set, start `loadlynx-devd` directly from this repository, for example through `just devd-serve ...`. Do not pass hardware port arguments to the devd startup command.
- For local Web development, start the Web app with `VITE_LOADLYNX_DEVD_URL` pointed at the active devd URL, but skill-driven hardware operations still use CLI.
- For CLI/devd firmware flashing, use the devd HTTP/CLI flash operation. The real ESP32-S3 digital flash path must hold a valid lease/session, resolve the selected artifact, verify hashes, and invoke direct `espflash` against the approved repo-local development digital port target. ELF artifacts use `espflash flash`; raw image artifacts require `flash_address` and use `espflash write-bin`.
- Prove real-device coverage through devd-owned USB CDC evidence: the selected candidate path, a lease/session, and decoded JSONL frames or successful `hello`/`get_identity`/`get_status` responses from the device.
- Mock identity, mock status, serial-open-only probes, or firmware dry-run target evidence are not sufficient to claim real-device CLI/devd verification.
- Do not call `just agentd selector set`, `just agentd flash`, `just agentd reset`, or `just agentd monitor` as part of CLI/devd control-plane verification. If the task explicitly includes CLI/devd digital firmware flashing, stay on devd's direct `espflash` path; `mcu-agentd` remains for non-devd firmware workflows and analog/probe operations.

### Hardware Safety Guardrails (STRICT)

These rules exist to prevent an Agent from silently switching the owner's connected devices (wrong probe/port) during HIL work.

- **Never change cached ports/probes without explicit owner permission.**
  - Forbidden unless the owner explicitly approves the *exact command*:
    - `just agentd selector set analog ...`
    - `just agentd selector set digital ...`
    - any direct `mcu-agentd selector set ...` equivalent
  - **Important:** vague instructions like “继续 / 你自己做 / 再试试 / finish it” are *NOT* permission to change ports/probes. They only permit using the currently cached/approved device selection.
- **Do not "try switching probes/ports" as a debugging tactic.**
  - If `flash/reset/monitor` fails, collect evidence (logs + `just agentd-get-port ...` + `just agentd selector list ...`) and ask the owner which probe/port to use.
- **Avoid side-channel device selection changes.**
  - Do not edit or write to any repo-local device-selection cache files unless the owner explicitly asks.
- **Before any HIL action, echo the target device selection.**
  - Always state which `analog` probe selector / `digital` serial port will be used (as returned by `just agentd-get-port ...`) before running `flash/reset`.

## Commit & Pull Request Guidelines

- Use Conventional Commits: `feat:`, `fix:`, `docs:`, `chore:`, etc. Examples from history: `feat: scaffold ...`, `docs: add Markdown datasheets ...`.
- Keep commits small and focused; run format/lint before committing.
- PR requirements:
  - Clear description and rationale; link issues if any.
  - Build instructions and test plan (G431/S3), plus relevant logs/screenshots.
  - Note hardware used (board, probe, serial port).

## Security & Configuration Tips

- Chip/targets are configured via crate `.cargo/config.toml`; adjust only if board/chip changes.
- Do not commit secrets or machine‑specific paths; prefer flags/env (e.g., `--port /dev/tty.*`).
- Probe‑RS chip may vary by package; verify `STM32G431CB` before flashing.
- Digital firmware Wi‑Fi credentials MUST come from the repo-root `.env` file (copy from `.env.example`); do not override `DIGITAL_WIFI_*` ad-hoc (e.g., `DIGITAL_WIFI_SSID=dummy`) unless the owner explicitly approves it.

## Documentation & Datasheet Localization

- The `docs/` tree hosts design notes (`*.md`), domain-specific subfolders (e.g., `fans/`, `heatsinks/`), and vendor material converted into Markdown under `docs/other-datasheets/`.
- When new datasheets or external PDFs are needed, prefer converting them to Markdown via `mineru` (or an equivalent tool). Store the Markdown in `docs/other-datasheets/` using kebab-case filenames and keep the original title/version at the top of the file.
- Place extracted images/figures in `docs/assets/<document-name>/` and reference them from Markdown via relative paths such as `../assets/<document-name>/<file>.jpg`. Avoid committing raw PDFs or hotlinking external resources.
- If localized material is specific to a board or subsystem, add a dedicated summary under `docs/` and link it from the top-level README or relevant index so the team can discover it easily.
