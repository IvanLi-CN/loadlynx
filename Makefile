SHELL := /bin/sh

# Optional env overrides
#  Analog (STM32G431): CHIP, PROBE, PROTOCOL, SPEED, PROFILE, DEFMT_LOG
#  Digital (ESP32-S3): PORT, BAUD, LOGFMT, PROFILE, ESPFLASH_ARGS

.PHONY: help fmt FORCE \
        a-build a-run a-run-force a-attach a-reset a-reset-attach a-info a-clean a-size \
        d-build d-run d-attach d-monitor d-ports d-env d-clean

help:
	@echo "LoadLynx Makefile"
	@echo "  a-build                 Build STM32G431 firmware (profile via PROFILE)"
	@echo "  a-run                   Flash + run STM32G431 (probe-rs, requires PROBE)"
	@echo "  a-run-force             Clean then flash STM32G431"
	@echo "  a-run-pick              Interactively select probe then run"
	@echo "  a-attach                Attach to STM32G431 with symbols"
	@echo "  a-reset                 Reset STM32G431 via probe-rs"
	@echo "  a-reset-attach          Reset then attach STM32G431"
	@echo "  a-info                  Show STM32G431 build artifacts"
	@echo "  a-size                  Print STM32G431 ELF size (requires cargo-binutils)"
	@echo "  a-clean                 Clean STM32G431 artifacts"
	@echo "  a-probes               List STM32 debug probes (VID:PID[:SER])"
	@echo "  d-build                 Build ESP32-S3 firmware (cargo +esp)"
	@echo "  d-run                   Flash + monitor ESP32-S3 (espflash)"
	@echo "  d-attach                Monitor ESP32-S3 using existing ELF"
	@echo "  d-monitor               Alias for d-attach"
	@echo "  d-ports                 List ESP32-S3 serial ports"
	@echo "  d-env                   Show ESP32-S3 build environment"
	@echo "  d-clean                 Clean ESP32-S3 artifacts"
	@echo "Vars forwarded automatically (if set):"
	@echo "  Analog -> CHIP PROBE PROTOCOL SPEED PROFILE DEFMT_LOG"
	@echo "  Digital -> PORT BAUD LOGFMT PROFILE ESPFLASH_ARGS"

fmt:
	cargo fmt --all || true

# --- Analog (STM32G431) ----------------------------------------------------

a-build:
	$(MAKE) -C firmware/analog build \
	  $(if $(CHIP),CHIP=$(CHIP),) \
	  $(if $(PROBE),PROBE=$(PROBE),) \
	  $(if $(PROTOCOL),PROTOCOL=$(PROTOCOL),) \
	  $(if $(SPEED),SPEED=$(SPEED),) \
	  $(if $(PROFILE),PROFILE=$(PROFILE),) \
	  $(if $(DEFMT_LOG),DEFMT_LOG=$(DEFMT_LOG),)

a-run: FORCE
		@PROBE_SEL=$$(PROBE=$(PROBE) PORT=$(PORT) ./scripts/ensure_stm32_probe.sh) || { echo "[error] probe selection failed"; exit 2; }; \
	$(MAKE) -C firmware/analog run \
  $(if $(CHIP),CHIP=$(CHIP),) \
  PROBE="$$PROBE_SEL" \
  $(if $(PROTOCOL),PROTOCOL=$(PROTOCOL),) \
  $(if $(SPEED),SPEED=$(SPEED),) \
  $(if $(PROFILE),PROFILE=$(PROFILE),) \
  $(if $(DEFMT_LOG),DEFMT_LOG=$(DEFMT_LOG),)

a-run-force: FORCE
		@PROBE_SEL=$$(PROBE=$(PROBE) PORT=$(PORT) ./scripts/ensure_stm32_probe.sh) || { echo "[error] probe selection failed"; exit 2; }; \
	$(MAKE) -C firmware/analog run-force \
  $(if $(CHIP),CHIP=$(CHIP),) \
  PROBE="$$PROBE_SEL" \
  $(if $(PROTOCOL),PROTOCOL=$(PROTOCOL),) \
  $(if $(SPEED),SPEED=$(SPEED),) \
  $(if $(PROFILE),PROFILE=$(PROFILE),) \
  $(if $(DEFMT_LOG),DEFMT_LOG=$(DEFMT_LOG),)

