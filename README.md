# Librarian

Librarian is a local‑first media library that downloads, organizes, and streams your movies, shows, music, and audiobooks from a single machine or NAS. It runs privately on your hardware and provides a modern web UI for browsing and playback.

## What It Does
- Manages **Movies, TV Shows, Music, and Audiobooks** in one place
- Automates **downloading and organizing** into clean folder structures
- Streams in your browser with **persistent video and music players**
- Supports **Chromecast, AirPlay, and DLNA** casting
- Works offline and keeps your library **local and private**

## How It Runs
- Distributed as a **single binary** with the frontend bundled inside
- Windows installer and MSI options
- Linux/NAS friendly with optional systemd service support
- No nginx required

## Getting Started (Users)
### Windows
1. Download the installer or MSI
2. Run it and choose “Service” or “Tray” mode
3. Open the web UI from the provided URL

### Linux / NAS
1. Download the binary release for your platform
2. Run it directly or install it as a systemd service
3. Open the web UI from the provided URL

## For Developers
- Design and architecture: `docs/design.md`
- Development commands: `make dev`, `make test`, `make lint`

---

If you want to contribute or extend Librarian, start with the design document to understand the full intended scope and rules.
