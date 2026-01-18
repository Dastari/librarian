-- Librarian Database Schema
-- Consolidated schema - all tables in final state
-- Run with: sqlx migrate run

-- ============================================================================
-- Utility Functions
-- ============================================================================

-- Updated at trigger function
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Alias for compatibility with some migrations
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Application Settings
-- ============================================================================

CREATE TABLE IF NOT EXISTS app_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key VARCHAR(255) NOT NULL UNIQUE,
    value JSONB NOT NULL,
    description TEXT,
    category VARCHAR(100) NOT NULL DEFAULT 'general',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_app_settings_category ON app_settings(category);
CREATE INDEX idx_app_settings_key ON app_settings(key);

CREATE TRIGGER set_updated_at_app_settings
    BEFORE UPDATE ON app_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Default torrent settings
INSERT INTO app_settings (key, value, description, category) VALUES
    ('torrent.download_dir', '"/data/downloads"', 'Directory where torrents are downloaded to', 'torrent'),
    ('torrent.session_dir', '"/data/session"', 'Directory for torrent session data (resume info, DHT)', 'torrent'),
    ('torrent.enable_dht', 'true', 'Enable DHT for peer discovery', 'torrent'),
    ('torrent.listen_port', '6881', 'Port to listen for incoming torrent connections (0 = random)', 'torrent'),
    ('torrent.max_concurrent', '5', 'Maximum number of concurrent downloads', 'torrent'),
    ('torrent.upload_limit', '0', 'Upload speed limit in bytes/sec (0 = unlimited)', 'torrent'),
    ('torrent.download_limit', '0', 'Download speed limit in bytes/sec (0 = unlimited)', 'torrent')
ON CONFLICT (key) DO NOTHING;

-- ============================================================================
-- Application Logs
-- ============================================================================

CREATE TABLE IF NOT EXISTS app_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    level TEXT NOT NULL CHECK (level IN ('TRACE', 'DEBUG', 'INFO', 'WARN', 'ERROR')),
    target TEXT NOT NULL,
    message TEXT NOT NULL,
    fields JSONB,
    span_name TEXT,
    span_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_app_logs_timestamp ON app_logs(timestamp DESC);
CREATE INDEX idx_app_logs_level ON app_logs(level);
CREATE INDEX idx_app_logs_target ON app_logs(target);
CREATE INDEX idx_app_logs_created_at ON app_logs(created_at DESC);
CREATE INDEX idx_app_logs_fields ON app_logs USING GIN (fields);
CREATE INDEX idx_app_logs_message_search ON app_logs USING GIN (to_tsvector('english', message));
CREATE INDEX idx_app_logs_level_timestamp ON app_logs(level, timestamp DESC);
CREATE INDEX idx_app_logs_target_timestamp ON app_logs(target, timestamp DESC);

COMMENT ON TABLE app_logs IS 'Stores backend tracing logs for debugging and monitoring';

-- ============================================================================
-- Quality Profiles (deprecated but kept for compatibility)
-- ============================================================================

