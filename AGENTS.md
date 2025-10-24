# Repository Guidelines

## Project Structure & Module Organization
- `firmware/analog/` — STM32G431 (Rust + Embassy) core control loop.
- `firmware/digital/` — ESP32‑S3 (Rust + esp‑hal) UI/bridge.
- `libs/` — shared drivers/protocols (placeholders).
- `docs/` — design notes and datasheets.
- `scripts/` — flash/build helpers.

## Build, Test, and Development Commands
- G431 build: `make g431-build` or `(cd firmware/analog && cargo build)`.
- G431 run/flash (defmt RTT): `make g431-run` or `scripts/flash_g431.sh` after build.
- S3 build: `make s3-build` or `(cd firmware/digital && cargo +esp build)`.
- S3 flash/monitor: `scripts/flash_s3.sh [--release] [--port /dev/tty.*]`.
- Format: `make fmt` or `cargo fmt --all`.

Prerequisites: Rust (embedded), `thumbv7em-none-eabihf` target, `probe-rs`; for ESP32‑S3, `espup` toolchain and `espflash` via `cargo +esp`.

## Coding Style & Naming Conventions
- Rust 2024 edition, 4‑space indent, no tabs. Run `cargo fmt --all` before commits.
- Naming: modules `snake_case`, types `PascalCase`, functions `snake_case`, constants `SCREAMING_SNAKE_CASE`.
- Prefer `no_std` and async Embassy patterns; avoid heap unless justified.
- Optional lint: `cargo clippy --all-targets --all-features -D warnings` (where applicable).

## Testing Guidelines
- MCU crates are `no_std`; host unit tests are limited. Add testable logic to `libs/` and use `cargo test` there.
- Firmware verification relies on on‑device logs: look for `info!("tick")` on both targets.
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
