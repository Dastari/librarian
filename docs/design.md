## Librarian — System Design

### Goals & Scope

- **Local‑first, privacy‑preserving media library**
- **Offline‑first** on a single machine/NAS, with optional remote access
- **Features**:
  - **Torrents**: download via native Rust torrent client (librqbit)
  - **Streaming**: in-browser HLS, plus casting (Chromecast, AirPlay)
  - **Playback**: integrated web UI
  - **Metadata**: TV shows, movies, cover art from TVMaze, TMDB, and TheTVDB
  - **Subscriptions**: monitor shows; auto-fill gaps via RSS feeds and torrent search
  - **Organization**: auto-rename and file layout with configurable patterns
  - **Post-Processing**: extract archives, filter files, organize automatically

### High‑Level Architecture

- **Frontend**: TanStack Start (React with TanStack Router)
- **Backend**: Rust (Axum + Tokio), background workers, job queue
- **Identity & DB**: Supabase (Postgres + Auth + Storage) running locally via Docker
- **Torrent Engine**: `librqbit` (native Rust, embedded)
- **Indexer Management**: Native indexer system (Jackett-like) + RSS feeds
- **Transcoding/Packaging**: FFmpeg/FFprobe → HLS (m3u8 + TS/MP4 segments)
- **Casting**:
  - **Chromecast/Google Cast**: Native CASTV2 protocol via rust_cast + mdns-sd discovery
  - **Media Streaming**: HTTP with Range headers for seeking, direct play when compatible
  - **Transcoding**: On-demand FFmpeg transcoding for incompatible formats
  - **AirPlay**: Native Safari AirPlay support on the `<video>` element
- **File Watching / Library Scanner**: Rust watcher (inotify) + periodic full scan
- **Object Storage**: Supabase Storage for posters/backdrops/fanart

---

## TV Library Architecture

### Core Workflow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           TV LIBRARY LIFECYCLE                               │
└─────────────────────────────────────────────────────────────────────────────┘

1. CREATE LIBRARY
   ┌──────────────┐
   │ User creates │──→ Name: "TV Shows"
   │   library    │──→ Path: /mnt/nas/tv
   │              │──→ Type: TV
   │              │──→ Quality Profile (default)
   │              │──→ Scan interval / Watch toggle
   └──────┬───────┘
          │
          ▼
2. INITIAL SCAN (if pointing to existing folder)
   ┌──────────────┐
   │ Scan folder  │──→ Walk directory tree
   │              │──→ Parse filenames (S01E01, 1x01, etc.)
   │              │──→ Group by show name
   │              │──→ Match to TVMaze/TMDB/TVDB
   │              │──→ If no match: try OpenAI (if configured)
   │              │──→ Present discovered shows to user
   └──────┬───────┘
          │
          ▼
3. ADD SHOWS TO LIBRARY
   ┌──────────────┐
   │ User picks   │──→ "Auto-add all discovered" OR
   │   shows      │──→ Manual selection
   │              │──→ For each show:
   │              │    - Set monitoring: "All" or "Future only"
   │              │    - Inherit or override quality profile
   └──────┬───────┘
          │
          ▼
4. FETCH EPISODE LIST
   ┌──────────────┐
   │ For each     │──→ Query TVMaze/TMDB for full episode list
   │   show       │──→ Store all seasons/episodes in DB
   │              │──→ Mark existing files as "downloaded"
   │              │──→ Mark missing as "wanted" (based on monitoring)
   └──────┬───────┘
          │
          ▼
5. ONGOING MONITORING
   ┌──────────────────────────────────────────────────────┐
   │                                                      │
   │  ┌─────────────┐    ┌─────────────┐                 │
   │  │ RSS Poller  │    │ Torrent     │                 │
   │  │ (periodic)  │    │ Search      │                 │
   │  └──────┬──────┘    └──────┬──────┘                 │
   │         │                  │                         │
   │         └────────┬─────────┘                         │
   │                  ▼                                   │
   │         ┌──────────────┐                            │
   │         │ Match wanted │──→ Quality filter          │
   │         │  episodes    │──→ Add to download queue   │
   │         └──────────────┘                            │
   │                                                      │
   └──────────────────────────────────────────────────────┘
