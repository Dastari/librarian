-- TV Library Schema Migration
-- Adds support for TV shows, episodes, RSS feeds, and enhanced quality tracking

-- ============================================================================
-- Update Libraries Table
-- ============================================================================

-- Add new columns to libraries table
ALTER TABLE libraries 
ADD COLUMN IF NOT EXISTS scan_interval_minutes INTEGER NOT NULL DEFAULT 60,
ADD COLUMN IF NOT EXISTS watch_for_changes BOOLEAN NOT NULL DEFAULT false,
ADD COLUMN IF NOT EXISTS post_download_action VARCHAR(20) NOT NULL DEFAULT 'copy' CHECK (post_download_action IN ('copy', 'move', 'hardlink')),
ADD COLUMN IF NOT EXISTS auto_rename BOOLEAN NOT NULL DEFAULT true,
ADD COLUMN IF NOT EXISTS naming_pattern TEXT DEFAULT '{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}',
ADD COLUMN IF NOT EXISTS default_quality_profile_id UUID REFERENCES quality_profiles(id) ON DELETE SET NULL,
ADD COLUMN IF NOT EXISTS auto_add_discovered BOOLEAN NOT NULL DEFAULT false;

-- Drop old column if exists and add with new name
ALTER TABLE libraries DROP COLUMN IF EXISTS scan_interval_hours;

-- ============================================================================
-- Quality Profiles (Enhanced)
-- ============================================================================

-- Drop and recreate quality_profiles with new schema
DROP TABLE IF EXISTS quality_profiles CASCADE;

CREATE TABLE quality_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    -- Resolution preferences
    preferred_resolution VARCHAR(20) DEFAULT '1080p' CHECK (preferred_resolution IN ('2160p', '1080p', '720p', '480p', 'any')),
    min_resolution VARCHAR(20) DEFAULT '720p' CHECK (min_resolution IN ('2160p', '1080p', '720p', '480p', 'any')),
    -- Codec preferences
    preferred_codec VARCHAR(20) DEFAULT 'any' CHECK (preferred_codec IN ('hevc', 'h264', 'av1', 'any')),
    -- Audio preferences
    preferred_audio VARCHAR(20) DEFAULT 'any' CHECK (preferred_audio IN ('atmos', 'truehd', 'dts-hd', 'dts', 'ac3', 'aac', 'any')),
    -- HDR requirements
    require_hdr BOOLEAN NOT NULL DEFAULT false,
    hdr_types TEXT[] DEFAULT '{}', -- array of: hdr10, hdr10plus, dolbyvision, hlg
    -- Language
    preferred_language VARCHAR(10) DEFAULT 'en',
    -- Size limits
    max_size_gb DECIMAL(10, 2),
    min_seeders INTEGER DEFAULT 1,
    -- Release group preferences
    release_group_whitelist TEXT[] DEFAULT '{}',
    release_group_blacklist TEXT[] DEFAULT '{}',
    -- Upgrade behavior
    upgrade_until VARCHAR(20) DEFAULT '1080p' CHECK (upgrade_until IN ('2160p', '1080p', '720p', '480p', 'any')),
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_quality_profiles_user ON quality_profiles(user_id);

ALTER TABLE quality_profiles ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can manage own quality profiles"
    ON quality_profiles FOR ALL
    USING (auth.uid() = user_id);

-- Trigger for updated_at
CREATE TRIGGER set_updated_at_quality_profiles
    BEFORE UPDATE ON quality_profiles
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- TV Shows
-- ============================================================================