CREATE TABLE IF NOT EXISTS quality_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    preferred_resolution VARCHAR(20) DEFAULT '1080p' CHECK (preferred_resolution IN ('2160p', '1080p', '720p', '480p', 'any')),
    min_resolution VARCHAR(20) DEFAULT '720p' CHECK (min_resolution IN ('2160p', '1080p', '720p', '480p', 'any')),
    preferred_codec VARCHAR(20) DEFAULT 'any' CHECK (preferred_codec IN ('hevc', 'h264', 'av1', 'any')),
    preferred_audio VARCHAR(20) DEFAULT 'any' CHECK (preferred_audio IN ('atmos', 'truehd', 'dts-hd', 'dts', 'ac3', 'aac', 'any')),
    require_hdr BOOLEAN NOT NULL DEFAULT false,
    hdr_types TEXT[] DEFAULT '{}',
    preferred_language VARCHAR(10) DEFAULT 'en',
    max_size_gb DECIMAL(10, 2),
    min_seeders INTEGER DEFAULT 1,
    release_group_whitelist TEXT[] DEFAULT '{}',
    release_group_blacklist TEXT[] DEFAULT '{}',
    upgrade_until VARCHAR(20) DEFAULT '1080p' CHECK (upgrade_until IN ('2160p', '1080p', '720p', '480p', 'any')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_quality_profiles_user ON quality_profiles(user_id);

CREATE TRIGGER set_updated_at_quality_profiles
    BEFORE UPDATE ON quality_profiles
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE quality_profiles IS 'DEPRECATED: Use inline quality settings on libraries and tv_shows instead.';

-- ============================================================================
-- Libraries
-- ============================================================================

CREATE TABLE IF NOT EXISTS libraries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    path TEXT NOT NULL,
    library_type VARCHAR(50) NOT NULL CHECK (library_type IN ('movies', 'tv', 'music', 'audiobooks', 'other')),
    -- Display settings
    icon VARCHAR(50) DEFAULT 'folder',
    color VARCHAR(20) DEFAULT 'blue',
    -- Scan settings
    auto_scan BOOLEAN NOT NULL DEFAULT true,
    scan_interval_minutes INTEGER NOT NULL DEFAULT 60,
    watch_for_changes BOOLEAN NOT NULL DEFAULT false,
    last_scanned_at TIMESTAMPTZ,
    scanning BOOLEAN NOT NULL DEFAULT FALSE,
    -- Post-download behavior
    post_download_action VARCHAR(20) NOT NULL DEFAULT 'copy' CHECK (post_download_action IN ('copy', 'move', 'hardlink')),
    organize_files BOOLEAN NOT NULL DEFAULT true,
    rename_style VARCHAR(20) NOT NULL DEFAULT 'none' CHECK (rename_style IN ('none', 'clean', 'preserve_info')),
    naming_pattern TEXT DEFAULT '{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}',
    -- Automation settings
    auto_download BOOLEAN NOT NULL DEFAULT true,
    auto_hunt BOOLEAN NOT NULL DEFAULT false,
    auto_add_discovered BOOLEAN NOT NULL DEFAULT false,
    -- Inline quality settings (empty = any)
    allowed_resolutions TEXT[] NOT NULL DEFAULT '{}',
    allowed_video_codecs TEXT[] NOT NULL DEFAULT '{}',
    allowed_audio_formats TEXT[] NOT NULL DEFAULT '{}',
    require_hdr BOOLEAN NOT NULL DEFAULT false,
    allowed_hdr_types TEXT[] NOT NULL DEFAULT '{}',
    allowed_sources TEXT[] NOT NULL DEFAULT '{}',
    release_group_blacklist TEXT[] NOT NULL DEFAULT '{}',
    release_group_whitelist TEXT[] NOT NULL DEFAULT '{}',
    -- Subtitle settings
    auto_download_subtitles BOOLEAN DEFAULT false,
    preferred_subtitle_languages TEXT[] DEFAULT '{}',
    -- Deprecated: quality profile reference
    default_quality_profile_id UUID,
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_libraries_user ON libraries(user_id);
CREATE INDEX idx_libraries_scanning ON libraries(scanning) WHERE scanning = TRUE;

CREATE TRIGGER set_updated_at_libraries
    BEFORE UPDATE ON libraries
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON COLUMN libraries.default_quality_profile_id IS 'DEPRECATED: Use inline quality settings instead.';

-- ============================================================================
-- TV Shows
-- ============================================================================

CREATE TABLE IF NOT EXISTS tv_shows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    -- Basic info
    name VARCHAR(500) NOT NULL,
    sort_name VARCHAR(500),
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
    runtime INTEGER,
    genres TEXT[] DEFAULT '{}',
    -- Artwork URLs
    poster_url TEXT,
    backdrop_url TEXT,
    -- Monitoring settings
    monitored BOOLEAN NOT NULL DEFAULT true,
    monitor_type VARCHAR(20) NOT NULL DEFAULT 'all' CHECK (monitor_type IN ('all', 'future', 'none')),
    -- Deprecated quality profile reference
    quality_profile_id UUID,
    -- Path within library
    path TEXT,
    -- Statistics
    episode_count INTEGER DEFAULT 0,
    episode_file_count INTEGER DEFAULT 0,
    size_bytes BIGINT DEFAULT 0,
    -- Override settings (NULL = inherit from library)
    auto_download_override BOOLEAN DEFAULT NULL,
    backfill_existing BOOLEAN NOT NULL DEFAULT true,
    organize_files_override BOOLEAN DEFAULT NULL,
    rename_style_override VARCHAR(20) DEFAULT NULL CHECK (rename_style_override IS NULL OR rename_style_override IN ('none', 'clean', 'preserve_info')),
    auto_hunt_override BOOLEAN DEFAULT NULL,
    -- Inline quality overrides (NULL = inherit)
    allowed_resolutions_override TEXT[] DEFAULT NULL,
    allowed_video_codecs_override TEXT[] DEFAULT NULL,
    allowed_audio_formats_override TEXT[] DEFAULT NULL,
    require_hdr_override BOOLEAN DEFAULT NULL,
    allowed_hdr_types_override TEXT[] DEFAULT NULL,
    allowed_sources_override TEXT[] DEFAULT NULL,
    release_group_blacklist_override TEXT[] DEFAULT NULL,
    release_group_whitelist_override TEXT[] DEFAULT NULL,
    -- Subtitle settings override
    subtitle_settings_override JSONB,
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
CREATE INDEX idx_tv_shows_auto_download ON tv_shows(library_id, auto_download_override) WHERE monitored = true;

CREATE TRIGGER set_updated_at_tv_shows
    BEFORE UPDATE ON tv_shows
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE tv_shows IS 'TV shows tracked in a library with monitoring settings';
COMMENT ON COLUMN tv_shows.quality_profile_id IS 'DEPRECATED: Use inline quality settings instead.';
COMMENT ON COLUMN tv_shows.monitor_type IS 'all = download all episodes, future = only new episodes, none = track but do not download';

-- ============================================================================
-- Episodes
-- ============================================================================

CREATE TABLE IF NOT EXISTS episodes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tv_show_id UUID NOT NULL REFERENCES tv_shows(id) ON DELETE CASCADE,
    -- Episode identification
    season INTEGER NOT NULL,
    episode INTEGER NOT NULL,
    absolute_number INTEGER,
    -- Metadata
    title VARCHAR(500),
    overview TEXT,
    air_date DATE,
    runtime INTEGER,
    -- External IDs
    tvmaze_id INTEGER,
    tmdb_id INTEGER,
    tvdb_id INTEGER,
    -- Status tracking
    status VARCHAR(20) NOT NULL DEFAULT 'missing' CHECK (status IN ('missing', 'wanted', 'available', 'downloading', 'downloaded', 'ignored')),
    -- Torrent link for available episodes
    torrent_link TEXT,
    torrent_link_added_at TIMESTAMPTZ,
    matched_rss_item_id UUID,
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
CREATE INDEX idx_episodes_available ON episodes(status) WHERE status = 'available';

CREATE TRIGGER set_updated_at_episodes
    BEFORE UPDATE ON episodes
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE episodes IS 'Individual episodes with air dates and download status';
COMMENT ON COLUMN episodes.status IS 'missing = not aired yet or no file, wanted = should download, available = found in RSS, downloading = in progress, downloaded = have file, ignored = skip';
COMMENT ON COLUMN episodes.torrent_link IS 'URL/magnet link to download this episode from RSS feed match';

-- ============================================================================
-- RSS Feeds
-- ============================================================================

CREATE TABLE IF NOT EXISTS rss_feeds (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    library_id UUID REFERENCES libraries(id) ON DELETE CASCADE,
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

CREATE TRIGGER set_updated_at_rss_feeds
    BEFORE UPDATE ON rss_feeds
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE rss_feeds IS 'RSS feed URLs for polling torrent releases';

-- ============================================================================
-- Torrents
-- ============================================================================

CREATE TABLE IF NOT EXISTS torrents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    -- Torrent identification
    info_hash VARCHAR(40) NOT NULL,
    magnet_uri TEXT,
    -- Display info
    name VARCHAR(500) NOT NULL,
    -- Status tracking
    state VARCHAR(50) NOT NULL DEFAULT 'queued',
    progress REAL NOT NULL DEFAULT 0,
    -- Size info
    total_bytes BIGINT NOT NULL DEFAULT 0,
    downloaded_bytes BIGINT NOT NULL DEFAULT 0,
    uploaded_bytes BIGINT NOT NULL DEFAULT 0,
    -- Path info
    save_path TEXT NOT NULL,
    download_path TEXT,
    source_url TEXT,
    -- Library/episode links
    library_id UUID REFERENCES libraries(id) ON DELETE SET NULL,
    episode_id UUID REFERENCES episodes(id) ON DELETE SET NULL,
    source_feed_id UUID REFERENCES rss_feeds(id) ON DELETE SET NULL,
    -- Post-processing
    post_process_status VARCHAR(20) DEFAULT 'pending' CHECK (post_process_status IN ('pending', 'processing', 'completed', 'failed', 'skipped')),
    post_process_error TEXT,
    processed_at TIMESTAMPTZ,
    -- Timestamps
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    -- Unique constraint
    CONSTRAINT torrents_user_infohash_unique UNIQUE (user_id, info_hash)
);

CREATE INDEX idx_torrents_user ON torrents(user_id);
CREATE INDEX idx_torrents_state ON torrents(state);
CREATE INDEX idx_torrents_info_hash ON torrents(info_hash);
CREATE INDEX idx_torrents_library ON torrents(library_id) WHERE library_id IS NOT NULL;
CREATE INDEX idx_torrents_episode ON torrents(episode_id) WHERE episode_id IS NOT NULL;
CREATE INDEX idx_torrents_post_process ON torrents(post_process_status) WHERE post_process_status = 'pending';

-- ============================================================================
-- RSS Feed Items
-- ============================================================================

CREATE TABLE IF NOT EXISTS rss_feed_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    feed_id UUID NOT NULL REFERENCES rss_feeds(id) ON DELETE CASCADE,
    -- Item identification
    guid TEXT,
    link_hash VARCHAR(64) NOT NULL,
    title_hash VARCHAR(64) NOT NULL,
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
    parsed_audio VARCHAR(50),
    parsed_hdr VARCHAR(50),
    -- Processing
    processed BOOLEAN NOT NULL DEFAULT false,
    matched_episode_id UUID REFERENCES episodes(id) ON DELETE SET NULL,
    torrent_id UUID REFERENCES torrents(id) ON DELETE SET NULL,
    skipped_reason TEXT,
    -- Metadata
    seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(feed_id, link_hash)
);

