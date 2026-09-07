# Librarian

Librarian is a local‑first media library that downloads, organizes, and streams your movies, shows, music, and audiobooks from a single machine or NAS. It runs privately on your hardware and provides a modern web UI for browsing and playback.

## What It Does
- Manages **Movies, TV Shows, Music, and Audiobooks** in one place
- Downloads through the built-in **torrent client** and organizes media into clean folder structures
- Streams in your browser with **persistent video and music players**
- Supports **Chromecast/Google Cast** playback; DLNA devices are discovery-only
- Works offline and keeps your library **local and private**

## How It Runs
- Distributed as a **single binary** with the frontend bundled inside
- Runs in server mode on Linux, macOS, and Windows
- Linux/NAS friendly; service-manager packaging is deployment-specific
- No nginx required

Usenet/NNTP acquisition, first-party AirPlay control, DLNA playback, automated archive extraction,
and Windows service/tray lifecycle are not currently supported.

## Getting Started (Users)
### Windows
1. Download or build the server binary
2. Run it in the supported server mode
3. Open the web UI from the configured URL

### Linux / NAS
1. Download the binary release for your platform
2. Run it directly or install it as a systemd service
3. Open the web UI from the provided URL

## For Developers
- Design and architecture: `docs/design.md`
- Development commands: `make dev`, `make test`, `make lint`

---

If you want to contribute or extend Librarian, start with the design document to understand the full intended scope and rules.
