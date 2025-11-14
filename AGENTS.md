# Repository Guidelines

## Project Structure & Module Organization

- `firmware/analog/` — STM32G431 (Rust + Embassy) analog board firmware：控制环路与遥测上报。
- `firmware/digital/` — ESP32‑S3 (Rust + esp‑hal) digital host firmware：本地显示、桥接与 UART 链路终端。
- `libs/` — shared crates；当前包含 `libs/protocol`（`loadlynx-protocol`，负责 MCU↔MCU 帧格式/CBOR/SLIP/CRC）。
- `docs/` — hardware/firmware design notes（boards、interfaces、components、thermal、power、dev-notes、other-datasheets 等）。
- `scripts/` — flash/build helpers（probe 选择与烧录脚本，如 `flash_g431.sh` / `flash_s3.sh`）。

## Build, Test, and Development Commands

- Analog firmware (STM32G431) — build: `make a-build` or `(cd firmware/analog && cargo build --release)` (defaults to `PROFILE=release`).
- Analog firmware (STM32G431) — flash + run (defmt RTT): `make a-run PROBE=<VID:PID[:SER]>` or `scripts/flash_g431.sh [release|dev] [PROBE=...]`.
- Digital firmware (ESP32‑S3) — build: `make d-build` or `(cd firmware/digital && cargo +esp build --release)` (defaults to `PROFILE=release`).
- Digital firmware (ESP32‑S3) — flash + monitor: `make d-run [PORT=/dev/tty.*]` or `scripts/flash_s3.sh [--release] [--port /dev/tty.*] [EXTRA_MAKE_VARS...]`.

> Analog and digital Makefiles default to `PROFILE=release`. Use `PROFILE=dev` (or omit `--release` with `scripts/flash_s3.sh`) only for short‑lived debugging; release images are recommended for hardware testing.

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

This section defines a non‑interactive workflow for agents to build, flash (only when needed), and verify firmware on real hardware, with bounded log collection suitable for automated analysis.

### Build‑time versioning and on‑device logs

- Both firmware crates embed a build identifier via `build.rs`:
  - `firmware/analog/build.rs` writes `LOADLYNX_FW_VERSION` and a copy to `tmp/analog-fw-version.txt`.
  - `firmware/digital/build.rs` writes `LOADLYNX_FW_VERSION` and a copy to `tmp/digital-fw-version.txt`.
- The version string has the form:
  - `"<crate> <semver> (profile <profile>, <git describe|unknown>, src 0xXXXXXXXXXXXXXXX)"`.
  - It changes whenever:
    - Git commit or dirty state changes (via `git describe --tags --dirty --always`), or
    - Any `src/*.rs` file in the crate changes (via a simple source hash).
- On startup, firmware logs the current version as early as possible:
  - Analog (`firmware/analog/src/main.rs`):
    - `info!("LoadLynx analog firmware version: {}", FW_VERSION);`
    - Followed by the existing `info!("LoadLynx analog alive; streaming mock FAST_STATUS frames ...")`.
  - Digital (`firmware/digital/src/main.rs`):
    - `info!("LoadLynx digital firmware version: {}", FW_VERSION);`
    - Followed by the existing `info!("LoadLynx digital alive; initializing local peripherals");`
- Agents should treat the version line as the authoritative runtime identity and cross‑check it against `tmp/{analog|digital}-fw-version.txt` when deciding whether the board is running the latest build.

### New Make targets (digital reset/attach/flash)

- Analog (`firmware/analog/Makefile`):
  - Adds `flash` target:
    - `make flash [PROBE=...] [PROFILE=...]` → `probe-rs download` of the ELF only (no logging session).
  - Existing `reset-attach` remains:
    - `make reset-attach [PROBE=...] [PROFILE=...]` → `probe-rs reset` + `attach` (defmt RTT logging).
- Digital (`firmware/digital/Makefile`):
  - New targets:
    - `make flash [PORT=...] [PROFILE=...]` → `espflash flash` of the ELF only (no monitor).
    - `make reset` → `espflash reset` (no monitor).
    - `make reset-attach [PORT=...] [PROFILE=...]` → `reset` + `monitor --elf ...` (defmt logging).
- Root `Makefile`:
  - Adds:
    - `make d-reset [PORT=...] [PROFILE=...]`
    - `make d-reset-attach [PORT=...] [PROFILE=...]`
  - These wrap the digital `reset` / `reset-attach` targets and are convenient for manual use; the agent scripts below call the per‑crate Makefiles directly.

### Log capture and flash minimization for agents

- Logs for automated analysis are written under:
  - `tmp/agent-logs/analog-YYYYmmdd-HHMMSS.log`
  - `tmp/agent-logs/digital-YYYYmmdd-HHMMSS.log`
- High‑level behavior:
  - Always build first (to ensure the version metadata is current).
  - Decide whether to flash by comparing:
    - `tmp/{analog|digital}-fw-version.txt` (current build) vs
    - `tmp/{analog|digital}-fw-last-flashed.txt` (last version that this workflow successfully flashed).
  - If the version changed or is unknown:
    - Flash once using a `flash` target (`probe-rs download` / `espflash flash`), without starting a long‑running log session.
    - Update `tmp/{analog|digital}-fw-last-flashed.txt`.
  - In all cases, perform a `reset-attach` session to capture logs from a fresh boot.
  - The logging session is bounded in time to avoid indefinitely stuck probes/serial monitors.

