## Librarian — Incremental Build Plan

### Guiding Principles

- **Ship vertical slices** that exercise frontend → API → DB → worker paths.
- **Keep it local-first** and private by default; remote access is an add‑on.
- **Prefer direct play**; add transcoding only where required.
- **Automate via jobs**: scanners, pollers, and post-download processing.
- **Observability from day 1**: health, tracing logs, minimal metrics.

---

## Progress Summary

### ✅ Completed

| Stage | Status | Notes |
|-------|--------|-------|
| Stage 0 | ✅ Complete | Full scaffold with all components |
| Stage 1 | ✅ Complete | Frontend/backend auth with JWT middleware working |
| Stage 2A | ✅ Complete | Database schema, all repositories (libraries, tv_shows, episodes, quality_profiles, rss_feeds, media_files, logs) |
| Stage 2B | ✅ Complete | Library scanning with file discovery and filename parsing |
| Stage 2C | 🟡 Partial | TVMaze integration complete; TMDB/TheTVDB scaffolded but not implemented |
| Stage 2D | ✅ Complete | Show management with episode tracking and monitoring |
| Stage 3A | ✅ Complete | RSS feed polling with episode matching |
| Stage 3B | ✅ Complete | Auto-download from RSS (core functionality done, quality filters are future enhancement) |
| Stage 3C | ✅ Complete | Post-download processing integrated with scheduler (runs every minute) |
| Stage 3D | ✅ Complete | Auto-organization with show-level overrides, respects copy/move/hardlink |
| Stage 4 | ✅ Complete | Native torrent client (librqbit) + GraphQL subscriptions |
| Stage 5 | ✅ Complete | Chromecast casting with device discovery, media streaming, playback controls |

### Key Decisions Made

1. **Frontend Framework**: Changed from Next.js to **TanStack Start** with TanStack Router
2. **UI Library**: Changed from Shadcn/Radix to **HeroUI** (formerly NextUI)
3. **Package Manager**: Using **pnpm** instead of npm for frontend
4. **Project Structure**: Simplified folder names (`backend/`, `frontend/` instead of `librarian-*`)
5. **Production Architecture**: Single domain with reverse proxy (Caddy) routing:
   - `/` → Frontend
   - `/graphql` → GraphQL API (single API surface)
   - Supabase accessed directly from frontend for auth
6. **Torrent Client**: Changed from qBittorrent to **librqbit** (native Rust, embedded)
7. **API Architecture**: **GraphQL-first API** with subscriptions (async-graphql)
   - Single endpoint for all operations: `/graphql`
   - WebSocket subscriptions: `/graphql/ws`
   - Centralized auth via JWT verification in GraphQL context
   - REST only for: health checks, torrent file uploads (multipart), filesystem browsing
8. **Metadata Providers**: TVMaze (primary, free) → TMDB → TheTVDB
9. **Indexers**: RSS feeds first, Prowlarr/search engines later
10. **Post-Download**: Copy by default (preserves seeding), Move optional

---

## Phase 1: TV Library Foundation

### Stage 2A — Database Schema & Libraries CRUD
**Status**: ✅ Complete

#### Goals
- Create database tables for TV library system
- Wire up library CRUD operations (no more mock data)
- File browser for selecting library paths

#### Deliverables

**Database Migration (`004_tv_library_schema.sql`)**:
- [x] `tv_shows` table
- [x] `episodes` table  
- [x] `quality_profiles` table (enhanced)
- [x] `rss_feeds` table
- [ ] `unmatched_files` table (future enhancement)
- [x] Update `libraries` table with new columns
- [x] Update `downloads` table with episode/library links
- [x] Update `media_files` table with quality metadata

**Backend**:
- [x] `db/libraries.rs` - Library repository
- [x] `db/tv_shows.rs` - TV show repository
- [x] `db/episodes.rs` - Episode repository
- [x] `db/quality_profiles.rs` - Quality profile repository
- [x] Wire GraphQL library queries/mutations to database
- [x] Library creation with path validation

