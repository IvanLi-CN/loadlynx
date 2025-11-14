#!/usr/bin/env bash
set -euo pipefail

# Agent-only helper for the ESP32-S3 (digital) firmware.
# - Builds the digital firmware (PROFILE=release by default).
# - Uses a version string embedded by firmware/digital/build.rs to decide
#   whether a fresh flash is required.
# - If the board is already running the same build, only performs a
#   reset-attach (no extra flash).
# - In all cases, performs a reset-attach logging session with a bounded
#   duration and writes logs to tmp/agent-logs/.
#
# This script is intended to be non-interactive. Port selection follows:
#   1) Respect explicit PORT env (if provided).
#   2) If espflash list-ports reports exactly one /dev/cu.* entry, use it.
#   3) If there is a single serial port candidate overall, use it.
#   4) Otherwise, fail with an error and request an explicit PORT.

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
LOG_DIR="$REPO_ROOT/tmp/agent-logs"
mkdir -p "$LOG_DIR"

PROFILE="${PROFILE:-release}"
TIME_LIMIT_SECONDS=20
# Hard cap for any non-build phase (flash, reset-attach); build time is unbounded.
PHASE_HARD_LIMIT_SECONDS=30
DO_LOG=1

usage() {
    cat <<EOF
Usage: scripts/agent_verify_digital.sh [--timeout SECONDS] [--profile {release|dev}] [--no-log] [EXTRA_MAKE_VARS...]

Environment:
  PORT            Optional explicit serial port (e.g. /dev/cu.usbmodemXXXX).
  PROFILE         Build profile (defaults to 'release').

Behavior:
  - Builds firmware/digital with the selected PROFILE.
  - Reads tmp/digital-fw-version.txt to detect the current build identity.
  - If this differs from tmp/digital-fw-last-flashed.txt, performs a flash
    via 'make -C firmware/digital flash' and updates the last-flashed marker.
  - By default performs 'make -C firmware/digital reset-attach' afterwards,
    capturing logs for up to --timeout seconds into tmp/agent-logs/.
  - When --no-log is passed, only build/flash are performed; no reset-attach
    session is started.
EOF
}

EXTRA_MAKE_VARS=""

while [ $# -gt 0 ]; do
    case "$1" in
        --no-log)
            DO_LOG=0
            shift
            ;;
        --timeout)
            if [ $# -lt 2 ]; then
                echo "[agent-digital] missing value for --timeout" >&2
                exit 2
            fi
            TIME_LIMIT_SECONDS="$2"
            shift 2
            ;;
        --profile)
            if [ $# -lt 2 ]; then
                echo "[agent-digital] missing value for --profile" >&2
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
        echo "[agent-digital] espflash not found; install with 'cargo install espflash'" >&2
        return 127
    fi

    local output
    output=$(espflash list-ports || true)

    local port_lines
    port_lines=$(echo "$output" | grep '^/dev/' || true)

    if [ -z "$port_lines" ]; then
        echo "[agent-digital] no serial ports detected; set PORT explicitly." >&2
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
        echo "[agent-digital] multiple serial ports detected:" >&2
        echo "$port_lines" >&2
        echo "[agent-digital] set PORT explicitly (e.g. PORT=/dev/cu.usbmodemXXXX)." >&2
        return 1
    fi

    local chosen_port
    chosen_port=$(echo "$chosen_line" | awk '{print $1}')
    if [ -z "$chosen_port" ]; then
        echo "[agent-digital] failed to parse serial port from espflash output." >&2
        return 1
    fi

    echo "$chosen_port"
}