CREATE TABLE IF NOT EXISTS tv_shows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    -- Basic info
    name VARCHAR(500) NOT NULL,
    sort_name VARCHAR(500), -- for sorting (e.g., "Office, The")
    year INTEGER,
    status VARCHAR(50) DEFAULT 'unknown' CHECK (status IN ('continuing', 'ended', 'upcoming', 'cancelled', 'unknown')),
    -- External IDs
    tvmaze_id INTEGER,
    tmdb_id INTEGER,
    tvdb_id INTEGER,
    imdb_id VARCHAR(20),
    -- Metadata
    overview TEXT,
    network VARCHAR(255),
    runtime INTEGER, -- typical episode runtime in minutes
    genres TEXT[] DEFAULT '{}',
    -- Artwork URLs (cached from metadata providers)
    poster_url TEXT,
    backdrop_url TEXT,
    -- Monitoring settings
    monitored BOOLEAN NOT NULL DEFAULT true,
    monitor_type VARCHAR(20) NOT NULL DEFAULT 'all' CHECK (monitor_type IN ('all', 'future', 'none')),
    quality_profile_id UUID REFERENCES quality_profiles(id) ON DELETE SET NULL,
    -- Path within library (e.g., "Chicago Fire" or custom)
    path TEXT,
    -- Statistics (cached, updated on scan)
    episode_count INTEGER DEFAULT 0,
    episode_file_count INTEGER DEFAULT 0,
    size_bytes BIGINT DEFAULT 0,
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Constraints
    UNIQUE(library_id, tvmaze_id),
    UNIQUE(library_id, tmdb_id),
    UNIQUE(library_id, tvdb_id)
);

CREATE INDEX idx_tv_shows_library ON tv_shows(library_id);
CREATE INDEX idx_tv_shows_user ON tv_shows(user_id);
CREATE INDEX idx_tv_shows_tvmaze ON tv_shows(tvmaze_id) WHERE tvmaze_id IS NOT NULL;
CREATE INDEX idx_tv_shows_tmdb ON tv_shows(tmdb_id) WHERE tmdb_id IS NOT NULL;
CREATE INDEX idx_tv_shows_tvdb ON tv_shows(tvdb_id) WHERE tvdb_id IS NOT NULL;
CREATE INDEX idx_tv_shows_monitored ON tv_shows(library_id, monitored) WHERE monitored = true;

ALTER TABLE tv_shows ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view own shows"
    ON tv_shows FOR SELECT
    USING (auth.uid() = user_id);

CREATE POLICY "Users can create own shows"
    ON tv_shows FOR INSERT
    WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can update own shows"
    ON tv_shows FOR UPDATE
    USING (auth.uid() = user_id);

CREATE POLICY "Users can delete own shows"
    ON tv_shows FOR DELETE
    USING (auth.uid() = user_id);

CREATE TRIGGER set_updated_at_tv_shows
    BEFORE UPDATE ON tv_shows
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- Episodes
-- ============================================================================

CREATE TABLE IF NOT EXISTS episodes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tv_show_id UUID NOT NULL REFERENCES tv_shows(id) ON DELETE CASCADE,
    -- Episode identification
    season INTEGER NOT NULL,
    episode INTEGER NOT NULL,
    absolute_number INTEGER, -- for anime ordering
    -- Metadata
    title VARCHAR(500),
    overview TEXT,
    air_date DATE,
    runtime INTEGER, -- in minutes
    -- External IDs
    tvmaze_id INTEGER,
    tmdb_id INTEGER,
    tvdb_id INTEGER,
    -- Status tracking
    status VARCHAR(20) NOT NULL DEFAULT 'missing' CHECK (status IN ('missing', 'wanted', 'downloading', 'downloaded', 'ignored')),
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Constraints
    UNIQUE(tv_show_id, season, episode)
);

CREATE INDEX idx_episodes_show ON episodes(tv_show_id);
CREATE INDEX idx_episodes_show_season ON episodes(tv_show_id, season);
CREATE INDEX idx_episodes_status ON episodes(tv_show_id, status);
CREATE INDEX idx_episodes_air_date ON episodes(air_date) WHERE air_date IS NOT NULL;
CREATE INDEX idx_episodes_wanted ON episodes(status) WHERE status IN ('missing', 'wanted');

CREATE TRIGGER set_updated_at_episodes
    BEFORE UPDATE ON episodes
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- Media Files (Enhanced)
-- ============================================================================

