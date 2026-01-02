set shell := ["/bin/sh", "-c"]

# External mcu-agentd checkout (override if needed)
MCU_AGENTD_MANIFEST := env_var_or_default("MCU_AGENTD_MANIFEST", "../mcu-agentd/Cargo.toml")

# Default: list available recipes
default:
  @just --list

# Formatting
fmt:
  make fmt

# --- Analog (STM32G431) ----------------------------------------------------

# Build analog firmware (PROFILE, DEFMT_LOG, PROBE, etc. passed via env)
a-build:
  make a-build

a-run:
  make a-run

a-run-force:
  make a-run-force

a-attach:
  make a-attach

a-reset:
  make a-reset

a-reset-attach:
  make a-reset-attach

a-info:
  make a-info

a-size:
  make a-size

a-clean:
  make a-clean

a-probes:
  make a-probes

a-run-pick:
  make a-run-pick

select-probe:
  make select-probe

# --- Digital (ESP32-S3) ----------------------------------------------------

d-build:
  make d-build

d-run:
  make d-run

d-reset:
  make d-reset

d-reset-attach:
  make d-reset-attach

d-attach:
  make d-attach

d-monitor:
  make d-monitor

d-ports:
  make d-ports

d-env:
  make d-env

d-clean:
  make d-clean

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

agnetd-init: agentd-init

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