a-attach: FORCE
		@PROBE_SEL=$$(PROBE=$(PROBE) PORT=$(PORT) ./scripts/ensure_stm32_probe.sh) || { echo "[error] probe selection failed"; exit 2; }; \
	$(MAKE) -C firmware/analog attach \
  $(if $(CHIP),CHIP=$(CHIP),) \
  PROBE="$$PROBE_SEL" \
  $(if $(PROTOCOL),PROTOCOL=$(PROTOCOL),) \
  $(if $(SPEED),SPEED=$(SPEED),) \
  $(if $(PROFILE),PROFILE=$(PROFILE),)

a-reset: FORCE
		@PROBE_SEL=$$(PROBE=$(PROBE) PORT=$(PORT) ./scripts/ensure_stm32_probe.sh) || { echo "[error] probe selection failed"; exit 2; }; \
	$(MAKE) -C firmware/analog reset \
  $(if $(CHIP),CHIP=$(CHIP),) \
  PROBE="$$PROBE_SEL" \
  $(if $(PROTOCOL),PROTOCOL=$(PROTOCOL),) \
  $(if $(SPEED),SPEED=$(SPEED),)

a-reset-attach: FORCE
		@PROBE_SEL=$$(PROBE=$(PROBE) PORT=$(PORT) ./scripts/ensure_stm32_probe.sh) || { echo "[error] probe selection failed"; exit 2; }; \
	$(MAKE) -C firmware/analog reset-attach \
  $(if $(CHIP),CHIP=$(CHIP),) \
  PROBE="$$PROBE_SEL" \
  $(if $(PROTOCOL),PROTOCOL=$(PROTOCOL),) \
  $(if $(SPEED),SPEED=$(SPEED),) \
  $(if $(PROFILE),PROFILE=$(PROFILE),)

a-info:
	$(MAKE) -C firmware/analog info \
  $(if $(CHIP),CHIP=$(CHIP),) \
  $(if $(PROBE),PROBE=$(PROBE),) \
  $(if $(PROFILE),PROFILE=$(PROFILE),)

a-size:
	$(MAKE) -C firmware/analog size \
	  $(if $(PROFILE),PROFILE=$(PROFILE),)

a-clean:
	$(MAKE) -C firmware/analog clean \
  $(if $(PROFILE),PROFILE=$(PROFILE),)

# convenience: list STM32 debug probes
a-probes:
	$(MAKE) -C firmware/analog list-probes

# interactive probe selection
a-run-pick:
	./scripts/select_stm32_probe.sh a-run $(if $(PROFILE),PROFILE=$(PROFILE),)

# explicit reselection: forget cached probe and pick again
select-probe:
	rm -f .stm32-probe
	./scripts/select_stm32_probe.sh

# Always-out-of-date phony dependency
FORCE:

# --- Digital (ESP32-S3) ----------------------------------------------------

d-build:
	$(MAKE) -C firmware/digital build \
	  $(if $(PROFILE),PROFILE=$(PROFILE),)

d-run:
	$(MAKE) -C firmware/digital run \
	  $(if $(PROFILE),PROFILE=$(PROFILE),) \
	  $(if $(PORT),PORT=$(PORT),) \
	  $(if $(BAUD),BAUD=$(BAUD),) \
	  $(if $(LOGFMT),LOGFMT=$(LOGFMT),) \
	  $(if $(ESPFLASH_ARGS),ESPFLASH_ARGS="$(ESPFLASH_ARGS)",)

d-attach:
	$(MAKE) -C firmware/digital attach \
	  $(if $(PROFILE),PROFILE=$(PROFILE),) \
	  $(if $(PORT),PORT=$(PORT),) \
	  $(if $(BAUD),BAUD=$(BAUD),) \
	  $(if $(LOGFMT),LOGFMT=$(LOGFMT),) \
	  $(if $(ESPFLASH_ARGS),ESPFLASH_ARGS="$(ESPFLASH_ARGS)",)

d-monitor: d-attach

d-ports:
	$(MAKE) -C firmware/digital ports

d-env:
	$(MAKE) -C firmware/digital env \
	  $(if $(PROFILE),PROFILE=$(PROFILE),) \
	  $(if $(PORT),PORT=$(PORT),) \
	  $(if $(BAUD),BAUD=$(BAUD),) \
	  $(if $(LOGFMT),LOGFMT=$(LOGFMT),)

d-clean:
	$(MAKE) -C firmware/digital clean \
	  $(if $(PROFILE),PROFILE=$(PROFILE),)
