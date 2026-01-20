-- ============================================================================
-- Migration: Unified Playback System
-- ============================================================================
-- Consolidates playback and watch progress to work with all content types:
-- episodes, movies, tracks, and audiobooks.

-- ============================================================================
-- 1. Add content_type to media_files
-- ============================================================================
-- This column indicates what type of content this media file represents

ALTER TABLE media_files ADD COLUMN IF NOT EXISTS content_type TEXT;

-- Populate based on existing foreign keys
UPDATE media_files SET content_type = 
  CASE 
    WHEN episode_id IS NOT NULL THEN 'episode'
    WHEN movie_id IS NOT NULL THEN 'movie'
    WHEN track_id IS NOT NULL THEN 'track'
    WHEN audiobook_id IS NOT NULL THEN 'audiobook'
    ELSE NULL
  END
WHERE content_type IS NULL;

-- Add check constraint
ALTER TABLE media_files DROP CONSTRAINT IF EXISTS check_content_type;
ALTER TABLE media_files ADD CONSTRAINT check_content_type 
  CHECK (content_type IS NULL OR content_type IN ('episode', 'movie', 'track', 'audiobook'));

CREATE INDEX IF NOT EXISTS idx_media_files_content_type ON media_files(content_type) WHERE content_type IS NOT NULL;

COMMENT ON COLUMN media_files.content_type IS 'Type of content: episode, movie, track, or audiobook';

-- ============================================================================
-- 2. Extend watch_progress for all content types
-- ============================================================================
-- Currently only supports episodes, now supports all content types

-- Make episode_id nullable (was NOT NULL)
ALTER TABLE watch_progress ALTER COLUMN episode_id DROP NOT NULL;

-- Add new content type columns
ALTER TABLE watch_progress ADD COLUMN IF NOT EXISTS movie_id UUID REFERENCES movies(id) ON DELETE CASCADE;
ALTER TABLE watch_progress ADD COLUMN IF NOT EXISTS track_id UUID REFERENCES tracks(id) ON DELETE CASCADE;
ALTER TABLE watch_progress ADD COLUMN IF NOT EXISTS audiobook_id UUID REFERENCES audiobooks(id) ON DELETE CASCADE;
ALTER TABLE watch_progress ADD COLUMN IF NOT EXISTS content_type TEXT NOT NULL DEFAULT 'episode';

-- Drop the old unique constraint and add a new one
ALTER TABLE watch_progress DROP CONSTRAINT IF EXISTS unique_user_episode_progress;

-- Add check constraint for content_type
ALTER TABLE watch_progress DROP CONSTRAINT IF EXISTS check_wp_content_type;
ALTER TABLE watch_progress ADD CONSTRAINT check_wp_content_type 
  CHECK (content_type IN ('episode', 'movie', 'track', 'audiobook'));

-- Ensure exactly one content ID is set based on content_type
ALTER TABLE watch_progress DROP CONSTRAINT IF EXISTS check_wp_content_id;
ALTER TABLE watch_progress ADD CONSTRAINT check_wp_content_id CHECK (
  (content_type = 'episode' AND episode_id IS NOT NULL AND movie_id IS NULL AND track_id IS NULL AND audiobook_id IS NULL) OR
  (content_type = 'movie' AND movie_id IS NOT NULL AND episode_id IS NULL AND track_id IS NULL AND audiobook_id IS NULL) OR
  (content_type = 'track' AND track_id IS NOT NULL AND episode_id IS NULL AND movie_id IS NULL AND audiobook_id IS NULL) OR
  (content_type = 'audiobook' AND audiobook_id IS NOT NULL AND episode_id IS NULL AND movie_id IS NULL AND track_id IS NULL)
);

-- Create a unique constraint that uses the appropriate ID based on type
-- We use a generated column or a partial unique index approach
CREATE UNIQUE INDEX IF NOT EXISTS idx_watch_progress_user_episode 
  ON watch_progress(user_id, episode_id) WHERE content_type = 'episode';
CREATE UNIQUE INDEX IF NOT EXISTS idx_watch_progress_user_movie 
  ON watch_progress(user_id, movie_id) WHERE content_type = 'movie';
CREATE UNIQUE INDEX IF NOT EXISTS idx_watch_progress_user_track 
  ON watch_progress(user_id, track_id) WHERE content_type = 'track';
