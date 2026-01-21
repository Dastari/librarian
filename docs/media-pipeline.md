# Media Pipeline Architecture

This document defines the unified media pipeline for Librarian - how files flow from torrents into organized libraries.

## Core Principles

1. **File-level matching** - Each file in a torrent is matched independently to individual items (episodes, tracks, chapters, movies)
2. **Quality is verified, not assumed** - Every file is analyzed with ffprobe to determine true quality, never rely solely on filename
3. **No auto-delete files** - Move conflicts to a designated folder, never auto-delete user files
4. **Trust explicit user choices** - When user manually links a torrent to an item, trust their selection
5. **Partial fulfillment is OK** - Downloading 8 of 12 album tracks is valid; remaining 4 stay "wanted"
6. **Status reflects reality** - "downloading" means in download queue, "downloaded" means file in library folder

---

## Item Status Lifecycle

### Episode/Track/Chapter/Movie Status Values

| Status | Meaning | Triggers |
|--------|---------|----------|
| `missing` | No file exists, hasn't aired yet (for episodes) | Default for future content |
| `wanted` | Aired/released, no file, actively looking | Air date passed, monitored=true |
| `suboptimal` | Has file but below quality target | ffprobe detects quality below library/item setting |
| `downloading` | In torrent download queue | Torrent added with matched file |
| `downloaded` | File exists in library folder | File organized to library path |
| `ignored` | User explicitly skipped | Manual action |

### Status Transitions

```
                    ┌──────────────────────────────────────────────────┐
                    │                                                  │
                    ▼                                                  │
┌─────────┐    ┌─────────┐    ┌─────────────┐    ┌────────────┐    │
│ missing │───▶│ wanted  │───▶│ downloading │───▶│ downloaded │    │
└─────────┘    └─────────┘    └─────────────┘    └────────────┘    │
     │              │                                   │           │
     │              │                                   ▼           │
     │              │              ┌────────────┐   ┌───────────┐   │
     │              └─────────────▶│ suboptimal │◀──│ (upgrade) │───┘
     │                             └────────────┘   └───────────┘
     │                                   │
     └───────────────────────────────────┘
                        │
                        ▼
                  ┌─────────┐
                  │ ignored │
                  └─────────┘
```

### Key Rules

- **missing → wanted**: When air_date passes (for episodes) or immediately (for movies/albums added)
- **wanted → downloading**: When file in torrent matches this item
- **downloading → downloaded**: When file is organized to library folder
- **downloaded → suboptimal**: When ffprobe reveals quality below target
- **suboptimal → wanted**: Only via explicit user action (or auto-upgrade if implemented)
- **Any → ignored**: Explicit user action only

---

## The Unified Pipeline

### Entry Points

All content enters through one of these paths:

1. **Auto-Hunt** - System searches indexers for wanted items (torrent or usenet)
2. **RSS Feed** - System polls feeds and matches against wanted items  
3. **Manual Hunt** - User searches on `/hunt` page, may or may not link to library/item
4. **Direct Add** - User adds magnet/URL/NZB directly, no library context
5. **Library Scan** - System discovers files already in library folder

### Download Sources

The pipeline supports two download sources with unified processing:

| Source | Protocol | Tracking Table | File Matches Table |
|--------|----------|----------------|-------------------|
| Torrent | BitTorrent (librqbit) | `torrents` | `torrent_file_matches` |
| Usenet | NNTP (native) | `usenet_downloads` | `usenet_file_matches` |

Both sources flow through the same post-download processing pipeline.

### Pipeline Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           UNIFIED MEDIA PIPELINE                             │
└─────────────────────────────────────────────────────────────────────────────┘