CREATE INDEX idx_rss_items_feed ON rss_feed_items(feed_id);
CREATE INDEX idx_rss_items_processed ON rss_feed_items(feed_id, processed) WHERE NOT processed;
CREATE INDEX idx_rss_items_seen ON rss_feed_items(seen_at);

COMMENT ON TABLE rss_feed_items IS 'Parsed RSS items with deduplication and match tracking';

-- Add FK from episodes to rss_feed_items (deferred to avoid circular dependency)
ALTER TABLE episodes 
    ADD CONSTRAINT fk_episodes_matched_rss_item 
    FOREIGN KEY (matched_rss_item_id) REFERENCES rss_feed_items(id) ON DELETE SET NULL;

-- ============================================================================
-- Media Files
-- ============================================================================

CREATE TABLE IF NOT EXISTS media_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    episode_id UUID REFERENCES episodes(id) ON DELETE SET NULL,
    -- File info
    path TEXT NOT NULL UNIQUE,
    relative_path TEXT,
    original_name TEXT,
    size BIGINT NOT NULL,
    -- Basic media properties
    container VARCHAR(50),
    video_codec VARCHAR(50),
    audio_codec VARCHAR(50),
    width INTEGER,
    height INTEGER,
    duration INTEGER,
    bitrate INTEGER,
    -- Enhanced video properties
    resolution VARCHAR(20),
    video_bitrate INTEGER,
    is_hdr BOOLEAN DEFAULT false,
    hdr_type VARCHAR(20),
    -- Enhanced audio properties
    audio_channels VARCHAR(20),
    audio_language VARCHAR(10),
    audio_bitrate INTEGER,
    sample_rate INTEGER,
    -- Detailed analysis from FFmpeg
    container_format TEXT,
    frame_rate TEXT,
    avg_frame_rate TEXT,
    pixel_format TEXT,
    color_space TEXT,
    color_transfer TEXT,
    color_primaries TEXT,
    bit_depth INTEGER,
    aspect_ratio TEXT,
    chapter_count INTEGER DEFAULT 0,
    analyzed_at TIMESTAMPTZ,
    analysis_data JSONB,
    -- File tracking
    file_hash VARCHAR(64),
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    modified_at TIMESTAMPTZ,
    -- Organization tracking
    organized BOOLEAN NOT NULL DEFAULT false,
    organized_at TIMESTAMPTZ,
    original_path TEXT
);

