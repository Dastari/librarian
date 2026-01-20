# Implementation Plan v2: File-Level Matching Pipeline

This plan restructures the backend to implement the unified media pipeline with file-level matching.

## Overview

### Current Issues
1. Torrents matched to single items, not files within torrents
2. No immediate status update to "downloading" at add time
3. No archive extraction
4. FFprobe analysis exists but doesn't trigger status updates
5. Quality verification doesn't set "suboptimal" status
6. Tracks/chapters don't have status fields
7. Post-download action is per-library, not per-indexer/feed

### Goals
1. Match each file in a torrent to individual wanted items
2. Update item status to "downloading" immediately when torrent added
3. Extract archives after download, process contents
4. Use FFprobe to verify quality and set "suboptimal" when needed
5. Add status to tracks and audiobook chapters
6. Allow indexer/feed-level post-download action

---

## Phase 1: Database Schema Updates

### Migration: `028_file_level_matching.sql`

```sql
-- Track file-to-item matches within torrents
CREATE TABLE torrent_file_matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    torrent_info_hash VARCHAR(64) NOT NULL,
    file_index INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    
    -- Match target (exactly one should be set)
    episode_id UUID REFERENCES episodes(id) ON DELETE SET NULL,
    movie_id UUID REFERENCES movies(id) ON DELETE SET NULL,
    track_id UUID REFERENCES tracks(id) ON DELETE SET NULL,
    audiobook_chapter_id UUID,  -- Will reference audiobook_chapters when created
    
    -- Match info
    match_type VARCHAR(20) NOT NULL CHECK (match_type IN ('exact', 'upgrade', 'suboptimal', 'manual', 'unmatched')),
    parsed_quality JSONB,       -- Quality from filename parsing
    verified_quality JSONB,     -- Quality from FFprobe
    
    -- State
    skip_download BOOLEAN DEFAULT false,
    is_sample BOOLEAN DEFAULT false,
    is_archive BOOLEAN DEFAULT false,
    processed BOOLEAN DEFAULT false,
    media_file_id UUID REFERENCES media_files(id) ON DELETE SET NULL,
    error_message TEXT,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(torrent_info_hash, file_index)
);

CREATE INDEX idx_torrent_file_matches_hash ON torrent_file_matches(torrent_info_hash);
CREATE INDEX idx_torrent_file_matches_episode ON torrent_file_matches(episode_id) WHERE episode_id IS NOT NULL;
CREATE INDEX idx_torrent_file_matches_movie ON torrent_file_matches(movie_id) WHERE movie_id IS NOT NULL;
CREATE INDEX idx_torrent_file_matches_track ON torrent_file_matches(track_id) WHERE track_id IS NOT NULL;

-- Add status to tracks (like episodes have)
ALTER TABLE tracks ADD COLUMN IF NOT EXISTS status VARCHAR(20) 
    DEFAULT 'missing' 
    CHECK (status IN ('missing', 'wanted', 'suboptimal', 'downloading', 'downloaded', 'ignored'));

CREATE INDEX idx_tracks_wanted ON tracks(status) WHERE status IN ('missing', 'wanted');

-- Audiobook chapters table
CREATE TABLE IF NOT EXISTS audiobook_chapters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    audiobook_id UUID NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
    chapter_number INTEGER NOT NULL,
    title TEXT,
    duration_secs INTEGER,
    media_file_id UUID REFERENCES media_files(id) ON DELETE SET NULL,
    status VARCHAR(20) DEFAULT 'missing' 
        CHECK (status IN ('missing', 'wanted', 'suboptimal', 'downloading', 'downloaded', 'ignored')),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(audiobook_id, chapter_number)
);

CREATE INDEX idx_audiobook_chapters_wanted ON audiobook_chapters(status) WHERE status IN ('missing', 'wanted');

-- Add post_download_action to indexers (overrides library setting)
ALTER TABLE indexer_configs ADD COLUMN IF NOT EXISTS post_download_action VARCHAR(20) DEFAULT NULL;

-- Add post_download_action to RSS feeds (overrides library setting)
ALTER TABLE rss_feeds ADD COLUMN IF NOT EXISTS post_download_action VARCHAR(20) DEFAULT NULL;

-- Add conflicts_folder to libraries
ALTER TABLE libraries ADD COLUMN IF NOT EXISTS conflicts_folder TEXT DEFAULT '_conflicts';

-- Add quality_status to media_files for tracking suboptimal files
ALTER TABLE media_files ADD COLUMN IF NOT EXISTS quality_status VARCHAR(20) 
    DEFAULT 'unknown'
    CHECK (quality_status IN ('unknown', 'optimal', 'suboptimal', 'below_minimum'));

-- Trigger to update updated_at
CREATE OR REPLACE FUNCTION update_torrent_file_matches_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER torrent_file_matches_updated_at
    BEFORE UPDATE ON torrent_file_matches
    FOR EACH ROW
    EXECUTE FUNCTION update_torrent_file_matches_updated_at();
```

