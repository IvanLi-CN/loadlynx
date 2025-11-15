#!/usr/bin/env bash
set -euo pipefail

# Agent-only helper for the ESP32-S3 (digital) firmware.
# - Builds the digital firmware (PROFILE=release by default).
# - Relies on espflash's built-in skip behavior (it will not reflash unchanged
#   regions unless --no-skip is provided).
# - In default mode, runs the equivalent of `make d-run` with a bounded
#   logging window and writes logs to tmp/agent-logs/.
# - In --no-log mode, only performs build + flash (no monitor/logging).
#
# Port selection is delegated to scripts/ensure_esp32_port.sh, which:
#   1) Respects explicit PORT env (if provided and valid).
#   2) Prefers a unique /dev/cu.* entry.
#   3) Falls back to a unique /dev/* entry.
#   4) Otherwise fails and asks for an explicit PORT or .esp32-port cache.

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
LOG_DIR="$REPO_ROOT/tmp/agent-logs"
mkdir -p "$LOG_DIR"

PROFILE="${PROFILE:-release}"
TIME_LIMIT_SECONDS=20
DO_LOG=1
EXTRA_MAKE_VARS=""

usage() {
    cat <<EOF
Usage: scripts/agent_verify_digital.sh [--timeout SECONDS] [--no-log] [EXTRA_MAKE_VARS...]

Environment:
  PORT            Optional explicit serial port (e.g. /dev/cu.usbmodemXXXX).

Behavior:
  - Builds firmware/digital in release profile.
  - Selects a serial port non-interactively via scripts/ensure_esp32_port.sh.
  - By default runs 'make -C firmware/digital run' (flash + monitor) with
    espflash in non-interactive mode, capturing logs for up to --timeout
    seconds into tmp/agent-logs/.
  - When --no-log is passed, only build + flash are performed; no monitor/log
    session is started.
EOF
}

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

run_with_timeout() {
    local seconds="$1"; shift
    local cmd=( "$@" )

    if command -v gtimeout >/dev/null 2>&1; then
        gtimeout "$seconds" "${cmd[@]}"
        return $?
    elif command -v timeout >/dev/null 2>&1; then
        timeout "$seconds" "${cmd[@]}"
        return $?
    elif command -v python3 >/dev/null 2>&1; then
        # Use python3 to enforce a real wall-clock timeout and kill the entire
        # process group (esp. espflash monitor) if it overruns.
        python3 - "$seconds" "${cmd[@]}" <<'PY'
import os
import signal
import subprocess
import sys

if len(sys.argv) < 3:
    sys.exit(2)

timeout = int(sys.argv[1])
cmd = sys.argv[2:]

try:
    # Start the command in a new process group so we can terminate all children.
    proc = subprocess.Popen(cmd, preexec_fn=os.setsid)
except Exception as e:
    print(f"[agent-digital] failed to start command: {e}", file=sys.stderr)
    sys.exit(1)

try:
    proc.wait(timeout=timeout)
    rc = proc.returncode
except subprocess.TimeoutExpired:
    print(f"[agent-digital] timeout {timeout}s reached; stopping logging session (SIGINT)...", file=sys.stderr)
    try:
        os.killpg(proc.pid, signal.SIGINT)
    except ProcessLookupError:
        pass
    try:
        proc.wait(timeout=5)
        # Treat a timeout-induced shutdown as a successful, bounded session.
        rc = 0
    except subprocess.TimeoutExpired:
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        # Even if we had to SIGKILL, consider the session "finished" for the
        # purposes of agent log collection.
        rc = 0

sys.exit(rc)
PY
        return $?
    fi

    # Fallback: manual timeout implementation. Kill the whole process group to
    # ensure espflash children do not keep the session alive.
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

echo "[agent-digital] building firmware (PROFILE=${PROFILE})..."
(
    cd "$REPO_ROOT/firmware/digital"
    PROFILE="$PROFILE" make build $EXTRA_MAKE_VARS
)

if ! PORT_VALUE=$(PORT="${PORT:-}" "$REPO_ROOT/scripts/ensure_esp32_port.sh"); then
    echo "[agent-digital] failed to determine serial port" >&2
    exit 1
fi
echo "[agent-digital] using serial port: ${PORT_VALUE}"

if [ "$DO_LOG" != "1" ]; then
    echo "[agent-digital] no-log mode: performing flash only (no monitor)" >&2
    (
        cd "$REPO_ROOT/firmware/digital"
        run_with_timeout "$TIME_LIMIT_SECONDS" \
            make flash PROFILE="$PROFILE" PORT="$PORT_VALUE" \
                 ESPFLASH_ARGS="--ignore_app_descriptor --non-interactive --skip-update-check" \
                 $EXTRA_MAKE_VARS
    )
    echo "[agent-digital] flash-only mode completed"
    exit 0
fi

timestamp=$(date +"%Y%m%d-%H%M%S")
log_file="$LOG_DIR/digital-${timestamp}.log"
echo "[agent-digital] starting run (flash + monitor) for up to ${TIME_LIMIT_SECONDS}s..."
echo "[agent-digital] log file: $log_file"

    (
        cd "$REPO_ROOT/firmware/digital"
        run_with_timeout "$TIME_LIMIT_SECONDS" \
            make run PROFILE="$PROFILE" PORT="$PORT_VALUE" \
             ESPFLASH_ARGS="--ignore_app_descriptor --non-interactive --skip-update-check" \
             $EXTRA_MAKE_VARS
) | tee "$log_file"

echo "[agent-digital] done. Logs captured in: $log_file"