CREATE INDEX idx_media_files_library ON media_files(library_id);
CREATE INDEX idx_media_files_episode ON media_files(episode_id) WHERE episode_id IS NOT NULL;
CREATE INDEX idx_media_files_resolution ON media_files(resolution);
CREATE INDEX idx_media_files_unorganized ON media_files(library_id, organized) WHERE organized = false;
CREATE INDEX idx_media_files_analyzed ON media_files(analyzed_at) WHERE analyzed_at IS NOT NULL;

COMMENT ON COLUMN media_files.analysis_data IS 'Complete FFmpeg analysis JSON for reference';

-- ============================================================================
-- Video Streams
-- ============================================================================

CREATE TABLE IF NOT EXISTS video_streams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_file_id UUID NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    stream_index INTEGER NOT NULL,
    codec TEXT NOT NULL,
    codec_long_name TEXT,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    aspect_ratio TEXT,
    frame_rate TEXT,
    avg_frame_rate TEXT,
    bitrate BIGINT,
    pixel_format TEXT,
    color_space TEXT,
    color_transfer TEXT,
    color_primaries TEXT,
    hdr_type TEXT,
    bit_depth INTEGER,
    language TEXT,
    title TEXT,
    is_default BOOLEAN DEFAULT false,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(media_file_id, stream_index)
);