```

### Post-Download Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         POST-DOWNLOAD PROCESSING                             │
└─────────────────────────────────────────────────────────────────────────────┘

Torrent Completes
       │
       ▼
┌──────────────┐
│ 1. EXTRACT   │──→ Is archive? (zip/tar/rar/7z)
│              │    YES → Extract to temp folder
│              │    NO  → Continue with files
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ 2. FILTER    │──→ Keep: video files (.mkv, .mp4, .avi, etc.)
│              │──→ Keep: subtitles (.srt, .sub, .ass, .idx)
│              │──→ Keep: NFO if desired
│              │──→ Discard: samples, screenshots, txt, exe
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ 3. IDENTIFY  │──→ Parse filename for show/season/episode
│              │──→ Match to known show in library
│              │──→ If unmatched: queue for manual review
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ 4. ANALYZE   │──→ Run ffprobe for media info
│              │──→ Resolution, codec, bitrate, HDR, audio
│              │──→ Compare to quality requirements
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ 5. ORGANIZE  │──→ Apply naming pattern:
│              │    {Show Name}/Season {S}/{Show} - S{SS}E{EE} - {Title}.{ext}
│              │──→ Copy or Move (user preference)
│              │──→ Set correct permissions
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ 6. UPDATE DB │──→ Mark episode as downloaded
│              │──→ Link media_file to episode
│              │──→ Store quality metadata
│              │──→ Trigger artwork fetch if needed
└──────────────┘
```

---

## Key Technology Choices

### Frontend (TanStack Start)

- **Framework**: TanStack Start with TanStack Router (file-based routing)
- **Language**: TypeScript across the stack
- **UI**: HeroUI (formerly NextUI) + Tailwind CSS
- **Package Manager**: pnpm
- **Auth**: `@supabase/supabase-js` v2 with client helpers
- **Video Playback**: `hls.js` for HLS where needed
- **Casting**:
  - Google Cast Web Sender SDK (loaded where casting is available)
  - Safari's native AirPlay button on `<video>` provides AirPlay

### Backend (Rust)

- **Web framework**: `axum` (async, router-first, tower-compatible)
- **Async runtime**: `tokio`
- **DB**: `sqlx` (Postgres) with compile‑time checked queries
- **Auth/JWT**: verify Supabase JWTs via JWKS using `jsonwebtoken` or `josekit`; cache keys
- **HTTP client**: `reqwest` for external APIs (TVMaze/TMDB/TVDB)
- **Torrent control**: librqbit (native Rust, embedded)
- **Scheduler / Jobs**: `tokio-cron-scheduler` for periodic tasks
- **Filesystem**: `notify` (inotify watcher), `walkdir`, `tokio::fs`
- **Archives**: `unrar` crate + `sevenz-rust` or shell out to `7z`/`unrar`
- **Renaming**: `regex`, `sanitize-filename`
- **Transcoding**: spawn `ffmpeg`; parse streams via `ffprobe` JSON
- **Casting**: `rust_cast` for CASTV2 protocol, `mdns-sd` for device discovery
- **AI Matching** (optional): `async-openai` crate for filename identification
- **Observability**: `tracing`, `tracing-subscriber`, optional OpenTelemetry exporter

### Metadata Providers

| Provider | Auth Required | Free Tier | Best For |
|----------|---------------|-----------|----------|
| **TVMaze** | No | Unlimited | TV shows (primary, default) |
| **TMDB** | API key (free) | High limits | Movies + TV, artwork |
| **TheTVDB** | API key + subscription | Limited | Legacy support, comprehensive |

Priority order: TVMaze → TMDB → TheTVDB

### RSS Feed Parsing

Standard RSS format (like IPTorrents example):
```xml
<item>
  <title>Chicago Fire S14E08 1080p WEB h264-ETHEL</title>
  <link>https://example.com/download.php/12345/file.torrent</link>
  <pubDate>Thu, 08 Jan 2026 10:01:59 +0000</pubDate>
  <description>1.48 GB; TV/Web-DL</description>
</item>
```