-- Add new columns to media_files
ALTER TABLE media_files
ADD COLUMN IF NOT EXISTS episode_id UUID REFERENCES episodes(id) ON DELETE SET NULL,
ADD COLUMN IF NOT EXISTS relative_path TEXT,
ADD COLUMN IF NOT EXISTS original_name TEXT,
ADD COLUMN IF NOT EXISTS video_bitrate INTEGER, -- kbps
ADD COLUMN IF NOT EXISTS audio_channels VARCHAR(20), -- 2.0, 5.1, 7.1, atmos
ADD COLUMN IF NOT EXISTS audio_language VARCHAR(10),
ADD COLUMN IF NOT EXISTS resolution VARCHAR(20), -- 2160p, 1080p, 720p, 480p
ADD COLUMN IF NOT EXISTS is_hdr BOOLEAN DEFAULT false,
ADD COLUMN IF NOT EXISTS hdr_type VARCHAR(20); -- hdr10, hdr10plus, dolbyvision, hlg

CREATE INDEX IF NOT EXISTS idx_media_files_episode ON media_files(episode_id) WHERE episode_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_media_files_resolution ON media_files(resolution);

-- ============================================================================
-- RSS Feeds
-- ============================================================================

CREATE TABLE IF NOT EXISTS rss_feeds (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    library_id UUID REFERENCES libraries(id) ON DELETE CASCADE, -- NULL = global feed
    -- Feed info
    name VARCHAR(255) NOT NULL,
    url TEXT NOT NULL,
    -- Settings
    enabled BOOLEAN NOT NULL DEFAULT true,
    poll_interval_minutes INTEGER NOT NULL DEFAULT 15,
    -- Tracking
    last_polled_at TIMESTAMPTZ,
    last_successful_at TIMESTAMPTZ,
    last_error TEXT,
    consecutive_failures INTEGER DEFAULT 0,
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rss_feeds_user ON rss_feeds(user_id);
CREATE INDEX idx_rss_feeds_library ON rss_feeds(library_id) WHERE library_id IS NOT NULL;
CREATE INDEX idx_rss_feeds_enabled ON rss_feeds(enabled, last_polled_at) WHERE enabled = true;

ALTER TABLE rss_feeds ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can manage own RSS feeds"
    ON rss_feeds FOR ALL
    USING (auth.uid() = user_id);

CREATE TRIGGER set_updated_at_rss_feeds
    BEFORE UPDATE ON rss_feeds
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- RSS Feed History (for deduplication)
-- ============================================================================

CREATE TABLE IF NOT EXISTS rss_feed_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    feed_id UUID NOT NULL REFERENCES rss_feeds(id) ON DELETE CASCADE,
    -- Item identification (for deduplication)
    guid TEXT, -- RSS guid if provided
    link_hash VARCHAR(64) NOT NULL, -- SHA256 of link URL
    title_hash VARCHAR(64) NOT NULL, -- SHA256 of title
    -- Parsed data
    title TEXT NOT NULL,
    link TEXT NOT NULL,
    pub_date TIMESTAMPTZ,
    description TEXT,
    -- Parsed metadata
    parsed_show_name VARCHAR(500),
    parsed_season INTEGER,
    parsed_episode INTEGER,
    parsed_resolution VARCHAR(20),
    parsed_codec VARCHAR(20),
    parsed_source VARCHAR(50),
    -- Processing
    processed BOOLEAN NOT NULL DEFAULT false,
    matched_episode_id UUID REFERENCES episodes(id) ON DELETE SET NULL,
    torrent_id UUID REFERENCES torrents(id) ON DELETE SET NULL,
    skipped_reason TEXT, -- why we didn't download (quality, already have, etc.)
    -- Metadata
    seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(feed_id, link_hash)
);

CREATE INDEX idx_rss_items_feed ON rss_feed_items(feed_id);
CREATE INDEX idx_rss_items_processed ON rss_feed_items(feed_id, processed) WHERE NOT processed;
CREATE INDEX idx_rss_items_seen ON rss_feed_items(seen_at);

-- ============================================================================
-- Torrents (Enhanced) - Note: table was renamed from downloads to torrents in migration 002
-- ============================================================================

