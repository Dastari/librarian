-- ============================================================================
-- Migration: Unified Watch Progress (SQLite version)
-- ============================================================================
-- Extend watch_progress to support all content types (episode, movie, track, audiobook)
-- This matches PostgreSQL migration 024_unified_playback.sql
--
-- NOTE: This migration is idempotent. If the schema already has the new structure
-- (current_position instead of position_secs), the migration is already complete.

-- Check if migration is already done by looking for current_position column
-- SQLite doesn't have conditional DDL, so we use a different approach:
-- Just ensure the indexes exist (they're created with IF NOT EXISTS)

-- Ensure all indexes exist for the unified schema
CREATE INDEX IF NOT EXISTS idx_watch_progress_user ON watch_progress(user_id);
CREATE INDEX IF NOT EXISTS idx_watch_progress_episode ON watch_progress(user_id, episode_id) WHERE episode_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_watch_progress_movie ON watch_progress(user_id, movie_id) WHERE movie_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_watch_progress_track ON watch_progress(user_id, track_id) WHERE track_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_watch_progress_audiobook ON watch_progress(user_id, audiobook_id) WHERE audiobook_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_watch_progress_content_type ON watch_progress(user_id, content_type);
CREATE INDEX IF NOT EXISTS idx_watch_progress_recent ON watch_progress(user_id, last_watched_at DESC);

-- Unique constraints (partial indexes) - use IF NOT EXISTS
CREATE UNIQUE INDEX IF NOT EXISTS idx_watch_progress_user_episode ON watch_progress(user_id, episode_id) WHERE content_type = 'episode';
CREATE UNIQUE INDEX IF NOT EXISTS idx_watch_progress_user_movie ON watch_progress(user_id, movie_id) WHERE content_type = 'movie';
CREATE UNIQUE INDEX IF NOT EXISTS idx_watch_progress_user_track ON watch_progress(user_id, track_id) WHERE content_type = 'track';
CREATE UNIQUE INDEX IF NOT EXISTS idx_watch_progress_user_audiobook ON watch_progress(user_id, audiobook_id) WHERE content_type = 'audiobook';