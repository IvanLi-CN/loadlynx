#!/usr/bin/env bash
set -euo pipefail

# Format only staged Rust files without requiring a Cargo workspace at repo root.
files=$(git diff --cached --name-only --diff-filter=ACM -- '*.rs')
if [ -z "${files}" ]; then
  exit 0
fi

repo_root=$(git rev-parse --show-toplevel)

resolve_edition() {
  local path="$1"
  local dir
  dir=$(dirname "$path")

  while [ "$dir" != "." ] && [ "$dir" != "/" ]; do
    if [ -f "$dir/Cargo.toml" ]; then
      local edition
      edition=$(sed -nE 's/^edition *= *"([0-9]+)".*/\1/p' "$dir/Cargo.toml" | head -n 1)
      if [ -n "$edition" ]; then
        printf '%s\n' "$edition"
        return 0
      fi
    fi
    dir=$(dirname "$dir")
  done

  if [ -f "$repo_root/Cargo.toml" ]; then
    local root_edition
    root_edition=$(sed -nE 's/^edition *= *"([0-9]+)".*/\1/p' "$repo_root/Cargo.toml" | head -n 1)
    if [ -n "$root_edition" ]; then
      printf '%s\n' "$root_edition"
      return 0
    fi
  fi

  printf '2024\n'
}

while IFS= read -r file; do
  [ -n "$file" ] || continue
  edition=$(resolve_edition "$file")
  rustfmt --edition "$edition" "$file"
done <<< "$files"
