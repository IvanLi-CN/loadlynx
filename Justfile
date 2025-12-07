set shell := ["/bin/sh", "-c"]

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

# --- Agent daemon passthrough (tools/mcu-agentd) ---------------------------

# Generic loadlynx-agentd passthrough (release)
agentd +args:
  cd tools/mcu-agentd && cargo run --release -- {{args}}

agentd-start:
  just agentd start

agentd-status:
  just agentd status

agentd-stop:
  just agentd stop

agentd-set-port mcu path="":
  if [ -z "{{path}}" ]; then \
    cd tools/mcu-agentd && cargo run --release -- set-port {{mcu}}; \
  else \
    cd tools/mcu-agentd && cargo run --release -- set-port {{mcu}} {{path}}; \
  fi

agentd-get-port mcu:
  cd tools/mcu-agentd && cargo run --release -- get-port {{mcu}}
