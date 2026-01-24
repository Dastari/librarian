# Media Pipeline Architecture

This document defines the unified media pipeline for Librarian - how files flow from any download source into organized libraries.

## Core Principles

1. **Source-agnostic matching** - The same matching logic handles files from torrents, usenet, IRC, FTP, or library scans
2. **Always COPY, never move** - Files are always copied from download folders to library folders
3. **Library owns files** - Unlinking a download source never affects library files
4. **Quality is verified, not assumed** - Every file is analyzed with FFprobe to determine true quality
5. **No auto-delete files** - Conflicts require user resolution; never overwrite or auto-delete library files
6. **Partial fulfillment is OK** - Downloading 8 of 12 album tracks is valid; remaining 4 stay "wanted"
7. **Status reflects reality** - "downloading" means in download queue, "downloaded" means file in library folder

---

## Architecture Overview

The matching and processing logic is **source-agnostic** - implemented in two central services:

```mermaid
flowchart TD
    subgraph sources [File Sources]
        TORRENT[Torrent]
        USENET[Usenet]
        IRC[IRC/FTP]
        SCAN[Library Scan]
    end
    
    subgraph core [Core Services - Source Agnostic]
        FM[FileMatcher]
        FP[FileProcessor]
    end
    
    subgraph library [Library Domain]
        LI[Library Items]
        MF[Media Files]
        LO[Library Organize]
    end
    
    TORRENT -->|"file path"| FM
    USENET -->|"file path"| FM
    IRC -->|"file path"| FM
    SCAN -->|"file path"| FM
    
    FM -->|"returns matches"| PFM[(pending_file_matches)]
    FP -->|"copies file"| MF
    MF --> LI
    LO -->|"renames within library"| MF
    
    PFM -.->|"links to"| LI
```

**Key Services:**

| Service | Location | Responsibility |
|---------|----------|----------------|
| `FileMatcher` | `backend/src/services/file_matcher.rs` | THE ONLY place matching logic exists |
| `FileProcessor` | `backend/src/services/file_processor.rs` | THE ONLY place file copying happens |
| `LibraryOrganizer` | `backend/src/services/organizer.rs` | Renames files within library folder only |

---

## Item Status Lifecycle

### Episode/Track/Chapter/Movie Status Values

| Status | Meaning | Triggers |
|--------|---------|----------|
| `missing` | No file exists, hasn't aired yet (for episodes) | Default for future content |
| `wanted` | Aired/released, no file, actively looking | Air date passed, monitored=true |
| `downloading` | Pending match exists for this item | FileMatcher created `pending_file_matches` |
| `downloaded` | File exists in library folder | File copied via FileProcessor |
| `suboptimal` | Has file but below quality target | FFprobe detects quality below target |
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
- **wanted → downloading**: When FileMatcher creates a match for this item
- **downloading → downloaded**: When FileProcessor copies the file to library
- **downloaded → suboptimal**: When FFprobe reveals quality below target
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

### Pipeline Flow

```mermaid
sequenceDiagram
    participant User
    participant AutoHunt
    participant DownloadService as Torrent/Usenet
    participant FileMatcher
    participant FileProcessor
    participant Library

    User->>AutoHunt: Add album to library
    AutoHunt->>DownloadService: add_torrent(magnet, library_id)
    DownloadService->>FileMatcher: match_files(files, library_id)
    FileMatcher->>Library: Find wanted tracks
    FileMatcher-->>DownloadService: save matches to pending_file_matches
    Note over DownloadService: Download in progress...
    DownloadService->>FileProcessor: process_source("torrent", torrent_id)
    FileProcessor->>Library: Copy files to library folder
    FileProcessor->>Library: Create media_files records
    FileProcessor->>Library: Link items to media_files
    FileProcessor->>Library: Update status="downloaded"
```

### Phase 1: Download Acquisition

When a download is added:

1. **Create download record** - `torrents` or `usenet_downloads` table
2. **Get file list** - From torrent metadata or NZB
3. **Match files** - `FileMatcher.match_files()` finds matching wanted items
4. **Save matches** - Creates `pending_file_matches` records
5. **Update item status** - Sets matched items to "downloading"
   - Unmatched files are stored in `pending_file_matches` with `unmatched_reason` for manual review

### Phase 2: Post-Download Processing

When download completes (triggered by Download Monitor Job):

1. **Process pending matches** - `FileProcessor.process_source()` processes all uncopied matches
2. **For each match:**
   - Determine destination path using library naming pattern
   - Copy file from download folder to library folder
   - Create `media_file` record with new path
   - Link item to media_file
   - Update item status to "downloaded"
   - Queue file for FFprobe analysis

### Phase 3: Library Scanning (Alternate Entry)

When scanning existing library files:

1. **Walk directory tree** - Find all media files
2. **For new files** - Use `FileMatcher.match_file()` to find matching wanted item
3. **If matched** - Use `FileProcessor.link_existing_file()` to create media_file and link
4. **If not matched + auto_add_discovered** - Create new item from file metadata

---

## Database Schema

### `pending_file_matches` (Source-Agnostic)

Replaces the old `torrent_file_matches` table with a unified, source-agnostic design:

```sql
pending_file_matches (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    
    -- Source file info (works for any source)
    source_path TEXT NOT NULL,           -- Full path to source file
    source_type VARCHAR(20) NOT NULL,    -- 'torrent', 'usenet', 'scan', 'manual'
    source_id UUID,                      -- torrent_id, usenet_download_id, etc.
    source_file_index INTEGER,           -- For multi-file sources (torrents)
    file_size BIGINT NOT NULL,
    
    -- Match target (only one set per row)
    episode_id UUID REFERENCES episodes(id) ON DELETE CASCADE,
    movie_id UUID REFERENCES movies(id) ON DELETE CASCADE,
    track_id UUID REFERENCES tracks(id) ON DELETE CASCADE,
    chapter_id UUID REFERENCES chapters(id) ON DELETE CASCADE,
    unmatched_reason TEXT,
    
    -- Match metadata
    match_type VARCHAR(20) DEFAULT 'auto',  -- 'auto', 'manual', 'unmatched'
    match_confidence DECIMAL(3,2),
    match_attempts INTEGER DEFAULT 1,
    verification_status TEXT,
    verification_reason TEXT,
    
    -- Parsed quality info (from filename)
    parsed_resolution VARCHAR(20),
    parsed_codec VARCHAR(50),
    parsed_source VARCHAR(50),
    parsed_audio VARCHAR(100),
    
    -- Processing status
    copied_at TIMESTAMPTZ,               -- null = not yet copied
    copy_error TEXT,                     -- error if copy failed
    copy_attempts INTEGER DEFAULT 0,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index for finding matches by source
CREATE INDEX idx_pending_file_matches_source ON pending_file_matches(source_type, source_id);
```

---

## GraphQL API

### Queries

```graphql
# Get pending matches for any source
query PendingFileMatches($sourceType: String!, $sourceId: String!) {
  pendingFileMatches(sourceType: $sourceType, sourceId: $sourceId) {
    id
    sourcePath
    sourceType
    episodeId movieId trackId chapterId
    unmatchedReason
    matchConfidence matchAttempts
    verificationStatus
    copied copiedAt copyError copyAttempts
  }
}
```

### Mutations