### Analog agent workflow (STM32G431)

- Script: `scripts/agent_verify_analog.sh`
- Default behavior:
  - Builds `firmware/analog` via `make build` with `PROFILE=${PROFILE:-release}` and `DEFMT_LOG=${DEFMT_LOG:-info}`.
  - Reads the current build identity from `tmp/analog-fw-version.txt`.
  - Compares against `tmp/analog-fw-last-flashed.txt`:
    - If different or missing → flash via `make -C firmware/analog flash`.
    - If identical → skip flashing to preserve flash endurance.
  - Selects a debug probe non‑interactively:
    - Prefers `PROBE` env, then `PORT` env alias, then cached `.stm32-probe`, then:
      - unique ST‑Link, else unique probe overall.
    - If still ambiguous → exits with an error and asks for an explicit `PROBE` or for the user to run `scripts/select_stm32_probe.sh` once interactively (agents themselves must not call interactive selectors).
  - Runs `make -C firmware/analog reset-attach` with the chosen probe and captures logs for a bounded window.
  - Uses a timeout (default 20s) implemented via `gtimeout`/`timeout` if available, otherwise a manual `sleep+kill` wrapper.
  - Logs are streamed to the console and simultaneously written to `tmp/agent-logs/analog-*.log`.
- Usage for agents:
  - Recommended:
    - `scripts/agent_verify_analog.sh` (assumes a unique probe or pre‑selected `.stm32-probe`).
    - `PROBE=<VID:PID[:SER]> scripts/agent_verify_analog.sh --timeout 30` for explicit selection.
  - Inputs:
    - `--timeout SECONDS` (optional; default 20).
    - `--profile {release|dev}` (optional; default `release`).
    - Any extra arguments are forwarded as `make` variables to the `build`, `flash`, and `reset-attach` targets.
  - Output:
    - Non‑interactive, bounded run with defmt logs in `tmp/agent-logs/analog-*.log`, including:
      - Version line (`LoadLynx analog firmware version: ...`).
      - Alive line and subsequent FAST_STATUS streaming logs.

### Digital agent workflow (ESP32‑S3)

- Script: `scripts/agent_verify_digital.sh`
- Default behavior:
  - Builds `firmware/digital` via `make build` with `PROFILE=${PROFILE:-release}`.
  - Reads the current build identity from `tmp/digital-fw-version.txt`.
  - Compares against `tmp/digital-fw-last-flashed.txt`:
    - If different or missing → flash via `make -C firmware/digital flash`.
    - If identical → skip flashing and only reset+attach.
  - Selects a serial port non‑interactively:
    - If `PORT` is set → use it directly.
    - Else, runs `espflash list-ports` and:
      - If exactly one `/dev/cu.*` entry → use it.
      - Else, if exactly one `/dev/*` entry overall → use it.
      - Else → error and ask for an explicit `PORT` (agent should surface this to the user).
  - Runs `make -C firmware/digital reset-attach` with the chosen port and captures logs for a bounded window.
  - Uses the same timeout strategy as the analog script (default 20s).
  - Logs are streamed to the console and written to `tmp/agent-logs/digital-*.log`.
- Usage for agents:
  - Recommended:
    - `PORT=/dev/cu.usbmodemXXXX scripts/agent_verify_digital.sh`
    - `scripts/agent_verify_digital.sh --timeout 30` if there is a single suitable port and auto‑detection works.
  - Inputs:
    - `--timeout SECONDS` (optional; default 20).
    - `--profile {release|dev}` (optional; default `release`).
    - Any extra arguments are forwarded as `make` variables to the `build`, `flash`, and `reset-attach` targets (e.g. `BAUD=921600 LOGFMT=defmt`).
  - Output:
    - Non‑interactive, bounded `espflash monitor` session with defmt logs in `tmp/agent-logs/digital-*.log`, including:
      - Version line (`LoadLynx digital firmware version: ...`).
      - Alive line and subsequent UART/link status logs (e.g. `fast_status ok (count=...)`).

### Agent expectations

- Agents should:
  - Run `scripts/agent_verify_analog.sh` / `scripts/agent_verify_digital.sh` **直接在 Codex 环境中**执行，不要把“跑脚本”这一步转交给用户；只有在脚本明确失败时，才向用户请求额外信息（如 `PROBE` / `PORT`）。
  - Prefer `scripts/agent_verify_analog.sh` / `scripts/agent_verify_digital.sh` over raw `make a-run` / `make d-run` to:
    - Avoid unnecessary flashes.
    - Ensure bounded runtime and stable log capture.
    - Get explicit firmware version information for correlation with local builds.
  - Fall back to manual `make a-reset-attach` / `make d-reset-attach` only when自定义、一次性的调试需求无法通过 agent 脚本覆盖时，由用户显式要求时再使用。
  - When analyzing logs, always confirm that the reported `LOADLYNX_FW_VERSION` matches the expected one from `tmp/{analog|digital}-fw-version.txt` before trusting behavior as belonging to the latest build.
  - When `probe-rs` / `espflash` 等工具因配置或镜像格式报错（如芯片不匹配、app descriptor 检查等）导致 `flash` / `reset-attach` 失败时，优先视为**需要修改本仓库配置的工程问题**：查阅官方文档、调整 `Makefile` / `ESPFLASH_ARGS` / 构建脚本等，使 `scripts/agent_verify_*.sh` 在当前工具链版本下可以自动跑通，而不是简单把执行责任推回给用户。

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
