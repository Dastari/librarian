-- Add missing video metadata columns to media_files table
-- These were added in PostgreSQL migration 037 but missing from SQLite

-- Video container metadata
ALTER TABLE media_files ADD COLUMN meta_show_name TEXT;
ALTER TABLE media_files ADD COLUMN meta_season INTEGER;
ALTER TABLE media_files ADD COLUMN meta_episode INTEGER;

-- Processing timestamps
ALTER TABLE media_files ADD COLUMN ffprobe_analyzed_at TEXT;
ALTER TABLE media_files ADD COLUMN metadata_extracted_at TEXT;
ALTER TABLE media_files ADD COLUMN matched_at TEXT;

-- Indexes for finding files that need processing
CREATE INDEX IF NOT EXISTS idx_media_files_needs_ffprobe 
    ON media_files(library_id) 
    WHERE ffprobe_analyzed_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_media_files_needs_metadata 
    ON media_files(library_id) 
    WHERE metadata_extracted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_media_files_unmatched 
    ON media_files(library_id) 
    WHERE episode_id IS NULL 
      AND movie_id IS NULL 
      AND track_id IS NULL 
      AND audiobook_id IS NULL;