```graphql
# Re-match all files from a source
mutation RematchSource($sourceType: String!, $sourceId: ID!, $libraryId: ID) {
  rematchSource(sourceType: $sourceType, sourceId: $sourceId, libraryId: $libraryId) {
    success matchCount error
  }
}

# Process pending matches (copy files to library)
mutation ProcessSource($sourceType: String!, $sourceId: ID!) {
  processSource(sourceType: $sourceType, sourceId: $sourceId) {
    success filesProcessed filesFailed error
  }
}

# Manually set a match target
mutation SetMatch($matchId: ID!, $targetType: String!, $targetId: ID!) {
  setMatch(matchId: $matchId, targetType: $targetType, targetId: $targetId) {
    success error
  }
}

# Remove a specific match
mutation RemoveMatch($matchId: ID!) {
  removeMatch(matchId: $matchId) {
    success error
  }
}
```

---

## File Matching Logic

### Match Priority

When matching a file to wanted items:

1. **Explicit link** - If user/auto-hunt provided item_id, use it (trust selection)
2. **Library context** - If library_id provided, only match within that library
3. **All libraries** - If no context, search all user's libraries with auto_download=true

### 3-Tier Matching Priority

For each file, matching is attempted in this order:

1. **Embedded Metadata** - ID3 tags (MP3), Vorbis comments (FLAC/OGG), container metadata (MKV/MP4)
2. **Original Filename** - If the file was renamed, we try matching the stored `original_name`
3. **Current Filename** - Fall back to parsing the current filename

```rust
fn match_media_file(&self, file: &MediaFileRecord, library: &LibraryRecord) -> MatchResult {
    // Tier 1: Try embedded metadata (highest priority)
    if file.metadata_extracted_at.is_some() {
        if let Some(result) = self.try_match_by_stored_metadata(file, library) {
            return result;
        }
    }
    
    // Tier 2: Try original filename (if different from current)
    if let Some(original) = &file.original_name {
        if original != current_filename {
            if let Some(result) = self.try_match_by_filename(original, library) {
                return result;
            }
        }
    }
    
    // Tier 3: Try current filename
    self.try_match_by_filename(current_filename, library)
}
```

### Weighted Fuzzy Matching

Uses `rapidfuzz` with field-specific weights and proportional scoring via `match_scorer.rs`:

**Music Matching (100 points max):**
| Field | Weight | Notes |
|-------|--------|-------|
| Artist | 30 | Exact match gets full points |
| Album | 25 | Fuzzy match with proportional score |
| Track Title | 25 | Fuzzy match with proportional score |
| Track Number | 15 | Exact match only |
| Year | 5 | Within ±1 year tolerance |

**TV Show Matching (100 points max):**
| Field | Weight | Notes |
|-------|--------|-------|
| Show Name | 35 | Fuzzy match with proportional score |
| Season | 25 | Exact match only |
| Episode | 25 | Exact match only |
| Episode Title | 15 | Fuzzy match bonus |

**Movie Matching (100 points max):**
| Field | Weight | Notes |
|-------|--------|-------|
| Title | 50 | Fuzzy match with proportional score |
| Year | 40 | Exact match or ±1 year |
| Director | 10 | Fuzzy match bonus |

**Audiobook Matching (100 points max):**
| Field | Weight | Notes |
|-------|--------|-------|
| Author | 30 | Fuzzy match with proportional score |
| Book Title | 30 | Fuzzy match with proportional score |
| Chapter Title | 20 | Fuzzy match with proportional score |
| Chapter Number | 20 | Exact match only |

**Thresholds:**
- **Auto-link**: Score ≥ 70 → automatically link file to item
- **Suggest**: Score ≥ 40 → show in suggestions for manual review
- **Reject**: Score < 40 → no match

### Metadata Storage

Extracted metadata is stored in the `media_files` table for consistent matching:

```sql
-- Audio/Music metadata
meta_artist TEXT,
meta_album TEXT,
meta_title TEXT,
meta_track_number INTEGER,
meta_disc_number INTEGER,
meta_year INTEGER,
meta_genre TEXT,

-- Video/TV metadata
meta_show_name TEXT,
meta_season INTEGER,
meta_episode INTEGER,

-- Processing timestamps
ffprobe_analyzed_at TIMESTAMPTZ,
metadata_extracted_at TIMESTAMPTZ,
matched_at TIMESTAMPTZ,

-- Album art and lyrics
cover_art_base64 TEXT,
cover_art_mime TEXT,
lyrics TEXT
```

