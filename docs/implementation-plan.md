# Librarian — Implementation Status & Roadmap

This document tracks the implementation status of Librarian's features and outlines future work.

---

## Core Principles

- **Ship vertical slices** that exercise frontend → API → DB → worker paths
- **Keep it local-first** and private by default; remote access is an add-on
- **Prefer direct play**; add transcoding only where required
- **Automate via jobs**: scanners, pollers, and post-download processing
- **Observability from day 1**: health, tracing logs, minimal metrics

---

## Technology Stack

| Component | Technology |
|-----------|------------|
| Frontend | React 19, TanStack Router, TypeScript, HeroUI, Tailwind CSS v4, pnpm |
| Backend | Rust, Axum, Tokio, async-graphql |
| Database | PostgreSQL via Supabase, sqlx (compile-time checks) |
| Auth | Supabase Auth (JWT) |
| Storage | Supabase Storage (artwork) |
| Torrent Client | librqbit (native Rust, embedded) |
| Usenet Client | Native Rust (NNTP + yEnc) |
| Indexers | Native system (Torznab/Newznab compatible) |
| Metadata | TVMaze, TMDB, MusicBrainz, Audible/OpenLibrary |
| Transcoding | FFmpeg/FFprobe |
| Casting | Chromecast via rust_cast + mDNS |

---

## Feature Status Overview

| Feature | Status | Notes |
|---------|--------|-------|
| **Core Infrastructure** | ✅ Complete | GraphQL API, auth, database, job queue |
| **TV Libraries** | ✅ Complete | Full scanning, metadata, episode tracking |
| **Movie Libraries** | ✅ Complete | TMDB integration, auto-hunt |
| **Music Libraries** | ✅ Complete | MusicBrainz integration, track matching |
| **Audiobook Libraries** | ✅ Complete | Audible/OpenLibrary integration |
| **Native Torrent Client** | ✅ Complete | librqbit with real-time subscriptions |
| **File-Level Matching** | ✅ Complete | Individual files matched to items |
| **Post-Download Processing** | ✅ Complete | Auto-organize with quality verification |
| **RSS Feed Polling** | ✅ Complete | Automatic episode detection |
| **Native Indexers** | ✅ Complete | IPTorrents, Cardigann, Newznab |
| **Auto-Hunt** | ✅ Complete | Event-driven content hunting |
| **Chromecast Casting** | ✅ Complete | Device discovery, playback controls |
| **Usenet Downloads** | ✅ Complete | NNTP client, NZB parsing |
| **Source Priorities** | ✅ Complete | Per-library-type source ordering |
| **LLM Filename Parsing** | ✅ Complete | Ollama integration for difficult filenames |
| **Media Chapters** | ✅ Complete | Chapter extraction and playback |
| **Watch Progress** | ✅ Complete | Cross-device resume playback |
| **Subtitle Downloads** | 🟡 Partial | OpenSubtitles integration (manual) |
| **Archive Extraction** | 🟡 Partial | ZIP/RAR support (limited) |
| **AirPlay Casting** | ⏳ Planned | Native Safari support only |
| **Hardware Transcoding** | ⏳ Planned | NVENC/VAAPI/QSV |
| **Quality Upgrading** | ⏳ Planned | Auto-upgrade to better quality |

---

## Completed Features

### Phase 1: Foundation (Complete)

#### TV Library System
- ✅ Library CRUD with file browser path selection
- ✅ Library scanning with filename parsing
- ✅ TVMaze metadata integration (primary)
- ✅ TMDB fallback support
- ✅ Show management with season/episode tracking
- ✅ Episode status tracking (missing → wanted → downloading → downloaded)
- ✅ Quality settings per library (resolution, codec, source, audio)

#### Movie Library System
- ✅ Movie CRUD with TMDB metadata
- ✅ Release date tracking and monitoring
- ✅ File-level matching and organization
- ✅ Cast and crew information

#### Music Library System
- ✅ Album/Artist management with MusicBrainz
- ✅ Track-level status tracking
- ✅ Cover art from Cover Art Archive
- ✅ Audio quality settings (FLAC, lossy preferences)

#### Audiobook Library System
- ✅ Audiobook management with Audible/OpenLibrary
- ✅ Chapter-based tracking
- ✅ Author and narrator metadata

### Phase 2: Automation (Complete)

#### RSS Feed System
- ✅ Feed management (add, edit, delete, test)
- ✅ Automatic polling on configurable schedule
- ✅ Episode matching against wanted list
- ✅ Quality filtering before download
- ✅ Per-feed post-download action override