**Frontend**:
- [x] `/libraries` route - List all libraries
- [x] Library creation wizard with file browser
- [x] File browser component for path selection
- [x] Library card component

#### Acceptance Criteria
- [x] User can create a TV library with a path
- [x] Libraries persist to database
- [x] Libraries list loads from database (not mock data)
- [x] User can delete a library

---

### Stage 2B — Library Scanning & File Discovery
**Status**: ✅ Complete (Core), ffprobe pending

#### Goals
- Walk library directories and discover media files
- Parse filenames to extract show/season/episode info
- Run ffprobe to get media properties
- Group discovered files by show

#### Deliverables

**Backend Services**:
- [x] `services/scanner.rs` - Directory walking and file discovery
- [x] `services/filename_parser.rs` - Scene naming pattern parser (comprehensive regex-based)
- [ ] `services/ffprobe.rs` - Media file analysis (not yet implemented - use for future quality detection)
- [x] Update `jobs/scanner.rs` with real implementation

**Filename Parser Patterns**:
```rust
// Priority order
S01E01, s01e01           // Most common
1x01                     // Alternative
Season 1 Episode 1       // Verbose
101, 102 (3 digits)      // Compact (risky, needs context)
```

**Quality Detection**:
- [ ] Resolution: 2160p, 1080p, 720p, 480p
- [ ] Codec: HEVC/x265, H264/x264, AV1, XviD
- [ ] Source: WEB-DL, WEBRip, BluRay, HDTV
- [ ] HDR: HDR10, HDR10+, Dolby Vision
- [ ] Audio: Atmos, TrueHD, DTS, AC3, AAC

**GraphQL**:
- [ ] `scanLibrary` mutation triggers scan job
- [ ] `libraryScanProgress` subscription for real-time updates
- [ ] `discoveredShows` query for scan results

**Frontend**:
- [ ] Scan button on library page
- [ ] Scan progress indicator
- [ ] Discovered shows list after scan

#### Acceptance Criteria
- [ ] Scanning a folder discovers media files
- [ ] Filenames are parsed to extract show/season/episode
- [ ] Media properties (resolution, codec) are detected via ffprobe
- [ ] Discovered shows are grouped and displayed

---

### Stage 2C — Metadata Providers (TVMaze/TMDB)
**Status**: 🟡 Partial (TVMaze complete, TMDB/TheTVDB scaffolded)

#### Goals
- Integrate TVMaze API for show/episode metadata
- Match discovered shows to TVMaze entries
- Fetch episode lists for shows
- Download artwork (posters, backdrops)

#### Deliverables

**Backend Services**:
- [x] `services/tvmaze.rs` - TVMaze API client (fully implemented)
- [ ] `services/tmdb.rs` - TMDB API client (scaffolded in `media/metadata.rs`, TODO in `services/metadata.rs`)
- [x] `services/metadata.rs` - Unified metadata interface with artwork caching
- [x] `services/artwork.rs` - Image downloading and Supabase storage caching

**TVMaze Integration**:
```rust
// API endpoints
GET /search/shows?q={query}           // Search shows
GET /shows/{id}                       // Show details
GET /shows/{id}/episodes              // All episodes
GET /shows/{id}/seasons               // Seasons list
```

**Show Matching**:
- [ ] Fuzzy string matching for show names
- [ ] Year disambiguation (for remakes)
- [ ] Confidence scoring
- [ ] Manual override capability

**GraphQL**:
- [ ] `searchTvShows(query: String!)` - Search TVMaze
- [ ] `addTvShow` mutation with TVMaze ID
- [ ] `refreshTvShowMetadata` mutation

**Frontend**:
- [ ] Show search dialog
- [ ] Show details page with poster
- [ ] Episode list by season

#### Acceptance Criteria
- [ ] Can search TVMaze for shows
- [ ] Discovered files match to TVMaze shows
- [ ] Episode metadata is fetched and stored
- [ ] Artwork is downloaded and displayed

