#!/usr/bin/env bash
set -euo pipefail

# Agent-only helper for the STM32G431 (analog) firmware.
# - Builds the analog firmware (PROFILE=release by default).
# - Uses a version string embedded by firmware/analog/build.rs to decide
#   whether a fresh flash is required.
# - If the board is already running the same build, only performs a
#   reset-attach (no extra flash).
# - In all cases, performs a reset-attach logging session with a bounded
#   duration and writes logs to tmp/agent-logs/.
#
# This script is intended to be non-interactive. Probe selection follows:
#   1) Respect explicit PROBE env (if present and currently connected).
#   2) Respect PORT env alias (if present and currently connected).
#   3) Reuse cached selector from .stm32-probe (if still connected).
#   4) If exactly one ST-Link is present, use it.
#   5) If exactly one probe is present, use it.
#   6) Otherwise, fail with an error (no interactive selection).

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
LOG_DIR="$REPO_ROOT/tmp/agent-logs"
mkdir -p "$LOG_DIR"

PROFILE="${PROFILE:-release}"
TIME_LIMIT_SECONDS=20

usage() {
    cat <<EOF
Usage: scripts/agent_verify_analog.sh [--timeout SECONDS] [--profile {release|dev}] [EXTRA_MAKE_VARS...]

Environment:
  PROBE           Optional explicit probe selector (VID:PID[:SER] or serial).
  PROFILE         Build profile (defaults to 'release').

Behavior:
  - Builds firmware/analog with the selected PROFILE.
  - Reads tmp/analog-fw-version.txt to detect the current build identity.
  - If this differs from tmp/analog-fw-last-flashed.txt, performs a flash
    via 'make -C firmware/analog flash' and updates the last-flashed marker.
  - Always performs 'make -C firmware/analog reset-attach' afterwards,
    capturing logs for up to --timeout seconds into tmp/agent-logs/.
EOF
}

EXTRA_MAKE_VARS=()

