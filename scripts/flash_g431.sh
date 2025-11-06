#!/usr/bin/env bash
set -euo pipefail

# Flash STM32G431 via make a-run helper
# Usage: scripts/flash_g431.sh [profile]
# Default profile: release

PROFILE=${1:-release}

shift || true

PROFILE=$PROFILE make a-run "$@"
