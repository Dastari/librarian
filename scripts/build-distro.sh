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
  if command -v rg >/dev/null 2>&1; then
    rg -n '^version\\s*=\\s*"' "${BACKEND_DIR}/Cargo.toml" | head -n1 | awk -F'"' '{print $2}'
    return
  fi
  grep -E '^[[:space:]]*version[[:space:]]*=' "${BACKEND_DIR}/Cargo.toml" | head -n1 | sed -E 's/.*"([^"]+)".*/\1/'
}

require pnpm
require cargo

VERSION="$(version_from_cargo)"
log "Building Librarian distro version: ${VERSION}"

mkdir -p "${DIST_DIR}"

BUILD_LINUX=1
BUILD_WINDOWS=1
SKIP_FRONTEND=0
SKIP_WINDOWS_BUILD=0

case "${1:-}" in
  --windows-only)
    BUILD_LINUX=0
    ;;
  --linux-only)
    BUILD_WINDOWS=0
    ;;
  --windows-package-only)
    BUILD_LINUX=0
    BUILD_WINDOWS=1
    SKIP_FRONTEND=1
    SKIP_WINDOWS_BUILD=1
    ;;
esac

if [[ "${SKIP_FRONTEND}" -eq 0 ]]; then
  log "Building frontend..."
  (cd "${FRONTEND_DIR}" && pnpm install && pnpm run build)
fi

if [[ "${BUILD_LINUX}" -eq 1 ]]; then
  log "Building Linux release binary..."
  (cd "${BACKEND_DIR}" && cargo build --release --features embed-frontend)

  mkdir -p "${DIST_DIR}/linux"
  cp "${BACKEND_DIR}/target/release/librarian" "${DIST_DIR}/linux/librarian"

  log "Packaging Linux tarball..."
  tar -czf "${DIST_DIR}/librarian-${VERSION}-linux-x86_64.tar.gz" -C "${DIST_DIR}/linux" librarian
fi

if [[ "${BUILD_WINDOWS}" -eq 1 ]] && is_wsl; then
  log "Building Windows binaries via host toolchain..."

  WIN_BACKEND_DIR="$(wslpath -w "${BACKEND_DIR}")"
  WIN_DIST_DIR="$(wslpath -w "${DIST_DIR}")"
  WIN_WIX_WXS="$(wslpath -w "${WINDOWS_WIX_WXS}")"
  WIN_INNO_ISS="$(wslpath -w "${WINDOWS_INNO_ISS}")"

  if [[ "${SKIP_WINDOWS_BUILD}" -eq 0 ]]; then
    win_run "Set-Location '${WIN_BACKEND_DIR}'; rustup target add x86_64-pc-windows-msvc; cargo build --release --features embed-frontend --target x86_64-pc-windows-msvc"
  fi

  mkdir -p "${DIST_DIR}/windows/x86_64"
  cp "${BACKEND_DIR}/target/x86_64-pc-windows-msvc/release/librarian.exe" "${DIST_DIR}/windows/x86_64/librarian.exe"

  log "Packaging Windows zip artifacts..."
  win_run "Set-Location '${WIN_DIST_DIR}'; Compress-Archive -Path '${WIN_DIST_DIR}\\windows\\x86_64\\librarian.exe' -DestinationPath '${WIN_DIST_DIR}\\librarian-${VERSION}-windows-x86_64.zip' -Force"

  if [[ -f "${WINDOWS_WIX_WXS}" ]]; then
    log "Building MSI via WiX (x86_64)..."
    win_run "\$temp = Join-Path \$env:TEMP 'librarian-dist'; if (Test-Path \$temp) { Remove-Item -Recurse -Force \$temp }; New-Item -ItemType Directory -Force -Path \$temp | Out-Null; Copy-Item -Recurse -Force '${WIN_DIST_DIR}\\windows\\x86_64' (Join-Path \$temp 'windows\\x86_64') | Out-Null; Copy-Item -Force '${WIN_WIX_WXS}' (Join-Path \$temp 'librarian.wxs') | Out-Null; \$wxs = Join-Path \$temp 'librarian.wxs'; \$sourceDir = Join-Path \$temp 'windows\\x86_64'; \$out = Join-Path \$temp 'librarian-${VERSION}-windows-x86_64.msi'; \$wix = (Get-Command wix.exe -ErrorAction SilentlyContinue).Source; if (-not \$wix) { \$wix = Join-Path \$env:USERPROFILE '.dotnet\\tools\\wix.exe' }; Set-Location \$temp; if (Test-Path \$wix) { & \$wix build -d Version='${VERSION}' -d SourceDir=\$sourceDir -o \$out \$wxs } elseif ((Get-Command candle.exe -ErrorAction SilentlyContinue) -and (Get-Command light.exe -ErrorAction SilentlyContinue)) { candle.exe -dVersion='${VERSION}' -dSourceDir=\$sourceDir \$wxs -out 'librarian-x86_64.wixobj'; light.exe 'librarian-x86_64.wixobj' -out \$out } else { Write-Error 'WiX tools not found (wix.exe or candle.exe/light.exe).'; exit 1 }; Copy-Item -Force \$out '${WIN_DIST_DIR}\\librarian-${VERSION}-windows-x86_64.msi'"
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
  if [[ "${BUILD_WINDOWS}" -eq 1 ]]; then
    log "Not running under WSL; skipping Windows builds."
  fi
fi

log "Distro build complete. Artifacts are in ${DIST_DIR}"
