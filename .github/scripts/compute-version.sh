#!/usr/bin/env bash
set -euo pipefail

# Version strategy:
# 1) Base version comes from APP_BASE_VERSION if set, otherwise from web/package.json "version".
# 2) git metadata adds uniqueness: short SHA always appended except when explicitly on main and a clean base is desired.
# 3) main branch -> <base>+<sha>; other branches -> <base>-dev+<sha>.
# 4) Prints one line APP_EFFECTIVE_VERSION=... to stdout; logs the computed value to stderr.

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo_root"

# Prefer explicit base version
base_version="${APP_BASE_VERSION:-}"

if [ -z "$base_version" ]; then
  if [ ! -f "$repo_root/web/package.json" ]; then
    echo "package.json not found under web/" >&2
    exit 1
  fi
  base_version=$(sed -n 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"\(.*\)".*/\1/p' "$repo_root/web/package.json" | head -n 1)
fi

if [ -z "$base_version" ]; then
  echo "Failed to determine base version" >&2
  exit 1
fi

# git info
short_sha=$(git rev-parse --short HEAD)
branch_name=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "detached")

# Optional latest tag (for debugging/visibility)
latest_tag=$(git describe --tags --abbrev=0 2>/dev/null || true)

if [ "$branch_name" = "main" ]; then
  app_version="${base_version}+${short_sha}"
else
  app_version="${base_version}-dev+${short_sha}"
fi

APP_EFFECTIVE_VERSION="$app_version"

if [ -n "$latest_tag" ]; then
  echo "[version] latest tag: $latest_tag" >&2
fi
echo "[version] branch: $branch_name, sha: $short_sha" >&2
echo "Computed APP_EFFECTIVE_VERSION=${APP_EFFECTIVE_VERSION}" >&2

echo "APP_EFFECTIVE_VERSION=${APP_EFFECTIVE_VERSION}"
