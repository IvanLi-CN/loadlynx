#!/usr/bin/env bash
set -euo pipefail

REPO_URL="https://github.com/IvanLi-CN/loadlynx"
VERSION="latest"
INSTALL_DIR="${HOME}/.local/bin"
FORCE=0
DRY_RUN=0

usage() {
  cat <<'EOF'
Install LoadLynx host tools for the current user.

Usage:
  install-loadlynx-host.sh [--version <tag>] [--install-dir <dir>] [--force] [--dry-run]

Defaults:
  --version latest
  --install-dir ~/.local/bin
EOF
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      [ "$#" -ge 2 ] || die "--version requires a value"
      VERSION="$2"
      shift 2
      ;;
    --install-dir)
      [ "$#" -ge 2 ] || die "--install-dir requires a value"
      INSTALL_DIR="$2"
      shift 2
      ;;
    --force)
      FORCE=1
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

case "$(uname -s)" in
  Darwin)
    case "$(uname -m)" in
      arm64) SLUG="macos-aarch64" ;;
      x86_64) SLUG="macos-x86_64" ;;
      *) die "unsupported macOS architecture: $(uname -m)" ;;
    esac
    ;;
  Linux)
    [ "$(uname -m)" = "x86_64" ] || die "unsupported Linux architecture: $(uname -m); expected x86_64"
    SLUG="linux-x86_64"
    ;;
  *)
    die "unsupported operating system: $(uname -s)"
    ;;
esac

ARCHIVE="loadlynx-host-tools-${SLUG}.tar.gz"
if [ "$VERSION" = "latest" ]; then
  BASE_URL="${REPO_URL}/releases/latest/download"
else
  BASE_URL="${REPO_URL}/releases/download/${VERSION}"
fi
ARCHIVE_URL="${BASE_URL}/${ARCHIVE}"
CHECKSUM_URL="${BASE_URL}/SHA256SUMS"

printf 'LoadLynx host tools install plan\n'
printf '  source: %s\n' "$BASE_URL"
printf '  archive: %s\n' "$ARCHIVE"
printf '  install_dir: %s\n' "$INSTALL_DIR"
printf '  force: %s\n' "$FORCE"

if [ "$DRY_RUN" -eq 1 ]; then
  printf 'dry-run: no files downloaded or installed\n'
  exit 0
fi

need_cmd tar
need_cmd curl

if command -v shasum >/dev/null 2>&1; then
  SHA_CMD="shasum -a 256"
elif command -v sha256sum >/dev/null 2>&1; then
  SHA_CMD="sha256sum"
else
  die "missing checksum command: shasum or sha256sum"
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

archive_path="${tmp_dir}/${ARCHIVE}"
checksums_path="${tmp_dir}/SHA256SUMS"
archive_effective="${tmp_dir}/archive.effective-url"

curl -fsSL -w '%{url_effective}' -o "$archive_path" "$ARCHIVE_URL" > "$archive_effective"
curl -fsSL -o "$checksums_path" "$CHECKSUM_URL"

target_tag="$VERSION"
if [ "$VERSION" = "latest" ]; then
  target_tag="$(sed -n 's#.*\/releases\/download\/\([^/]*\)\/.*#\1#p' "$archive_effective")"
  [ -n "$target_tag" ] || target_tag="latest"
fi

expected_sha="$(awk -v file="$ARCHIVE" '$2 == file { print $1 }' "$checksums_path")"
[ -n "$expected_sha" ] || die "SHA256SUMS does not contain ${ARCHIVE}"
actual_sha="$($SHA_CMD "$archive_path" | awk '{ print $1 }')"
[ "$expected_sha" = "$actual_sha" ] || die "checksum mismatch for ${ARCHIVE}"

extract_dir="${tmp_dir}/extract"
mkdir -p "$extract_dir"
tar -xzf "$archive_path" -C "$extract_dir"

[ -f "${extract_dir}/loadlynx" ] || die "archive missing loadlynx"
[ -f "${extract_dir}/loadlynx-devd" ] || die "archive missing loadlynx-devd"

if [ -x "${INSTALL_DIR}/loadlynx" ] && [ "$FORCE" -ne 1 ]; then
  installed_version="$("${INSTALL_DIR}/loadlynx" --version 2>/dev/null | awk '{print $NF}' || true)"
  if [ -n "$installed_version" ] && [ "$target_tag" != "latest" ] && [ "$installed_version" = "$target_tag" ]; then
    printf 'loadlynx %s is already installed; use --force to reinstall\n' "$installed_version"
    exit 0
  fi
fi

mkdir -p "$INSTALL_DIR"
install -m 0755 "${extract_dir}/loadlynx" "${INSTALL_DIR}/loadlynx"
install -m 0755 "${extract_dir}/loadlynx-devd" "${INSTALL_DIR}/loadlynx-devd"

"${INSTALL_DIR}/loadlynx" --help >/dev/null
"${INSTALL_DIR}/loadlynx-devd" --help >/dev/null

printf 'installed LoadLynx host tools to %s\n' "$INSTALL_DIR"
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    printf 'PATH note: add this directory before using loadlynx from a new shell:\n'
    printf '  export PATH="%s:$PATH"\n' "$INSTALL_DIR"
    ;;
esac