ALTER TABLE torrents
ADD COLUMN IF NOT EXISTS library_id UUID REFERENCES libraries(id) ON DELETE SET NULL,
ADD COLUMN IF NOT EXISTS episode_id UUID REFERENCES episodes(id) ON DELETE SET NULL,
ADD COLUMN IF NOT EXISTS download_path TEXT,
ADD COLUMN IF NOT EXISTS source_url TEXT,
ADD COLUMN IF NOT EXISTS source_feed_id UUID REFERENCES rss_feeds(id) ON DELETE SET NULL,
ADD COLUMN IF NOT EXISTS post_process_status VARCHAR(20) DEFAULT 'pending' CHECK (post_process_status IN ('pending', 'processing', 'completed', 'failed', 'skipped')),
ADD COLUMN IF NOT EXISTS post_process_error TEXT,
ADD COLUMN IF NOT EXISTS processed_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_torrents_library ON torrents(library_id) WHERE library_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_torrents_episode ON torrents(episode_id) WHERE episode_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_torrents_post_process ON torrents(post_process_status) WHERE post_process_status = 'pending';

-- ============================================================================
-- Unmatched Files
-- ============================================================================

CREATE TABLE IF NOT EXISTS unmatched_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    -- File info
    path TEXT NOT NULL UNIQUE,
    filename TEXT NOT NULL,
    size_bytes BIGINT,
    -- Our best guess from parsing
    parsed_show_name VARCHAR(500),
    parsed_season INTEGER,
    parsed_episode INTEGER,
    parsed_year INTEGER,
    -- Suggested match (from pattern matching or AI)
    suggested_show_id UUID REFERENCES tv_shows(id) ON DELETE SET NULL,
    suggested_season INTEGER,
    suggested_episode INTEGER,
    confidence DECIMAL(3, 2) CHECK (confidence >= 0 AND confidence <= 1),
    match_source VARCHAR(20) DEFAULT 'pattern' CHECK (match_source IN ('pattern', 'ai', 'manual')),
    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'matched', 'ignored')),
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at TIMESTAMPTZ
);

CREATE INDEX idx_unmatched_library ON unmatched_files(library_id);
CREATE INDEX idx_unmatched_status ON unmatched_files(library_id, status) WHERE status = 'pending';

-- ============================================================================
-- Jobs (Enhanced)
-- ============================================================================

ALTER TABLE jobs
ADD COLUMN IF NOT EXISTS library_id UUID REFERENCES libraries(id) ON DELETE CASCADE,
ADD COLUMN IF NOT EXISTS tv_show_id UUID REFERENCES tv_shows(id) ON DELETE CASCADE,
ADD COLUMN IF NOT EXISTS scheduled_at TIMESTAMPTZ DEFAULT NOW(),
ADD COLUMN IF NOT EXISTS recurring_cron VARCHAR(100);

-- Rename run_at to scheduled_at if it exists
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'jobs' AND column_name = 'run_at') THEN
        ALTER TABLE jobs RENAME COLUMN run_at TO scheduled_at_old;
        UPDATE jobs SET scheduled_at = scheduled_at_old WHERE scheduled_at IS NULL;
        ALTER TABLE jobs DROP COLUMN scheduled_at_old;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_jobs_library ON jobs(library_id) WHERE library_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_jobs_scheduled ON jobs(state, scheduled_at) WHERE state = 'pending';

-- ============================================================================
-- Default Quality Profiles
-- ============================================================================

-- Insert default quality profiles (will be associated with users on first use)
-- These are templates that will be copied for each user

-- Note: We can't insert with auth.uid() in a migration, so we'll handle this
-- in the application code when a user first accesses quality profiles.

-- ============================================================================
-- Comments for documentation
-- ============================================================================

COMMENT ON TABLE tv_shows IS 'TV shows tracked in a library with monitoring settings';
COMMENT ON TABLE episodes IS 'Individual episodes with air dates and download status';
COMMENT ON TABLE rss_feeds IS 'RSS feed URLs for polling torrent releases';
COMMENT ON TABLE rss_feed_items IS 'Parsed RSS items with deduplication and match tracking';
COMMENT ON TABLE unmatched_files IS 'Media files that could not be automatically matched to episodes';
COMMENT ON COLUMN tv_shows.monitor_type IS 'all = download all episodes, future = only new episodes, none = track but do not download';
COMMENT ON COLUMN episodes.status IS 'missing = not aired yet or no file, wanted = should download, downloading = in progress, downloaded = have file, ignored = skip';
COMMENT ON COLUMN quality_profiles.upgrade_until IS 'Stop trying to upgrade quality once this resolution is reached';