#### Auto-Download Pipeline
- ✅ Automatic download when RSS matches found
- ✅ Episode status updates in real-time
- ✅ Duplicate prevention
- ✅ Library-linked downloads

#### Post-Download Processing
- ✅ Completion detection (every minute check)
- ✅ File-level matching to library items
- ✅ FFprobe quality analysis
- ✅ Automatic file organization
- ✅ Status updates (downloading → downloaded/suboptimal)
- ✅ Conflict handling (move to _conflicts folder)

#### File Organization
- ✅ Configurable naming patterns with tokens
- ✅ copy/move/hardlink actions
- ✅ Show-level overrides for organization settings
- ✅ Rename styles: none, clean, preserve_info
- ✅ Library consolidation for duplicate folder cleanup

### Phase 3: Content Acquisition (Complete)

#### Native Indexer System
- ✅ IndexerManager with instance caching
- ✅ AES-256-GCM credential encryption
- ✅ IPTorrents scraper (cookie auth)
- ✅ Cardigann YAML definitions (generic tracker support)
- ✅ Newznab/Torznab protocol support
- ✅ Torznab API endpoint for external tools
- ✅ Per-indexer post-download action

#### Hunt System (Search)
- ✅ `/hunt` page for cross-indexer search
- ✅ Quality filtering in search results
- ✅ Authenticated .torrent downloads
- ✅ Direct linking to library items
- ✅ Global keyboard shortcut (Cmd/Ctrl+K)

#### Auto-Hunt
- ✅ Event-driven (triggers on add + after scans)
- ✅ Multi-library support
- ✅ Quality scoring and release ranking
- ✅ Automatic download of best match

### Phase 4: Advanced Features (Complete)

#### File-Level Matching
- ✅ `torrent_file_matches` table for per-file tracking
- ✅ Match individual files to episodes/movies/tracks
- ✅ Quality parsed from filename vs verified from FFprobe
- ✅ Skip download for already-owned files
- ✅ Partial downloads (8 of 12 tracks OK)

#### Usenet Support
- ✅ Usenet server configuration (NNTP)
- ✅ NZB parsing and download
- ✅ `usenet_downloads` tracking (parallel to torrents)
- ✅ `usenet_file_matches` for file-level matching
- ✅ Newznab indexer type
- ✅ Settings page for server management

#### Source Priority System
- ✅ `source_priority_rules` table
- ✅ Global defaults
- ✅ Per-library-type priorities
- ✅ Per-library overrides
- ✅ Settings page for priority management

#### LLM Filename Parsing
- ✅ Ollama integration for difficult filenames
- ✅ Per-library-type model configuration
- ✅ Fallback when regex parsing fails
- ✅ Settings page for model selection

#### Media Chapters
- ✅ Chapter extraction from video files
- ✅ `media_chapters` table
- ✅ Chapter navigation in player

#### Chromecast Casting
- ✅ CASTV2 protocol via rust_cast
- ✅ mDNS device discovery
- ✅ Manual device entry
- ✅ Play/pause/seek/volume controls
- ✅ Session management
- ✅ HTTP streaming with Range headers

#### Watch Progress
- ✅ Cross-device resume playback
- ✅ Episode/movie progress tracking
- ✅ Unified playback position storage

### Phase 5: Quality of Life (Complete)

#### Playback Features
- ✅ Direct play for compatible formats
- ✅ HLS transcoding for incompatible formats
- ✅ Subtitle track selection
- ✅ Audio track selection

#### Settings Pages
- ✅ `/settings/indexers` - Indexer management
- ✅ `/settings/rss` - RSS feed management
- ✅ `/settings/torrent` - Torrent client settings
- ✅ `/settings/usenet` - Usenet server management
- ✅ `/settings/source-priorities` - Source ordering
- ✅ `/settings/parser` - LLM parser settings
- ✅ `/settings/metadata` - Metadata provider settings
- ✅ `/settings/organization` - File organization defaults
- ✅ `/settings/casting` - Cast device management
- ✅ `/settings/logs` - System logs viewer

---

## Remaining Work

### High Priority

#### Archive Extraction Enhancement
- [ ] Full RAR support (multi-part archives)
- [ ] 7z extraction
- [ ] Automatic extraction after download
- [ ] Cleanup of archive files after extraction

#### Subtitle System
- [ ] Automatic subtitle search on download
- [ ] Subtitle sync with video
- [ ] Multiple subtitle language support
- [ ] OCR for PGS subtitles

