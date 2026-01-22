-- Add album art and lyrics storage to media_files
-- These were added in PostgreSQL migration 038

ALTER TABLE media_files ADD COLUMN cover_art_base64 TEXT;
ALTER TABLE media_files ADD COLUMN cover_art_mime TEXT;
ALTER TABLE media_files ADD COLUMN lyrics TEXT;

-- Index for files with cover art
CREATE INDEX IF NOT EXISTS idx_media_files_has_cover_art 
    ON media_files(library_id) 
    WHERE cover_art_base64 IS NOT NULL;
