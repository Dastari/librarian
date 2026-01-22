-- Migration 040: Torrent Files Table
--
-- Creates a table to store all files within torrents with live progress updates.
-- This table is synced from librqbit every 10 seconds alongside the torrents table.
-- The media_file_id column provides the canonical link from torrent content to library files.

-- ============================================================================
-- Torrent Files Table
-- Stores all files in every torrent with download progress
-- ============================================================================

CREATE TABLE IF NOT EXISTS torrent_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Torrent reference
    torrent_id UUID NOT NULL REFERENCES torrents(id) ON DELETE CASCADE,
    -- File within torrent
    file_index INTEGER NOT NULL,
    file_path TEXT NOT NULL,           -- Full path on disk
    relative_path TEXT NOT NULL,       -- Relative path within torrent
    file_size BIGINT NOT NULL,
    -- Download progress (live updated)
    downloaded_bytes BIGINT NOT NULL DEFAULT 0,
    progress REAL NOT NULL DEFAULT 0,  -- 0.0 to 1.0
    -- Link to organized media file (set after processing)
    media_file_id UUID REFERENCES media_files(id) ON DELETE SET NULL,
    -- Exclusion flag (if file should not be downloaded)
    is_excluded BOOLEAN NOT NULL DEFAULT false,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Constraints
    UNIQUE(torrent_id, file_index)
);

-- ============================================================================
-- Indexes
-- ============================================================================

-- Primary lookup: files by torrent
CREATE INDEX idx_torrent_files_torrent ON torrent_files(torrent_id);

-- Find torrent file for a media file
CREATE INDEX idx_torrent_files_media_file ON torrent_files(media_file_id) 
    WHERE media_file_id IS NOT NULL;

-- Find actively downloading files (progress < 1.0 and not excluded)
CREATE INDEX idx_torrent_files_downloading ON torrent_files(torrent_id, progress) 
    WHERE progress < 1.0 AND NOT is_excluded;

-- ============================================================================
-- Updated At Trigger
-- ============================================================================

CREATE TRIGGER set_updated_at_torrent_files
    BEFORE UPDATE ON torrent_files
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- Comments
-- ============================================================================

COMMENT ON TABLE torrent_files IS 'All files within torrents, synced from librqbit with live progress updates';
COMMENT ON COLUMN torrent_files.file_index IS 'Index of the file within the torrent (0-based)';
COMMENT ON COLUMN torrent_files.file_path IS 'Full absolute path to the file on disk';
COMMENT ON COLUMN torrent_files.relative_path IS 'Relative path within the torrent structure';
COMMENT ON COLUMN torrent_files.progress IS 'Download progress from 0.0 (not started) to 1.0 (complete)';
COMMENT ON COLUMN torrent_files.media_file_id IS 'Link to the organized media file in the library (set after file processing)';
COMMENT ON COLUMN torrent_files.is_excluded IS 'True if this file is excluded from download';
