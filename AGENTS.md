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