### Files to Update

1. `backend/src/db/mod.rs` - Add torrent_file_matches, audiobook_chapters repositories
2. Create `backend/src/db/torrent_file_matches.rs` - New repository
3. Create `backend/src/db/audiobook_chapters.rs` - New repository
4. Update `backend/src/db/tracks.rs` - Add status-related methods
5. Update `backend/src/db/indexers.rs` - Add post_download_action field
6. Update `backend/src/db/rss_feeds.rs` - Add post_download_action field
7. Update `backend/src/db/libraries.rs` - Add conflicts_folder field
8. Update `backend/src/db/media_files.rs` - Add quality_status field

---

## Phase 2: Core Services

### New Service: `TorrentFileMatcher`

Location: `backend/src/services/torrent_file_matcher.rs`

```rust
/// Matches individual files within a torrent to wanted library items
pub struct TorrentFileMatcher {
    db: Database,
}

impl TorrentFileMatcher {
    /// Match all files in a torrent to wanted items
    /// Called when torrent is first added (before download starts)
    pub async fn match_torrent_files(
        &self,
        info_hash: &str,
        files: &[TorrentFileInfo],
        context: MatchContext,
    ) -> Result<Vec<TorrentFileMatch>>;
    
    /// Re-match after FFprobe analysis completes
    pub async fn verify_file_quality(
        &self,
        match_id: Uuid,
        ffprobe_data: &FfprobeResult,
    ) -> Result<QualityVerification>;
    
    /// Find wanted items that could match a file
    async fn find_matching_items(
        &self,
        parsed: &ParsedFilename,
        context: &MatchContext,
    ) -> Result<Vec<PotentialMatch>>;
    
    /// Check if file quality meets/exceeds target
    fn evaluate_quality(
        &self,
        parsed: &ParsedQuality,
        target: &QualitySettings,
        current: Option<&MediaFileRecord>,
    ) -> QualityEvaluation;
}

pub struct MatchContext {
    pub user_id: Uuid,
    pub library_id: Option<Uuid>,
    pub explicit_item_id: Option<ExplicitItemLink>,
    pub indexer_id: Option<String>,
    pub feed_id: Option<Uuid>,
}

pub enum ExplicitItemLink {
    Episode(Uuid),
    Movie(Uuid),
    Track(Uuid),
    AudiobookChapter(Uuid),
    Album(Uuid),      // Match tracks within album
    TvShow(Uuid),     // Match episodes within show
    Audiobook(Uuid),  // Match chapters within audiobook
}

pub enum QualityEvaluation {
    MeetsTarget,
    Upgrade { current_resolution: String },
    Suboptimal { reason: String },
    BelowMinimum { reason: String },
}
```

### New Service: `ArchiveExtractor`

Location: `backend/src/services/archive_extractor.rs`

```rust
/// Extracts archive files (zip, rar, 7z) after torrent download
pub struct ArchiveExtractor {
    downloads_path: PathBuf,
}

impl ArchiveExtractor {
    /// Check if a file is an archive
    pub fn is_archive(path: &Path) -> bool;
    
    /// Extract archive to a subdirectory
    /// Returns paths to all extracted files
    pub async fn extract(&self, archive_path: &Path) -> Result<ExtractionResult>;
}

pub struct ExtractionResult {
    pub extracted_files: Vec<PathBuf>,
    pub extraction_dir: PathBuf,
    pub errors: Vec<String>,
}
```

### Modified Service: `TorrentProcessor`

The existing TorrentProcessor needs significant changes:

```rust
impl TorrentProcessor {
    /// Process a torrent when first added (BEFORE download)
    /// Creates file matches and updates item statuses
    pub async fn process_torrent_added(
        &self,
        info_hash: &str,
        files: &[TorrentFileInfo],
        context: MatchContext,
    ) -> Result<AddedTorrentResult>;
    
    /// Process a completed torrent (AFTER download)
    /// Runs FFprobe, verifies quality, organizes files
    pub async fn process_torrent_completed(
        &self,
        info_hash: &str,
        force: bool,
    ) -> Result<CompletedTorrentResult>;
    
    /// Process a single file after FFprobe analysis
    async fn process_matched_file(
        &self,
        file_match: &TorrentFileMatch,
        file_path: &Path,
        ffprobe_result: &FfprobeResult,
    ) -> Result<ProcessedFileResult>;
}
```