The RSS poller will:
1. Parse title using scene naming patterns
2. Extract show name, season, episode, quality info
3. Match against wanted episodes in monitored shows
4. Apply quality filters before downloading

### Torrent Indexers (Native)

Native Jackett-like indexer system built into the backend:

**Architecture:**
```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          TORRENT INDEXER SYSTEM                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  GraphQL API │     │  Torznab API │     │  Auto Hunt   │
│  (Settings)  │     │  (External)  │     │  (Jobs)      │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │                    │                    │
       └────────────────────┼────────────────────┘
                            ▼
              ┌──────────────────────────┐
              │    IndexerManager        │
              │  (Instance Cache)        │
              └────────────┬─────────────┘
                           │
       ┌───────────────────┼───────────────────┐
       ▼                   ▼                   ▼
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ IPTorrents  │    │ Cardigann   │    │  (Future)   │
│  Indexer    │    │  (YAML)     │    │  Indexers   │
└─────────────┘    └─────────────┘    └─────────────┘
       │                   │                   │
       └───────────────────┼───────────────────┘
                           ▼
              ┌──────────────────────────┐
              │   HTTP Request + HTML    │
              │   Parsing (scraper)      │
              └──────────────────────────┘
```

**Supported Indexers:**
- **IPTorrents**: Private tracker, cookie-based authentication, HTML scraping
- **Cardigann (planned)**: YAML-based definitions for generic tracker support

**Key Features:**
- Credentials encrypted with AES-256-GCM (key stored in database)
- Torznab-compatible API at `/api/torznab/{indexer_id}` for external tools
- GraphQL API for all management (no REST for config)
- Rate limiting and request throttling

**Database Tables:**
- `indexer_configs`: Indexer instances (name, type, enabled)
- `indexer_credentials`: Encrypted credentials (cookie, api_key, etc.)
- `indexer_settings`: Per-indexer settings
- `indexer_search_cache`: Cached search results (TTL-based)

### Auto Hunt System

Automatic torrent hunting for missing content. **Auto Hunt is event-driven, not scheduled.**

**Triggers:**
- **On Add**: Adding a movie/album/audiobook immediately triggers hunt for that item
- **After Scan**: Library scans trigger auto-hunt for all missing content
- **Manual**: `triggerAutoHunt` mutation for on-demand hunting

**Flow:**
```
1. Trigger event (add item or scan completes)
          │
          ▼
2. Find missing content in library
   - Movies: monitored=true, has_file=false
   - TV: episodes with status=wanted
   - Music/Audiobooks: monitored=true, has_files=false
          │
          ▼
3. Query enabled indexers
   - Movies: IMDB/TMDB ID + title/year
   - TV: "Show Name S01E05"
   - Music: Artist + Album
          │
          ▼
4. Filter results by library quality settings
   - Resolution, codec, source (video)
   - Audio format, bit depth (audio)
   - Preferred release groups
          │
          ▼
5. Score and rank releases
   - Seeders, freeleech bonus
   - Quality match score
          │
          ▼
6. Download via IndexerManager (authenticated)
          │
          ▼
7. Link torrent to item
          │
          ▼
8. Download monitor handles organization
```

**Configuration (per library):**
- `auto_hunt`: Enable automatic searching
- Quality filters embedded in library settings (not separate profiles)

---

## Data Model

### Libraries

Each user can have multiple libraries of different types:

```sql
libraries
├── id (UUID)
├── user_id (UUID, FK → auth.users)
├── name (VARCHAR) - e.g., "TV Shows", "Kids TV", "Documentaries"
├── path (TEXT) - e.g., "/mnt/nas/tv"
├── library_type (ENUM) - movies|tv|music|audiobooks|other
├── icon (VARCHAR) - display icon
├── color (VARCHAR) - theme color
├── auto_scan (BOOLEAN)
├── scan_interval_minutes (INTEGER) - how often to scan
├── watch_for_changes (BOOLEAN) - use inotify where supported
├── post_download_action (ENUM) - copy|move|hardlink
├── organize_files (BOOLEAN) - automatically organize files
├── naming_pattern (TEXT) - e.g., "{show}/Season {season}/{show} - S{season}E{episode} - {title}.{ext}"
├── auto_add_discovered (BOOLEAN) - auto-create entries from downloaded content
├── auto_download (BOOLEAN) - auto-download from RSS feeds
├── auto_hunt (BOOLEAN) - auto-search indexers for missing content
├── quality_* (various) - embedded quality settings (see Quality Settings section)
├── last_scanned_at (TIMESTAMPTZ)
├── created_at, updated_at
```

