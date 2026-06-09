set shell := ["/bin/sh", "-c"]

# Default: list available recipes
default:
  @just --list

# Install root Node.js dependencies used by repo policy/tooling checks.
deps-root:
  npm ci

# Install Web dependencies from the committed Bun lockfile.
deps-web:
  (cd web && bun ci)

# Install browser binaries needed by Playwright-backed checks.
deps-web-browsers:
  (cd web && node scripts/run-playwright.mjs install)

# Install the default local dependencies for repo and Web checks.
deps:
  just deps-root
  just deps-web

# Format Rust crates and the Web workspace in-place.
fmt:
  just _require-web-deps
  just _require-esp-toolchain
  cargo fmt --manifest-path libs/protocol/Cargo.toml --all
  cargo fmt --manifest-path libs/calibration-format/Cargo.toml --all
  cargo fmt --manifest-path libs/led-effects/Cargo.toml --all
  cargo fmt --manifest-path libs/screen-power/Cargo.toml --all
  cargo fmt --manifest-path tools/loadlynx-devd/Cargo.toml --all
  cargo fmt --manifest-path tools/ui-mock/Cargo.toml --all
  cargo fmt --manifest-path firmware/analog/Cargo.toml -p analog
  cargo +esp fmt --manifest-path firmware/digital/Cargo.toml -p digital
  (cd web && node ./node_modules/@biomejs/biome/bin/biome format --write .)

# Check formatting without mutating files.
fmt-check:
  just _require-web-deps
  just _require-esp-toolchain
  cargo fmt --manifest-path libs/protocol/Cargo.toml --all -- --check
  cargo fmt --manifest-path libs/calibration-format/Cargo.toml --all -- --check
  cargo fmt --manifest-path libs/led-effects/Cargo.toml --all -- --check
  cargo fmt --manifest-path libs/screen-power/Cargo.toml --all -- --check
  cargo fmt --manifest-path tools/loadlynx-devd/Cargo.toml --all -- --check
  cargo fmt --manifest-path tools/ui-mock/Cargo.toml --all -- --check
  cargo fmt --manifest-path firmware/analog/Cargo.toml -p analog -- --check
  cargo +esp fmt --manifest-path firmware/digital/Cargo.toml -p digital -- --check
  (cd web && node ./node_modules/@biomejs/biome/bin/biome format .)

_require-root-deps:
  test -d node_modules || { \
    echo "[error] root Node.js dependencies are missing."; \
    echo "[hint] run: just deps-root"; \
    exit 2; \
  }

_require-web-deps:
  test -d web/node_modules || { \
    echo "[error] web dependencies are missing."; \
    echo "[hint] run: just deps-web"; \
    exit 2; \
  }

_require-esp-toolchain:
  cargo +esp --version >/dev/null 2>&1 || { \
    echo "[error] ESP Rust toolchain alias '+esp' is unavailable."; \
    echo "[hint] install the ESP toolchain with espup, then retry"; \
    exit 2; \
  }

_require-esp-export:
  test -f "$HOME/export-esp.sh" || { \
    echo "[error] ESP environment export script is missing: $HOME/export-esp.sh"; \
    echo "[hint] run: espup install --targets esp32s3"; \
    exit 2; \
  }

_require-digital-wifi-config:
  if [ -n "${DIGITAL_WIFI_SSID:-}" ] && [ -n "${DIGITAL_WIFI_PSK:-}" ]; then \
    exit 0; \
  fi; \
  if [ -f .env ] && \
     grep -Eq '^[[:space:]]*DIGITAL_WIFI_SSID=.+' .env && \
     grep -Eq '^[[:space:]]*DIGITAL_WIFI_PSK=.+' .env; then \
    exit 0; \
  fi; \
  echo "[error] digital Wi-Fi config is missing."; \
  echo "[hint] copy .env.example to .env and set DIGITAL_WIFI_SSID / DIGITAL_WIFI_PSK"; \
  exit 2

_require-embassy-submodule:
  test -f third_party/embassy/embassy-embedded-hal/Cargo.toml || { \
    echo "[error] third_party/embassy is missing or not initialized."; \
    echo "[hint] run: git submodule update --init --recursive"; \
    exit 2; \
  }

