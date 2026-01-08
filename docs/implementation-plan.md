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
| Stage 1 | 🟡 Partial | Frontend auth complete, backend JWT middleware scaffolded |
| Stage 4 | ✅ Complete | Native torrent client (librqbit) + GraphQL subscriptions |

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
7. **API Architecture**: **GraphQL-only API** with subscriptions (async-graphql)
   - Single endpoint for all operations: `/graphql`
   - WebSocket subscriptions: `/graphql/ws`
   - Centralized auth via JWT verification in GraphQL context
   - No REST API (except health endpoints)
8. **Metadata Providers**: TVMaze (primary, free) → TMDB → TheTVDB
9. **Indexers**: RSS feeds first, Prowlarr/search engines later
10. **Post-Download**: Copy by default (preserves seeding), Move optional

---

## Phase 1: TV Library Foundation

### Stage 2A — Database Schema & Libraries CRUD
**Status**: ⏳ Next Up
**Priority**: High

#### Goals
- Create database tables for TV library system
- Wire up library CRUD operations (no more mock data)
- File browser for selecting library paths

#### Deliverables

**Database Migration (`004_tv_library_schema.sql`)**:
- [ ] `tv_shows` table
- [ ] `episodes` table  
- [ ] `quality_profiles` table (enhanced)
- [ ] `rss_feeds` table
- [ ] `unmatched_files` table
- [ ] Update `libraries` table with new columns
- [ ] Update `downloads` table with episode/library links
- [ ] Update `media_files` table with quality metadata

**Backend**:
- [ ] `db/libraries.rs` - Library repository
- [ ] `db/tv_shows.rs` - TV show repository
- [ ] `db/episodes.rs` - Episode repository
- [ ] `db/quality_profiles.rs` - Quality profile repository
- [ ] Wire GraphQL library queries/mutations to database
- [ ] Library creation with path validation

**Frontend**:
- [ ] `/libraries` route - List all libraries
- [ ] `/libraries/new` route - Create library wizard
- [ ] File browser component for path selection (reuse from downloads settings)
- [ ] Library card component

#### Acceptance Criteria
- [ ] User can create a TV library with a path
- [ ] Libraries persist to database
- [ ] Libraries list loads from database (not mock data)
- [ ] User can delete a library

---

### Stage 2B — Library Scanning & File Discovery
**Status**: ⏳ Pending
**Depends on**: Stage 2A

#### Goals
- Walk library directories and discover media files
- Parse filenames to extract show/season/episode info
- Run ffprobe to get media properties
- Group discovered files by show

#### Deliverables

**Backend Services**:
- [ ] `services/scanner.rs` - Directory walking and file discovery
- [ ] `services/filename_parser.rs` - Scene naming pattern parser
- [ ] `services/ffprobe.rs` - Media file analysis
- [ ] Update `jobs/scanner.rs` with real implementation

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
**Status**: ⏳ Pending
**Depends on**: Stage 2B

#### Goals
- Integrate TVMaze API for show/episode metadata
- Match discovered shows to TVMaze entries
- Fetch episode lists for shows
- Download artwork (posters, backdrops)

#### Deliverables

**Backend Services**:
- [ ] `services/tvmaze.rs` - TVMaze API client
- [ ] `services/tmdb.rs` - TMDB API client (fallback)
- [ ] `services/metadata.rs` - Unified metadata interface
- [ ] `services/artwork.rs` - Image downloading and storage

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
**Status**: ⏳ Pending
**Depends on**: Stage 2C

#### Goals
- Add shows to library with monitoring settings
- Track episode status (missing/wanted/downloaded)
- Quality profile management
- Episode wanted list

#### Deliverables

**Backend**:
- [ ] `db/rss_feeds.rs` - RSS feed repository
- [ ] Show monitoring logic (all vs future)
- [ ] Episode status calculation
- [ ] Quality profile CRUD

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
**Status**: ⏳ Pending
**Depends on**: Stage 2D

#### Goals
- Add and manage RSS feeds
- Poll feeds on schedule
- Parse RSS items to extract show/episode/quality
- Match RSS items to wanted episodes

#### Deliverables