CREATE UNIQUE INDEX IF NOT EXISTS idx_watch_progress_user_audiobook 
  ON watch_progress(user_id, audiobook_id) WHERE content_type = 'audiobook';

-- Additional indexes for lookups
CREATE INDEX IF NOT EXISTS idx_watch_progress_movie ON watch_progress(movie_id) WHERE movie_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_watch_progress_track ON watch_progress(track_id) WHERE track_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_watch_progress_audiobook ON watch_progress(audiobook_id) WHERE audiobook_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_watch_progress_content_type ON watch_progress(user_id, content_type);

COMMENT ON TABLE watch_progress IS 'Tracks user watch/listen progress for all content types';
COMMENT ON COLUMN watch_progress.content_type IS 'Type of content: episode, movie, track, or audiobook';

-- ============================================================================
-- 3. Extend playback_sessions for all content types
-- ============================================================================
-- Currently only supports TV shows, now supports all content types

-- Add new content columns
ALTER TABLE playback_sessions ADD COLUMN IF NOT EXISTS movie_id UUID REFERENCES movies(id) ON DELETE CASCADE;
ALTER TABLE playback_sessions ADD COLUMN IF NOT EXISTS track_id UUID REFERENCES tracks(id) ON DELETE CASCADE;
ALTER TABLE playback_sessions ADD COLUMN IF NOT EXISTS audiobook_id UUID REFERENCES audiobooks(id) ON DELETE CASCADE;
ALTER TABLE playback_sessions ADD COLUMN IF NOT EXISTS album_id UUID REFERENCES albums(id) ON DELETE CASCADE;
ALTER TABLE playback_sessions ADD COLUMN IF NOT EXISTS content_type TEXT;

-- Add check constraint for content_type
ALTER TABLE playback_sessions DROP CONSTRAINT IF EXISTS check_ps_content_type;
ALTER TABLE playback_sessions ADD CONSTRAINT check_ps_content_type 
  CHECK (content_type IS NULL OR content_type IN ('episode', 'movie', 'track', 'audiobook'));

-- Indexes for new columns
CREATE INDEX IF NOT EXISTS idx_playback_sessions_movie ON playback_sessions(movie_id) WHERE movie_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_playback_sessions_track ON playback_sessions(track_id) WHERE track_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_playback_sessions_audiobook ON playback_sessions(audiobook_id) WHERE audiobook_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_playback_sessions_content_type ON playback_sessions(content_type) WHERE content_type IS NOT NULL;

-- Update existing sessions to have content_type
UPDATE playback_sessions SET content_type = 'episode' WHERE episode_id IS NOT NULL AND content_type IS NULL;

COMMENT ON COLUMN playback_sessions.content_type IS 'Type of content being played: episode, movie, track, or audiobook';
COMMENT ON COLUMN playback_sessions.movie_id IS 'Movie being played (for movie content)';
COMMENT ON COLUMN playback_sessions.track_id IS 'Track being played (for music content)';
COMMENT ON COLUMN playback_sessions.audiobook_id IS 'Audiobook being played (for audiobook content)';
COMMENT ON COLUMN playback_sessions.album_id IS 'Album context for track playback';

-- ============================================================================
-- 4. Migrate audiobook_progress data to watch_progress (if any exists)
-- ============================================================================
-- This preserves any existing audiobook progress

INSERT INTO watch_progress (
  user_id, audiobook_id, content_type, media_file_id,
  current_position, duration, progress_percent, is_watched, last_watched_at
)
SELECT 
  ap.user_id,
  ap.audiobook_id,
  'audiobook',
  NULL, -- media_file_id
  ap.current_position_secs,
  a.duration_secs,
  CASE WHEN a.duration_secs > 0 THEN (ap.current_position_secs::float / a.duration_secs) ELSE 0 END,
  false,
  ap.last_played_at
FROM audiobook_progress ap
JOIN audiobooks a ON a.id = ap.audiobook_id
ON CONFLICT DO NOTHING;

-- Note: We keep audiobook_progress table for now but it's deprecated
-- It can be dropped in a future migration after confirming data integrity

COMMENT ON TABLE audiobook_progress IS 'DEPRECATED: Use watch_progress with content_type=audiobook instead';
