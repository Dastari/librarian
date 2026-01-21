-- Migration: Add embedded metadata storage to media_files
-- This allows storing ID3/Vorbis tags and tracking processing state

-- Embedded metadata from ID3/Vorbis/container tags (audio)
ALTER TABLE media_files
    ADD COLUMN IF NOT EXISTS meta_artist TEXT,
    ADD COLUMN IF NOT EXISTS meta_album TEXT,
    ADD COLUMN IF NOT EXISTS meta_title TEXT,
    ADD COLUMN IF NOT EXISTS meta_track_number INTEGER,
    ADD COLUMN IF NOT EXISTS meta_disc_number INTEGER,
    ADD COLUMN IF NOT EXISTS meta_year INTEGER,
    ADD COLUMN IF NOT EXISTS meta_genre TEXT;

-- Embedded metadata from container tags (video)
ALTER TABLE media_files
    ADD COLUMN IF NOT EXISTS meta_show_name TEXT,
    ADD COLUMN IF NOT EXISTS meta_season INTEGER,
    ADD COLUMN IF NOT EXISTS meta_episode INTEGER;

-- Processing timestamps (null = not yet done, allows skipping already-processed files)
ALTER TABLE media_files
    ADD COLUMN IF NOT EXISTS ffprobe_analyzed_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS metadata_extracted_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS matched_at TIMESTAMPTZ;

-- Index for finding files that need processing
CREATE INDEX IF NOT EXISTS idx_media_files_needs_ffprobe 
    ON media_files(library_id) 
    WHERE ffprobe_analyzed_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_media_files_needs_metadata 
    ON media_files(library_id) 
    WHERE metadata_extracted_at IS NULL;

-- Index for finding unmatched files
CREATE INDEX IF NOT EXISTS idx_media_files_unmatched 
    ON media_files(library_id) 
    WHERE episode_id IS NULL 
      AND movie_id IS NULL 
      AND track_id IS NULL 
      AND audiobook_id IS NULL;

COMMENT ON COLUMN media_files.meta_artist IS 'Artist name from ID3/Vorbis tags';
COMMENT ON COLUMN media_files.meta_album IS 'Album name from ID3/Vorbis tags';
COMMENT ON COLUMN media_files.meta_title IS 'Track/episode title from embedded tags';
COMMENT ON COLUMN media_files.meta_track_number IS 'Track number from ID3/Vorbis tags';
COMMENT ON COLUMN media_files.meta_disc_number IS 'Disc number from ID3/Vorbis tags';
COMMENT ON COLUMN media_files.meta_year IS 'Year from embedded tags';
COMMENT ON COLUMN media_files.meta_genre IS 'Genre from embedded tags';
COMMENT ON COLUMN media_files.meta_show_name IS 'Show name from video container metadata';
COMMENT ON COLUMN media_files.meta_season IS 'Season number from video container metadata';
COMMENT ON COLUMN media_files.meta_episode IS 'Episode number from video container metadata';
COMMENT ON COLUMN media_files.ffprobe_analyzed_at IS 'When FFprobe analysis was completed (null = not done)';
COMMENT ON COLUMN media_files.metadata_extracted_at IS 'When ID3/Vorbis metadata was extracted (null = not done)';
COMMENT ON COLUMN media_files.matched_at IS 'When file was last matched to a library item';
