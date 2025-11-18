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
    - `make reset-attach [PORT=...] [PROFILE=...]` → `espflash reset` followed by `espflash monitor` (via the `attach` target), providing a simple manual "reset + log" helper without any built-in timeout.
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
  - Analog (STM32G431):
    - Decide whether to flash by comparing:
      - `tmp/analog-fw-version.txt` (current build) vs
      - `tmp/analog-fw-last-flashed.txt` (last version that this workflow successfully flashed).
    - If the version changes or is unknown:
      - Flash once using the `flash` target (`probe-rs download`), without starting a long‑running log session.
      - Update `tmp/analog-fw-last-flashed.txt`.
  - Digital (ESP32‑S3):
    - Because `espflash flash` will, by default, skip regions whose contents already match the image (unless `--no-skip` is passed), the ESP32‑S3 side does not need an additional version gate to protect flash endurance.
    - The digital version files `tmp/digital-fw-version.txt` / `digital-fw-last-flashed.txt` are mainly for correlating logs with builds, not for deciding whether to flash.
    - In simple scenarios, agents may call `make d-run` directly and rely on espflash to decide when a flash write is actually required.
  - When only logs are needed without re-flashing (especially on the analog side), perform a `reset-attach` session to capture logs from a fresh boot.
  - In automated scenarios, logging sessions should be wrapped with timeouts in scripts to avoid indefinitely stuck probes/serial monitors.

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
    - `scripts/agent_verify_analog.sh --no-log` when only build/flash are desired (e.g. before a dual-board monitor session).
  - Inputs:
    - `--timeout SECONDS` (optional; default 20).
    - `--profile {release|dev}` (optional; default `release`).
    - Any extra arguments are forwarded as `make` variables to the `build`, `flash`, and `reset-attach` targets.
  - Output:
    - Non‑interactive, bounded run with defmt logs in `tmp/agent-logs/analog-*.log`, including:
      - Version line (`LoadLynx analog firmware version: ...`).
      - Alive line and subsequent FAST_STATUS streaming logs.

### Digital agent workflow (ESP32‑S3)

- Note: For ESP32‑S3, flash endurance is less critical because `espflash flash` will, by default, skip regions whose contents already match the image. For routine “build + flash + log” verification, agents can call `make d-run` directly, configuring `PORT` / `BAUD` / `LOGFMT` via environment variables, without an extra version gate.
- Script: `scripts/agent_verify_digital.sh` (used when structured log capture and bounded automated verification are desired).
- Default behavior:
  - Builds `firmware/digital` in the `release` profile via `make build` (`PROFILE=release` is hard-coded in the agent script).
  - Selects a serial port non‑interactively using `scripts/ensure_esp32_port.sh`.
  - Runs `make -C firmware/digital run` (equivalent to `make d-run`), passing `ESPFLASH_ARGS="--ignore_app_descriptor --non-interactive --skip-update-check"` so that espflash:
    - Skips writing regions whose contents have not changed.
    - Does not prompt for interactive input.
    - Does not perform online update checks.
  - Wraps the `make run` call in `run_with_timeout` to ensure the logging session is time‑bounded.
  - Logs are streamed to the console and written to `tmp/agent-logs/digital-*.log`.

### Dual-board agent workflow (analog + digital)

For scenarios where both boards need to run concurrently and their interactions are important, agents should:

- Preparation phase (can be sequential; build time is not bounded):
  - First update and, if needed, flash the digital firmware **without** starting a logging session:
    - `scripts/agent_verify_digital.sh --no-log`
  - Then update/flash the analog firmware, also without logging:
    - `scripts/agent_verify_analog.sh --no-log`
- Dual reset-attach phase (both boards monitored concurrently):
  - Use `scripts/agent_dual_monitor.sh` to start simultaneous reset-attach sessions:
    - `scripts/agent_dual_monitor.sh --timeout 60`
  - Behavior:
    - Selects the ESP32‑S3 serial port non‑interactively (or uses `PORT` env if set).
    - Selects the STM32G431 probe non‑interactively (or uses `PROBE` / `.stm32-probe` if set).
    - Starts **digital** reset-attach first (`make -C firmware/digital reset-attach ...`), then **analog** reset-attach (`make -C firmware/analog reset-attach ...`).
    - Each session is bounded by `--timeout` seconds.
    - Returns immediately after spawning both sessions.
    - Writes logs and PID files to:
      - `tmp/agent-logs/digital-dual-<timestamp>.log` / `.pid`
      - `tmp/agent-logs/analog-dual-<timestamp>.log` / `.pid`
- Log analysis:
  - Agents should:
    - Sleep for an appropriate duration (e.g. `sleep 60`) according to `--timeout`.
    - Read both logs and correlate behavior (e.g. digital waiting for analog to come up, link establishment, `fast_status ok` cadence).
    - Optionally, use the `.pid` files to terminate remaining sessions early if necessary.

### Agent expectations

- Agents should:
  - Run `scripts/agent_verify_analog.sh` / `scripts/agent_verify_digital.sh` directly in the Codex environment instead of asking the user to execute scripts manually. Only when a script fails should agents request additional information (such as `PROBE` / `PORT`).
  - Prefer `scripts/agent_verify_analog.sh` / `scripts/agent_verify_digital.sh` over raw `make a-run` / `make d-run` to:
    - Avoid unnecessary flashes.
    - Ensure bounded runtime and stable log capture.
    - Get explicit firmware version information for correlation with local builds.
  - Fall back to manual `make a-reset-attach` / `make d-reset-attach` only for one-off, custom debugging scenarios that cannot be covered by the agent scripts, and only when the user explicitly requests this.
  - When analyzing logs, always confirm that the reported `LOADLYNX_FW_VERSION` matches the expected one from `tmp/{analog|digital}-fw-version.txt` before trusting behavior as belonging to the latest build.
  - When tools such as `probe-rs` / `espflash` fail during `flash` / `reset-attach` due to configuration or image format issues (for example chip mismatch or app descriptor checks), agents should first treat this as an engineering/configuration issue in this repository: consult official documentation, adjust `Makefile` / `ESPFLASH_ARGS` / build scripts, and make `scripts/agent_verify_*.sh` work reliably with the current toolchain, rather than pushing execution back to the user.

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