### TV Shows

```sql
tv_shows
├── id (UUID)
├── library_id (UUID, FK → libraries)
├── user_id (UUID, FK → auth.users)
├── name (VARCHAR) - canonical show name
├── year (INTEGER) - premiere year
├── status (VARCHAR) - continuing|ended|upcoming|cancelled
├── tvmaze_id (INTEGER)
├── tmdb_id (INTEGER)
├── tvdb_id (INTEGER)
├── imdb_id (VARCHAR)
├── overview (TEXT)
├── network (VARCHAR)
├── runtime (INTEGER) - typical episode runtime in minutes
├── poster_url (TEXT)
├── backdrop_url (TEXT)
├── monitored (BOOLEAN) - is this show being tracked
├── monitor_type (VARCHAR) - all|future|none
├── auto_hunt_override (BOOLEAN) - NULL = inherit from library
├── organize_override (BOOLEAN) - NULL = inherit from library
├── path (TEXT) - show-specific folder within library
├── created_at, updated_at
```

### Episodes

```sql
episodes
├── id (UUID)
├── tv_show_id (UUID, FK → tv_shows)
├── season (INTEGER)
├── episode (INTEGER)
├── absolute_number (INTEGER) - for anime
├── title (VARCHAR)
├── overview (TEXT)
├── air_date (DATE)
├── runtime (INTEGER)
├── tvmaze_id (INTEGER)
├── tmdb_id (INTEGER)
├── tvdb_id (INTEGER)
├── status (VARCHAR) - missing|wanted|downloading|downloaded|ignored
├── file_id (UUID, FK → media_files) - NULL if not downloaded
├── created_at, updated_at
├── UNIQUE(tv_show_id, season, episode)
```

### Quality Settings (Embedded in Libraries)

Quality settings are stored directly in the `libraries` table, not as separate profiles:

```sql
-- Video library quality columns
├── quality_resolutions (TEXT[]) - ["1080p", "2160p"]
├── quality_video_codecs (TEXT[]) - ["x265", "x264"]
├── quality_hdr_types (TEXT[]) - ["HDR10", "DolbyVision"]
├── quality_audio_formats (TEXT[]) - ["TrueHD", "DTS-HD"]
├── quality_sources (TEXT[]) - ["BluRay", "WEB-DL"]
├── quality_release_groups (TEXT[]) - preferred groups
├── quality_blocked_groups (TEXT[]) - avoid these

-- Audio library quality columns (music/audiobooks)
├── quality_audio_formats (TEXT[]) - ["FLAC", "MP3 320"]
├── quality_bit_depths (TEXT[]) - ["24-bit", "16-bit"]
├── quality_sample_rates (TEXT[]) - ["96kHz", "44.1kHz"]
```

This simplifies the model - each library has its own quality preferences without needing to reference a separate profile table.

### Media Files

```sql
media_files
├── id (UUID)
├── library_id (UUID, FK → libraries)
├── episode_id (UUID, FK → episodes) - NULL for unmatched
├── path (TEXT) - full filesystem path
├── relative_path (TEXT) - path within library
├── original_name (TEXT) - original filename before rename
├── size_bytes (BIGINT)
├── container (VARCHAR) - mkv|mp4|avi
├── video_codec (VARCHAR) - hevc|h264|av1|mpeg2
├── video_bitrate (INTEGER) - kbps
├── audio_codec (VARCHAR)
├── audio_channels (VARCHAR) - 2.0|5.1|7.1|atmos
├── audio_language (VARCHAR)
├── resolution (VARCHAR) - 2160p|1080p|720p|480p
├── width (INTEGER)
├── height (INTEGER)
├── duration_seconds (INTEGER)
├── is_hdr (BOOLEAN)
├── hdr_type (VARCHAR) - hdr10|hdr10plus|dolbyvision|hlg
├── file_hash (VARCHAR) - for deduplication
├── added_at (TIMESTAMPTZ)
├── modified_at (TIMESTAMPTZ)
```

