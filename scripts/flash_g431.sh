#!/usr/bin/env bash
set -euo pipefail

# flash G431 using probe-rs directly
# Usage: scripts/flash_g431.sh [build-profile]
# Default profile: debug

PROFILE=${1:-debug}
FW=firmware/eload-core/target/thumbv7em-none-eabihf/${PROFILE}/eload-core

if [ ! -f "$FW" ]; then
  echo "Firmware not found: $FW\nTry: (cd firmware/eload-core && cargo build --profile ${PROFILE})" >&2
  exit 1
fi

probe-rs run --chip STM32G431CB --protocol swd --speed 4000 --firmware "$FW"

