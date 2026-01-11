-- Add auto-download settings to libraries and shows
-- Allows library-level control with per-show overrides

-- ============================================================================
-- Library Settings
-- ============================================================================

-- Add auto_download setting to libraries (default true for automation)
ALTER TABLE libraries
ADD COLUMN IF NOT EXISTS auto_download BOOLEAN NOT NULL DEFAULT true;

COMMENT ON COLUMN libraries.auto_download IS 'Automatically download episodes marked as available from RSS feeds';

-- ============================================================================
-- TV Show Settings (Override library defaults)
-- ============================================================================

-- Add override settings to tv_shows
-- NULL = inherit from library, true = force on, false = force off
ALTER TABLE tv_shows
ADD COLUMN IF NOT EXISTS auto_download_override BOOLEAN DEFAULT NULL,
ADD COLUMN IF NOT EXISTS backfill_existing BOOLEAN NOT NULL DEFAULT true;

COMMENT ON COLUMN tv_shows.auto_download_override IS 'Override library auto_download setting. NULL = inherit, true = always download, false = never download';
COMMENT ON COLUMN tv_shows.backfill_existing IS 'When adding show, search RSS cache for existing episodes to download';

-- ============================================================================
-- Index for efficient querying
-- ============================================================================

-- Index for finding shows that should auto-download
CREATE INDEX IF NOT EXISTS idx_tv_shows_auto_download 
    ON tv_shows(library_id, auto_download_override) 
    WHERE monitored = true;
