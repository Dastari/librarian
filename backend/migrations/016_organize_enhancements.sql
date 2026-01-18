-- Migration: Add organize_status tracking to media_files
-- This enables conflict detection and tracking during file organization

-- Add organize_status column to track organization state
ALTER TABLE media_files ADD COLUMN IF NOT EXISTS organize_status VARCHAR(20) 
    DEFAULT 'pending' CHECK (organize_status IN ('pending', 'organized', 'skipped', 'conflicted', 'error'));

-- Add organize_error column to store error/conflict details
ALTER TABLE media_files ADD COLUMN IF NOT EXISTS organize_error TEXT;

-- Create index for querying files by organization status
CREATE INDEX IF NOT EXISTS idx_media_files_organize_status 
    ON media_files(library_id, organize_status) 
    WHERE organize_status != 'organized';

COMMENT ON COLUMN media_files.organize_status IS 'Organization status: pending, organized, skipped, conflicted, error';
COMMENT ON COLUMN media_files.organize_error IS 'Error or conflict details when organization fails';
