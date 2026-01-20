-- Migration: File-level torrent matching and unified media pipeline
-- 
-- This migration adds support for:
-- 1. File-level matching within torrents (not just torrent-level)
-- 2. Status fields for tracks and movies (unified with episodes)
-- 3. Quality status tracking for media files
-- 4. Post-download action per indexer/feed (for seeding requirements)
-- 5. Conflicts folder for libraries
-- 6. Audiobook chapter status tracking

-- ============================================================================
-- Torrent File Matches Table
-- Tracks which files within a torrent match to which library items
-- ============================================================================

CREATE TABLE IF NOT EXISTS torrent_file_matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Torrent reference
    torrent_id UUID NOT NULL REFERENCES torrents(id) ON DELETE CASCADE,
    -- File within torrent
    file_index INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    -- What it matches to (only one should be set)
    episode_id UUID REFERENCES episodes(id) ON DELETE SET NULL,
    movie_id UUID REFERENCES movies(id) ON DELETE SET NULL,
    track_id UUID REFERENCES tracks(id) ON DELETE SET NULL,
    audiobook_chapter_id UUID REFERENCES audiobook_chapters(id) ON DELETE SET NULL,
    -- Match metadata
    match_type VARCHAR(20) NOT NULL DEFAULT 'auto' CHECK (match_type IN ('auto', 'manual', 'forced')),
    match_confidence DECIMAL(3, 2) CHECK (match_confidence >= 0 AND match_confidence <= 1),
    -- Quality info parsed from filename
    parsed_resolution VARCHAR(20),
    parsed_codec VARCHAR(50),
    parsed_source VARCHAR(50),
    parsed_audio VARCHAR(50),
    -- Processing status
    skip_download BOOLEAN NOT NULL DEFAULT false,
    processed BOOLEAN NOT NULL DEFAULT false,
    processed_at TIMESTAMPTZ,
    -- Resulting media file (after organization)
    media_file_id UUID REFERENCES media_files(id) ON DELETE SET NULL,
    -- Error tracking
    error_message TEXT,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Constraints
    UNIQUE(torrent_id, file_index)
);

CREATE INDEX idx_torrent_file_matches_torrent ON torrent_file_matches(torrent_id);
CREATE INDEX idx_torrent_file_matches_episode ON torrent_file_matches(episode_id) WHERE episode_id IS NOT NULL;
CREATE INDEX idx_torrent_file_matches_movie ON torrent_file_matches(movie_id) WHERE movie_id IS NOT NULL;
CREATE INDEX idx_torrent_file_matches_track ON torrent_file_matches(track_id) WHERE track_id IS NOT NULL;
CREATE INDEX idx_torrent_file_matches_chapter ON torrent_file_matches(audiobook_chapter_id) WHERE audiobook_chapter_id IS NOT NULL;
CREATE INDEX idx_torrent_file_matches_unprocessed ON torrent_file_matches(torrent_id, processed) WHERE NOT processed;

CREATE TRIGGER set_updated_at_torrent_file_matches
    BEFORE UPDATE ON torrent_file_matches
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE torrent_file_matches IS 'Maps individual files within torrents to library items for file-level matching';
COMMENT ON COLUMN torrent_file_matches.match_type IS 'auto = system matched, manual = user linked, forced = user override without quality check';
COMMENT ON COLUMN torrent_file_matches.skip_download IS 'True if this file should not be downloaded (already have it)';

-- ============================================================================
-- Add status field to tracks (mirrors episodes.status)
-- ============================================================================

ALTER TABLE tracks 
ADD COLUMN IF NOT EXISTS status VARCHAR(20) NOT NULL DEFAULT 'missing' 
CHECK (status IN ('missing', 'wanted', 'downloading', 'downloaded', 'ignored'));

CREATE INDEX IF NOT EXISTS idx_tracks_status ON tracks(album_id, status);
CREATE INDEX IF NOT EXISTS idx_tracks_wanted ON tracks(status) WHERE status IN ('missing', 'wanted');