### Modified Service: `OrganizerService`

Add conflict handling:

```rust
impl OrganizerService {
    /// Organize a file, handling conflicts
    pub async fn organize_file_with_conflicts(
        &self,
        media_file: &MediaFileRecord,
        target_path: &Path,
        action: &str,
        conflicts_folder: &str,
    ) -> Result<OrganizeResult>;
    
    /// Move an existing file to the conflicts folder
    async fn move_to_conflicts(
        &self,
        existing_path: &Path,
        library_path: &Path,
        conflicts_folder: &str,
    ) -> Result<PathBuf>;
}
```

---

## Phase 3: Integration Points

### TorrentService Changes

When adding a torrent, immediately trigger file matching:

```rust
// In TorrentService::add_magnet / add_torrent_bytes / add_torrent_url

// 1. Add torrent to librqbit
let handle = self.session.add_torrent(...)?;

// 2. Get file list from torrent metadata
let files = handle.get_files()?;

// 3. Create torrent record in DB
self.db.torrents().create(...)?;

// 4. Match files to wanted items
let matches = self.file_matcher.match_torrent_files(
    &info_hash,
    &files,
    context,
).await?;

// 5. Update item statuses to "downloading"
for m in &matches {
    self.update_item_status_downloading(m).await?;
}

// 6. Configure file exclusions if needed
for m in &matches {
    if m.skip_download {
        handle.exclude_file(m.file_index)?;
    }
}
```

### Download Monitor Changes

When torrent completes, process with new pipeline:

```rust
pub async fn process_completed_torrents(...) {
    for torrent in completed_torrents {
        // 1. Check for archives, extract if needed
        let files = get_torrent_files(&torrent)?;
        for file in &files {
            if ArchiveExtractor::is_archive(file) {
                extractor.extract(file).await?;
            }
        }
        
        // 2. Get all file matches for this torrent
        let matches = db.torrent_file_matches()
            .list_by_torrent(&torrent.info_hash)
            .await?;
        
        // 3. Process each matched file
        for file_match in matches {
            if file_match.processed || file_match.is_sample {
                continue;
            }
            
            let file_path = resolve_file_path(&torrent, &file_match)?;
            
            // 4. Run FFprobe
            let ffprobe = run_ffprobe(&file_path).await?;
            
            // 5. Verify quality, update status
            let quality = file_matcher.verify_file_quality(
                file_match.id,
                &ffprobe,
            ).await?;
            
            // 6. Create/update media_file
            let media_file = create_media_file(&file_match, &ffprobe)?;
            
            // 7. Organize if enabled
            if library.organize_files {
                let action = get_post_download_action(&torrent, &library)?;
                organizer.organize_file_with_conflicts(
                    &media_file,
                    &target_path,
                    &action,
                    &library.conflicts_folder,
                ).await?;
            }
            
            // 8. Update item status based on quality
            update_item_final_status(&file_match, &quality).await?;
        }
    }
}
```

### Scanner Changes

Use same quality verification for scanned files:

```rust
// In ScannerService::process_file

// 1. Create media_file record
let media_file = create_media_file(...)?;

// 2. Queue for FFprobe analysis (async)
analysis_queue.submit(MediaAnalysisJob {
    media_file_id: media_file.id,
    path: file_path,
    check_subtitles: true,
})?;

// 3. After FFprobe completes (in worker):
let ffprobe = run_ffprobe(&path)?;
let quality_status = evaluate_quality(&ffprobe, &library_settings)?;

// 4. Update media_file with quality_status
db.media_files().update_quality_status(
    media_file.id,
    quality_status,
    &ffprobe,
)?;

// 5. Update item status
match quality_status {
    QualityStatus::Optimal => set_status(item_id, "downloaded"),
    QualityStatus::Suboptimal => set_status(item_id, "suboptimal"),
    QualityStatus::BelowMinimum => {
        // Log warning, maybe don't link?
    }
}
```

---

## Phase 4: Post-Download Action Resolution

### Priority Order

