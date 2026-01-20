-- ============================================================================
-- Fix watch_progress episode unique index
-- ============================================================================
-- The original migration 023 created idx_watch_progress_user_episode as a
-- regular index, which caused migration 024's "IF NOT EXISTS" to skip
-- creating the proper unique partial index.

-- Drop the incorrect regular index
DROP INDEX IF EXISTS idx_watch_progress_user_episode;

-- Create the correct unique partial index for episodes
CREATE UNIQUE INDEX idx_watch_progress_user_episode 
  ON watch_progress(user_id, episode_id) 
  WHERE content_type = 'episode';

COMMENT ON INDEX idx_watch_progress_user_episode IS 'Unique constraint: one progress record per user per episode';
