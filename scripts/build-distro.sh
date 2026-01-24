#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
BACKEND_DIR="${ROOT_DIR}/backend"
FRONTEND_DIR="${ROOT_DIR}/frontend"
WINDOWS_WIX_WXS="${ROOT_DIR}/installer/windows/librarian.wxs"
WINDOWS_INNO_ISS="${ROOT_DIR}/installer/windows/librarian.iss"

log() {
  printf '%s\n' "$*"
}

require() {
  if ! command -v "$1" >/dev/null 2>&1; then
    log "Missing required command: $1"
    exit 1
  fi
}

is_wsl() {
  grep -qi microsoft /proc/version 2>/dev/null
}

win_run() {
  local cmd="$1"
  powershell.exe -NoProfile -Command "$cmd"
}

version_from_cargo() {
  rg -n '^version\\s*=\\s*"' "${BACKEND_DIR}/Cargo.toml" | head -n1 | awk -F'"' '{print $2}'
}

require rg
require pnpm
require cargo

VERSION="$(version_from_cargo)"
log "Building Librarian distro version: ${VERSION}"

mkdir -p "${DIST_DIR}"

log "Building frontend..."
(cd "${FRONTEND_DIR}" && pnpm install && pnpm run build)

log "Building Linux release binary..."
(cd "${BACKEND_DIR}" && cargo build --release --features embed-frontend)

mkdir -p "${DIST_DIR}/linux"
cp "${BACKEND_DIR}/target/release/librarian" "${DIST_DIR}/linux/librarian"

log "Packaging Linux tarball..."
tar -czf "${DIST_DIR}/librarian-${VERSION}-linux-x86_64.tar.gz" -C "${DIST_DIR}/linux" librarian

if is_wsl; then
  log "Building Windows binaries via host toolchain..."

  WIN_BACKEND_DIR="$(wslpath -w "${BACKEND_DIR}")"
  WIN_DIST_DIR="$(wslpath -w "${DIST_DIR}")"
  WIN_WIX_WXS="$(wslpath -w "${WINDOWS_WIX_WXS}")"
  WIN_INNO_ISS="$(wslpath -w "${WINDOWS_INNO_ISS}")"

  win_run "Set-Location '${WIN_BACKEND_DIR}'; rustup target add x86_64-pc-windows-msvc; cargo build --release --features embed-frontend --target x86_64-pc-windows-msvc"

  mkdir -p "${DIST_DIR}/windows/x86_64"
  cp "${BACKEND_DIR}/target/x86_64-pc-windows-msvc/release/librarian.exe" "${DIST_DIR}/windows/x86_64/librarian.exe"

  if command -v zip >/dev/null 2>&1; then
    log "Packaging Windows zip artifacts..."
    (cd "${DIST_DIR}/windows/x86_64" && zip -r "${DIST_DIR}/librarian-${VERSION}-windows-x86_64.zip" librarian.exe)
  else
    log "zip not found; skipping Windows zip packaging."
  fi

  if [[ -f "${WINDOWS_WIX_WXS}" ]]; then
    log "Building MSI via WiX (x86_64)..."
    win_run "Set-Location '${WIN_DIST_DIR}'; candle.exe -dVersion='${VERSION}' -dSourceDir='${WIN_DIST_DIR}\\windows\\x86_64' '${WIN_WIX_WXS}' -out 'librarian-x86_64.wixobj'; light.exe 'librarian-x86_64.wixobj' -out 'librarian-${VERSION}-windows-x86_64.msi'"
  else
    log "Missing ${WINDOWS_WIX_WXS}; skipping MSI build."
  fi

  if [[ -f "${WINDOWS_INNO_ISS}" ]]; then
    log "Building EXE installer via Inno Setup (x86_64)..."
    win_run "Set-Location '${WIN_DIST_DIR}'; iscc.exe /DMyAppVersion='${VERSION}' /DSourceDir='${WIN_DIST_DIR}\\windows\\x86_64' /DMyAppArch='x64' '${WIN_INNO_ISS}'"
  else
    log "Missing ${WINDOWS_INNO_ISS}; skipping EXE installer build."
  fi
else
  log "Not running under WSL; skipping Windows builds."
fi

log "Distro build complete. Artifacts are in ${DIST_DIR}"
