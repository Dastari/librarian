## Librarian — System Design

### Goals & Scope

- **Local‑first, privacy‑preserving media library**
- **Offline‑first** on a single machine/NAS, with optional remote access
- **Features**:
  - **Torrents**: download via qBittorrent
  - **Streaming**: in-browser HLS, plus casting (Chromecast, AirPlay)
  - **Playback**: integrated web UI
  - **Metadata**: TV shows, movies, cover art from TheTVDB and TMDB
  - **Subscriptions**: monitor shows; auto-fill gaps via indexers/feeds
  - **Organization**: auto-rename and file layout

### High‑Level Architecture

- **Frontend**: TanStack Start (React with TanStack Router)
- **Backend**: Rust (Axum + Tokio), background workers, job queue
- **Identity & DB**: Supabase (Postgres + Auth + Storage) running locally via Docker
- **Torrent Engine**: `qbittorrent-nox` (headless) via its Web API
- **Indexer Management**: Prowlarr (recommended) or Jackett
- **Transcoding/Packaging**: FFmpeg/FFprobe → HLS (m3u8 + TS/MP4 segments)
- **Casting**:
  - **Chromecast**: Google Cast Web Sender SDK in frontend (cast HLS URL)
  - **AirPlay**: native Safari AirPlay support on the `<video>` element
- **File Watching / Library Scanner**: Rust watcher + periodic full scan
- **Object Storage**: Supabase Storage for posters/backdrops/fanart

## Key Technology Choices (recommended)

### Frontend (TanStack Start)

- **Framework**: TanStack Start with TanStack Router (file-based routing)
- **Language**: TypeScript across the stack
- **UI**: HeroUI (formerly NextUI) + Tailwind CSS
- **Package Manager**: pnpm
- **Auth**: `@supabase/supabase-js` v2 with client helpers
- **Video Playback**: `hls.js` for HLS where needed
- **Casting**:
  - Google Cast Web Sender SDK (loaded where casting is available)
  - Safari’s native AirPlay button on `<video>` provides AirPlay

### Backend (Rust)

- **Web framework**: `axum` (async, router-first, tower-compatible)
- **Async runtime**: `tokio`
- **DB**: `sqlx` (Postgres) with compile‑time checked queries
- **Auth/JWT**: verify Supabase JWTs via JWKS using `jsonwebtoken` or `josekit`; cache keys
- **HTTP client**: `reqwest` for external APIs (TheTVDB/TMDB/qBittorrent/Prowlarr)
- **Torrent control**: qBittorrent Web API (avoid FFI to libtorrent)
- **Scheduler / Jobs**: `apalis` (Redis-backed) or `tokio-cron-scheduler` (no external deps)
- **Filesystem**: `notify` (watcher), `walkdir`, `tokio::fs`
- **Renaming**: `regex`, `sanitize-filename`
- **Transcoding**: spawn `ffmpeg`; parse streams via `ffprobe` JSON
- **Image processing (optional)**: `image` crate for thumbnails
- **Observability**: `tracing`, `tracing-subscriber`, optional OpenTelemetry exporter

### Metadata & Indexers

- **TheTVDB API v4**: series/episodes metadata, art
- **TMDB API**: complementary source (movies, images; often richer art)
- **Prowlarr (recommended)**: centrally manage indexers and expose a single Torznab endpoint; or Jackett

### Supabase (local)

- **Postgres** with RLS (row-level security)
- **Auth (GoTrue)** for email/password + optional OAuth
- **PostgREST** present; primarily use Rust API + direct SQL via `sqlx`
- **Storage** for posters/backdrops and generated thumbnails
- **Service role key** only on the backend (never in the browser)

### Media Packaging/Streaming

- **HLS** with on‑the‑fly or just‑in‑time transcoding:
  - Direct play: serve original file if codec/container supported
  - Transcode: `ffmpeg` → HLS segments in per‑session cache directory
  - Serve playlists/segments via Axum static routes with tokenized URLs

## Integration Overview

### Auth flow

1. Frontend uses Supabase Auth for sign-in (email/password).
2. Frontend includes the Supabase access token in requests to the Rust API.
3. Rust API verifies JWTs against Supabase JWKS, extracts `user_id`, applies authorization.

**Supported auth methods:**
- Email/password (primary)
- Future: OAuth providers (Google, GitHub, etc.) via Supabase

### Database

- Rust backend uses `sqlx` to Postgres.
- RLS stays enabled. The backend uses:
  - a standard (non‑service) role for user‑scoped reads/writes, and
  - a service‑role pool only for worker/system tasks that must bypass RLS (e.g., scheduled RSS pulls).

### Torrents

- `qbittorrent-nox` runs in Docker.
- Rust backend controls it via Web API:
  - Add magnet/Torznab URLs
  - Monitor state/progress
  - Set download categories and save paths
- Prowlarr exposes Torznab feeds the backend queries to find releases for subscriptions.

### Subscriptions & “fill‑the‑gaps”