---

### Stage 2D — Show Management & Episode Tracking
**Status**: ✅ Complete

#### Goals
- Add shows to library with monitoring settings
- Track episode status (missing/wanted/downloaded)
- Quality profile management
- Episode wanted list

#### Deliverables

**Backend**:
- [x] `db/rss_feeds.rs` - RSS feed repository
- [x] Show monitoring logic (all vs future vs none)
- [x] Episode status calculation (missing → wanted → available → downloading → downloaded)
- [x] Quality profile CRUD

**GraphQL**:
- [ ] `tvShows(libraryId)` - List shows in library
- [ ] `episodes(showId, season)` - Episodes with status
- [ ] `wantedEpisodes(libraryId)` - Missing episodes
- [ ] `qualityProfiles` - List profiles
- [ ] `createQualityProfile` / `updateQualityProfile`

**Frontend**:
- [ ] Library detail page with shows grid
- [ ] Show detail page with seasons/episodes
- [ ] Episode status badges (missing, downloaded, etc.)
- [ ] Quality profile editor
- [ ] Monitor type selector (All / Future Only)

#### Acceptance Criteria
- [ ] User can add a show to library
- [ ] Shows display with episode counts
- [ ] Missing episodes are identified
- [ ] Quality profiles can be created/edited

---

## Phase 2: Automation Pipeline

### Stage 3A — RSS Feed Polling
**Status**: ✅ Complete
**Depends on**: Stage 2D

#### Goals
- Add and manage RSS feeds
- Poll feeds on schedule
- Parse RSS items to extract show/episode/quality
- Match RSS items to wanted episodes

#### Deliverables

**Backend Services**:
- [x] `services/rss.rs` - RSS feed fetching and parsing
- [x] RSS item parser (title → show, season, episode, quality) - uses `filename_parser.rs`
- [x] Wanted episode matcher - matches to episodes with 'wanted' status
- [x] Update `jobs/rss_poller.rs` with real implementation

**RSS Parsing** (based on IPT format):
```xml
<title>Chicago Fire S14E08 1080p WEB h264-ETHEL</title>
<link>https://example.com/download.php/12345/file.torrent</link>
<description>1.48 GB; TV/Web-DL</description>
```

**GraphQL**:
- [x] `rssFeeds` / `rssFeed` queries
- [x] `createRssFeed` / `updateRssFeed` / `deleteRssFeed`
- [x] `testRssFeed` - Fetch and show items
- [x] `pollRssFeed` - Manual poll trigger

**Frontend**:
- [x] RSS feeds list in settings (`/settings/rss`)
- [x] Add RSS feed dialog
- [x] Feed test results view
- [x] Feed polling status

#### Acceptance Criteria
- [x] User can add RSS feed URLs
- [x] Feeds are polled on schedule (every 15 minutes)
- [x] RSS items are parsed correctly
- [x] Matches to wanted episodes are identified

#### Episode Status Flow
```
wanted → available (RSS match found, torrent link stored) → downloading → downloaded
```

The `available` status indicates an RSS item matched the episode and the torrent link is ready for download.

---

### Stage 3B — Auto-Download from RSS
**Status**: ✅ Complete (core functionality)
**Depends on**: Stage 3A

#### Goals
- Automatically download torrents for wanted episodes
- Apply quality filters before downloading
- Link downloads to episodes

#### Deliverables

**Backend**:
- [x] Auto-download job (`jobs/auto_download.rs`) - runs every 5 minutes
- [x] Downloads episodes with 'available' status
- [x] Updates episode status to 'downloading'
- [x] Links torrent to episode via `torrent_info_hash` column
- [x] Duplicate prevention (via episode status check)
- [ ] Quality filter matching logic (future enhancement)