COMMENT ON COLUMN tracks.status IS 'Download status: missing = no file, wanted = actively seeking, downloading = in progress, downloaded = have file, ignored = skip';

-- Update existing tracks: set status based on media_file_id
UPDATE tracks SET status = 'downloaded' WHERE media_file_id IS NOT NULL AND status = 'missing';

-- ============================================================================
-- Add download status field to movies (unified with episodes)
-- movies.status is already used for release status (released, upcoming, etc.)
-- So we add a new download_status field
-- ============================================================================

ALTER TABLE movies 
ADD COLUMN IF NOT EXISTS download_status VARCHAR(20) NOT NULL DEFAULT 'missing' 
CHECK (download_status IN ('missing', 'wanted', 'downloading', 'downloaded', 'ignored', 'suboptimal'));

CREATE INDEX IF NOT EXISTS idx_movies_download_status ON movies(library_id, download_status);
CREATE INDEX IF NOT EXISTS idx_movies_wanted ON movies(download_status) WHERE download_status IN ('missing', 'wanted');

COMMENT ON COLUMN movies.download_status IS 'Download status: missing = no file, wanted = actively seeking, downloading = in progress, downloaded = have file, suboptimal = have file but quality below target, ignored = skip';

-- Update existing movies: set download_status based on has_file
UPDATE movies SET download_status = 'downloaded' WHERE has_file = true AND download_status = 'missing';
UPDATE movies SET download_status = 'wanted' WHERE has_file = false AND monitored = true AND download_status = 'missing';

-- ============================================================================
-- Add status field to audiobook_chapters
-- ============================================================================

ALTER TABLE audiobook_chapters 
ADD COLUMN IF NOT EXISTS status VARCHAR(20) NOT NULL DEFAULT 'missing' 
CHECK (status IN ('missing', 'wanted', 'downloading', 'downloaded', 'ignored'));

CREATE INDEX IF NOT EXISTS idx_audiobook_chapters_status ON audiobook_chapters(audiobook_id, status);

COMMENT ON COLUMN audiobook_chapters.status IS 'Download status for chapter-based audiobooks';

-- Update existing chapters: set status based on media_file_id
UPDATE audiobook_chapters SET status = 'downloaded' WHERE media_file_id IS NOT NULL AND status = 'missing';

-- ============================================================================
-- Add suboptimal status to episodes
-- ============================================================================

-- Need to recreate the check constraint to add 'suboptimal'
ALTER TABLE episodes DROP CONSTRAINT IF EXISTS episodes_status_check;
ALTER TABLE episodes ADD CONSTRAINT episodes_status_check 
CHECK (status IN ('missing', 'wanted', 'available', 'downloading', 'downloaded', 'ignored', 'suboptimal'));

COMMENT ON COLUMN episodes.status IS 'Status: missing = not aired or no file, wanted = should download, available = found in RSS, downloading = in progress, downloaded = have file, suboptimal = have file but quality below target, ignored = skip';

-- ============================================================================
-- Add quality_status to media_files for tracking suboptimal files
-- ============================================================================

ALTER TABLE media_files 
ADD COLUMN IF NOT EXISTS quality_status VARCHAR(20) NOT NULL DEFAULT 'unknown' 
CHECK (quality_status IN ('unknown', 'optimal', 'suboptimal', 'exceeds'));

CREATE INDEX IF NOT EXISTS idx_media_files_quality_status ON media_files(library_id, quality_status);

COMMENT ON COLUMN media_files.quality_status IS 'Quality relative to library/item target: unknown = not analyzed, optimal = meets target, suboptimal = below target, exceeds = better than target';

-- ============================================================================
-- Add post_download_action to indexer_configs
-- Allows different seeding behavior per indexer (e.g., copy for private trackers)
-- ============================================================================

ALTER TABLE indexer_configs 
ADD COLUMN IF NOT EXISTS post_download_action VARCHAR(20) DEFAULT NULL 
CHECK (post_download_action IS NULL OR post_download_action IN ('copy', 'move', 'hardlink'));