PHASE 1: DOWNLOAD ACQUISITION (Torrent or Usenet)
═══════════════════════════════════════════════════════════════════════════════

     Auto-Hunt         RSS Feed         Manual /hunt        Direct Add
         │                 │                  │                  │
         └────────────┬────┴──────────────────┴──────────────────┘
                      │
                      ▼
         ┌────────────────────────────┐
         │  1. ADD DOWNLOAD           │
         │  Torrent:                  │
         │  - Create `torrents` record│
         │  - Add to librqbit         │
         │  Usenet:                   │
         │  - Create `usenet_         │
         │    downloads` record       │
         │  - Queue for NNTP download │
         │                            │
         │  Store source context:     │
         │    • library_id (optional) │
         │    • item_id (if explicit) │
         │    • indexer_id (for auth) │
         │  - Get file list from      │
         │    metadata (torrent) or   │
         │    NZB (usenet)            │
         └─────────────┬──────────────┘
                       │
                       ▼
         ┌────────────────────────────┐
         │  2. ANALYZE TORRENT FILES  │
         │  For each file in torrent: │
         │  - Is it a media file?     │
         │  - Is it an archive?       │
         │  - Is it a sample?         │
         │  - Is it artwork/subs?     │
         │  - Parse filename for info │
         └─────────────┬──────────────┘
                       │
                       ▼
         ┌────────────────────────────┐
         │  3. MATCH FILES TO ITEMS   │
         │  For each media file:      │
         │  - Find matching wanted    │
         │    item across libraries   │
         │    with auto_download=true │
         │  - Check quality threshold │
         │    (based on filename)     │
         │  - Create torrent_file_    │
         │    match record            │
         │  - Update item status      │
         │    to 'downloading'        │
         │                            │
         │  If file already exists:   │
         │  - Skip file in torrent    │
         │    (don't download)        │
         │  - OR accept if upgrade    │
         └─────────────┬──────────────┘
                       │
                       ▼
         ┌────────────────────────────┐
         │  4. CONFIGURE DOWNLOAD     │
         │  - Exclude sample files    │
         │  - Exclude already-have    │
         │    files (if possible)     │
         │  - Start download          │
         └─────────────┬──────────────┘
                       │
                       ▼
              ┌────────────────┐
              │  DOWNLOADING   │
              │  Torrent:      │
              │   (librqbit)   │
              │  Usenet:       │
              │   (NNTP+yEnc)  │
              └───────┬────────┘
                      │
                      ▼

PHASE 2: POST-DOWNLOAD PROCESSING (Unified for both sources)
═══════════════════════════════════════════════════════════════════════════════

         ┌────────────────────────────┐
         │  5. TORRENT COMPLETES      │
         │  Download Monitor Job      │
         │  (runs every minute)       │
         └─────────────┬──────────────┘
                       │
           ┌───────────┴───────────┐
           │                       │
           ▼                       ▼
    ┌──────────────┐       ┌──────────────┐
    │ Has Archives │       │ No Archives  │
    │              │       │              │
    │ Extract to:  │       │ Process      │
    │ {torrent}/   │       │ directly     │
    │ _extracted/  │       │              │
    └──────┬───────┘       └──────┬───────┘
           │                       │
           └───────────┬───────────┘
                       │
                       ▼
         ┌────────────────────────────┐
         │  6. PROCESS EACH FILE      │
         │  For each media file:      │
         │                            │
         │  a) Run ffprobe analysis   │
         │     - True resolution      │
         │     - Codec, bitrate       │
         │     - HDR type             │
         │     - Audio tracks         │
         │     - Embedded subtitles   │
         │                            │
         │  b) Verify/update match    │
         │     - Use ffprobe data     │
         │     - Flag if suboptimal   │
         │                            │
         │  c) Create media_file      │
         │     record with real data  │
         │                            │
         │  d) Handle related files   │
         │     - External subtitles   │
         │     - Album artwork        │
         └─────────────┬──────────────┘
                       │
                       ▼
         ┌────────────────────────────┐
         │  7. ORGANIZE FILES         │
         │  (If library.organize)     │
         │                            │
         │  Get post_download_action: │
         │  - From indexer/feed       │
         │    (if seeding required)   │
         │  - Fall back to library    │
         │                            │
         │  For each matched file:    │
         │  - Generate target path    │
         │    using naming_pattern    │
         │  - copy/move/hardlink      │
         │  - Update media_file.path  │
         │                            │
         │  Handle conflicts:         │
         │  - Move to _conflicts/     │
         │    folder, don't delete    │
         └─────────────┬──────────────┘
                       │
                       ▼
         ┌────────────────────────────┐
         │  8. FINALIZE               │
         │  - Update item status      │
         │    to 'downloaded'         │
         │    OR 'suboptimal'         │
         │  - Update stats            │
         │  - Mark torrent complete   │
         └────────────────────────────┘


