-- Migration 035: Source-agnostic pending file matches
-- Replaces torrent_file_matches with a generic table that works for any download source

-- 1. Create new pending_file_matches table (source-agnostic)
CREATE TABLE IF NOT EXISTS pending_file_matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    
    -- Source file info (works for any source: torrent, usenet, scan, manual)
    source_path TEXT NOT NULL,
    source_type VARCHAR(20) NOT NULL,  -- 'torrent', 'usenet', 'irc', 'scan', 'manual'
    source_id UUID,                     -- Optional: torrent_id, usenet_download_id, etc.
    source_file_index INTEGER,          -- For multi-file sources (e.g., torrent file index)
    file_size BIGINT NOT NULL,
    
    -- Match target (only one should be set per row)
    episode_id UUID REFERENCES episodes(id) ON DELETE CASCADE,
    movie_id UUID REFERENCES movies(id) ON DELETE CASCADE,
    track_id UUID REFERENCES tracks(id) ON DELETE CASCADE,
    chapter_id UUID REFERENCES chapters(id) ON DELETE CASCADE,
    
    -- Match metadata
    match_type VARCHAR(20) DEFAULT 'auto',  -- 'auto', 'manual'
    match_confidence DECIMAL(3,2),
    
    -- Parsed quality info (from filename)
    parsed_resolution VARCHAR(20),
    parsed_codec VARCHAR(50),
    parsed_source VARCHAR(50),
    parsed_audio VARCHAR(100),
    
    -- Processing status
    copied_at TIMESTAMPTZ,               -- null = not yet copied to library
    copy_error TEXT,                     -- error message if copy failed
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- Constraints: a file can match to multiple library items (different libraries)
    -- but the same source file shouldn't match the same item twice
    CONSTRAINT unique_source_episode UNIQUE (source_path, episode_id),
    CONSTRAINT unique_source_movie UNIQUE (source_path, movie_id),
    CONSTRAINT unique_source_track UNIQUE (source_path, track_id),
    CONSTRAINT unique_source_chapter UNIQUE (source_path, chapter_id),
    
    -- Ensure exactly one target is set
    CONSTRAINT one_target_set CHECK (
        (episode_id IS NOT NULL)::int +
        (movie_id IS NOT NULL)::int +
        (track_id IS NOT NULL)::int +
        (chapter_id IS NOT NULL)::int = 1
    )
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_pending_file_matches_source 
    ON pending_file_matches(source_type, source_id);
CREATE INDEX IF NOT EXISTS idx_pending_file_matches_user 
    ON pending_file_matches(user_id);
CREATE INDEX IF NOT EXISTS idx_pending_file_matches_uncopied 
    ON pending_file_matches(source_type, source_id) WHERE copied_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_pending_file_matches_episode 
    ON pending_file_matches(episode_id) WHERE episode_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_pending_file_matches_movie 
    ON pending_file_matches(movie_id) WHERE movie_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_pending_file_matches_track 
    ON pending_file_matches(track_id) WHERE track_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_pending_file_matches_chapter 
    ON pending_file_matches(chapter_id) WHERE chapter_id IS NOT NULL;

-- 2. Add active_download_id to library items (source-agnostic download tracking)
-- This links to pending_file_matches to show download progress for any source type

ALTER TABLE tracks 
    ADD COLUMN IF NOT EXISTS active_download_id UUID REFERENCES pending_file_matches(id) ON DELETE SET NULL;

ALTER TABLE episodes 
    ADD COLUMN IF NOT EXISTS active_download_id UUID REFERENCES pending_file_matches(id) ON DELETE SET NULL;

ALTER TABLE movies 
    ADD COLUMN IF NOT EXISTS active_download_id UUID REFERENCES pending_file_matches(id) ON DELETE SET NULL;

ALTER TABLE chapters 
    ADD COLUMN IF NOT EXISTS active_download_id UUID REFERENCES pending_file_matches(id) ON DELETE SET NULL;

-- Indexes for looking up items by their active download
CREATE INDEX IF NOT EXISTS idx_tracks_active_download ON tracks(active_download_id) WHERE active_download_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_episodes_active_download ON episodes(active_download_id) WHERE active_download_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_movies_active_download ON movies(active_download_id) WHERE active_download_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_chapters_active_download ON chapters(active_download_id) WHERE active_download_id IS NOT NULL;

-- 3. Drop the old torrent_file_matches table (replaced by pending_file_matches)
DROP TABLE IF EXISTS torrent_file_matches;

-- 4. Clean up existing data (fresh start as confirmed by user)
-- Truncate in dependency order
TRUNCATE media_files CASCADE;
TRUNCATE torrents CASCADE;

-- Reset item statuses to 'wanted' since we're starting fresh
UPDATE tracks SET status = 'wanted', media_file_id = NULL WHERE status IN ('downloading', 'downloaded');
UPDATE episodes SET status = 'wanted' WHERE status IN ('downloading', 'downloaded');
UPDATE movies SET download_status = 'wanted', has_file = false WHERE download_status IN ('downloading', 'downloaded');
UPDATE chapters SET status = 'wanted', media_file_id = NULL WHERE status IN ('downloading', 'downloaded');

-- 5. Create updated_at trigger for pending_file_matches
CREATE OR REPLACE FUNCTION update_pending_file_matches_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_pending_file_matches_updated_at ON pending_file_matches;
CREATE TRIGGER trigger_pending_file_matches_updated_at
    BEFORE UPDATE ON pending_file_matches
    FOR EACH ROW
    EXECUTE FUNCTION update_pending_file_matches_updated_at();
