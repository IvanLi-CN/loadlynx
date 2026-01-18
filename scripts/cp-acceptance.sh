#!/usr/bin/env bash
set -euo pipefail

# CP 1ms transient acceptance (internal cp_perf).
#
# Usage:
#   IP=192.168.31.216 CYCLES=5 DWELL_S=0.20 ./scripts/cp-acceptance.sh
#
# Notes:
# - Keeps high-power dwell short to protect the PD source.
# - Triggers a new acceptance run by toggling output OFF->ON at the start.

IP="${IP:-192.168.31.216}"
CYCLES="${CYCLES:-5}"
DWELL_S="${DWELL_S:-0.20}"
PRESET_ID="${PRESET_ID:-1}"
MAX_I_MA_TOTAL="${MAX_I_MA_TOTAL:-5000}"
MAX_P_MW="${MAX_P_MW:-100000}"

function set_preset_cp() {
  local target_p_mw="$1"
  curl -fsS --max-time 2 \
    -H 'Content-Type: application/json' \
    -X PUT "http://${IP}/api/v1/presets" \
    -d "{\"preset_id\":${PRESET_ID},\"mode\":\"cp\",\"target_p_mw\":${target_p_mw},\"target_i_ma\":0,\"target_v_mv\":0,\"min_v_mv\":0,\"max_i_ma_total\":${MAX_I_MA_TOTAL},\"max_p_mw\":${MAX_P_MW}}" \
    >/dev/null
}

function set_output() {
  local enabled="$1"
  curl -fsS --max-time 2 \
    -H 'Content-Type: application/json' \
    -X PUT "http://${IP}/api/v1/control" \
    -d "{\"output_enabled\":${enabled}}" \
    >/dev/null
}

echo "[cp-acceptance] IP=${IP} cycles=${CYCLES} dwell=${DWELL_S}s preset_id=${PRESET_ID}"

# Baseline 10W
set_preset_cp 10000

# New run: OFF -> ON edge resets acceptance counters in analog.
set_output false
sleep 0.10
set_output true
sleep 0.20

# Multi-level steps (mW): 10W, 30W, 50W, 70W, 90W, then back down.
targets=(10000 30000 50000 70000 90000 70000 50000 30000 10000)

for ((c = 1; c <= CYCLES; c++)); do
  for t in "${targets[@]}"; do
    set_preset_cp "${t}"
    sleep "${DWELL_S}"
  done
done

set_output false
echo "[cp-acceptance] done"

