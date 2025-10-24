#!/usr/bin/env bash
set -euo pipefail

# flash ESP32-S3 Rust (esp-hal) using espflash via cargo runner
# Usage: scripts/flash_s3.sh [--release] [--port /dev/tty.usbserial-xxxx]

set -euo pipefail

PORT_ARG=
PROFILE=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --port)
      PORT_ARG="--port $2"; shift 2;;
    --release)
      PROFILE=--release; shift;;
    *) echo "Unknown arg: $1" >&2; exit 1;;
  esac
done

cd firmware/digital
# Runner in .cargo/config.toml uses espflash; allow overriding port via env
ESPFLASH_OPTS=${PORT_ARG} cargo +esp run ${PROFILE}
