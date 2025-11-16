SHELL := /bin/sh

TARGET   ?= xtensa-esp32s3-none-elf
BIN      ?= digital
PROFILE  ?= release
PORT     ?=
BAUD     ?= 921600
MONITOR_BAUD ?= $(BAUD)
CHIP     ?= esp32s3
LOGFMT   ?= defmt
ESPFLASH ?= espflash
CARGO    ?= cargo +esp
ENSURE_ESP32_PORT ?= $(abspath ../../scripts/ensure_esp32_port.sh)

ifeq ($(PORT),)
  ifneq ($(wildcard $(ENSURE_ESP32_PORT)),)
    PORT := $(shell $(ENSURE_ESP32_PORT))
  endif
endif

ifeq ($(PROFILE),release)
  CARGO_FLAGS := --release
else
  CARGO_FLAGS :=
endif

BINDIR := target/$(TARGET)/$(PROFILE)
ELF    := $(BINDIR)/$(BIN)

PORT_FLAG   := $(if $(PORT),--port $(PORT),)
BAUD_FLAG   := $(if $(BAUD),-B $(BAUD),)
MONITOR_BAUD_FLAG := $(if $(MONITOR_BAUD),-r $(MONITOR_BAUD),)
CHIP_FLAG   := --chip $(CHIP)
LOGFMT_FLAG := $(if $(LOGFMT),--log-format $(LOGFMT),)
ESPFLASH_ARGS ?=

.PHONY: help env ports build clean run flash attach reset reset-attach monitor

help:
	@echo "Digital (ESP32-S3) firmware make targets"
	@echo "  build        Build $(PROFILE) firmware with cargo +esp"
	@echo "  run          Flash + monitor via espflash"
	@echo "  flash        Flash firmware only (no monitor)"
	@echo "  attach       Monitor existing firmware"
	@echo "  reset        Reset device via espflash"
	@echo "  reset-attach Reset device then attach monitor"
	@echo "  monitor      Alias for attach"
	@echo "  ports        List available serial ports"
	@echo "  env          Show computed paths"
	@echo "  clean        Clean target artifacts"

env:
	@echo TARGET=$(TARGET)
	@echo BIN=$(BIN)
	@echo PROFILE=$(PROFILE)
	@echo ELF=$(ELF)
	@echo PORT=$(PORT)
	@echo BAUD=$(BAUD)
	@echo CHIP=$(CHIP)
	@echo LOGFMT=$(LOGFMT)

ports:
	$(ESPFLASH) scan-ports || $(ESPFLASH) list-ports || true

build:
	$(CARGO) build $(CARGO_FLAGS)

clean:
	cargo clean

run: build
	$(ESPFLASH) flash "$(ELF)" --monitor $(PORT_FLAG) $(BAUD_FLAG) $(MONITOR_BAUD_FLAG) $(CHIP_FLAG) $(LOGFMT_FLAG) $(if $(filter defmt,$(LOGFMT)),--elf "$(ELF)",) $(ESPFLASH_ARGS)

flash: build
	$(ESPFLASH) flash "$(ELF)" $(PORT_FLAG) $(BAUD_FLAG) $(CHIP_FLAG) $(LOGFMT_FLAG) $(if $(filter defmt,$(LOGFMT)),--elf "$(ELF)",) $(ESPFLASH_ARGS)

attach: build
	$(ESPFLASH) monitor $(PORT_FLAG) $(BAUD_FLAG) $(MONITOR_BAUD_FLAG) $(CHIP_FLAG) $(LOGFMT_FLAG) --elf "$(ELF)" $(ESPFLASH_ARGS)

reset:
	$(ESPFLASH) reset $(PORT_FLAG) $(BAUD_FLAG) $(CHIP_FLAG) $(ESPFLASH_ARGS)

reset-attach: reset attach

monitor: attach
