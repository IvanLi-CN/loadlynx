SHELL := /bin/sh

# Build configuration
TARGET_TRIPLE ?= thumbv7em-none-eabihf
PROFILE       ?= release
BIN_NAME      ?= analog

# Probe/MCU configuration
CHIP      ?= STM32G431CB
PROBE     ?=
PROTOCOL  ?= swd
SPEED     ?= 4000
DEFMT_LOG ?= info

CARGO    ?= cargo
PROBE_RS ?= probe-rs

ifeq ($(PROFILE),release)
  CARGO_FLAGS := --release
else
  CARGO_FLAGS :=
endif

BINDIR := target/$(TARGET_TRIPLE)/$(PROFILE)
ELF    := $(BINDIR)/$(BIN_NAME)

PROBE_FLAG     := $(if $(PROBE),--probe $(PROBE),)
PROTOCOL_FLAG  := $(if $(PROTOCOL),--protocol $(PROTOCOL),)
SPEED_FLAG     := $(if $(SPEED),--speed $(SPEED),)

.PHONY: help build run run-force attach reset reset-attach size info clean

help:
	@echo "Analog (STM32G431) firmware make targets"
	@echo "  build        Build $(PROFILE) firmware"
	@echo "  run          Flash + run via probe-rs" 
	@echo "  run-force    Clean then flash"
	@echo "  attach       Attach with symbols"
	@echo "  reset        Hardware reset via probe-rs"
	@echo "  reset-attach Reset then attach"
	@echo "  size         Show binary size (requires cargo-binutils)"
	@echo "  info         Path + hash info"
	@echo "  clean        Clean target artifacts"

build:
	@echo "[build] target=$(TARGET_TRIPLE) profile=$(PROFILE)"
	DEFMT_LOG=$(DEFMT_LOG) $(CARGO) build $(CARGO_FLAGS) --target $(TARGET_TRIPLE)
	@echo "[build] elf=$(ELF)"
	@if [ -f "$(ELF)" ]; then \
	  stat_cmd=$$(command -v gstat || command -v stat); \
	  if echo $$($$stat_cmd --version 2>/dev/null) | grep -qi 'gnu'; then \
	    $$stat_cmd -c '[build] mtime=%y size=%s' "$(ELF)"; \
	  else \
	    $$stat_cmd -f '[build] mtime=%Sm size=%z' -t '%Y-%m-%d %H:%M:%S' "$(ELF)"; \
	  fi; \
	  (command -v shasum >/dev/null && shasum -a 256 "$(ELF)" | sed 's/^/[build] sha256=/') || true; \
	fi

run: build
	@echo "[run] chip=$(CHIP) probe=$(PROBE)"
	$(PROBE_RS) run $(PROBE_FLAG) $(PROTOCOL_FLAG) $(SPEED_FLAG) --chip $(CHIP) "$(ELF)"

run-force: clean run

attach: build
	@echo "[attach] chip=$(CHIP) probe=$(PROBE)"
	$(PROBE_RS) attach $(PROBE_FLAG) $(PROTOCOL_FLAG) --chip $(CHIP) --elf "$(ELF)"

reset:
	@echo "[reset] chip=$(CHIP) probe=$(PROBE)"
	$(PROBE_RS) reset $(PROBE_FLAG) $(PROTOCOL_FLAG) --chip $(CHIP)

reset-attach: reset attach

size: build
	@if cargo size --help >/dev/null 2>&1; then \
	  cargo size $(CARGO_FLAGS) --target $(TARGET_TRIPLE) -- -A; \
	else \
	  echo "[size] cargo-binutils (cargo size) not installed"; \
	fi

info:
	@echo "[info] chip=$(CHIP) probe=$(PROBE)"
	@echo "[info] target=$(TARGET_TRIPLE) profile=$(PROFILE)"
	@echo "[info] elf=$(ELF)"
	@if [ -f "$(ELF)" ]; then \
	  (command -v shasum >/dev/null && shasum -a 256 "$(ELF)") || true; \
	else \
	  echo "[info] ELF not built"; \
	fi

clean:
	@echo "[clean] target=$(TARGET_TRIPLE) profile=$(PROFILE)"
	$(CARGO) clean --target $(TARGET_TRIPLE)