- Users subscribe to shows and select quality profiles.
- Scheduler checks per‑show missing episodes (based on metadata + local files).
- Queries Prowlarr Torznab for candidates; applies filtering (quality, codec, size, release group), then sends selected torrents to qBittorrent.
- Post‑download, a worker triggers rename/move, then library refresh.

### Metadata

- On scan or new file detection: identify media by filename heuristics + optional hash; call TheTVDB/TMDB to fetch canonical title, season/episode, art, and overviews.
- Store normalized records in Postgres; cache images in Supabase Storage with signed URLs.

### Transcoding/Streaming

- For playback requests, backend checks if direct play is possible. If not, spawn `ffmpeg` to generate HLS on‑the‑fly into a temp cache dir keyed by user/session.
- Axum serves `/_stream/{media_id}/{session_id}/index.m3u8` and segment files.
- Frontend uses `hls.js` to play.
- Casting:
  - **Chromecast**: page registers a Cast session and passes the HLS URL.
  - **AirPlay**: Safari user clicks the native AirPlay button on the `<video>` element.

### Organization & Rename

- After torrent completion:
  - Extract file(s)
  - Identify media
  - Rename using a consistent scheme:
    - Movies: `Movies/{Title} ({Year})/{Title} ({Year}).{ext}`
    - TV: `TV/{Show Name}/Season {S}/{Show Name} - S{xx}E{xx} - {Episode Title}.{ext}`
  - Update DB, create symlinks if desired (optional), refresh library.

### Web UI

- Browse library (grid), details pages, search, play, cast
- Manage subscriptions (show, quality profile, monitored seasons/episodes)
- View downloads and control the client (pause/resume/remove, speed limits)
- Admin settings: paths, transcoding presets, indexers, auth providers

## Data Model (core tables)

### Libraries (Multiple Library Support)

Each user can have multiple libraries of different types:

- **Library Types**: `movies`, `tv`, `music`, `audiobooks`, `other`
- **Per-Library Settings**: 
  - Custom name and path
  - Auto-scan toggle and interval
  - Display icon and color
  - Independent scan schedules

```
libraries
├── id (UUID)
├── user_id (UUID, FK → auth.users)
├── name (VARCHAR) - e.g., "Movies", "Kids TV", "Documentaries"
├── path (TEXT) - e.g., "/data/media/Movies"
├── library_type (ENUM) - movies|tv|music|audiobooks|other
├── icon (VARCHAR) - display icon
├── color (VARCHAR) - theme color
├── auto_scan (BOOLEAN)
├── scan_interval_hours (INTEGER)
├── last_scanned_at (TIMESTAMPTZ)
├── created_at, updated_at
```

### Other Core Tables

- `users` (from Supabase auth.users)
- `media_items` (id, type: movie|episode|show, title, year, show_id, season, episode, tvdb_id, tmdb_id, overview, runtime, rating)
- `media_files` (id, media_item_id, library_id, path, size, container, video_codec, audio_codec, width, height, duration, hash, added_at)
- `artwork` (media_item_id, kind: poster|backdrop|thumb, storage_key, width, height)
- `subscriptions` (id, user_id, show_tvdb_id, quality_profile_id, monitored: bool, latest_wanted, created_at)
- `quality_profiles` (id, name, rules JSON)
- `downloads` (id, user_id, qbittorrent_hash, state, progress, added_at, media_linked_item_id)
- `jobs` (id, kind, payload JSON, state, next_run_at, last_error)
- `events` (audit log)

### Row-Level Security

- `libraries` - Users can only access their own libraries
- `subscriptions`, `downloads` - Restricted to `user_id`
- `media_files` - Access via library ownership
- Library content can be shared with roles (future feature)

## API Surface (Rust / Axum)

### Authentication
- `GET /api/me` → user profile

### Libraries (Multiple Library Support)
- `GET /api/libraries` → list all user libraries
- `POST /api/libraries` → create new library
- `GET /api/libraries/{id}` → get library details
- `PATCH /api/libraries/{id}` → update library settings
- `DELETE /api/libraries/{id}` → delete library
- `POST /api/libraries/{id}/scan` → trigger library scan
- `GET /api/libraries/{id}/stats` → get library statistics

### Media
- `GET /api/media/{id}` → media item details + sources
- `GET /api/media/{id}/stream/hls` → tokenized HLS playlist URL
- `GET /api/media/{id}/cast/session` → pre-authorize cast playback

### Torrents
- `GET /api/torrents` → list active torrents
- `POST /api/torrents` → add by magnet/URL

### Subscriptions
- `GET /api/subscriptions` → list subscriptions
- `POST /api/subscriptions` → create subscription
- `POST /api/subscriptions/{id}/search` → manual search

### Admin
- `POST /api/admin/reindexers/test` → verify Prowlarr/Jackett
- `POST /api/admin/settings` → save paths, ffmpeg preset, etc.

Auth: Bearer Supabase JWT. Middleware verifies, injects `UserContext`.

## Background Workers

- **Scanner**: Walk library paths, detect new/missing files, run ffprobe, identify content, fetch metadata, upsert DB, queue artwork jobs.
- **RSS/Indexer Poller**: Periodically query Torznab feeds for monitored shows; enqueue download jobs.
- **Download Monitor**: Poll qBittorrent for added torrents; update states; on complete, trigger rename/move + rescan.
- **Transcode Session GC**: Clean old HLS caches.
- **Artwork Fetcher**: Download/store art in Supabase Storage; update artwork table.

