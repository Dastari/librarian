# Librarian Distribution Plan

This document captures the distribution strategy, build pipeline, and platform-specific considerations for shipping Librarian as a single binary (plus optional `ffprobe`), with service support, installers, and Docker deployment.

---

## Goals

- Ship a **single binary** for Windows x64/x86 and Ubuntu/Debian (x86_64).
- Allow **one external dependency**: `ffprobe` (bundled where practical).
- Provide a **self-contained Windows installer**.
- Support **service registration/deregistration** on Windows and Unix.
- Bundle the **frontend dist** into the binary (or alongside it) and serve on port `3000`.
- Provide **Unix install scripts** for easy setup.
- Enable **Docker deployment**.
- Avoid nginx in the default distribution; rely on UPNP for port forwarding where possible.
- Standardize **versioning**, reset to `1.0.0`.
- Add **database backup/restore** accessible from both frontend UI and TUI.

---

## Distribution Targets

### Native Binaries

- Windows: `x86_64-pc-windows-msvc`
- Linux: `x86_64-unknown-linux-gnu` (Ubuntu/Debian)
- Optional future targets: `aarch64-unknown-linux-gnu`
- Consider macOS if demand warrants: `x86_64-apple-darwin`, `aarch64-apple-darwin`

### Docker

- Linux `x86_64` image with bundled `ffprobe`
- Optional multi-arch image later

---

## Packaging Strategy

### Single Binary + `ffprobe`

- Default: embed frontend assets into the binary.
- Bundle `ffprobe` alongside the binary when possible:
  - Windows: include `ffprobe.exe` in installer payload.
  - Linux: include `ffprobe` in tarball + install script places it in `libexec/` or `bin/`.
- Provide a runtime config option to point to a system `ffprobe` if installed.
- Document `ffprobe` location overrides in README.

### Frontend Dist Bundling

- Build frontend `dist/` during release.
- Option A (preferred): embed with `rust-embed` or `include_dir!`, serve as static assets.
- Option B: ship `dist/` next to binary and serve from disk.
- Add a build feature flag to select embed vs external assets if needed.
- Current decision: **embed assets in the binary** and serve from an in-memory virtual filesystem.

---

## Windows Installer

### Recommended Tooling

Pick one and standardize:

- **WiX Toolset** (MSI) + `cargo-wix` or custom `.wxs`
- **Inno Setup** (EXE installer)
- **NSIS** (EXE installer)

Preferred path: WiX for enterprise-friendly MSI + upgrades.
Build step should produce **both**:
- MSI installer (WiX)
- EXE installer (Inno Setup or NSIS)

### Installer Contents

- `librarian.exe`
- `ffprobe.exe` (optional)
- Config defaults
- Service install helper (or embedded subcommands)
- Optional desktop/start menu shortcuts
- Uninstaller entry in Windows Apps & Features

### `ffprobe` Installation

- Default path: bundle `ffprobe.exe` directly in the installer payload.
- Alternate path: prompt the user and run `winget install Gyan.FFmpeg` (or similar) during install.
  - Must clearly disclose the external package installation.
  - Fall back to bundled `ffprobe.exe` if `winget` is not available or fails.

### Code Signing

- Use a **self-signed certificate** for now.
- Provide a `scripts/windows/sign.ps1` to:
  - create a self-signed cert
  - sign the EXE/MSI
  - export/import into Trusted Publishers (developer or power users)
- Document that unsigned builds will show SmartScreen warnings.

---

## Service Registration

### Windows

- Implement `librarian service install|uninstall|start|stop|status` subcommands.
- Use `windows-service` crate for native service management.
- Service runs as LocalService or configurable user.
- Installer optionally runs `service install`.
- Provide a user-mode tray option for interactive control (services cannot show UI).
  - Modes: `--tray` for user session, `service` subcommands for background.
  - First-run prompt (or config) chooses tray-first vs service-first.
  - Tray mode can auto-start on login; service mode uses SCM.

### Windows UX Flow (Mode Selection)

- First run (interactive):
  - Prompt: "Run as background service" vs "Run in system tray".
  - Persist choice in config (`run_mode = service|tray`).
- CLI overrides:
  - `--tray` forces tray mode for the session.
  - `--service` forces service mode (no UI).
- Installer:
  - Optional checkbox: "Install as Windows service (recommended for headless)".
  - Optional checkbox: "Start in tray after install".

### Windows Crates (Recommended)

- Service: `windows-service`
- Tray icon: `tray-icon` + `tao` (or `winit` if already in use)

### CLI Flags and Config Schema (Draft)

- CLI flags:
  - `--tray` (force tray mode for the session)
  - `--service` (force service mode for the session)
  - `--server` (force normal server mode)
  - `--run-mode tray|service|server`
- Environment variables:
  - `RUN_MODE=tray|service|server` (default: `server`)
  - `TRAY_AUTOSTART=true|false` (default: `false`)

### Linux (systemd)

- Ship a `librarian.service` unit template.
- Install script places unit at `/etc/systemd/system/librarian.service`.
- Provide `librarian service install|uninstall|start|stop|status` that wraps systemctl when available.

---