**Quality Matching** (Future Enhancement):
- [ ] Resolution check (meets minimum, prefers target)
- [ ] Codec preference matching
- [ ] Audio format preference
- [ ] HDR requirement checking
- [ ] Size limits
- [ ] Release group whitelist/blacklist

**Frontend**:
- [ ] Download activity showing linked episode
- [ ] Auto-download toggle per show
- [ ] Download history per episode

#### Acceptance Criteria
- [x] Matching RSS items trigger downloads automatically
- [x] Downloads are linked to episodes (via `torrent_info_hash`)
- [x] Duplicates are not re-downloaded (episode status prevents re-download)
- [ ] Quality filters are applied (future enhancement)

---

### Stage 3C — Post-Download Processing
**Status**: ✅ Complete (integrated with scheduler)

#### Goals
- Process completed downloads automatically
- Extract archives (zip, tar, rar)
- Filter files (keep video + subtitles)
- Identify content and match to episodes

#### Deliverables

**Backend Services**:
- [ ] `services/extractor.rs` - Archive extraction (future enhancement)
- [x] File filtering logic in `jobs/download_monitor.rs` (is_video_file)
- [x] `jobs/download_monitor.rs` - Full implementation (process_completed_torrents)
- [x] Media file creation and episode linking implemented
- [x] **DONE**: Integrated `process_completed_torrents` into scheduler (runs every minute)

**Archive Support**:
- [ ] ZIP (native Rust)
- [ ] TAR/GZ (native Rust)
- [ ] RAR (shell out to `unrar` or use crate)
- [ ] 7Z (shell out to `7z` or use crate)

**File Filtering**:
```rust
// Keep
.mkv, .mp4, .avi, .m4v, .mov, .wmv
.srt, .sub, .ass, .ssa, .idx, .vtt

// Discard
*sample*, *proof*, *.txt, *.nfo, *.exe, *.jpg, *.png
```

**Frontend**:
- [ ] Processing status in downloads view
- [ ] Processing errors/warnings display

#### Acceptance Criteria
- [ ] Completed torrents trigger processing
- [ ] Archives are extracted
- [ ] Only video/subtitle files are kept
- [ ] Content is matched to episodes

---

### Stage 3D — Auto-Rename & Organization
**Status**: ✅ Complete (integrated with scheduler)

#### Goals
- Rename files using configurable patterns
- Copy/move to library folder
- Update database with file locations
- Mark episodes as downloaded

#### Deliverables

**Backend Services**:
- [x] `services/organizer.rs` - Full implementation with RenameStyle support
  - Supports: copy, move, hardlink actions (respects library.post_download_action)
  - Supports: none, clean, preserve_info rename styles
  - Creates show folders, season folders
  - Updates media_file records with new paths
  - **Show-level overrides**: organize_files_override, rename_style_override
- [x] `media/organizer.rs` - Alternative simple organizer (legacy)
- [x] Library settings for organize behavior (organize_files, rename_style, post_download_action)
- [x] GraphQL mutation: `organizeTorrent`
- [x] **DONE**: Auto-triggered on download completion via `download_monitor.rs`

**Naming Tokens**:
```
{show}, {show_clean}, {season}, {season:02}
{episode}, {episode:02}, {title}, {year}
{air_date}, {quality}, {ext}
```

**Default Pattern**:
```
{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}
```

**GraphQL**:
- [ ] Library settings for naming pattern
- [ ] Post-download action setting (copy/move)

**Frontend**:
- [ ] Naming pattern editor with preview
- [ ] Post-download action selector

#### Acceptance Criteria
- [ ] Files are renamed according to pattern
- [ ] Files are copied/moved to library
- [ ] Episode is marked as downloaded
- [ ] Original files remain for seeding (if copy mode)

---

## Phase 3: Advanced Features

### Stage 4A — Filesystem Watching (inotify)
**Status**: ⏳ Future
**Depends on**: Stage 3D

#### Goals
- Real-time detection of new files via inotify
- Fallback to periodic scan for unsupported filesystems
- Per-library toggle for watch mode