**Backend Services**:
- [ ] `services/rss.rs` - RSS feed fetching and parsing
- [ ] RSS item parser (title → show, season, episode, quality)
- [ ] Wanted episode matcher
- [ ] Update `jobs/rss_poller.rs` with real implementation

**RSS Parsing** (based on IPT format):
```xml
<title>Chicago Fire S14E08 1080p WEB h264-ETHEL</title>
<link>https://example.com/download.php/12345/file.torrent</link>
<description>1.48 GB; TV/Web-DL</description>
```

**GraphQL**:
- [ ] `rssFeeds` / `rssFeed` queries
- [ ] `createRssFeed` / `updateRssFeed` / `deleteRssFeed`
- [ ] `testRssFeed` - Fetch and show items
- [ ] `pollRssFeed` - Manual poll trigger

**Frontend**:
- [ ] RSS feeds list in settings
- [ ] Add RSS feed dialog
- [ ] Feed test results view
- [ ] Feed polling status

#### Acceptance Criteria
- [ ] User can add RSS feed URLs
- [ ] Feeds are polled on schedule
- [ ] RSS items are parsed correctly
- [ ] Matches to wanted episodes are identified

---

### Stage 3B — Auto-Download from RSS
**Status**: ⏳ Pending
**Depends on**: Stage 3A

#### Goals
- Automatically download torrents for wanted episodes
- Apply quality filters before downloading
- Link downloads to episodes

#### Deliverables

**Backend**:
- [ ] Quality filter matching logic
- [ ] Auto-download decision engine
- [ ] Download → Episode linking
- [ ] Duplicate prevention

**Quality Matching**:
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
- [ ] Matching RSS items trigger downloads automatically
- [ ] Quality filters are applied
- [ ] Downloads are linked to episodes
- [ ] Duplicates are not re-downloaded

---

### Stage 3C — Post-Download Processing
**Status**: ⏳ Pending
**Depends on**: Stage 3B

#### Goals
- Process completed downloads automatically
- Extract archives (zip, tar, rar)
- Filter files (keep video + subtitles)
- Identify content and match to episodes

#### Deliverables

**Backend Services**:
- [ ] `services/extractor.rs` - Archive extraction
- [ ] `services/file_filter.rs` - Keep/discard logic
- [ ] `services/post_processor.rs` - Orchestration
- [ ] Wire torrent completion events to processing

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
**Status**: ⏳ Pending
**Depends on**: Stage 3C

#### Goals
- Rename files using configurable patterns
- Copy/move to library folder
- Update database with file locations
- Mark episodes as downloaded

#### Deliverables

**Backend Services**:
- [ ] `services/renamer.rs` - Pattern-based renaming
- [ ] `services/organizer.rs` - File copy/move
- [ ] Naming pattern tokenizer

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
```

---

## Current Sprint: Stage 2A

### Immediate Tasks

1. **Create database migration** (`004_tv_library_schema.sql`)
   - New tables: `tv_shows`, `episodes`, `rss_feeds`, `unmatched_files`
   - Alter `libraries`, `downloads`, `media_files`, `quality_profiles`

2. **Create backend repositories**
   - `db/libraries.rs` with full CRUD
   - `db/quality_profiles.rs` with defaults

3. **Wire GraphQL to database**
   - Replace mock data in `libraries` query
   - Implement `createLibrary`, `updateLibrary`, `deleteLibrary`

4. **Build frontend `/libraries` route**
   - List libraries from API
   - Create library wizard with file browser
   - Library cards with stats

### Files to Create/Modify

```
backend/
├── migrations/
│   └── 004_tv_library_schema.sql    # NEW
├── src/
│   ├── db/
│   │   ├── libraries.rs             # NEW
│   │   ├── tv_shows.rs              # NEW
│   │   ├── episodes.rs              # NEW
│   │   └── quality_profiles.rs      # NEW
│   └── graphql/
│       ├── schema.rs                # UPDATE - wire to DB
│       └── types.rs                 # UPDATE - new types

frontend/
└── src/
    ├── routes/
    │   ├── libraries/
    │   │   ├── index.tsx            # Library list
    │   │   ├── new.tsx              # Create wizard
    │   │   └── $id.tsx              # Library detail
    └── components/
        ├── LibraryCard.tsx          # NEW
        └── FileBrowser.tsx          # EXISTS (reuse)
```

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