COMMENT ON COLUMN indexer_configs.post_download_action IS 'Override library post_download_action for this indexer (NULL = use library default)';

-- ============================================================================
-- Add post_download_action to rss_feeds
-- Same as indexers - allows per-feed seeding behavior
-- ============================================================================

ALTER TABLE rss_feeds 
ADD COLUMN IF NOT EXISTS post_download_action VARCHAR(20) DEFAULT NULL 
CHECK (post_download_action IS NULL OR post_download_action IN ('copy', 'move', 'hardlink'));

COMMENT ON COLUMN rss_feeds.post_download_action IS 'Override library post_download_action for this feed (NULL = use library default)';

-- ============================================================================
-- Add conflicts_folder to libraries
-- Where conflicting files are moved instead of deleted
-- ============================================================================

ALTER TABLE libraries 
ADD COLUMN IF NOT EXISTS conflicts_folder TEXT DEFAULT '_conflicts';

COMMENT ON COLUMN libraries.conflicts_folder IS 'Subfolder within library for conflicting/duplicate files (default: _conflicts)';

-- ============================================================================
-- Add source tracking to torrents
-- Track which indexer/feed a torrent came from for post_download_action resolution
-- ============================================================================

ALTER TABLE torrents 
ADD COLUMN IF NOT EXISTS source_indexer_id UUID REFERENCES indexer_configs(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_torrents_source_indexer ON torrents(source_indexer_id) WHERE source_indexer_id IS NOT NULL;

COMMENT ON COLUMN torrents.source_indexer_id IS 'Which indexer this torrent was downloaded from';

-- ============================================================================
-- Update post_process_status to include new states
-- ============================================================================

ALTER TABLE torrents DROP CONSTRAINT IF EXISTS torrents_post_process_status_check;
ALTER TABLE torrents ADD CONSTRAINT torrents_post_process_status_check 
CHECK (post_process_status IN ('pending', 'processing', 'completed', 'failed', 'skipped', 'error', 'matched', 'unmatched', 'partial'));

COMMENT ON COLUMN torrents.post_process_status IS 'Processing status: pending = waiting, processing = in progress, completed = all files organized, matched = matched but organize disabled, unmatched = no matches found, partial = some files processed, error/failed = processing failed';

-- ============================================================================
-- Add file-level exclusion tracking for librqbit
-- Stores which file indices should be skipped
-- ============================================================================

ALTER TABLE torrents 
ADD COLUMN IF NOT EXISTS excluded_files INTEGER[] DEFAULT '{}';

COMMENT ON COLUMN torrents.excluded_files IS 'Array of file indices to exclude from download (0-based)';

-- ============================================================================
-- Add audiobook download status (unified with movies)
-- ============================================================================

ALTER TABLE audiobooks 
ADD COLUMN IF NOT EXISTS download_status VARCHAR(20) NOT NULL DEFAULT 'missing' 
CHECK (download_status IN ('missing', 'wanted', 'downloading', 'downloaded', 'ignored', 'suboptimal'));

CREATE INDEX IF NOT EXISTS idx_audiobooks_download_status ON audiobooks(library_id, download_status);

COMMENT ON COLUMN audiobooks.download_status IS 'Download status for the audiobook';

-- Update existing audiobooks: set download_status based on has_files
UPDATE audiobooks SET download_status = 'downloaded' WHERE has_files = true AND download_status = 'missing';

-- ============================================================================
-- Add album download status (for music libraries)
-- ============================================================================

ALTER TABLE albums 
ADD COLUMN IF NOT EXISTS download_status VARCHAR(20) NOT NULL DEFAULT 'missing' 
CHECK (download_status IN ('missing', 'wanted', 'downloading', 'downloaded', 'ignored', 'suboptimal', 'partial'));

CREATE INDEX IF NOT EXISTS idx_albums_download_status ON albums(library_id, download_status);

COMMENT ON COLUMN albums.download_status IS 'Download status: partial = some tracks downloaded';

-- Update existing albums: set download_status based on has_files
UPDATE albums SET download_status = 'downloaded' WHERE has_files = true AND download_status = 'missing';