---

## Implementation Modules

| Module | Purpose | Status |
|--------|---------|--------|
| `file_matcher.rs` | Source-agnostic file matching (THE ONLY matching code) | ✅ Complete |
| `match_scorer.rs` | Weighted fuzzy matching for all media types | ✅ Complete |
| `file_processor.rs` | Source-agnostic file copying (THE ONLY copy code) | ✅ Complete |
| `pending_file_matches.rs` | Database repository for pending matches | ✅ Complete |
| `organizer.rs` | Library-only file organization | ✅ Complete |
| `quality_evaluator.rs` | FFprobe quality verification | ✅ Complete |
| `scanner.rs` | Library scanning with mismatch detection/correction | ✅ Complete |
| `torrent_completion_handler.rs` | Torrent add/complete handling | ✅ Complete |
| `download_monitor.rs` | Scheduled processing job | ✅ Complete |
| `auto_hunt.rs` | Auto-hunt service | ✅ Complete |
| `queues.rs` | Background job processing (metadata extraction) | ✅ Complete |

---

## Implementation Status

### Completed

- ✅ `pending_file_matches` database table (source-agnostic)
- ✅ `FileMatcher` service with weighted fuzzy matching
- ✅ `match_scorer.rs` with field-specific weights for all media types
- ✅ 3-tier matching priority (metadata → original filename → current filename)
- ✅ Metadata storage in `media_files` table (ID3/Vorbis tags, album art, lyrics)
- ✅ `FileProcessor` service with copy and link
- ✅ GraphQL mutations: rematchSource, processSource, setMatch, removeMatch, rematchMediaFile, extractMediaFileMetadata
- ✅ GraphQL query: pendingFileMatches, mediaFileDetails (with embedded metadata)
- ✅ Torrent integration (match on add, process on complete)
- ✅ Download monitor job updated
- ✅ Scanner with mismatch detection and auto-correction
- ✅ Bidirectional link consistency (media_files.track_id ↔ tracks.media_file_id)
- ✅ Frontend: TorrentTable with Process/Rematch actions
- ✅ Frontend: TorrentInfoModal with copy status and remove match
- ✅ Frontend: FilePropertiesModal with Metadata tab (shows extracted tags, album art, lyrics)
- ✅ Frontend: Progress fractions (e.g., "9/15") instead of status chips
- ✅ Notifications for repeated unmatched files and processing failures

### Pending

- 🟡 Frontend: Library item progress bar driven by pending matches

### Future Enhancements

- ⏳ Usenet integration with new services
- ⏳ IRC/FTP download source support
- ⏳ Conflict resolution UI for duplicates vs incorrect matches

---

## UI Features

### Downloads Page

| Feature | Status |
|---------|--------|
| File matches per torrent | ✅ In Info modal |
| Copy status display (Copied/Pending/Error) | ✅ Complete |
| "Process" action (copy files to library) | ✅ Complete |
| "Rematch" action (re-run matching) | ✅ Complete |
| "Remove Match" per file | ✅ Complete |

### Library Items

| Feature | Status |
|---------|--------|
| Progress bar when downloading | 🟡 Needs pending match integration |
| Suboptimal quality indicator | ✅ Complete |

---

## Archive Extraction

Fully implemented with support for:

| Format | Status | Tool |
|--------|--------|------|
| ZIP | ✅ | Native Rust |
| RAR (single) | ✅ | `unrar` |
| RAR (multi-part) | ✅ | `unrar` (auto-handles .r00, .r01, etc.) |
| 7z | ✅ | `7z` command |

Extraction is triggered automatically:
- After torrent completion
- After usenet download completion
- Archives extracted to `{download_path}/_extracted/`
- Original archives preserved for seeding