CREATE INDEX idx_video_streams_media_file ON video_streams(media_file_id);

COMMENT ON TABLE video_streams IS 'Video streams extracted from media files via FFmpeg analysis';

-- ============================================================================
-- Audio Streams
-- ============================================================================

CREATE TABLE IF NOT EXISTS audio_streams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_file_id UUID NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    stream_index INTEGER NOT NULL,
    codec TEXT NOT NULL,
    codec_long_name TEXT,
    channels INTEGER NOT NULL,
    channel_layout TEXT,
    sample_rate INTEGER,
    bitrate BIGINT,
    bit_depth INTEGER,
    language TEXT,
    title TEXT,
    is_default BOOLEAN DEFAULT false,
    is_commentary BOOLEAN DEFAULT false,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(media_file_id, stream_index)
);

CREATE INDEX idx_audio_streams_media_file ON audio_streams(media_file_id);

COMMENT ON TABLE audio_streams IS 'Audio streams extracted from media files via FFmpeg analysis';

-- ============================================================================
-- Subtitles
-- ============================================================================

CREATE TABLE IF NOT EXISTS subtitles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_file_id UUID NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    -- Source type
    source_type TEXT NOT NULL CHECK (source_type IN ('embedded', 'external', 'downloaded')),
    -- Location
    stream_index INTEGER,
    file_path TEXT,
    -- Metadata
    codec TEXT,
    codec_long_name TEXT,
    language TEXT,
    title TEXT,
    is_default BOOLEAN DEFAULT false,
    is_forced BOOLEAN DEFAULT false,
    is_hearing_impaired BOOLEAN DEFAULT false,
    -- Download info
    opensubtitles_id TEXT,
    downloaded_at TIMESTAMPTZ,
    -- Stream metadata
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    -- Constraint
    CONSTRAINT subtitle_source_check CHECK (
        (source_type = 'embedded' AND stream_index IS NOT NULL) OR
        (source_type IN ('external', 'downloaded') AND file_path IS NOT NULL)
    )
);

CREATE INDEX idx_subtitles_media_file ON subtitles(media_file_id);
CREATE INDEX idx_subtitles_language ON subtitles(language);
CREATE INDEX idx_subtitles_source_type ON subtitles(source_type);

COMMENT ON TABLE subtitles IS 'Subtitle tracks - embedded in media files, external files, or downloaded';

-- ============================================================================
-- Chapters
-- ============================================================================

CREATE TABLE IF NOT EXISTS chapters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_file_id UUID NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    chapter_index INTEGER NOT NULL,
    start_secs DOUBLE PRECISION NOT NULL,
    end_secs DOUBLE PRECISION NOT NULL,
    title TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(media_file_id, chapter_index)
);

CREATE INDEX idx_chapters_media_file ON chapters(media_file_id);

COMMENT ON TABLE chapters IS 'Chapter markers extracted from media files';

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
    -- Parsed guess
    parsed_show_name VARCHAR(500),
    parsed_season INTEGER,
    parsed_episode INTEGER,
    parsed_year INTEGER,
    -- Suggested match
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

COMMENT ON TABLE unmatched_files IS 'Media files that could not be automatically matched to episodes';

-- ============================================================================
-- Cast Devices
-- ============================================================================

CREATE TABLE IF NOT EXISTS cast_devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    address TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 8009,
    model TEXT,
    device_type TEXT NOT NULL DEFAULT 'chromecast',
    is_favorite BOOLEAN NOT NULL DEFAULT false,
    is_manual BOOLEAN NOT NULL DEFAULT false,
    last_seen_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_cast_device_address UNIQUE (address)
);

CREATE INDEX idx_cast_devices_address ON cast_devices(address);