### RSS Feeds

```sql
rss_feeds
├── id (UUID)
├── user_id (UUID)
├── library_id (UUID, FK → libraries) - optional, can be global
├── name (VARCHAR) - display name
├── url (TEXT) - feed URL
├── enabled (BOOLEAN)
├── poll_interval_minutes (INTEGER) - default 15
├── last_polled_at (TIMESTAMPTZ)
├── last_error (TEXT)
├── created_at, updated_at
```

### Downloads (enhanced)

```sql
downloads
├── id (UUID)
├── user_id (UUID)
├── library_id (UUID, FK → libraries)
├── episode_id (UUID, FK → episodes) - what we're downloading for
├── info_hash (VARCHAR) - torrent hash
├── name (VARCHAR)
├── state (VARCHAR) - queued|downloading|seeding|completed|processing|failed
├── progress (DECIMAL)
├── size_bytes (BIGINT)
├── download_path (TEXT) - where files are downloading to
├── source_url (TEXT) - RSS item link or magnet
├── source_feed_id (UUID, FK → rss_feeds)
├── post_process_status (VARCHAR) - pending|processing|completed|failed
├── post_process_error (TEXT)
├── added_at (TIMESTAMPTZ)
├── completed_at (TIMESTAMPTZ)
├── processed_at (TIMESTAMPTZ)
```

### Jobs (enhanced)

```sql
jobs
├── id (UUID)
├── kind (VARCHAR) - library_scan|rss_poll|post_process|metadata_fetch|episode_search
├── library_id (UUID, FK → libraries) - if library-specific
├── payload (JSONB) - job-specific data
├── state (VARCHAR) - pending|running|completed|failed|cancelled
├── priority (INTEGER) - higher = sooner
├── scheduled_at (TIMESTAMPTZ) - when to run next
├── started_at (TIMESTAMPTZ)
├── completed_at (TIMESTAMPTZ)
├── recurring_cron (VARCHAR) - for periodic jobs, e.g., "*/15 * * * *"
├── attempts (INTEGER)
├── max_attempts (INTEGER)
├── last_error (TEXT)
├── created_at (TIMESTAMPTZ)
```

### Unmatched Files (for manual review)

```sql
unmatched_files
├── id (UUID)
├── library_id (UUID, FK → libraries)
├── path (TEXT)
├── parsed_show_name (VARCHAR) - our best guess
├── parsed_season (INTEGER)
├── parsed_episode (INTEGER)
├── suggested_show_id (UUID, FK → tv_shows) - AI/pattern suggestion
├── confidence (DECIMAL) - 0-1 confidence score
├── status (VARCHAR) - pending|matched|ignored
├── created_at (TIMESTAMPTZ)
```

---

## API Surface (GraphQL)

### Libraries
```graphql
type Query {
  libraries: [Library!]!
  library(id: ID!): Library
}

type Mutation {
  createLibrary(input: CreateLibraryInput!): LibraryResult!
  updateLibrary(id: ID!, input: UpdateLibraryInput!): LibraryResult!
  deleteLibrary(id: ID!): MutationResult!
  scanLibrary(id: ID!): ScanStatus!
}

type Subscription {
  libraryScanProgress(libraryId: ID!): LibraryScanProgress!
}
```

### TV Shows
```graphql
type Query {
  tvShows(libraryId: ID): [TvShow!]!
  tvShow(id: ID!): TvShow
  searchTvShows(query: String!): [TvShowSearchResult!]!
}

type Mutation {
  addTvShow(libraryId: ID!, input: AddTvShowInput!): TvShowResult!
  updateTvShow(id: ID!, input: UpdateTvShowInput!): TvShowResult!
  removeTvShow(id: ID!): MutationResult!
  refreshTvShowMetadata(id: ID!): TvShowResult!
}
```

