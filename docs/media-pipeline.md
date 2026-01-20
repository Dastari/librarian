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

1. **Auto-Hunt** - System searches indexers for wanted items
2. **RSS Feed** - System polls feeds and matches against wanted items  
3. **Manual Hunt** - User searches on `/hunt` page, may or may not link to library/item
4. **Direct Add** - User adds magnet/URL directly, no library context
5. **Library Scan** - System discovers files already in library folder

### Pipeline Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           UNIFIED MEDIA PIPELINE                             │
└─────────────────────────────────────────────────────────────────────────────┘

PHASE 1: TORRENT ACQUISITION
═══════════════════════════════════════════════════════════════════════════════

     Auto-Hunt         RSS Feed         Manual /hunt        Direct Add
         │                 │                  │                  │
         └────────────┬────┴──────────────────┴──────────────────┘
                      │
                      ▼
         ┌────────────────────────────┐
         │  1. ADD TORRENT TO CLIENT  │
         │  - Create `torrents` record│
         │  - Store source context:   │
         │    • library_id (optional) │
         │    • item_id (if explicit) │
         │    • indexer_id (for auth) │
         │  - Get torrent file list   │
         │    from metadata           │
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
              │  (librqbit)    │
              └───────┬────────┘
                      │
                      ▼

PHASE 2: POST-DOWNLOAD PROCESSING
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

## Database Changes Required

### New Table: `torrent_file_matches`

Tracks the relationship between torrent files and library items:

```sql
CREATE TABLE torrent_file_matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    torrent_id UUID NOT NULL REFERENCES torrents(id) ON DELETE CASCADE,
    file_index INTEGER NOT NULL,  -- Index within torrent
    file_path TEXT NOT NULL,      -- Path within torrent
    file_size BIGINT NOT NULL,
    
    -- Match target (one of these will be set)
    episode_id UUID REFERENCES episodes(id),
    movie_id UUID REFERENCES movies(id),
    track_id UUID REFERENCES tracks(id),
    audiobook_chapter_id UUID REFERENCES audiobook_chapters(id),
    
    -- Match metadata
    match_type VARCHAR(20) NOT NULL, -- 'exact', 'upgrade', 'suboptimal', 'manual'
    quality_parsed JSONB,            -- Quality parsed from filename
    quality_verified JSONB,          -- Quality from ffprobe (after download)
    
    -- State
    skip_download BOOLEAN DEFAULT false,  -- Don't download this file
    processed BOOLEAN DEFAULT false,
    media_file_id UUID REFERENCES media_files(id),
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Add to `tracks` table

```sql
ALTER TABLE tracks ADD COLUMN status VARCHAR(20) 
    DEFAULT 'missing' 
    CHECK (status IN ('missing', 'wanted', 'suboptimal', 'downloading', 'downloaded', 'ignored'));
```

### Add to `audiobooks` table (chapters)

```sql
CREATE TABLE audiobook_chapters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    audiobook_id UUID NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
    chapter_number INTEGER NOT NULL,
    title TEXT,
    duration_secs INTEGER,
    media_file_id UUID REFERENCES media_files(id),
    status VARCHAR(20) DEFAULT 'missing' 
        CHECK (status IN ('missing', 'wanted', 'suboptimal', 'downloading', 'downloaded', 'ignored')),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(audiobook_id, chapter_number)
);
```

### Modify `indexer_configs` table

```sql
ALTER TABLE indexer_configs ADD COLUMN post_download_action VARCHAR(20) 
    DEFAULT NULL; -- NULL means use library setting
```

### Modify `rss_feeds` table

```sql
ALTER TABLE rss_feeds ADD COLUMN post_download_action VARCHAR(20) 
    DEFAULT NULL; -- NULL means use library setting
```

---

## Library Settings Clarification

### Current Settings (Keep)

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

### Settings to Remove

| Setting | Reason |
|---------|--------|
| (none currently) | |

### Settings to Add

| Setting | Purpose |
|---------|---------|
| `conflicts_folder` | Where to move conflicting files (default: `_conflicts`) |

---

## Monitored Field Clarification

The `monitored` field on shows/albums/audiobooks controls **which items are wanted**:

- `monitor_type = 'all'`: All existing episodes are wanted
- `monitor_type = 'future'`: Only unaired/unreleased items are wanted
- `monitor_type = 'none'`: No items are automatically wanted

This is separate from `auto_hunt` and `auto_download` which control automation.

---

## Implementation Modules

### New Services

1. **TorrentFileMatcher** - Matches files within torrents to wanted items
2. **ArchiveExtractor** - Extracts zip/rar/7z archives
3. **QualityVerifier** - Uses ffprobe to verify actual quality
4. **ConflictHandler** - Manages file conflicts during organization

### Modified Services

1. **TorrentProcessor** - Use TorrentFileMatcher, handle file-level processing
2. **Scanner** - Use same QualityVerifier for consistency
3. **Organizer** - Add conflict handling, check indexer/feed for post_download_action
4. **AutoHunt** - Link torrents properly for file-level matching

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