while [ $# -gt 0 ]; do
    case "$1" in
        --timeout)
            if [ $# -lt 2 ]; then
                echo "[agent-analog] missing value for --timeout" >&2
                exit 2
            fi
            TIME_LIMIT_SECONDS="$2"
            shift 2
            ;;
        --profile)
            if [ $# -lt 2 ]; then
                echo "[agent-analog] missing value for --profile" >&2
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
            EXTRA_MAKE_VARS+=("$1")
            shift
            ;;
    esac
done

select_probe() {
    if ! command -v probe-rs >/dev/null 2>&1; then
        echo "[agent-analog] probe-rs not found; install with 'cargo install probe-rs'" >&2
        return 127
    fi

    local all_output
    all_output=$(probe-rs list || true)

    # Helper: filter lines that represent indexed probes
    index_lines() {
        echo "$all_output" | grep -E '^[[:space:]]*[0-9]+:|^\[[0-9]+\]:' || true
    }

    # Extract tokens (selector strings after '-- ')
    tokens() { echo "$all_output" | sed -n 's/.*-- \([^ ]\+\).*/\1/p'; }
    has_token() { tokens | grep -Fxq "$1"; }

    local repo_cache="$REPO_ROOT/.stm32-probe"

    # 1) Explicit PROBE env
    if [ "${PROBE:-}" != "" ] && has_token "$PROBE"; then
        echo "$PROBE"
        return 0
    fi

    # 2) PORT alias
    if [ "${PORT:-}" != "" ] && has_token "$PORT"; then
        echo "$PORT"
        return 0
    fi

    # 3) Cached selector (if still present)
    if [ -f "$repo_cache" ]; then
        local cached
        cached=$(cat "$repo_cache" 2>/dev/null || true)
        if [ "$cached" != "" ] && has_token "$cached"; then
            echo "$cached"
            return 0
        fi
    fi

    # 4) Single ST-Link present
    local st_lines
    st_lines=$(index_lines | grep -Ei 'ST[- ]?Link|0483:3748' || true)
    local count_st
    count_st=$(echo "$st_lines" | grep -E '^[[:space:]]*[0-9]+:|^\[[0-9]+\]:' | wc -l | tr -d ' ')
    if [ "$count_st" = "1" ]; then
        local sel
        sel=$(echo "$st_lines" | sed -n 's/.*-- \([^ ]\+\).*/\1/p' | head -n1)
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
        sel=$(index_lines | sed -n 's/.*-- \([^ ]\+\).*/\1/p')
        if [ "$sel" != "" ]; then
            echo "$sel"
            return 0
        fi
    fi

    echo "[agent-analog] unable to pick a unique debug probe automatically." >&2
    echo "[agent-analog] set PROBE=VID:PID[:SER] or run 'scripts/select_stm32_probe.sh' once interactively." >&2
    return 1
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

    # Fallback: manual timeout implementation.
    "${cmd[@]}" &
    local cmd_pid=$!

    (
        sleep "$seconds"
        if kill -0 "$cmd_pid" 2>/dev/null; then
            echo "[agent-analog] timeout ${seconds}s reached; stopping logging session (SIGINT)..." >&2
            kill -INT "$cmd_pid" 2>/dev/null || kill "$cmd_pid" 2>/dev/null || true
        fi
    ) &
    local watcher_pid=$!

    wait "$cmd_pid" || true
    kill "$watcher_pid" 2>/dev/null || true
}

ANALOG_BUILD_VERSION_FILE="$REPO_ROOT/tmp/analog-fw-version.txt"
ANALOG_LAST_FLASHED_FILE="$REPO_ROOT/tmp/analog-fw-last-flashed.txt"

echo "[agent-analog] building firmware (PROFILE=${PROFILE})..."
(
    cd "$REPO_ROOT/firmware/analog"
    PROFILE="$PROFILE" DEFMT_LOG="${DEFMT_LOG:-info}" make build "${EXTRA_MAKE_VARS[@]}"
)

BUILD_VERSION="unknown"
if [ -f "$ANALOG_BUILD_VERSION_FILE" ]; then
    BUILD_VERSION=$(cat "$ANALOG_BUILD_VERSION_FILE" 2>/dev/null || echo "unknown")
fi

LAST_VERSION="none"
if [ -f "$ANALOG_LAST_FLASHED_FILE" ]; then
    LAST_VERSION=$(cat "$ANALOG_LAST_FLASHED_FILE" 2>/dev/null || echo "none")
fi

echo "[agent-analog] build version: ${BUILD_VERSION}"
echo "[agent-analog] last flashed: ${LAST_VERSION}"

NEED_FLASH=0
if [ "$BUILD_VERSION" = "unknown" ]; then
    echo "[agent-analog] build version unknown; will flash to be safe." >&2
    NEED_FLASH=1
elif [ "$BUILD_VERSION" != "$LAST_VERSION" ]; then
    echo "[agent-analog] build version differs from last flashed; flashing..." >&2
    NEED_FLASH=1
fi

PROBE_SEL=$(select_probe)
echo "[agent-analog] using probe selector: ${PROBE_SEL}"

if [ "$NEED_FLASH" = "1" ]; then
    (
        cd "$REPO_ROOT/firmware/analog"
        PROFILE="$PROFILE" PROBE="$PROBE_SEL" make flash "${EXTRA_MAKE_VARS[@]}"
    )
    echo "$BUILD_VERSION" > "$ANALOG_LAST_FLASHED_FILE"
fi

timestamp=$(date +"%Y%m%d-%H%M%S")
log_file="$LOG_DIR/analog-${timestamp}.log"
echo "[agent-analog] starting reset-attach logging for up to ${TIME_LIMIT_SECONDS}s..."
echo "[agent-analog] log file: $log_file"

(
    cd "$REPO_ROOT/firmware/analog"
    run_with_timeout "$TIME_LIMIT_SECONDS" \
        make reset-attach PROFILE="$PROFILE" PROBE="$PROBE_SEL" "${EXTRA_MAKE_VARS[@]}"
) | tee "$log_file"

echo "[agent-analog] done. Logs captured in: $log_file"