### Episodes
```graphql
type Query {
  episodes(showId: ID!, season: Int): [Episode!]!
  episode(id: ID!): Episode
  wantedEpisodes(libraryId: ID): [Episode!]!
}

type Mutation {
  setEpisodeStatus(id: ID!, status: EpisodeStatus!): Episode!
  searchEpisode(id: ID!): [SearchResult!]!
}
```

### RSS Feeds
```graphql
type Query {
  rssFeeds(libraryId: ID): [RssFeed!]!
  rssFeed(id: ID!): RssFeed
}

type Mutation {
  createRssFeed(input: CreateRssFeedInput!): RssFeedResult!
  updateRssFeed(id: ID!, input: UpdateRssFeedInput!): RssFeedResult!
  deleteRssFeed(id: ID!): MutationResult!
  testRssFeed(id: ID!): RssFeedTestResult!
  pollRssFeed(id: ID!): [RssItem!]!
}
```

### Unmatched Files
```graphql
type Query {
  unmatchedFiles(libraryId: ID): [UnmatchedFile!]!
}

type Mutation {
  matchFile(id: ID!, showId: ID!, season: Int!, episode: Int!): MediaFile!
  ignoreUnmatchedFile(id: ID!): MutationResult!
  autoMatchFiles(libraryId: ID!): AutoMatchResult!
}
```

---

## Background Workers

| Worker | Schedule | Purpose |
|--------|----------|---------|
| **Library Scanner** | Per-library (configurable) | Walk paths, detect new/changed files |
| **Filesystem Watcher** | Real-time (inotify) | Immediate detection of new files |
| **RSS Poller** | Every 15 min (configurable) | Check RSS feeds for new releases |
| **Download Monitor** | Every 1 min | Process completed torrents, organize files |
| **Auto-Hunt** | Event-driven | Search indexers for missing content (triggers on add + after scans) |
| **Metadata Fetcher** | On demand | Fetch show/episode/movie info from APIs |
| **Transcode GC** | Daily at 3 AM | Clean old HLS transcodes |
| **Schedule Sync** | Hourly | Sync TV schedule from TVMaze |

---

## Filename Parsing Patterns

The system will use multiple regex patterns to parse scene-style filenames:

```rust
// Pattern examples (in priority order)
"(?P<show>.+?)\\s*[Ss](?P<season>\\d{1,2})[Ee](?P<episode>\\d{1,2})"  // S01E01
"(?P<show>.+?)\\s*(?P<season>\\d{1,2})x(?P<episode>\\d{2})"           // 1x01
"(?P<show>.+?)\\s*Season\\s*(?P<season>\\d+).*?Episode\\s*(?P<episode>\\d+)"
"(?P<show>.+?)\\s*(?P<season>\\d{1,2})(?P<episode>\\d{2})"            // 101, 102

// Quality patterns
"(?P<resolution>2160p|1080p|720p|480p)"
"(?P<source>HDTV|WEB-DL|WEBRip|BluRay|BDRip)"
"(?P<codec>x264|x265|H\\.?264|H\\.?265|HEVC|AV1|XviD)"
"(?P<hdr>HDR10\\+?|HDR|DV|DoVi|Dolby\\.?Vision)"
"(?P<audio>Atmos|TrueHD|DTS-HD|DTS|AC3|AAC|DD5\\.?1|DDP5\\.?1)"
```

---

## Naming Patterns

Configurable patterns with tokens:

| Token | Description | Example |
|-------|-------------|---------|
| `{show}` | Show name | "Chicago Fire" |
| `{show_clean}` | Show name (filesystem safe) | "Chicago Fire" |
| `{season}` | Season number | "14" |
| `{season:02}` | Season zero-padded | "14" |
| `{episode}` | Episode number | "8" |
| `{episode:02}` | Episode zero-padded | "08" |
| `{title}` | Episode title | "The One That Got Away" |
| `{year}` | Show premiere year | "2012" |
| `{air_date}` | Episode air date | "2026-01-08" |
| `{quality}` | Quality string | "1080p WEB h264" |
| `{ext}` | File extension | "mkv" |

