-- Add album art and lyrics storage to media_files
-- Album art is stored as base64 for easy frontend display

ALTER TABLE media_files
    ADD COLUMN IF NOT EXISTS cover_art_base64 TEXT,
    ADD COLUMN IF NOT EXISTS cover_art_mime TEXT,
    ADD COLUMN IF NOT EXISTS lyrics TEXT;

-- Index for files with cover art (for future cover art gallery features)
CREATE INDEX IF NOT EXISTS idx_media_files_has_cover_art 
    ON media_files(library_id) 
    WHERE cover_art_base64 IS NOT NULL;
