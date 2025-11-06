#!/usr/bin/env bash
set -euo pipefail

# Flash ESP32-S3 firmware via make digital-run helper
# Usage: scripts/flash_s3.sh [--release] [--port /dev/tty.*] [additional make vars]

PROFILE=dev
EXTRA_ARGS=()

while [ $# -gt 0 ]; do
  case "$1" in
    --release)
      PROFILE=release; shift ;;
    --port)
      if [ $# -lt 2 ]; then
        echo "Missing value for --port" >&2
        exit 2
      fi
      PORT_VALUE=$2
      EXTRA_ARGS+=("PORT=$PORT_VALUE")
      shift 2 ;;
    *)
      EXTRA_ARGS+=("$1")
      shift ;;
  esac
done

PROFILE=$PROFILE make d-run "${EXTRA_ARGS[@]}"