# Repo-local workflow/policy contract checks.
check-root:
  just _require-root-deps
  npm run test:release-labels
  npm run test:quality-gates
  npm run test:workflow-hygiene

# Host-side Rust crate and tool tests.
test-host:
  cargo test --manifest-path libs/protocol/Cargo.toml --locked
  cargo test --manifest-path libs/calibration-format/Cargo.toml --locked
  cargo test --manifest-path libs/led-effects/Cargo.toml --locked
  cargo test --manifest-path libs/screen-power/Cargo.toml --locked
  cargo test --manifest-path tools/loadlynx-devd/Cargo.toml --locked
  cargo test --manifest-path tools/ui-mock/Cargo.toml --locked

# Host-side lint/static checks that mirror Code Check.
lint-host:
  cargo clippy --manifest-path libs/protocol/Cargo.toml --all-targets --all-features --locked -- -D warnings
  cargo clippy --manifest-path libs/calibration-format/Cargo.toml --all-targets --all-features --locked -- -D warnings
  cargo clippy --manifest-path libs/led-effects/Cargo.toml --all-targets --all-features --locked -- -D warnings
  cargo clippy --manifest-path libs/screen-power/Cargo.toml --all-targets --all-features --locked -- -D warnings
  cargo clippy --manifest-path tools/loadlynx-devd/Cargo.toml --all-targets --all-features --locked -- -D warnings
  tools/loadlynx-devd/install/install-loadlynx-host.sh --dry-run
  bash -n tools/loadlynx-devd/install/install-loadlynx-host.sh

# Optional host checks that require extra local tooling (for example PowerShell).
lint-host-optional:
  if command -v pwsh >/dev/null 2>&1; then \
    pwsh -NoProfile -Command '$tokens = $null; $errors = $null; [void][System.Management.Automation.Language.Parser]::ParseFile("tools/loadlynx-devd/install/install-loadlynx-host.ps1", [ref]$tokens, [ref]$errors); if ($errors.Count -gt 0) { $errors | ForEach-Object { Write-Error $_.Message }; exit 1 }'; \
  else \
    echo "[info] pwsh not found; skipping install-loadlynx-host.ps1 syntax check"; \
  fi

# Web checks that do not require browser installation.
check-web:
  just _require-web-deps
  (cd web && node ./node_modules/@biomejs/biome/bin/biome check .)
  (cd web && bun run build)
  (cd web && bun run check:bundle:app)
  (cd web && bun run test:unit)

# Browser-backed web checks. Assumes Playwright/browser deps are installed.
check-web-full:
  just _require-web-deps
  (cd web && bun run test:storybook:ci)
  (cd web && bun run test:e2e)

# Embedded firmware builds used by CI.
check-embedded:
  just _require-embassy-submodule
  just _require-esp-toolchain
  just _require-esp-export
  (cd firmware/analog && cargo build --locked --release --target thumbv7em-none-eabihf)
  DIGITAL_WIFI_SSID="${DIGITAL_WIFI_SSID:-codex_dummy}" \
  DIGITAL_WIFI_PSK="${DIGITAL_WIFI_PSK:-codex_dummy}" \
  just d-build

# Embedded lint/static checks aligned with Code Check.
lint-embedded:
  just _require-embassy-submodule
  (cd firmware/analog && cargo clippy --locked --bins --no-deps --target thumbv7em-none-eabihf -- -D warnings)
  just d-clippy

# Fast local verification path aligned with repo quality gates.
check:
  just fmt-check
  just check-root
  just test-host
  just lint-host
  just check-web

# Broad verification path closest to CI minus release/push side effects.
check-full:
  just check
  just lint-host-optional
  just lint-embedded
  just check-embedded
  just check-web-full

# --- Local devd / CLI -------------------------------------------------------

devd-build:
  cargo build --manifest-path tools/loadlynx-devd/Cargo.toml --bins

devd-test:
  cargo test --manifest-path tools/loadlynx-devd/Cargo.toml

devd-serve +args:
  cargo run --manifest-path tools/loadlynx-devd/Cargo.toml --bin loadlynx-devd -- serve {{args}}

devd-bridge-http +args:
  cargo run --manifest-path tools/loadlynx-devd/Cargo.toml --bin loadlynx-devd -- bridge-http {{args}}

