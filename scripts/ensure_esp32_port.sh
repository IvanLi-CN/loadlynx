#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
CACHE_FILE="$REPO_ROOT/.esp32-port"

err() { printf '%s\n' "$1" >&2; }

cache_selector=""
cache_extras=()

trim_ws() {
  # Trim leading/trailing ASCII whitespace from a string.
  # Usage: trimmed="$(trim_ws "$value")"
  local s="${1:-}"
  s="${s#"${s%%[![:space:]]*}"}"
  s="${s%"${s##*[![:space:]]}"}"
  printf '%s' "$s"
}

load_cache() {
  cache_selector=""
  cache_extras=()

  [ -f "$CACHE_FILE" ] || return 0

  local line raw
  while IFS= read -r raw || [ -n "$raw" ]; do
    line="$(trim_ws "$raw")"
    [ -z "$line" ] && continue
    case "$line" in
      \#*) continue ;;
    esac

    if [ -z "$cache_selector" ]; then
      case "$line" in
        selector=*)
          cache_selector="$(trim_ws "${line#selector=}")"
          ;;
        *)
          cache_selector="$line"
          ;;
      esac
      continue
    fi

    cache_extras+=("$line")
  done < "$CACHE_FILE"

  return 0
}

if ! command -v espflash >/dev/null 2>&1; then
  err "[esp32-port] espflash not found; install via 'cargo install espflash'"
  exit 127
fi

# espflash list-ports 会在设备名前面带空格，这里允许行首存在任意空白后再匹配 /dev/...
PORT_LIST=$(espflash list-ports 2>/dev/null | awk '/^[[:space:]]*\/dev\// {print $1}')
if [ -z "$PORT_LIST" ]; then
  err "[esp32-port] 未检测到任何 ESP32 串口；请插入板子或通过 PORT=/dev/... 指定。"
  exit 1
fi

ALL_PORTS=()
while IFS= read -r line; do
  [ -n "$line" ] && ALL_PORTS+=("$line")
done <<<"$PORT_LIST"

CU_LIST=$(printf '%s
' "${ALL_PORTS[@]}" 2>/dev/null | grep '^/dev/cu\.' || true)
CU_PORTS=()
while IFS= read -r line; do
  [ -n "$line" ] && CU_PORTS+=("$line")
done <<<"$CU_LIST"

contains_port() {
  local needle="$1"
  shift || true
  local entry
  for entry in "$@"; do
    if [ "$entry" = "$needle" ]; then
      return 0
    fi
  done
  return 1
}

pick_port() {
  local candidate="$1"
  load_cache
  if [ "$candidate" = "$cache_selector" ] && [ "${#cache_extras[@]}" -gt 0 ]; then
    {
      printf '%s\n' "$candidate"
      for line in "${cache_extras[@]}"; do
        printf '%s\n' "$line"
      done
    } > "$CACHE_FILE"
  else
    printf '%s\n' "$candidate" > "$CACHE_FILE"
  fi
  echo "$candidate"
}

if [ -n "${PORT:-}" ]; then
  if contains_port "$PORT" "${ALL_PORTS[@]}"; then
    pick_port "$PORT"
    exit 0
  fi
  err "[esp32-port] 指定的 PORT=$PORT 当前不可用；有效端口: ${ALL_PORTS[*]}"
  exit 1
fi

if [ "${#CU_PORTS[@]}" -eq 1 ]; then
  pick_port "${CU_PORTS[0]}"
  exit 0
fi

if [ "${#ALL_PORTS[@]}" -eq 1 ]; then
  pick_port "${ALL_PORTS[0]}"
  exit 0
fi

if [ -f "$CACHE_FILE" ]; then
  load_cache
  if [ -n "$cache_selector" ] && contains_port "$cache_selector" "${ALL_PORTS[@]}"; then
    echo "$cache_selector"
    exit 0
  fi
fi

# Interactive selection when multiple ports are present and we are in a TTY.
if [ -t 0 ] && [ -t 1 ]; then
  err "[esp32-port] 检测到多个串口: ${ALL_PORTS[*]}"
  err "[esp32-port] 请选择要使用的端口（或 Ctrl+C 退出）："
  PS3="[esp32-port] 输入编号并回车: "
  select choice in "${ALL_PORTS[@]}" "取消/Cancel"; do
    case "$choice" in
      "取消/Cancel"|"")
        err "[esp32-port] 已取消。使用 PORT=/dev/cu.xxx make d-reset-attach 或写入 .esp32-port 指定。"
        exit 1 ;;
      *)
        if contains_port "$choice" "${ALL_PORTS[@]}"; then
          pick_port "$choice"
          exit 0
        fi
        err "[esp32-port] 无效选择，请重新输入。" ;;
    esac
  done
fi

err "[esp32-port] 检测到多个串口: ${ALL_PORTS[*]}"
err "[esp32-port] 请使用 PORT=/dev/cu.xxx make d-reset-attach（或写入 .esp32-port）来选择。"
exit 1