PHASE 3: LIBRARY SCANNING (ALTERNATE ENTRY)
═══════════════════════════════════════════════════════════════════════════════

         ┌────────────────────────────┐
         │  SCAN LIBRARY FOLDER       │
         │  - Walk directory tree     │
         │  - For each media file:    │
         └─────────────┬──────────────┘
                       │
                       ▼
         ┌────────────────────────────┐
         │  1. ANALYZE FILE           │
         │  - Run ffprobe             │
         │  - Parse filename          │
         │  - Extract real metadata   │
         └─────────────┬──────────────┘
                       │
                       ▼
         ┌────────────────────────────┐
         │  2. MATCH TO WANTED        │
         │  Does file match a         │
         │  wanted item in library?   │
         └─────────────┬──────────────┘
                       │
           ┌───────────┴───────────┐
           │                       │
           ▼                       ▼
    ┌──────────────┐       ┌──────────────┐
    │   MATCH      │       │   NO MATCH   │
    │              │       │              │
    │ Link file    │       │ auto_add?    │
    │ to item      │       │              │
    │ Check quality│       └──────┬───────┘
    │ Set status   │              │
    └──────────────┘      ┌───────┴───────┐
                          │               │
                          ▼               ▼
                   ┌──────────┐    ┌──────────┐
                   │   YES    │    │    NO    │
                   │          │    │          │
                   │ Search   │    │ Add to   │
                   │ metadata │    │ unmatched│
                   │ provider │    │ files    │
                   │ Create   │    │          │
                   │ show +   │    │          │
                   │ episodes │    │          │
                   │ Link file│    │          │
                   └──────────┘    └──────────┘