## Unix Install Scripts

### Script Responsibilities

- Create `librarian` user/group (optional).
- Create `/opt/librarian` and `/var/lib/librarian` directories.
- Install binary + `ffprobe`.
- Install systemd unit.
- Enable + start service.
- Print access URL and how to manage service.
- Provide an uninstall script that:
  - Stops/disables the systemd service
  - Removes installed files and service unit
  - Optionally removes user/group and data dir with confirmation

### Distribution

- Host in repo: `scripts/install-linux.sh` and `scripts/uninstall-linux.sh`.
- Provide `curl -fsSL ... | bash` option, but also show manual download usage.
- Include checksum validation for release artifacts.
- Use the host package manager to install `ffmpeg`/`ffprobe`:
  - Debian/Ubuntu: `apt-get install ffmpeg`
  - Fedora/RHEL: `dnf install ffmpeg`
  - Arch: `pacman -S ffmpeg`
  - Provide a manual fallback if the package is unavailable.

---

## Docker Deployment

### Image Layout

- `librarian` binary in `/usr/local/bin/`
- `ffprobe` included in image
- Volume for `/data` (DB, config, media cache)

### Runtime

- Default command starts server on port `3000`
- Healthcheck endpoint (if not already present)
- Document environment vars for data paths

---

## Build & Release Pipeline

### GitHub Actions (Recommended)

- Matrix build:
  - Windows x64
  - Linux x86_64
  - Linux aarch64 (recommended to add early)
  - macOS (optional)
- Steps:
  1. Install Rust toolchain + target
  2. Build frontend: `pnpm install` + `pnpm build`
  3. Build backend with embedded assets
  4. Package artifacts:
     - `.zip` for Windows
     - `.tar.gz` for Linux
     - MSI and EXE installers for Windows
  5. Compute checksums
  6. Upload to GitHub Releases

### Build Script

- Add `make distro` (preferred) backed by `scripts/build-distro.sh` to coordinate:
  - frontend build
  - asset embed
  - per-target compilation
  - packaging
  - MSI + EXE generation on Windows

### WSL/Windows Build Environment

The build is expected to run on Windows with WSL available. For MSI/EXE generation, ensure the host has:

- Visual Studio Build Tools (MSVC) for Rust Windows targets
- WiX Toolset (MSI)
- Inno Setup or NSIS (EXE)
- `winget` available for optional ffmpeg install during installer build

If building from WSL, `scripts/build-distro.sh` should invoke Windows host tools via `powershell.exe` (or `cmd.exe`) for MSI/EXE generation.

---

## Versioning Strategy

- Reset project version to `1.0.0`.
- Update versions in:
  - `backend/Cargo.toml` (workspace + crate)
  - `frontend/package.json`
  - any release metadata / README
- Tag releases as `v1.0.0`, `v1.0.1`, etc.

---

## Database Backup & Restore

### Backend Design

- Add API endpoints:
  - `POST /api/backup` → streams a `.db` or `.zip`
  - `POST /api/restore` → accepts uploaded backup
- Alternative (GraphQL-first): use a GraphQL mutation to generate a one-time download token/URL.
- Use SQLite backup APIs or `VACUUM INTO` for safe snapshot.
- Pause jobs while backup/restore runs.
- Validate file integrity before swapping DB.

### Frontend UI

- Settings page section:
  - "Create Backup" button (download file)
  - "Restore Backup" upload with confirmation dialog
  - show last backup timestamp

### TUI

- Add a "Backup/Restore" menu with file path prompts.
- For restore: warn that service will restart.

---

## README Updates (Planned)

- Add **Installation** sections per platform:
  - Windows installer steps
  - Linux install script steps
  - Docker deployment instructions
- Add **Service management** commands.
- Document **`ffprobe`** behavior and overrides.
- Add **Backup/Restore** usage instructions.
- Add **Versioning** notes and upgrade steps.
- Add **Config paths** per platform and portable mode usage.

---

## Risks & Considerations

- Windows code signing: self-signed cert still triggers warnings.
- Anti-virus false positives for bundled binaries.
- `ffprobe` licensing and redistribution terms (verify and document in LICENSE/README).
- Embedding large frontend assets increases binary size.
- Service install requires elevated privileges.
- Optional update checks should avoid auto-install without user consent.

---

## Additional Recommendations

### Platform Scope

- Drop `i686` Windows support; most installs are 64-bit and it simplifies the matrix.
- Add Linux `aarch64` to the initial release matrix if possible (Raspberry Pi, ARM clouds).
- macOS can be added later with launchd service support if demand warrants.

### Update Checks (Optional)

- Add a lightweight "check for updates" endpoint that queries GitHub Releases.
- Surface update availability in UI and TUI; do not auto-install.

### Config Paths and Portable Mode

- Standardize config/data paths:
  - Windows service: `%PROGRAMDATA%\\Librarian\\`
  - Windows user/tray: `%APPDATA%\\Librarian\\`
  - Linux service: `/etc/librarian/` + `/var/lib/librarian/`
  - Linux user: `~/.config/librarian/`
  - Docker: `/data/`
- Add `--portable` flag to keep config/data alongside the binary.
