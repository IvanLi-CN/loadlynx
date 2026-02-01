set shell := ["/bin/sh", "-c"]

# External mcu-agentd checkout (override if needed)
MCU_AGENTD_MANIFEST := env_var_or_default("MCU_AGENTD_MANIFEST", "../mcu-agentd/Cargo.toml")

# Default: list available recipes
default:
  @just --list

# Formatting
fmt:
  cargo fmt --all || true

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

d-build:
  set -eu; \
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

# --- Agent daemon passthrough (mcu-agentd) ---------------------------------

# Generic mcu-agentd passthrough (release)
agentd +args:
  if command -v mcu-agentd >/dev/null 2>&1; then \
    mcu-agentd {{args}}; \
  else \
    if [ ! -f "{{MCU_AGENTD_MANIFEST}}" ]; then \
      echo "[error] mcu-agentd not installed and manifest not found: {{MCU_AGENTD_MANIFEST}}"; \
      echo "[hint] run: just agentd-init  (or set MCU_AGENTD_MANIFEST / MCU_AGENTD_PATH)"; \
      exit 2; \
    fi; \
    cargo run --manifest-path "{{MCU_AGENTD_MANIFEST}}" --bin mcu-agentd --release -- {{args}}; \
  fi

# Install mcu-agentd/mcu-managerd from a local checkout.
_agentd-install path="":
  set -eu; \
  REPO="{{path}}"; \
  if [ "$REPO" = "path=" ]; then REPO=""; fi; \
  if [ -z "$REPO" ]; then REPO="${MCU_AGENTD_PATH:-../mcu-agentd}"; fi; \
  if [ ! -d "$REPO" ]; then \
    echo "mcu-agentd repo not found at: $REPO"; \
    echo "Usage: just agentd-init path=/path/to/mcu-agentd"; \
    echo "   or: MCU_AGENTD_PATH=/path/to/mcu-agentd just agentd-init"; \
    exit 2; \
  fi; \
  cargo install --force --path "$REPO" --bins; \
  mcu-agentd --version; \
  mcu-managerd --version

agentd-init path="":
  @just agentd stop >/dev/null 2>&1 || true
  @if [ -z "{{path}}" ]; then \
    just _agentd-install; \
  else \
    just _agentd-install path="{{path}}"; \
  fi
  @just agentd-start

agentd-start:
  just agentd start

agentd-status:
  just agentd status

agentd-stop:
  just agentd stop

agentd-set-port mcu path="":
  if [ -z "{{path}}" ]; then \
    just agentd selector set {{mcu}}; \
  else \
    just agentd selector set {{mcu}} {{path}}; \
  fi

agentd-get-port mcu:
  just agentd selector get {{mcu}}