#### Deliverables
- [ ] `services/watcher.rs` - inotify wrapper
- [ ] Detection of network mount vs local filesystem
- [ ] Graceful fallback logic
- [ ] Library setting for watch mode

---

### Stage 4B — OpenAI-Assisted Matching
**Status**: ⏳ Future
**Depends on**: Stage 2C

#### Goals
- Use OpenAI to identify difficult filenames
- Fallback when pattern matching fails
- Optional (requires API key)

#### Deliverables
- [ ] `services/ai_matcher.rs` - OpenAI integration
- [ ] Prompt engineering for filename parsing
- [ ] Confidence scoring
- [ ] Cost-aware rate limiting

---

### Stage 4C — Unmatched File Management
**Status**: ⏳ Future
**Depends on**: Stage 4B

#### Goals
- Queue unmatched files for manual review
- Suggest matches with confidence scores
- Manual link/unlink capability

#### Deliverables
- [ ] Unmatched files list in UI
- [ ] Match suggestion display
- [ ] Manual matching dialog
- [ ] Ignore/dismiss capability

---

### Stage 4D — Quality Upgrading
**Status**: ⏳ Future
**Depends on**: Stage 3B

#### Goals
- Detect when better quality is available
- Automatically upgrade if configured
- Replace files while preserving metadata

---

## Implementation Order (Recommended)

```
Phase 1: TV Library Foundation (4-6 weeks)
├── Stage 2A: Database & Libraries CRUD          ← START HERE
├── Stage 2B: Library Scanning
├── Stage 2C: Metadata Providers
└── Stage 2D: Show Management

Phase 2: Automation Pipeline (4-6 weeks)
├── Stage 3A: RSS Feed Polling
├── Stage 3B: Auto-Download
├── Stage 3C: Post-Download Processing
└── Stage 3D: Auto-Rename & Organization

Phase 3: Advanced Features (ongoing)
├── Stage 4A: Filesystem Watching
├── Stage 4B: OpenAI Matching
├── Stage 4C: Unmatched Files UI
└── Stage 4D: Quality Upgrading

Phase 4: Media Playback & Casting
└── Stage 5: Chromecast/Google Cast ✅ COMPLETE
```

---

## Stage 5: Chromecast Casting (Completed)

### Goals
- Cast media to Chromecast and Google Cast devices
- Auto-discover devices via mDNS and support manual IP entry
- Stream media with HTTP Range support for seeking
- Full playback control (play, pause, seek, volume)

### Deliverables

**Backend (Rust)**:
- ✅ `services/cast.rs` - CastService with CASTV2 protocol (rust_cast)
- ✅ `db/cast.rs` - Repository for devices, sessions, settings
- ✅ `api/media.rs` - HTTP streaming endpoint with Range headers
- ✅ Migration `014_cast_devices.sql` - Tables for devices, sessions, settings
- ✅ GraphQL queries: `castDevices`, `castSessions`, `castSettings`
- ✅ GraphQL mutations: `discoverCastDevices`, `addCastDevice`, `removeCastDevice`, `castMedia`, `castPlay`, `castPause`, `castStop`, `castSeek`, `castSetVolume`, `castSetMuted`, `updateCastSettings`
- ✅ GraphQL subscriptions: `castSessionUpdated`, `castDevicesChanged`

**Frontend (React)**:
- ✅ `hooks/useCast.ts` - Hook for managing cast state
- ✅ `components/cast/CastButton.tsx` - Device selection dropdown
- ✅ `components/cast/CastControlBar.tsx` - Playback controls bar
- ✅ `routes/settings/casting.tsx` - Device management page

### Key Technical Decisions
- **rust_cast** library for native CASTV2 protocol (no external dependencies)
- **mdns-sd** for mDNS device discovery on local network
- HTTP streaming with Range headers for efficient seeking
- Direct play for compatible formats, transcode for incompatible
- Settings stored in database, not config files

---

## Current Sprint: Complete