CREATE OR REPLACE FUNCTION update_cast_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER cast_devices_updated_at
    BEFORE UPDATE ON cast_devices
    FOR EACH ROW
    EXECUTE FUNCTION update_cast_updated_at();

-- ============================================================================
-- Cast Sessions
-- ============================================================================

CREATE TABLE IF NOT EXISTS cast_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id UUID REFERENCES cast_devices(id) ON DELETE SET NULL,
    media_file_id UUID REFERENCES media_files(id) ON DELETE SET NULL,
    episode_id UUID REFERENCES episodes(id) ON DELETE SET NULL,
    stream_url TEXT NOT NULL,
    player_state TEXT NOT NULL DEFAULT 'idle',
    current_position DOUBLE PRECISION NOT NULL DEFAULT 0,
    duration DOUBLE PRECISION,
    volume REAL NOT NULL DEFAULT 1.0,
    is_muted BOOLEAN NOT NULL DEFAULT false,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ,
    last_position DOUBLE PRECISION,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_cast_sessions_device_active ON cast_sessions(device_id) WHERE ended_at IS NULL;

CREATE TRIGGER cast_sessions_updated_at
    BEFORE UPDATE ON cast_sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_cast_updated_at();

-- ============================================================================
-- Cast Settings
-- ============================================================================

CREATE TABLE IF NOT EXISTS cast_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    auto_discovery_enabled BOOLEAN NOT NULL DEFAULT true,
    discovery_interval_seconds INTEGER NOT NULL DEFAULT 30,
    default_volume REAL NOT NULL DEFAULT 1.0,
    transcode_incompatible BOOLEAN NOT NULL DEFAULT true,
    preferred_quality TEXT DEFAULT '1080p',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO cast_settings (id, auto_discovery_enabled, discovery_interval_seconds, default_volume, transcode_incompatible, preferred_quality)
VALUES (gen_random_uuid(), true, 30, 1.0, true, '1080p')
ON CONFLICT DO NOTHING;

CREATE TRIGGER cast_settings_updated_at
    BEFORE UPDATE ON cast_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_cast_updated_at();

-- ============================================================================
-- Playback Sessions
-- ============================================================================

CREATE TABLE IF NOT EXISTS playback_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    episode_id UUID REFERENCES episodes(id) ON DELETE CASCADE,
    media_file_id UUID REFERENCES media_files(id) ON DELETE CASCADE,
    tv_show_id UUID REFERENCES tv_shows(id) ON DELETE CASCADE,
    -- Playback state
    current_position DOUBLE PRECISION NOT NULL DEFAULT 0,
    duration DOUBLE PRECISION,
    volume REAL NOT NULL DEFAULT 1.0,
    is_muted BOOLEAN NOT NULL DEFAULT false,
    is_playing BOOLEAN NOT NULL DEFAULT false,
    -- Timestamps
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- One active session per user
    CONSTRAINT unique_user_active_session UNIQUE (user_id)
);

CREATE INDEX idx_playback_sessions_user ON playback_sessions(user_id);
CREATE INDEX idx_playback_sessions_incomplete ON playback_sessions(user_id) WHERE completed_at IS NULL;

CREATE TRIGGER playback_sessions_updated_at
    BEFORE UPDATE ON playback_sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- Schedule Cache (TVMaze)
-- ============================================================================

CREATE TABLE IF NOT EXISTS schedule_cache (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Episode identification
    tvmaze_episode_id INTEGER NOT NULL,
    episode_name TEXT NOT NULL,
    season INTEGER NOT NULL,
    episode_number INTEGER NOT NULL,
    episode_type TEXT,
    -- Air date/time
    air_date DATE NOT NULL,
    air_time TEXT,
    air_stamp TIMESTAMPTZ,
    -- Episode metadata
    runtime INTEGER,
    episode_image_url TEXT,
    summary TEXT,
    -- Show information (denormalized)
    tvmaze_show_id INTEGER NOT NULL,
    show_name TEXT NOT NULL,
    show_network TEXT,
    show_poster_url TEXT,
    show_genres TEXT[] DEFAULT '{}',
    -- Cache metadata
    country_code VARCHAR(10) NOT NULL DEFAULT 'US',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tvmaze_episode_id, country_code)
);

CREATE INDEX idx_schedule_cache_air_date ON schedule_cache(air_date, country_code);
CREATE INDEX idx_schedule_cache_country ON schedule_cache(country_code);
CREATE INDEX idx_schedule_cache_show ON schedule_cache(tvmaze_show_id);

