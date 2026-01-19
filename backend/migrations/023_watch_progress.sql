-- ============================================================================
-- Watch Progress Tracking
-- ============================================================================
-- Stores per-user, per-episode watch progress for resume functionality
-- and tracking watched episodes.

CREATE TABLE IF NOT EXISTS watch_progress (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    episode_id UUID NOT NULL REFERENCES episodes(id) ON DELETE CASCADE,
    media_file_id UUID REFERENCES media_files(id) ON DELETE SET NULL,
    
    -- Progress tracking
    current_position DOUBLE PRECISION NOT NULL DEFAULT 0,
    duration DOUBLE PRECISION,
    progress_percent REAL NOT NULL DEFAULT 0,
    
    -- Watched status (true when >= 90% watched or manually marked)
    is_watched BOOLEAN NOT NULL DEFAULT false,
    watched_at TIMESTAMPTZ,
    
    -- Timestamps
    last_watched_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- One progress record per user per episode
    CONSTRAINT unique_user_episode_progress UNIQUE (user_id, episode_id)
);

-- Indexes for common queries
CREATE INDEX idx_watch_progress_user ON watch_progress(user_id);
CREATE INDEX idx_watch_progress_episode ON watch_progress(episode_id);
CREATE INDEX idx_watch_progress_user_episode ON watch_progress(user_id, episode_id);

-- Auto-update updated_at timestamp
CREATE TRIGGER watch_progress_updated_at
    BEFORE UPDATE ON watch_progress
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Comments
COMMENT ON TABLE watch_progress IS 'Tracks user watch progress for episodes (resume and watched status)';
COMMENT ON COLUMN watch_progress.progress_percent IS 'Watch progress as percentage (0.0 to 1.0)';
COMMENT ON COLUMN watch_progress.is_watched IS 'True when episode is considered watched (>=90% or manual)';
COMMENT ON COLUMN watch_progress.watched_at IS 'When the episode was marked as watched';
COMMENT ON COLUMN watch_progress.last_watched_at IS 'Last time this episode was actively watched';

-- ============================================================================
-- Default Playback Settings
-- ============================================================================
-- Insert default playback sync interval setting

INSERT INTO app_settings (key, value, category, description)
VALUES (
    'playback_sync_interval',
    '15',
    'playback',
    'How often to sync watch progress to the database (in seconds)'
)
ON CONFLICT (key) DO NOTHING;
