-- Add organize/rename settings to libraries and shows
-- Renames auto_rename to organize_files and adds rename_style

-- ============================================================================
-- Rename Style Enum Values
-- ============================================================================
-- 'none' - Keep original filename, just organize into folders
-- 'clean' - Rename to clean format: "Show Name - S01E01 - Episode Title.mkv"
-- 'preserve_info' - Clean format with quality info: "Show Name - S01E01 - Episode Title [1080p HEVC MeGusta].mkv"

-- ============================================================================
-- Library Settings
-- ============================================================================

-- Rename auto_rename to organize_files for clarity
ALTER TABLE libraries
RENAME COLUMN auto_rename TO organize_files;

-- Add rename_style column
ALTER TABLE libraries
ADD COLUMN IF NOT EXISTS rename_style VARCHAR(20) NOT NULL DEFAULT 'none' 
    CHECK (rename_style IN ('none', 'clean', 'preserve_info'));

COMMENT ON COLUMN libraries.organize_files IS 'Automatically organize files into Show/Season folder structure';
COMMENT ON COLUMN libraries.rename_style IS 'How to rename files: none (keep original), clean (standard naming), preserve_info (clean + quality info)';

-- ============================================================================
-- TV Show Override Settings
-- ============================================================================

-- Add override settings to tv_shows (NULL = inherit from library)
ALTER TABLE tv_shows
ADD COLUMN IF NOT EXISTS organize_files_override BOOLEAN DEFAULT NULL,
ADD COLUMN IF NOT EXISTS rename_style_override VARCHAR(20) DEFAULT NULL
    CHECK (rename_style_override IS NULL OR rename_style_override IN ('none', 'clean', 'preserve_info'));

COMMENT ON COLUMN tv_shows.organize_files_override IS 'Override library organize_files setting. NULL = inherit';
COMMENT ON COLUMN tv_shows.rename_style_override IS 'Override library rename_style setting. NULL = inherit';

-- ============================================================================
-- Media Files - Track organization status
-- ============================================================================

-- Add columns to track if file has been organized
ALTER TABLE media_files
ADD COLUMN IF NOT EXISTS organized BOOLEAN NOT NULL DEFAULT false,
ADD COLUMN IF NOT EXISTS organized_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS original_path TEXT;

COMMENT ON COLUMN media_files.organized IS 'Whether this file has been organized into library structure';
COMMENT ON COLUMN media_files.organized_at IS 'When the file was organized';
COMMENT ON COLUMN media_files.original_path IS 'Original file path before organization (for reference)';

-- Index for finding unorganized files
CREATE INDEX IF NOT EXISTS idx_media_files_unorganized 
    ON media_files(library_id, organized) 
    WHERE organized = false;
