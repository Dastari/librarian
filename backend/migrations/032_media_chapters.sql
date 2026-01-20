-- Migration: Restore media file chapters table
-- The original 'chapters' table was replaced by audiobook chapters in migration 029.
-- This creates a separate 'media_chapters' table for video/audio file chapter markers.

CREATE TABLE IF NOT EXISTS media_chapters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_file_id UUID NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    chapter_index INTEGER NOT NULL,
    start_secs DOUBLE PRECISION NOT NULL,
    end_secs DOUBLE PRECISION NOT NULL,
    title TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(media_file_id, chapter_index)
);

CREATE INDEX idx_media_chapters_media_file ON media_chapters(media_file_id);

COMMENT ON TABLE media_chapters IS 'Chapter markers extracted from media files (video/audio)';
COMMENT ON COLUMN media_chapters.chapter_index IS '0-based chapter index';
COMMENT ON COLUMN media_chapters.start_secs IS 'Chapter start time in seconds';
COMMENT ON COLUMN media_chapters.end_secs IS 'Chapter end time in seconds';
