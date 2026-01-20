-- ============================================================================
-- Migration: Drop deprecated audiobook_progress table
-- ============================================================================
-- The audiobook_progress table was deprecated in migration 024 when progress
-- tracking was unified into the watch_progress table with content_type='audiobook'.
-- Data was migrated in migration 024.

-- Drop the deprecated table and its indexes
DROP TABLE IF EXISTS audiobook_progress CASCADE;

-- Clean up any orphaned indexes (if table was already dropped)
DROP INDEX IF EXISTS idx_audiobook_progress_user;
DROP INDEX IF EXISTS idx_audiobook_progress_audiobook;
DROP INDEX IF EXISTS idx_audiobook_progress_last_played;
