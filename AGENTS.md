# Repository Guidelines

## Project Structure & Module Organization

- `firmware/analog/` — STM32G431 (Rust + Embassy) analog board firmware: control loop and telemetry reporting.
- `firmware/digital/` — ESP32‑S3 (Rust + esp‑hal) digital host firmware: local display, bridging, and UART link endpoint.
- `libs/` — shared crates; currently includes `libs/protocol` (`loadlynx-protocol`, MCU↔MCU frame format/CBOR/SLIP/CRC).
- `docs/` — hardware/firmware design notes (boards, interfaces, components, thermal, power, dev-notes, other-datasheets, etc.).
- `scripts/` — developer helpers (format hooks, test helpers, etc.).

## Build, Test, and Development Commands

- Analog firmware (STM32G431) — build: `just a-build` or `(cd firmware/analog && cargo build --release --target thumbv7em-none-eabihf)` (defaults to `PROFILE=release`).
- Analog firmware (STM32G431) — flash: build with `just a-build`, then use the `loadlynx` CLI + `loadlynx-devd` firmware flow for the approved saved device/analog target.
- Digital firmware (ESP32‑S3) — build: `just d-build` or `(cd firmware/digital && cargo +esp build --release)` (defaults to `PROFILE=release`).
- Digital firmware (ESP32‑S3) — flash: build with `just d-build`, then use `loadlynx flash digital --device <saved-id> ...` through `loadlynx-devd`.

- Format: `just fmt` or `cargo fmt --all`.

Prerequisites: Rust (embedded), `thumbv7em-none-eabihf` target, `probe-rs`; for ESP32‑S3, `espup`/Xtensa toolchain and `espflash` (invoked by `loadlynx-devd` firmware operations).

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

Firmware flash/reset/digital monitor/log operations are owned by the `loadlynx` CLI and `loadlynx-devd`. The daemon is the local hardware owner for USB CDC sessions, firmware flashing, reset, digital monitor, bounded logs, artifact verification, and target evidence. Do not route LoadLynx hardware work through an external MCU daemon.

Set the default digital USB CDC port through `loadlynx usb-port set digital <path>`, which writes the repo-local development digital port cache used by subsequent CLI/devd operations. Digital flash uses devd's lease-gated direct `espflash` path against the approved `.esp32-port` target. Analog flash/reset must also be exposed through `loadlynx` + `loadlynx-devd`; analog RTT/defmt monitor is a missing LoadLynx host-tool capability until implemented, not permission to use another daemon or route through digital USB monitor.

### Build-time versioning and boot logs

- Both firmwares still write version files during build: `tmp/analog-fw-version.txt` and `tmp/digital-fw-version.txt`, formatted as `"<crate> <semver> (profile <profile>, <git describe|unknown>, src 0xXXXXXXXXXXXXXXX)"`.
- On boot the first log lines include the version:
  - Analog: `LoadLynx analog firmware version: ...`
  - Digital: `LoadLynx digital firmware version: ...`
- When validating on hardware, compare these lines against the local version files to confirm the board is running the latest build.

### CLI/devd Operations

- Daemon control: normal CLI workflows should let `loadlynx` auto-start the sibling `loadlynx-devd serve` on the default IPC endpoint. Use `--ipc` / `--endpoint` only for explicit multi-instance or debugging overrides.
- Digital target cache: `just loadlynx usb-port set digital <path>` after the owner explicitly authorizes the exact ESP32-S3 USB CDC path.
- Device memory: `loadlynx devices`, `loadlynx device add`, `loadlynx device use <saved-id>`, and `loadlynx device remove <saved-id>`.
- Flash: `loadlynx flash digital --device <saved-id> --artifact <artifact-id> ...`; analog flash must use the corresponding `loadlynx flash analog ...` CLI/devd path once implemented.
- Reset/monitor/logs: use `loadlynx reset ...`, `loadlynx monitor ...`, and the devd bounded session/log APIs exposed through CLI/devd. Missing commands are product gaps.

When analyzing logs, always cross-check firmware boot/version evidence against local `tmp/{analog|digital}-fw-version.txt`.
Treat `probe-rs`/`espflash` failures as LoadLynx CLI/devd engineering issues first; adjust devd config or command arguments before escalating.

### CLI/Devd USB CDC Verification

