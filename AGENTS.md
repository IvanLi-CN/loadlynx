# Repository Guidelines

## Project Structure & Module Organization

- `firmware/analog/` — STM32G431 (Rust + Embassy) analog board firmware: control loop and telemetry reporting.
- `firmware/digital/` — ESP32‑S3 (Rust + esp‑hal) digital host firmware: local display, bridging, and UART link endpoint.
- `libs/` — shared crates; currently includes `libs/protocol` (`loadlynx-protocol`, MCU↔MCU frame format/CBOR/SLIP/CRC).
- `docs/` — hardware/firmware design notes (boards, interfaces, components, thermal, power, dev-notes, other-datasheets, etc.).
- `scripts/` — flash/build helpers (probe selection and flashing scripts such as `flash_g431.sh`).

## Build, Test, and Development Commands

- Analog firmware (STM32G431) — build: `make a-build` or `(cd firmware/analog && cargo build --release)` (defaults to `PROFILE=release`).
- Analog firmware (STM32G431) — flash + run (defmt RTT): `make a-run PROBE=<VID:PID[:SER]>` or `scripts/flash_g431.sh [release|dev] [PROBE=...]`.
- Digital firmware (ESP32‑S3) — build: `make d-build` or `(cd firmware/digital && cargo +esp build --release)` (defaults to `PROFILE=release`).
- Digital firmware (ESP32‑S3) — flash + monitor: `make d-run [PORT=/dev/tty.*]`.

> Analog and digital Makefiles default to `PROFILE=release`. Use `PROFILE=dev` only for short‑lived debugging; release images are recommended for hardware testing.

- Format: `make fmt` or `cargo fmt --all`.

Prerequisites: Rust (embedded), `thumbv7em-none-eabihf` target, `probe-rs`; for ESP32‑S3, `espup`/Xtensa toolchain and `espflash` used via `cargo +esp`.

## Coding Style & Naming Conventions

- Rust 2024 edition, 4‑space indent, no tabs. Run `cargo fmt --all` before commits.
- Naming: modules `snake_case`, types `PascalCase`, functions `snake_case`, constants `SCREAMING_SNAKE_CASE`.
- Prefer `no_std` and async Embassy patterns; avoid heap unless justified.
- Optional lint: `cargo clippy --all-targets --all-features -D warnings` (where applicable).

## Testing Guidelines

- MCU crates are `no_std`; host unit tests are limited. Add testable logic to `libs/` and use `cargo test` there.
- Firmware verification relies on on‑device logs: look for `info!("LoadLynx analog alive; ...")` on G431, `info!("LoadLynx digital alive; ...")` on S3, and periodic `info!("fast_status ok (count=...)")` once the UART link is running.
- Provide a short test plan in PRs (build, flash, logs, basic behavior observed).

## Agent Firmware Verification Workflow

Hardware-in-the-loop verification is now driven by the external `mcu-agentd` daemon + CLI (sibling checkout at `../mcu-agentd`), with per-project config `mcu-agentd.toml` and cached selectors in `.esp32-port` / `.stm32-port`. Prefer the `just` wrappers below.

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

### Legacy scripts (fallback only)

- `scripts/agent_verify_{analog,digital}.sh` and `scripts/agent_dual_monitor.sh` remain available but should only be used when the daemon is unavailable or special arguments are needed. Default to `mcu-agentd` for all routine work.

### Expectations

- Prefer `mcu-agentd` for build/flash/reset/log capture; do not ask the user to run scripts manually. If a port/probe is unknown,在获得用户明确同意后再使用 `just agentd selector set ...` 或 `just agentd selector list ...`，不得擅自调用或更换连接设备。
- `agentd` will first use its cached port/probe or auto-select a likely candidate; only if it cannot resolve a usable port/probe and returns an error should the user be asked to confirm or provide one.
- When analyzing logs, always cross-check `LOADLYNX_FW_VERSION` against local `tmp/{analog|digital}-fw-version.txt`.
- Treat probe-rs/espflash failures as repository engineering issues first; adjust config or command arguments before escalating.

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
  - Do not edit or write to any device-selection cache files (e.g. `.stm32-port`, `.esp32-port`) unless the owner explicitly asks.
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

- Chip/runners are configured via crate `.cargo/config.toml`; adjust only if board/chip changes.
- Do not commit secrets or machine‑specific paths; prefer flags/env (e.g., `--port /dev/tty.*`).
- Probe‑RS chip may vary by package; verify `STM32G431CB` before flashing.

## Documentation & Datasheet Localization

- The `docs/` tree hosts design notes (`*.md`), domain-specific subfolders (e.g., `fans/`, `heatsinks/`), and vendor material converted into Markdown under `docs/other-datasheets/`.
- When new datasheets or external PDFs are needed, prefer converting them to Markdown via `mineru` (or an equivalent tool). Store the Markdown in `docs/other-datasheets/` using kebab-case filenames and keep the original title/version at the top of the file.
- Place extracted images/figures in `docs/assets/<document-name>/` and reference them from Markdown via relative paths such as `../assets/<document-name>/<file>.jpg`. Avoid committing raw PDFs or hotlinking external resources.
- If localized material is specific to a board or subsystem, add a dedicated summary under `docs/` and link it from the top-level README or relevant index so the team can discover it easily.
