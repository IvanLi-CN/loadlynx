#!/usr/bin/env bash
set -euo pipefail

# Dual-board monitor helper for LoadLynx.
# - Assumes firmware for both boards has already been built and flashed
#   (e.g. via scripts/agent_verify_analog.sh --no-log and
#   scripts/agent_verify_digital.sh --no-log).
# - Starts simultaneous reset-attach sessions for:
#   - Digital (ESP32-S3) first, then
#   - Analog (STM32G431),
#   each with a bounded duration.
# - Returns immediately after spawning both sessions, writing logs and PID
#   files under tmp/agent-logs/.
#
# Agent usage pattern:
#   1) scripts/agent_verify_digital.sh --no-log
#   2) scripts/agent_verify_analog.sh --no-log
#   3) scripts/agent_dual_monitor.sh --timeout 60
#   4) sleep 60
#   5) Read tmp/agent-logs/analog-dual-*.log and digital-dual-*.log.

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
LOG_DIR="$REPO_ROOT/tmp/agent-logs"
mkdir -p "$LOG_DIR"

PROFILE="${PROFILE:-release}"
TIME_LIMIT_SECONDS=60
EXTRA_MAKE_VARS=""

usage() {
    cat <<EOF
Usage: scripts/agent_dual_monitor.sh [--timeout SECONDS] [--profile {release|dev}] [EXTRA_MAKE_VARS...]

Environment:
  PROFILE         Build profile (defaults to 'release').
  PROBE           Optional explicit STM32 debug probe selector; otherwise
                  .stm32-port / auto-selection is used（若仅存在旧版
                  .stm32-probe，将在首次使用时迁移到 .stm32-port）。
  PORT            Optional explicit ESP32-S3 serial port; otherwise auto
                  selection via espflash is used.

Behavior:
  - Selects a digital serial port (ESP32-S3) non-interactively.
  - Selects an analog debug probe (STM32G431) non-interactively, honoring
    .stm32-port as the canonical cache. If only legacy .stm32-probe is
    present, its value is migrated once into .stm32-port.
  - Starts digital reset-attach logging first, then analog reset-attach
    logging, each bounded by --timeout seconds.
  - Immediately returns after spawning both sessions.
  - Writes logs and PID files to:
      tmp/agent-logs/digital-dual-<ts>.log / .pid
      tmp/agent-logs/analog-dual-<ts>.log / .pid
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        --timeout)
            if [ $# -lt 2 ]; then
                echo "[dual-monitor] missing value for --timeout" >&2
                exit 2
            fi
            TIME_LIMIT_SECONDS="$2"
            shift 2
            ;;
        --profile)
            if [ $# -lt 2 ]; then
                echo "[dual-monitor] missing value for --profile" >&2
                exit 2
            fi
            PROFILE="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            EXTRA_MAKE_VARS="$EXTRA_MAKE_VARS $1"
            shift
            ;;
    esac
done

select_port() {
    if [ "${PORT:-}" != "" ]; then
        echo "$PORT"
        return 0
    fi

    if ! command -v espflash >/dev/null 2>&1; then
        echo "[dual-monitor] espflash not found; install with 'cargo install espflash'" >&2
        return 127
    fi

    local output
    output=$(espflash list-ports 2>/dev/null || true)

    local port_lines
    port_lines=$(echo "$output" | grep '^/dev/' || true)

    if [ -z "$port_lines" ]; then
        echo "[dual-monitor] no serial ports detected; set PORT explicitly." >&2
        return 1
    fi

    local cu_lines tty_lines
    cu_lines=$(echo "$port_lines" | grep '^/dev/cu.' || true)
    tty_lines=$(echo "$port_lines" | grep '^/dev/tty.' || true)

    local cu_count all_count
    cu_count=$(echo "$cu_lines" | sed '/^$/d' | wc -l | tr -d ' ')
    all_count=$(echo "$port_lines" | sed '/^$/d' | wc -l | tr -d ' ')

    local chosen_line
    if [ "$cu_count" = "1" ]; then
        chosen_line=$(echo "$cu_lines" | head -n1)
    elif [ "$all_count" = "1" ]; then
        chosen_line=$(echo "$port_lines" | head -n1)
    else
        echo "[dual-monitor] multiple serial ports detected:" >&2
        echo "$port_lines" >&2
        echo "[dual-monitor] set PORT explicitly (e.g. PORT=/dev/cu.usbmodemXXXX)." >&2
        return 1
    fi

    local chosen_port
    chosen_port=$(echo "$chosen_line" | awk '{print $1}')
    if [ -z "$chosen_port" ]; then
        echo "[dual-monitor] failed to parse serial port from espflash output." >&2
        return 1
    fi

    echo "$chosen_port"
}

select_probe() {
    if ! command -v probe-rs >/dev/null 2>&1; then
        echo "[dual-monitor] probe-rs not found; install with 'cargo install probe-rs'" >&2
        return 127
    fi

    local all_output
    # probe-rs list may emit ANSI color codes; strip them to simplify parsing.
    all_output=$(probe-rs list 2>/dev/null | sed 's/\x1b\[[0-9;]*m//g' || true)

    # Helper: filter lines that represent indexed probes
    index_lines() {
        echo "$all_output" | grep -E '^\[[0-9]+\]:' || true
    }

    # Extract tokens (selector strings after '-- ')
    tokens() {
        echo "$all_output" \
          | awk -F'-- ' '/--/{print $2}' \
          | awk '{print $1}'
    }
    has_token() { tokens | grep -Fxq "$1"; }

    local cache_file="$REPO_ROOT/.stm32-port"
    local legacy_file="$REPO_ROOT/.stm32-probe"

    # 0) Legacy migration: if .stm32-port is absent but .stm32-probe exists,
    #    copy its value over and drop the legacy file.
    if [ ! -f "$cache_file" ] && [ -f "$legacy_file" ]; then
        local legacy
        legacy=$(cat "$legacy_file" 2>/dev/null || true)
        rm -f "$legacy_file" || true
        if [ -n "$legacy" ]; then
            echo "$legacy" > "$cache_file"
        fi
    fi

    # 1) Explicit PROBE env
    if [ "${PROBE:-}" != "" ] && has_token "$PROBE"; then
        echo "$PROBE"
        return 0
    fi

    # 2) PORT alias (rarely used for STM32, but keep symmetry)
    if [ "${PORT:-}" != "" ] && has_token "$PORT"; then
        echo "$PORT"
        return 0
    fi

    # 3) Cached selector (if still present)
    if [ -f "$cache_file" ]; then
        local cached
        cached=$(cat "$cache_file" 2>/dev/null || true)
        if [ "$cached" != "" ] && has_token "$cached"; then
            echo "$cached"
            return 0
        fi
    fi

    # 4) Single ST-Link present
    local st_lines
    st_lines=$(index_lines | grep -Ei 'ST[- ]?Link|0483:3748' || true)
    local count_st
    count_st=$(echo "$st_lines" | grep -E '^\[[0-9]+\]:' | wc -l | tr -d ' ')
    if [ "$count_st" = "1" ]; then
        local sel
        sel=$(echo "$st_lines" | awk -F'-- ' '/--/{print $2}' | awk '{print $1}' | head -n1)
        if [ "$sel" != "" ]; then
            echo "$sel"
            return 0
        fi
    fi

    # 5) Single probe present overall
    local count_all
    count_all=$(index_lines | wc -l | tr -d ' ')
    if [ "$count_all" = "1" ]; then
        local sel
        sel=$(index_lines | awk -F'-- ' '/--/{print $2}' | awk '{print $1}')
        if [ "$sel" != "" ]; then
            echo "$sel"
            return 0
        fi
    fi

    echo "[dual-monitor] unable to pick a unique STM32 debug probe automatically." >&2
    echo "[dual-monitor] set PROBE=VID:PID[:SER] or run 'scripts/select_stm32_probe.sh' once interactively." >&2
    return 1
}

start_session() {
    # args: <seconds> <log_file> <cmd...>
    local seconds="$1"; shift
    local log_file="$1"; shift

    if command -v gtimeout >/dev/null 2>&1; then
        ( gtimeout "$seconds" "$@" >>"$log_file" 2>&1 ) &
        echo $!
        return
    fi

    if command -v timeout >/dev/null 2>&1; then
        ( timeout "$seconds" "$@" >>"$log_file" 2>&1 ) &
        echo $!
        return
    fi

    # Python fallback with explicit timeout and log redirection
    (
        python3 - "$seconds" "$log_file" "$@" <<'PY'
import os
import signal
import subprocess
import sys

if len(sys.argv) < 4:
    sys.exit(2)

timeout = int(sys.argv[1])
log_file = sys.argv[2]
cmd = sys.argv[3:]

os.makedirs(os.path.dirname(log_file), exist_ok=True)
with open(log_file, "a", buffering=1) as log:
    try:
        proc = subprocess.Popen(cmd, stdout=log, stderr=log, preexec_fn=os.setsid)
    except Exception as e:
        print(f"[dual-monitor] failed to start command: {e}", file=sys.stderr)
        sys.exit(1)

    try:
        proc.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        log.write(f"[dual-monitor] timeout {timeout}s reached; stopping session (SIGINT)...\n")
        try:
            os.killpg(proc.pid, signal.SIGINT)
        except ProcessLookupError:
            pass
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
PY
    ) &

    echo $!
}

# Determine selectors first; do not start any session until both are known.
PORT_VALUE=$(select_port)
echo "[dual-monitor] using digital port: ${PORT_VALUE}"

PROBE_SEL=$(select_probe)
echo "[dual-monitor] using analog probe: ${PROBE_SEL}"

timestamp=$(date +"%Y%m%d-%H%M%S")
DIGITAL_LOG="$LOG_DIR/digital-dual-${timestamp}.log"
ANALOG_LOG="$LOG_DIR/analog-dual-${timestamp}.log"
DIGITAL_PID_FILE="$LOG_DIR/digital-dual-${timestamp}.pid"
ANALOG_PID_FILE="$LOG_DIR/analog-dual-${timestamp}.pid"

echo "[dual-monitor] timeout per session: ${TIME_LIMIT_SECONDS}s"
echo "[dual-monitor] digital log: $DIGITAL_LOG"
echo "[dual-monitor] analog  log: $ANALOG_LOG"

# Start analog reset-attach first to ensure the receiver is ready
pid=$(start_session "$TIME_LIMIT_SECONDS" "$ANALOG_LOG" \
    make -C "$REPO_ROOT/firmware/analog" reset-attach PROFILE="$PROFILE" PROBE="$PROBE_SEL" \
         $EXTRA_MAKE_VARS)
echo "$pid" > "$ANALOG_PID_FILE"

# Give analog side time to come up before starting digital
sleep 5

# Then start digital reset-attach
pid=$(start_session "$TIME_LIMIT_SECONDS" "$DIGITAL_LOG" \
    make -C "$REPO_ROOT/firmware/digital" reset-attach PROFILE="$PROFILE" PORT="$PORT_VALUE" \
         ESPFLASH_ARGS="--non-interactive --skip-update-check" \
         $EXTRA_MAKE_VARS)
echo "$pid" > "$DIGITAL_PID_FILE"

echo "[dual-monitor] dual reset-attach sessions launched."
echo "[dual-monitor] PID files:"
echo "  digital: $DIGITAL_PID_FILE"
echo "  analog : $ANALOG_PID_FILE"