```

---

## File Matching Logic

### Match Priority

When matching a file to wanted items:

1. **Explicit link** - If user/auto-hunt provided item_id, use it (trust selection)
2. **Library context** - If library_id provided, only match within that library
3. **All libraries** - If no context, search all user's libraries with auto_download=true

### Match Criteria

For a file to match an item:

```rust
fn can_match(file: &ParsedFile, item: &WantedItem, settings: &QualitySettings) -> MatchResult {
    // 1. Content match (show/season/episode, artist/album/track, title/year, etc.)
    if !content_matches(file, item) {
        return MatchResult::NoMatch;
    }
    
    // 2. Quality check (based on filename - real check after ffprobe)
    let parsed_quality = parse_quality_from_filename(&file.name);
    
    if quality_meets_target(&parsed_quality, settings) {
        return MatchResult::Match;
    }
    
    if quality_is_upgrade(&parsed_quality, item.current_quality) {
        return MatchResult::Upgrade;
    }
    
    // Quality below target but still usable
    return MatchResult::Suboptimal;
}
```

### Quality Verification

After ffprobe analysis, quality is re-evaluated:

```rust
fn verify_quality(media_file: &MediaFile, settings: &QualitySettings) -> QualityStatus {
    // Use ACTUAL values from ffprobe, not filename parsing
    let actual = ActualQuality {
        resolution: media_file.height, // e.g., 1080
        codec: &media_file.video_codec,
        hdr: media_file.is_hdr,
        // etc.
    };
    
    if meets_target(&actual, settings) {
        QualityStatus::Optimal
    } else if above_minimum(&actual, settings) {
        QualityStatus::Suboptimal
    } else {
        QualityStatus::BelowMinimum
    }
}
```

---

## Handling Special Cases

### Sample Files

- **Detection**: Filename contains "sample" (case-insensitive) or size < 100MB for videos
- **Action**: Skip during organization, leave in torrent for seeding
- **Database**: Don't create media_file record

### Archive Files (zip, rar, 7z)

- **Detection**: Extension is .zip, .rar, .7z, .tar.gz
- **Extraction**: Extract to `{downloads}/{torrent_name}/_extracted/`
- **Processing**: Run normal file matching on extracted contents
- **Cleanup**: Leave archive in torrent for seeding

### Artwork Files

- **Detection**: Extension is .jpg, .jpeg, .png, .gif
- **For Albums**: Copy to album folder if no artwork exists
- **For Movies/Shows**: Ignore (use metadata provider artwork)

### Subtitle Files

- **Detection**: Extension is .srt, .sub, .ass, .ssa, .vtt, .idx
- **Action**: Copy alongside video file during organization
- **Future**: Link to subtitle system when implemented

### Conflicts

When organizing would overwrite an existing file:

1. **Same quality**: Skip organization, leave in downloads
2. **Better quality**: 
   - Move existing to `{library}/_conflicts/`
   - Organize new file to proper location
   - Log conflict for user review
3. **Worse quality**:
   - Leave new file in downloads
   - Mark as "suboptimal" if no other match

---

## Database Schema

All tables are implemented in migrations 028-034.

### `torrent_file_matches`

Tracks the relationship between torrent files and library items:

```sql
torrent_file_matches (
    id UUID PRIMARY KEY,
    torrent_id UUID NOT NULL,
    file_index INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    -- Match targets (one will be set)
    episode_id, movie_id, track_id, audiobook_chapter_id UUID,
    -- Match metadata
    match_type VARCHAR(20) NOT NULL, -- 'auto', 'manual', 'forced'
    match_confidence DECIMAL(3, 2),
    parsed_resolution, parsed_codec, parsed_source VARCHAR,
    -- State
    skip_download BOOLEAN DEFAULT false,
    processed BOOLEAN DEFAULT false,
    media_file_id UUID,
    UNIQUE(torrent_id, file_index)
);
```

### `usenet_file_matches`

Parallel table for usenet downloads:

```sql
usenet_file_matches (
    id UUID PRIMARY KEY,
    usenet_download_id UUID NOT NULL,
    file_path TEXT NOT NULL,
    file_size BIGINT,
    -- Match targets
    episode_id, movie_id, album_id, track_id, audiobook_id UUID,
    processed BOOLEAN DEFAULT false,
    media_file_id UUID
);
```

### Status Fields

Added to multiple tables for unified tracking:

| Table | Column | Values |
|-------|--------|--------|
| `episodes` | `status` | missing, wanted, available, downloading, downloaded, ignored, suboptimal |
| `tracks` | `status` | missing, wanted, downloading, downloaded, ignored |
| `audiobook_chapters` | `status` | missing, wanted, downloading, downloaded, ignored |
| `movies` | `download_status` | missing, wanted, downloading, downloaded, ignored, suboptimal |
| `albums` | `download_status` | missing, wanted, downloading, downloaded, ignored, suboptimal, partial |
| `audiobooks` | `download_status` | missing, wanted, downloading, downloaded, ignored, suboptimal |
| `media_files` | `quality_status` | unknown, optimal, suboptimal, exceeds |

### Post-Download Action Overrides

Both `indexer_configs` and `rss_feeds` have `post_download_action` column:
- `NULL` = use library default
- `'copy'` | `'move'` | `'hardlink'` = override library setting

---

## Library Settings

| Setting | Purpose |
|---------|---------|
| `auto_scan` | Run scan on schedule |
| `scan_interval_minutes` | How often to scan |
| `watch_for_changes` | Use inotify for real-time detection |
| `auto_add_discovered` | Create entries from unmatched files |
| `auto_download` | Auto-grab from RSS when match found |
| `auto_hunt` | Search indexers for missing content |
| `organize_files` | Automatically organize into folders |
| `naming_pattern` | How to name/structure files |
| `post_download_action` | copy/move/hardlink (default, overridden by indexer/feed) |
| `conflicts_folder` | Where to move conflicting files (default: `_conflicts`) |

---

## Monitored Field

The `monitored` field on shows/albums/audiobooks controls **which items are wanted**:

- `monitor_type = 'all'`: All existing episodes are wanted
- `monitor_type = 'future'`: Only unaired/unreleased items are wanted
- `monitor_type = 'none'`: No items are automatically wanted

This is separate from `auto_hunt` and `auto_download` which control automation.

---

## Implementation Modules

All modules are implemented in `backend/src/services/`:

| Module | Purpose |
|--------|---------|
| `torrent_file_matcher.rs` | Matches files within torrents to wanted items |
| `media_processor.rs` | Unified download processing (torrents + usenet) |
| `quality_evaluator.rs` | Uses ffprobe to verify actual quality |
| `organizer.rs` | File organization with conflict handling |
| `hunt.rs` | Auto-hunt service |
| `scanner.rs` | Library scanning |
| `usenet.rs` | Usenet NNTP client |
| `extractor.rs` | Archive extraction (zip/rar) |

---

## UI Additions Needed

### Downloads Page

- Show individual file matches within each torrent
- Show file status (downloading, processing, organized)
- "Fix Match" button to manually correct a file's target item

### Library Detail Page

- "Suboptimal" filter option alongside "Wanted"
- Conflicts section showing files in `_conflicts` folder

### Settings

- Per-indexer `post_download_action` setting
- Per-feed `post_download_action` setting