Default TV pattern:
```
{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}
```

Example output:
```
Chicago Fire/Season 14/Chicago Fire - S14E08 - The One That Got Away.mkv
```

---

## Settings

### Global Settings
- **OpenAI API Key**: For AI-based filename matching (optional)
- **TMDB API Key**: For enhanced metadata
- **TVDB API Key**: For legacy support
- **Download directory**: Where torrents download to
- **Temp directory**: For extraction/processing

### Per-Library Settings
- Path
- Default quality profile
- Scan interval
- Watch for changes (inotify)
- Auto-add discovered shows
- Post-download action (copy/move)
- Auto-rename
- Naming pattern

### Per-Show Settings (inherit from library)
- Monitor type (all/future/none)
- Quality profile (override)
- Custom path

---

## Security

- Verify Supabase JWTs via JWKS and cache keys
- Use short‑lived, signed URLs for HLS playlists/segments and artwork
- Keep the Supabase service key only on the backend
- RLS for user‑owned tables; policies like `user_id = auth.uid()`
- Sanitize all filenames before writing to filesystem
- Validate paths don't escape library boundaries

---

## Local Development & Deployment

### Docker Compose (services to run together)

- `supabase/*` stack (Auth, Postgres, Storage, PostgREST, Kong)
- `librarian-backend` (Rust)
- `librarian-frontend` (TanStack Start)
- Prowlarr (optional, for advanced indexer management)

### Shared volumes

- `/data/media` - library storage
- `/data/downloads` - torrent downloads
- `/data/cache` - transcode cache
- `/data/session` - torrent session data

### Environment (.env)

```bash
# Supabase
SUPABASE_URL=
SUPABASE_ANON_KEY=
SUPABASE_SERVICE_KEY=
JWT_SECRET=

# Database
DATABASE_URL=

# Torrent
DOWNLOADS_PATH=/data/downloads
SESSION_PATH=/data/session

# Metadata APIs (optional)
TVMAZE_ENABLED=true
TMDB_API_KEY=
TVDB_API_KEY=

# AI (optional)
OPENAI_API_KEY=
```

---

## MVP vs. Later

### MVP (Completed)
- ✅ Native torrent client (librqbit)
- ✅ GraphQL subscriptions for real-time updates
- ✅ TV library management (create, scan, browse)
- ✅ Show management (add, search, monitor)
- ✅ Episode tracking (wanted list)
- ✅ RSS feed polling
- ✅ Post-download processing with organization
- ✅ Auto-rename and organization with naming patterns
- ✅ Quality filters (per-library, type-aware)
- ✅ Native indexer system (Jackett-like)
- ✅ Auto-Hunt (event-driven, immediate on add + after scans)
- ✅ Movie library support
- ✅ Music library support (MusicBrainz metadata)
- ✅ Audiobook library support (OpenLibrary metadata)
- ✅ Two-way content acquisition (Library-first and Torrent-first workflows)
- ✅ Authenticated downloads for private trackers
- ✅ Chromecast casting support

### Later
- Multi‑quality HLS ladder + dynamic ABR
- Advanced quality upgrading
- DLNA server
- Subtitles management (embedded, external, OCR for PGS)
- Hardware transcoding (NVENC/VAAPI/QSV)
- Multi‑user sharing with roles
- Mobile‑friendly PWA, offline posters, push notifications
- AirPlay casting support

---

## Why These Choices?

**librqbit** provides a native Rust torrent client that's embedded in our process—no external dependencies, no network API calls, direct control and real-time events.

**TVMaze** as the default metadata source because it's completely free with no API key required, has excellent data quality for TV shows, and is fast.

**RSS feeds** as the initial indexer approach because they're universal—every private tracker supports them, they're simple to parse, and they give us everything we need (torrent links, release info, timestamps).

**inotify** for filesystem watching gives us instant detection of new files on supported filesystems, with graceful fallback to periodic scanning for network mounts where inotify doesn't work.

**Copy by default** for post-download because it preserves seeding capability—users who want to maintain ratio on private trackers can keep seeding while their files appear organized in the library.