```rust
fn get_post_download_action(
    torrent: &TorrentRecord,
    library: &LibraryRecord,
) -> String {
    // 1. Check if torrent came from a specific indexer
    if let Some(indexer_id) = &torrent.indexer_id {
        if let Some(indexer) = db.indexers().get_by_id(indexer_id).await? {
            if let Some(action) = indexer.post_download_action {
                return action;
            }
        }
    }
    
    // 2. Check if torrent came from an RSS feed
    if let Some(feed_id) = &torrent.source_feed_id {
        if let Some(feed) = db.rss_feeds().get_by_id(feed_id).await? {
            if let Some(action) = feed.post_download_action {
                return action;
            }
        }
    }
    
    // 3. Fall back to library setting
    library.post_download_action.clone()
}
```

---

## Phase 5: UI Updates

### Downloads Page

1. Show file-level matches for each torrent
2. Add "Fix Match" button per file
3. Show quality status (optimal/suboptimal/below minimum)

### Library Detail Page

1. Add "Suboptimal" filter tab
2. Show quality badge with warning for suboptimal items
3. Add conflicts section if `_conflicts` folder has contents

### Settings Pages

1. Per-indexer: Add post_download_action dropdown
2. Per-feed: Add post_download_action dropdown
3. Per-library: Add conflicts_folder setting

---

## Implementation Order

### Week 1: Database & Core Models
- [ ] Create migration `028_file_level_matching.sql`
- [ ] Create `db/torrent_file_matches.rs` repository
- [ ] Create `db/audiobook_chapters.rs` repository  
- [ ] Update existing repositories with new fields
- [ ] Add GraphQL types for new entities

### Week 2: File Matching Service
- [ ] Create `services/torrent_file_matcher.rs`
- [ ] Integrate with existing filename_parser
- [ ] Add quality evaluation logic
- [ ] Add tests for matching scenarios

### Week 3: Archive Extraction
- [ ] Create `services/archive_extractor.rs`
- [ ] Add zip extraction (native Rust)
- [ ] Add rar extraction (shell out to unrar or use crate)
- [ ] Add 7z extraction (shell out or use crate)
- [ ] Integrate with TorrentProcessor

### Week 4: Pipeline Integration
- [ ] Modify TorrentService to call file matcher at add time
- [ ] Modify download_monitor to use new processing
- [ ] Add status updates at each pipeline stage
- [ ] Add post_download_action resolution

### Week 5: Quality Verification
- [ ] Enhance FFprobe analysis to populate quality_status
- [ ] Add suboptimal detection logic
- [ ] Update scanner to use same verification
- [ ] Add quality status to GraphQL types

### Week 6: Conflict Handling & UI
- [ ] Add conflict folder logic to organizer
- [ ] Create frontend components for file matches
- [ ] Add suboptimal filter to library pages
- [ ] Add indexer/feed post_download_action UI

---

## Testing Scenarios

### Multi-Episode Torrent
1. Add season pack torrent linked to library
2. Verify all episodes marked "downloading"  
3. Wait for completion
4. Verify each episode's media_file is correct
5. Verify each episode marked "downloaded"

### Quality Upgrade
1. Have 720p episode already downloaded
2. Add 1080p torrent for same episode
3. Verify match type is "upgrade"
4. Verify old file moved to conflicts
5. Verify episode still marked "downloaded" (not suboptimal)

### Partial Album
1. Add album with 12 tracks
2. Add torrent with 8 tracks
3. Verify 8 tracks marked "downloading"
4. Verify 4 tracks still "wanted"
5. After completion, 8 are "downloaded", 4 still "wanted"

### Archive Extraction
1. Add torrent containing a .zip with video files
2. Verify zip detected as archive
3. After completion, verify extraction
4. Verify extracted files processed and matched

### Manual Link Override
1. Add torrent via /hunt, link to specific movie
2. Verify file matched to that movie (even if name doesn't match)
3. Verify quality still evaluated
4. Verify status set correctly

---

## Rollback Plan

If issues arise, the migration can be rolled back:

```sql
-- Rollback 028_file_level_matching.sql
DROP TABLE IF EXISTS torrent_file_matches;
DROP TABLE IF EXISTS audiobook_chapters;
ALTER TABLE tracks DROP COLUMN IF EXISTS status;
ALTER TABLE indexer_configs DROP COLUMN IF EXISTS post_download_action;
ALTER TABLE rss_feeds DROP COLUMN IF EXISTS post_download_action;
ALTER TABLE libraries DROP COLUMN IF EXISTS conflicts_folder;
ALTER TABLE media_files DROP COLUMN IF EXISTS quality_status;
```

The existing TorrentProcessor can continue to function without the new tables during transition.