CREATE TRIGGER set_updated_at_schedule_cache
    BEFORE UPDATE ON schedule_cache
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE schedule_cache IS 'Cached TV schedule entries from TVMaze';

-- ============================================================================
-- Schedule Sync State
-- ============================================================================

CREATE TABLE IF NOT EXISTS schedule_sync_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    country_code VARCHAR(10) NOT NULL UNIQUE,
    last_synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_sync_days INTEGER NOT NULL DEFAULT 7,
    sync_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER set_updated_at_schedule_sync_state
    BEFORE UPDATE ON schedule_sync_state
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE schedule_sync_state IS 'Tracks schedule sync state per country';

-- ============================================================================
-- Indexer Configuration
-- ============================================================================

CREATE TABLE IF NOT EXISTS indexer_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    indexer_type VARCHAR(50) NOT NULL,
    definition_id VARCHAR(100),
    name VARCHAR(255) NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    priority INTEGER NOT NULL DEFAULT 50,
    -- Site configuration
    site_url VARCHAR(500),
    -- Torznab capabilities
    supports_search BOOLEAN DEFAULT true,
    supports_tv_search BOOLEAN DEFAULT true,
    supports_movie_search BOOLEAN DEFAULT true,
    supports_music_search BOOLEAN DEFAULT false,
    supports_book_search BOOLEAN DEFAULT false,
    supports_imdb_search BOOLEAN DEFAULT false,
    supports_tvdb_search BOOLEAN DEFAULT false,
    capabilities JSONB,
    -- Health tracking
    last_error TEXT,
    error_count INTEGER NOT NULL DEFAULT 0,
    last_success_at TIMESTAMPTZ,
    last_error_at TIMESTAMPTZ,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_indexer_configs_user_id ON indexer_configs(user_id);
CREATE INDEX idx_indexer_configs_type ON indexer_configs(indexer_type);
CREATE INDEX idx_indexer_configs_enabled ON indexer_configs(enabled) WHERE enabled = true;

CREATE TRIGGER update_indexer_configs_updated_at
    BEFORE UPDATE ON indexer_configs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- Indexer Credentials (encrypted)
-- ============================================================================

CREATE TABLE IF NOT EXISTS indexer_credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    indexer_config_id UUID NOT NULL REFERENCES indexer_configs(id) ON DELETE CASCADE,
    credential_type VARCHAR(50) NOT NULL,
    encrypted_value TEXT NOT NULL,
    nonce VARCHAR(32) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(indexer_config_id, credential_type)
);

CREATE INDEX idx_indexer_credentials_config ON indexer_credentials(indexer_config_id);

CREATE TRIGGER update_indexer_credentials_updated_at
    BEFORE UPDATE ON indexer_credentials
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- Indexer Settings
-- ============================================================================

CREATE TABLE IF NOT EXISTS indexer_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    indexer_config_id UUID NOT NULL REFERENCES indexer_configs(id) ON DELETE CASCADE,
    setting_key VARCHAR(100) NOT NULL,
    setting_value TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(indexer_config_id, setting_key)
);

CREATE INDEX idx_indexer_settings_config ON indexer_settings(indexer_config_id);

CREATE TRIGGER update_indexer_settings_updated_at
    BEFORE UPDATE ON indexer_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- Indexer Search Cache
-- ============================================================================

CREATE TABLE IF NOT EXISTS indexer_search_cache (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    indexer_config_id UUID NOT NULL REFERENCES indexer_configs(id) ON DELETE CASCADE,
    query_hash VARCHAR(64) NOT NULL,
    query_type VARCHAR(20) NOT NULL,
    results JSONB NOT NULL,
    result_count INTEGER NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(indexer_config_id, query_hash)
);

CREATE INDEX idx_indexer_search_cache_expires ON indexer_search_cache(expires_at);
CREATE INDEX idx_indexer_search_cache_lookup ON indexer_search_cache(indexer_config_id, query_hash);

CREATE OR REPLACE FUNCTION clean_expired_indexer_cache()
RETURNS void AS $$
BEGIN
    DELETE FROM indexer_search_cache WHERE expires_at < NOW();
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Torznab Categories (static reference data)
-- ============================================================================

CREATE TABLE IF NOT EXISTS torznab_categories (
    id INTEGER PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    parent_id INTEGER REFERENCES torznab_categories(id),
    description TEXT
);