loadlynx +args:
  cargo run --manifest-path tools/loadlynx-devd/Cargo.toml --bin loadlynx -- {{args}}

# --- Analog (STM32G431) ----------------------------------------------------

# Build analog firmware (PROFILE/DEFMT_LOG passed via env)
a-build:
  set -eu; \
  PROFILE="${PROFILE:-release}"; \
  FEATURES="${FEATURES:-}"; \
  DEFMT_LOG="${DEFMT_LOG:-info}"; \
  if [ "$PROFILE" = "release" ]; then CARGO_FLAGS="--release"; else CARGO_FLAGS=""; fi; \
  if [ -n "$FEATURES" ]; then \
  (cd firmware/analog && DEFMT_LOG="$DEFMT_LOG" cargo build $CARGO_FLAGS --features "$FEATURES" --target thumbv7em-none-eabihf); \
  else \
  (cd firmware/analog && DEFMT_LOG="$DEFMT_LOG" cargo build $CARGO_FLAGS --target thumbv7em-none-eabihf); \
  fi

a-info:
  set -eu; \
  PROFILE="${PROFILE:-release}"; \
  ELF="firmware/analog/target/thumbv7em-none-eabihf/${PROFILE}/analog"; \
  echo "elf=$ELF"; \
  if [ -f "$ELF" ]; then \
  (command -v shasum >/dev/null 2>&1 && shasum -a 256 "$ELF") || true; \
  else \
  echo "[info] ELF not built"; \
  fi

a-size:
  set -eu; \
  PROFILE="${PROFILE:-release}"; \
  if [ "$PROFILE" = "release" ]; then CARGO_FLAGS="--release"; else CARGO_FLAGS=""; fi; \
  if cargo size --help >/dev/null 2>&1; then \
  (cd firmware/analog && cargo size $CARGO_FLAGS --target thumbv7em-none-eabihf -- -A); \
  else \
  echo "[size] cargo-binutils (cargo size) not installed"; \
  fi

a-clean:
  set -eu; \
  (cd firmware/analog && cargo clean --target thumbv7em-none-eabihf)

# --- Digital (ESP32-S3) ----------------------------------------------------

d-clippy:
  just _require-esp-toolchain
  just _require-esp-export
  set -eu; \
  . "$HOME/export-esp.sh"; \
  FEATURES="${FEATURES:-}"; \
  DIGITAL_WIFI_SSID="${DIGITAL_WIFI_SSID:-codex_dummy}"; \
  DIGITAL_WIFI_PSK="${DIGITAL_WIFI_PSK:-codex_dummy}"; \
  if [ -n "$FEATURES" ]; then \
  (cd firmware/digital && DIGITAL_WIFI_SSID="$DIGITAL_WIFI_SSID" DIGITAL_WIFI_PSK="$DIGITAL_WIFI_PSK" cargo +esp clippy --locked --bins --features "$FEATURES" -- -D warnings); \
  else \
  (cd firmware/digital && DIGITAL_WIFI_SSID="$DIGITAL_WIFI_SSID" DIGITAL_WIFI_PSK="$DIGITAL_WIFI_PSK" cargo +esp clippy --locked --bins -- -D warnings); \
  fi

d-build:
  just _require-esp-toolchain
  just _require-esp-export
  just _require-digital-wifi-config
  set -eu; \
  . "$HOME/export-esp.sh"; \
  PROFILE="${PROFILE:-release}"; \
  FEATURES="${FEATURES:-}"; \
  if [ "$PROFILE" = "release" ]; then CARGO_FLAGS="--release"; else CARGO_FLAGS=""; fi; \
  if [ -n "$FEATURES" ]; then \
  (cd firmware/digital && cargo +esp build $CARGO_FLAGS --features "$FEATURES"); \
  else \
  (cd firmware/digital && cargo +esp build $CARGO_FLAGS); \
  fi

d-env:
  set -eu; \
  PROFILE="${PROFILE:-release}"; \
  echo "profile=$PROFILE"; \
  echo "elf=firmware/digital/target/xtensa-esp32s3-none-elf/${PROFILE}/digital"; \
  if [ -n "${FEATURES:-}" ]; then echo "features=${FEATURES}"; fi

d-clean:
  set -eu; \
  (cd firmware/digital && cargo clean)
