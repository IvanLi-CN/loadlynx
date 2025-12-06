#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd "$(dirname "$0")/.." && pwd)
CACHE_FILE="$REPO_ROOT/.stm32-port"
LEGACY_FILE="$REPO_ROOT/.stm32-probe"

# Ensure a unique STM32 debug probe selector is available.
# Prints the selector to stdout and caches it in .stm32-port.
# Resolution order:
# 1) PROBE env (if currently detected by probe-rs)
# 2) PORT env alias (if currently detected by probe-rs)
# 3) cached .stm32-port (if non-empty)
# 4) legacy .stm32-probe (only when .stm32-port is absent; migrate by
#    reading -> writing .stm32-port -> deleting .stm32-probe)
# 5) single ST-Link present (VID:PID 0483:3748)
# 6) single probe present
# 7) interactive selection (scripts/select_stm32_probe.sh, which now
#    also writes .stm32-port)

if ! command -v probe-rs >/dev/null 2>&1; then
  echo "[error] probe-rs not found; install via 'cargo install probe-rs'" >&2
  exit 127
fi

ALL_OUTPUT=$(probe-rs list || true)

# Helper: filter lines that represent indexed probes
index_lines() {
  echo "$ALL_OUTPUT" | grep -E '^[[:space:]]*[0-9]+:|^\[[0-9]+\]:'
}

# Extract tokens (selector strings after '-- ')
tokens() { echo "$ALL_OUTPUT" | sed -n 's/.*-- \([^ ]\+\).*/\1/p'; }
has_token() { tokens | grep -Fxq "$1"; }

pick_and_cache() {
  local sel="$1"
  echo "$sel" > "$CACHE_FILE"
  # Best-effort cleanup of legacy cache to avoid confusion.
  if [ -f "$LEGACY_FILE" ]; then
    rm -f "$LEGACY_FILE" || true
  fi
  echo "$sel"
}

# 0) Legacy migration: if .stm32-port is absent but .stm32-probe exists,
#    move its contents to .stm32-port and delete the legacy file.
if [ ! -f "$CACHE_FILE" ] && [ -f "$LEGACY_FILE" ]; then
  LEGACY_VAL=$(cat "$LEGACY_FILE" 2>/dev/null || true)
  rm -f "$LEGACY_FILE" || true
  if [ -n "$LEGACY_VAL" ]; then
    echo "$LEGACY_VAL" > "$CACHE_FILE"
  fi
fi

# 1) PROBE env
if [ "${PROBE:-}" != "" ] && has_token "$PROBE"; then
  pick_and_cache "$PROBE"
  exit 0
fi

# 2) PORT alias
if [ "${PORT:-}" != "" ] && has_token "$PORT"; then
  pick_and_cache "$PORT"
  exit 0
fi

# 3) cached selector (trust cached until user reselects)
if [ -f "$CACHE_FILE" ]; then
  CACHED=$(cat "$CACHE_FILE" || true)
  if [ "$CACHED" != "" ]; then
    echo "$CACHED"
    exit 0
  fi
fi

# 4) single ST-Link present
ST_LINES=$(index_lines | grep -Ei 'ST[- ]?Link|0483:3748' || true)
COUNT_ST=$(echo "$ST_LINES" | grep -E '^[[:space:]]*[0-9]+:|^\[[0-9]+\]:' | wc -l | tr -d ' ')
if [ "$COUNT_ST" = "1" ]; then
  SEL=$(echo "$ST_LINES" | sed -n 's/.*-- \([^ ]\+\).*/\1/p' | head -n1)
  if [ "$SEL" != "" ]; then pick_and_cache "$SEL"; exit 0; fi
fi

# 5) single probe present overall
COUNT_ALL=$(index_lines | wc -l | tr -d ' ')
if [ "$COUNT_ALL" = "1" ]; then
  SEL=$(index_lines | sed -n 's/.*-- \([^ ]\+\).*/\1/p')
  if [ "$SEL" != "" ]; then pick_and_cache "$SEL"; exit 0; fi
fi

# 6) interactive selection
exec "$REPO_ROOT/scripts/select_stm32_probe.sh"