INSERT INTO torznab_categories (id, name, parent_id, description) VALUES
    -- Main categories
    (1000, 'Console', NULL, 'Console games'),
    (2000, 'Movies', NULL, 'Movies'),
    (3000, 'Audio', NULL, 'Audio/Music'),
    (4000, 'PC', NULL, 'PC software and games'),
    (5000, 'TV', NULL, 'TV shows'),
    (6000, 'XXX', NULL, 'Adult content'),
    (7000, 'Books', NULL, 'Books and comics'),
    (8000, 'Other', NULL, 'Other/Misc'),
    -- Movies subcategories
    (2010, 'Movies/Foreign', 2000, 'Foreign movies'),
    (2020, 'Movies/Other', 2000, 'Other movies'),
    (2030, 'Movies/SD', 2000, 'SD quality movies'),
    (2040, 'Movies/HD', 2000, 'HD quality movies'),
    (2045, 'Movies/UHD', 2000, '4K/UHD movies'),
    (2050, 'Movies/BluRay', 2000, 'BluRay movies'),
    (2060, 'Movies/3D', 2000, '3D movies'),
    (2070, 'Movies/DVD', 2000, 'DVD movies'),
    (2080, 'Movies/WEB-DL', 2000, 'WEB-DL movies'),
    -- TV subcategories
    (5010, 'TV/WEB-DL', 5000, 'WEB-DL TV shows'),
    (5020, 'TV/Foreign', 5000, 'Foreign TV shows'),
    (5030, 'TV/SD', 5000, 'SD TV shows'),
    (5040, 'TV/HD', 5000, 'HD TV shows'),
    (5045, 'TV/UHD', 5000, '4K/UHD TV shows'),
    (5050, 'TV/Other', 5000, 'Other TV shows'),
    (5060, 'TV/Sport', 5000, 'Sports'),
    (5070, 'TV/Anime', 5000, 'Anime'),
    (5080, 'TV/Documentary', 5000, 'Documentaries'),
    -- Audio subcategories
    (3010, 'Audio/MP3', 3000, 'MP3 audio'),
    (3020, 'Audio/Video', 3000, 'Music videos'),
    (3030, 'Audio/Audiobook', 3000, 'Audiobooks'),
    (3040, 'Audio/Lossless', 3000, 'Lossless audio'),
    (3050, 'Audio/Other', 3000, 'Other audio'),
    (3060, 'Audio/Foreign', 3000, 'Foreign audio'),
    -- Books subcategories
    (7010, 'Books/Mags', 7000, 'Magazines'),
    (7020, 'Books/EBook', 7000, 'E-Books'),
    (7030, 'Books/Comics', 7000, 'Comics'),
    (7040, 'Books/Technical', 7000, 'Technical books'),
    (7050, 'Books/Other', 7000, 'Other books'),
    (7060, 'Books/Foreign', 7000, 'Foreign books'),
    -- PC subcategories
    (4010, 'PC/0day', 4000, '0-day releases'),
    (4020, 'PC/ISO', 4000, 'ISO images'),
    (4030, 'PC/Mac', 4000, 'Mac software'),
    (4040, 'PC/Mobile-Other', 4000, 'Mobile software'),
    (4050, 'PC/Games', 4000, 'PC games'),
    (4060, 'PC/Mobile-iOS', 4000, 'iOS apps'),
    (4070, 'PC/Mobile-Android', 4000, 'Android apps'),
    -- Console subcategories
    (1010, 'Console/NDS', 1000, 'Nintendo DS'),
    (1020, 'Console/PSP', 1000, 'PlayStation Portable'),
    (1030, 'Console/Wii', 1000, 'Nintendo Wii'),
    (1040, 'Console/XBox', 1000, 'Xbox'),
    (1050, 'Console/XBox 360', 1000, 'Xbox 360'),
    (1060, 'Console/WiiWare', 1000, 'WiiWare'),
    (1070, 'Console/XBox 360 DLC', 1000, 'Xbox 360 DLC'),
    (1080, 'Console/PS3', 1000, 'PlayStation 3'),
    (1090, 'Console/Other', 1000, 'Other consoles'),
    (1110, 'Console/3DS', 1000, 'Nintendo 3DS'),
    (1120, 'Console/PS Vita', 1000, 'PlayStation Vita'),
    (1130, 'Console/WiiU', 1000, 'Wii U'),
    (1140, 'Console/XBox One', 1000, 'Xbox One'),
    (1150, 'Console/PS4', 1000, 'PlayStation 4'),
    (1180, 'Console/Switch', 1000, 'Nintendo Switch')
ON CONFLICT (id) DO NOTHING;