#### Quality Upgrading
- [ ] Detect when better quality is available
- [ ] Automatic upgrade downloads
- [ ] Replace files while preserving metadata
- [ ] Configurable upgrade thresholds

### Medium Priority

#### Filesystem Watching (inotify)
- [ ] Real-time detection of new files
- [ ] Fallback to periodic scan for network mounts
- [ ] Per-library toggle for watch mode

#### Hardware Transcoding
- [ ] NVIDIA NVENC support
- [ ] Intel QSV support
- [ ] AMD VAAPI support
- [ ] Auto-detection of available hardware

#### AirPlay Casting
- [ ] Native protocol implementation
- [ ] Device discovery
- [ ] Video streaming support

### Lower Priority

#### Multi-User Features
- [ ] User roles and permissions
- [ ] Per-user watch progress
- [ ] Sharing capabilities

#### Mobile Experience
- [ ] PWA improvements
- [ ] Offline poster caching
- [ ] Push notifications for downloads

#### DLNA Server
- [ ] UPnP discovery
- [ ] Media serving to DLNA clients

---

## Architecture Reference

### Production Deployment

```
librarian.example.com
         │
    ┌────┴────┐
    │  Caddy  │  (reverse proxy, auto HTTPS)
    └────┬────┘
         │
    ┌────┼────────────────┐
    │    │                │
    ▼    ▼                ▼
  /    /graphql        Supabase
Frontend  Backend       (auth/db)
 :3000    :3001
```

### Key Backend Modules

| Module | Purpose |
|--------|---------|
| `services/torrent.rs` | librqbit wrapper, torrent management |
| `services/usenet.rs` | NNTP client, NZB downloads |
| `services/torrent_file_matcher.rs` | File-to-item matching |
| `services/media_processor.rs` | Unified download processing |
| `services/organizer.rs` | File organization and renaming |
| `services/scanner.rs` | Library scanning |
| `services/hunt.rs` | Auto-hunt service |
| `services/metadata.rs` | Multi-provider metadata |
| `services/ffmpeg.rs` | FFprobe analysis |
| `services/quality_evaluator.rs` | Quality verification |
| `services/ollama.rs` | LLM filename parsing |
| `services/cast.rs` | Chromecast control |
| `indexer/manager.rs` | Indexer instance management |
| `indexer/definitions/` | Indexer implementations |
| `jobs/download_monitor.rs` | Completion processing |
| `jobs/auto_hunt.rs` | Event-driven hunting |
| `jobs/rss_poller.rs` | Feed polling |

### Key Frontend Routes

| Route | Purpose |
|-------|---------|
| `/libraries` | Library list |
| `/libraries/$id` | Library detail with content grid |
| `/downloads` | Active downloads |
| `/hunt` | Cross-indexer search |
| `/settings/*` | All settings pages |

---

## Database Migrations

The database schema has evolved through 34 migrations:

| Migration | Purpose |
|-----------|---------|
| 001 | Initial schema (libraries, torrents, users) |
| 016 | Organization enhancements |
| 017-021 | Naming patterns, movies, music, audiobooks |
| 022 | Torrent-media links |
| 023-025 | Watch progress, unified playback |
| 026-027 | Quality profile removal, fixes |
| 028 | File-level matching (torrent_file_matches) |
| 029-031 | Audiobook renames, LLM settings |
| 032 | Media chapters |
| 033 | Drop legacy torrent linking |
| 034 | Usenet support, source priorities |

---

## Code Quality

### Clippy Status
- Minimal warnings (style suggestions only)
- All unused code either removed or annotated with `#[allow(dead_code)]`

### Testing
- Integration tests for media pipeline
- Unit tests for filename parsing

### Documentation
- This implementation plan
- `design.md` - System architecture
- `media-pipeline.md` - Pipeline architecture
- `flows.md` - Mermaid flow diagrams
- `style-guide.md` - Frontend conventions

---

## Decision Log

| Decision | Rationale |
|----------|-----------|
| librqbit over qBittorrent | Native Rust, no external dependencies |
| TVMaze as primary | Free, no API key, excellent data |
| RSS feeds first | Universal tracker support |
| Copy by default | Preserves seeding capability |
| GraphQL-only API | Single endpoint, real-time subscriptions |
| Embedded quality settings | Simpler than separate profiles table |
| Event-driven auto-hunt | Immediate response, not scheduled |
| File-level matching | Season packs, multi-file torrents |
| Usenet support | Alternative to torrents, faster |