### Completed Tasks

1. ✅ **Integrated download_monitor with scheduler**
   - `process_completed_torrents` now runs every minute via scheduler
   - Processes completed torrents and organizes files automatically
   - Respects show-level overrides for organize_files and rename_style

2. ✅ **Updated organize_file to support copy/move/hardlink**
   - Now accepts `action` parameter from library.post_download_action
   - Copy: Preserves original for seeding
   - Move: Rename or copy+delete
   - Hardlink: Creates hard link (Unix), falls back to copy on Windows

### Next Priority Tasks

1. **Add ffprobe service** (optional but valuable)
   - Create `services/ffprobe.rs` for media file analysis
   - Extract resolution, codec, duration for quality detection

2. **Implement TMDB/TheTVDB clients** (Stage 2C completion)
   - Fill in TODO stubs in `services/metadata.rs`
   - Add fallback provider logic

3. **Quality filter matching** (Stage 3B enhancement)
   - Wire `torrent/quality.rs` profile matching to RSS episode selection
   - Score and filter releases before download

4. **Archive extraction** (Stage 3C enhancement)
   - Create `services/extractor.rs` for rar/zip/7z extraction
   - Wire into download_monitor for packed releases

### Code Quality Notes (from Code Review)

✅ **Fixed in this review:**
- Reduced clippy warnings from **115 → 3** (remaining are minor style suggestions)
- Added `#[allow(dead_code)]` annotations to scaffolded code with clear documentation
- Removed unused imports across all modules
- Auto-fixed useless `.into()` conversions and collapsible if statements

**Scaffolded Modules (need implementation):**
| Module | Status | Notes |
|--------|--------|-------|
| `services/prowlarr.rs` | Scaffolded | Torznab search client |
| `media/metadata.rs` | Scaffolded | Direct TMDB/TheTVDB clients |
| `media/transcoder.rs` | Scaffolded | HLS transcoding for playback |
| `jobs/transcode_gc.rs` | Scaffolded | Cache cleanup |
| `torrent/quality.rs` | Scaffolded | Quality profile matching |

**Dual/Legacy Modules to Consider Consolidating:**
| Active | Legacy | Notes |
|--------|--------|-------|
| `services/metadata.rs` | `media/metadata.rs` | Keep services version, remove media version after TMDB impl |
| `services/organizer.rs` | `media/organizer.rs` | Keep services version, has more features |

---

## Architecture Reference

### Production Deployment (Single Domain)

```
librarian.dastari.net
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

### Tech Stack

| Component | Technology |
|-----------|------------|
| Frontend | TanStack Start, React, TypeScript, HeroUI, Tailwind, pnpm |
| Backend | Rust, Axum, Tokio, sqlx |
| API | GraphQL-only with subscriptions (async-graphql) |
| Database | PostgreSQL (via Supabase) |
| Auth | Supabase Auth (email/password) |
| Storage | Supabase Storage (artwork) |
| Torrents | librqbit (native Rust, embedded) |
| Indexers | RSS feeds → Prowlarr (future) |
| Metadata | TVMaze → TMDB → TheTVDB |
| Transcoding | FFmpeg |
| Proxy | Caddy (production) |

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| FFmpeg performance on NAS | Slow transcoding | Prefer direct play; add hardware accel later |
| RSS feed variability | Failed parsing | Robust regex patterns; manual feed testing |
| Network mount inotify | No file events | Fall back to periodic scanning |
| OpenAI costs | Budget overrun | Rate limiting; make optional |
| Large libraries | Slow scans | Incremental scanning; progress updates |
| Scene naming variations | Misidentified files | Multiple patterns; AI fallback; manual review |

---

## Definition of Done (per stage)

- [ ] Code merged to main branch
- [ ] Database migrations run cleanly
- [ ] GraphQL schema updated
- [ ] Frontend routes functional
- [ ] Manual testing completed
- [ ] No console errors in browser
- [ ] API endpoints return proper errors