- Follow `skills/loadlynx-user-operations/SKILL.md` for released owner-facing hardware operation on a user machine: CLI-only operation, USB/devd first, HTTP fallback second, CLI-saved device memory, GitHub Release host-tools installation, released firmware download, and released CLI workflows that the installed `loadlynx --help` actually supports.
- Treat `skills/loadlynx-developer-operations/SKILL.md` as a superset of the user skill: when a developer task includes ordinary LoadLynx hardware operation, inherit the user skill's CLI-only business workflows, USB-first/HTTP-fallback order, device-memory behavior, and command-availability gates, then add source checkout, Just, local builds, release maintenance, missing CLI feature implementation, calibration writes, reset/monitor, and HIL verification.
- Use `skills/loadlynx-release-decision/SKILL.md` before choosing release labels, declaring `type:none`, backfilling a release, or changing owner-facing released operation guidance in `README.md`, `AGENTS.md`, or LoadLynx operation skills. Owner-facing/user-facing operation contract changes require `type:patch` or higher even when the diff is docs/skill-only.
- User-facing CLI device memory is managed with `loadlynx devices`, `loadlynx device add|list|use|remove`, and `loadlynx status --device <id>`. It is stored in the user's OS config directory, not in the repository checkout or developer port/probe caches.
- The default digital USB port exists only as an Agent safety guardrail: it prevents CLI/devd work from guessing or touching the wrong ESP32-S3 USB CDC device.
- During development, set the default digital USB port only through `just loadlynx usb-port set digital <path>` after the owner explicitly authorizes the specific ESP32-S3 digital USB CDC path. This writes the repo-local development digital port memory used by subsequent CLI/devd operations.
- The CLI also supports human interactive use (`just loadlynx usb-port set` or `just loadlynx usb-port set digital`) with arrow-key selection over espflash-style serial port candidates, but an Agent must not use interactive candidate selection to bypass explicit owner authorization.
- Reuse the existing repo-local development digital port memory for this setting; do not introduce a replacement file or alternate memory scheme.
- The repo-local development digital port memory may retain legacy metadata lines; CLI/devd must treat only the approved port path line as the default USB port.
- Never change the repo-local development digital port memory or rerun `just loadlynx usb-port set digital ...` without explicit owner authorization for the specific path. Vague instructions like “继续”, “再试”, or “你自己处理” are not authorization to change USB ports.
- Authorization can be natural language. Do not require the owner to answer with a fixed phrase or command string; the authorized action and target only need to be unambiguous.
- If the repo-local development digital port memory is missing, stale, unreadable, or does not match the ESP32-S3 digital device the owner authorized, stop and ask the owner which USB port to use. Do not scan candidates and silently pick one.
- After the default digital USB port is set, start `loadlynx-devd` directly from this repository, for example through `just devd-serve ...`. Do not pass hardware port arguments to the devd startup command.
- For local Web development, start the Web app with `VITE_LOADLYNX_DEVD_URL` pointed at the active devd URL, but skill-driven hardware operations still use CLI.
- For CLI/devd firmware flashing, use the devd HTTP/CLI flash operation. The real ESP32-S3 digital flash path must hold a valid lease/session, resolve the selected artifact, verify hashes, and invoke direct `espflash` against the approved repo-local development digital port target. ELF artifacts use `espflash flash`; raw image artifacts require `flash_address` and use `espflash write-bin`.
- Prove real-device coverage through devd-owned USB CDC evidence: the selected candidate path, a lease/session, and decoded JSONL frames or successful `hello`/`get_identity`/`get_status` responses from the device.
- Mock identity, mock status, serial-open-only probes, or firmware dry-run target evidence are not sufficient to claim real-device CLI/devd verification.
- Do not call external daemon selector, flash, reset, monitor, or logs commands as part of LoadLynx hardware work. If a firmware/reset/monitor/log workflow is missing from CLI/devd, implement it in CLI/devd or report the product gap.

### Hardware Safety Guardrails (STRICT)

These rules exist to prevent an Agent from silently switching the owner's connected devices (wrong probe/port) during HIL work.

- **Never change cached ports/probes without explicit owner permission.**
  - Forbidden unless the owner explicitly authorizes the specific selector/cache change:
    - editing `.esp32-port` / `.stm32-port`
    - rerunning `just loadlynx usb-port set ...` for a different target
    - any direct selector/cache mutation equivalent
  - **Important:** vague instructions like “继续 / 你自己做 / 再试试 / finish it” are *NOT* permission to change ports/probes. They only permit using the currently cached/approved device selection.
- **Do not "try switching probes/ports" as a debugging tactic.**
  - If `flash/reset/monitor` fails, collect CLI/devd evidence and ask the owner which probe/port to use.
- **Avoid side-channel device selection changes.**
  - Do not edit or write to any repo-local device-selection cache files unless the owner explicitly asks.
- **Before any HIL action, echo the target device selection.**
  - Always state which approved `analog` probe / `digital` serial port from CLI/devd device memory will be used before running `flash/reset`.

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