Use `apalis` + Redis or an in‑process `tokio-cron-scheduler` for simpler setups. For resilience, prefer a queue (`apalis`).

## File Layout & Paths

- **Config**: `~/.config/librarian/config.toml` or mounted volume
- **Libraries**: e.g., `/data/media/Movies`, `/data/media/TV`
- **Temp**: `/data/cache/transcode/{session_id}/...`
- **Downloads**: `/data/downloads/{category}`
- **Completed**: moved into library structure after rename

## Transcoding Strategy

- **Direct Play**: serve file with `Content-Range` when codec/container supported by client.
- **Transcode to HLS** (when needed):
  - Video: H.264 baseline/main/high
  - Audio: AAC stereo (downmix optional)
  - Ladder presets: 1080p/720p/480p variants via `-var_stream_map` or single rung based on client bandwidth
  - Subtitles: burn‑in or extract to WebVTT (preferred)
- **Security**: tokenize HLS URLs and expire quickly to prevent stale links.

## Casting Strategy

- **Chromecast**: Frontend loads Google Cast Sender SDK; cast a signed HLS URL from the backend. No device‑discovery code in Rust required.
- **AirPlay**: `<video>` with `x-webkit-airplay="allow"`; Safari exposes the AirPlay picker automatically.
- **DLNA (optional)**: Add a lightweight UPnP/DLNA media server later; not required for MVP.

## Security

- Verify Supabase JWTs via JWKS and cache keys.
- Use short‑lived, signed URLs for HLS playlists/segments and artwork.
- Keep the Supabase service key only on the backend.
- RLS for user‑owned tables; policies like `user_id = auth.uid()`.

## Local Development & Deployment

### Docker Compose (services to run together)

- `supabase/*` stack (Auth, Postgres, Storage, PostgREST, Kong)
- `qbittorrent-nox`
- Prowlarr or Jackett (optional but recommended)
- Redis (if using `apalis`)
- `librarian-backend` (Rust)
- `librarian-frontend` (TanStack Start)

### Shared volumes

- `/data/media`
- `/data/downloads`
- `/data/cache`
- Supabase volumes

### Environment (.env)

- Supabase URL, anon key, service key
- qBittorrent credentials
- Prowlarr/Jackett API key
- TheTVDB/TMDB keys
- Transcode presets

## Recommended Libraries (summary)

### Rust

- `axum`, `tokio`, `tower`, `tower-http`
- `sqlx` (features: `postgres`, `runtime-tokio-rustls`, `offline`)
- `serde`, `serde_json`, `thiserror`, `anyhow`
- `jsonwebtoken` or `josekit` (JWKS), `reqwest`
- `apalis` (+ `apalis-redis`) or `tokio-cron-scheduler`
- `notify`, `walkdir`, `regex`, `sanitize-filename`
- `tracing`, `tracing-subscriber`
- `uuid`, `time`, `once_cell`

### Frontend

- TanStack Start, TanStack Router, React, TypeScript
- `@supabase/supabase-js`
- HeroUI (formerly NextUI) + Tailwind CSS
- `hls.js`
- Google Cast Web Sender SDK (script include)
- Package manager: pnpm

## External Services

- `qbittorrent-nox` (Web API)
- Prowlarr (Torznab aggregation) or Jackett
- FFmpeg/FFprobe
- Supabase local stack

## Metadata APIs

- TheTVDB v4
- TMDB v3/v4

## MVP vs. Later

### MVP

- Local auth + library browsing
- Add/download torrents from a URL/magnet
- Play in browser with direct play + single‑rung HLS transcode fallback
- Basic metadata fetch + poster/backdrop
- Manual subscriptions + simple RSS polling
- Rename & organize

### Later

- Multi‑quality HLS ladder + dynamic ABR
- DLNA server
- Advanced quality profiles (release groups, codecs)
- Subtitles management (embedded, external, OCR for PGS)
- Hardware transcoding (NVENC/VAAPI/QSV) with selectable presets
- Multi‑user sharing with roles
- Mobile‑friendly PWA, offline posters, push notifications

## Testing & Observability

- Unit tests for parsers (filename → S/E), renamer, API handlers
- Integration tests with ephemeral Postgres and a mocked qBittorrent
- Golden tests for metadata normalization
- `tracing` with JSON logs; health and readiness endpoints
- Optional OpenTelemetry exporter to a Grafana stack

## Why these choices?

qBittorrent’s Web API is stable, well‑documented, and easy to control from Rust. It avoids brittle FFI to libtorrent and keeps updates independent. HLS via FFmpeg is the most interoperable for browsers and casting. Supabase local gives Postgres + Auth + Storage with minimal ops and a clean JWT/RLS story. Google Cast SDK in the browser reduces backend complexity and works reliably for local networks. Prowlarr centralizes indexers and offers powerful Torznab querying—ideal for subscriptions and “fill the gaps.”