run_with_timeout() {
    local seconds="$1"; shift
    local cmd=( "$@" )

    if command -v gtimeout >/dev/null 2>&1; then
        gtimeout "$seconds" "${cmd[@]}"
        return $?
    elif command -v timeout >/dev/null 2>&1; then
        timeout "$seconds" "${cmd[@]}"
        return $?
    fi

    # Fallback: manual timeout implementation. Kill the whole process group to
    # ensure espflash/probe-rs children do not keep the session alive.
    "${cmd[@]}" &
    local cmd_pid=$!

    (
        sleep "$seconds"
        if kill -0 "$cmd_pid" 2>/dev/null; then
            echo "[agent-digital] timeout ${seconds}s reached; stopping logging session (SIGINT)..." >&2
            # Negative PID = process group
            kill -INT "-$cmd_pid" 2>/dev/null || kill "-$cmd_pid" 2>/dev/null || kill -KILL "-$cmd_pid" 2>/dev/null || true
        fi
    ) &
    local watcher_pid=$!

    wait "$cmd_pid" || true
    kill "$watcher_pid" 2>/dev/null || true
}

DIGITAL_BUILD_VERSION_FILE="$REPO_ROOT/tmp/digital-fw-version.txt"
DIGITAL_LAST_FLASHED_FILE="$REPO_ROOT/tmp/digital-fw-last-flashed.txt"

echo "[agent-digital] building firmware (PROFILE=${PROFILE})..."
(
    cd "$REPO_ROOT/firmware/digital"
    PROFILE="$PROFILE" make build $EXTRA_MAKE_VARS
)

BUILD_VERSION="unknown"
if [ -f "$DIGITAL_BUILD_VERSION_FILE" ]; then
    BUILD_VERSION=$(cat "$DIGITAL_BUILD_VERSION_FILE" 2>/dev/null || echo "unknown")
fi

LAST_VERSION="none"
if [ -f "$DIGITAL_LAST_FLASHED_FILE" ]; then
    LAST_VERSION=$(cat "$DIGITAL_LAST_FLASHED_FILE" 2>/dev/null || echo "none")
fi

echo "[agent-digital] build version: ${BUILD_VERSION}"
echo "[agent-digital] last flashed: ${LAST_VERSION}"

NEED_FLASH=0
if [ "$BUILD_VERSION" = "unknown" ]; then
    echo "[agent-digital] build version unknown; will flash to be safe." >&2
    NEED_FLASH=1
elif [ "$BUILD_VERSION" != "$LAST_VERSION" ]; then
    echo "[agent-digital] build version differs from last flashed; flashing..." >&2
    NEED_FLASH=1
fi

PORT_VALUE=$(select_port)
echo "[agent-digital] using serial port: ${PORT_VALUE}"

if [ "$NEED_FLASH" = "1" ]; then
    (
        cd "$REPO_ROOT/firmware/digital"
        # Limit flash phase to a hard 30s wall-clock; build time is uncapped.
        run_with_timeout "$PHASE_HARD_LIMIT_SECONDS" \
            make flash PROFILE="$PROFILE" PORT="$PORT_VALUE" \
                 ESPFLASH_ARGS="--ignore_app_descriptor --non-interactive --skip-update-check" \
                 $EXTRA_MAKE_VARS
    )
    echo "$BUILD_VERSION" > "$DIGITAL_LAST_FLASHED_FILE"
fi

if [ "$DO_LOG" != "1" ]; then
    echo "[agent-digital] no-log mode: skipping reset-attach logging phase" >&2
    exit 0
fi

timestamp=$(date +"%Y%m%d-%H%M%S")
log_file="$LOG_DIR/digital-${timestamp}.log"
echo "[agent-digital] starting reset-attach logging for up to ${TIME_LIMIT_SECONDS}s..."
echo "[agent-digital] log file: $log_file"

# Logging window is controlled by --timeout, but never exceeds the per-phase hard cap.
LOG_TIMEOUT="$TIME_LIMIT_SECONDS"
if [ "$LOG_TIMEOUT" -gt "$PHASE_HARD_LIMIT_SECONDS" ]; then
    LOG_TIMEOUT="$PHASE_HARD_LIMIT_SECONDS"
fi

(
    cd "$REPO_ROOT/firmware/digital"
    run_with_timeout "$LOG_TIMEOUT" \
        make reset-attach PROFILE="$PROFILE" PORT="$PORT_VALUE" \
             ESPFLASH_ARGS="--non-interactive --skip-update-check" \
             $EXTRA_MAKE_VARS
) | tee "$log_file"

echo "[agent-digital] done. Logs captured in: $log_file"
