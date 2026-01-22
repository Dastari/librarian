-- Librarian SQLite Schema
-- Initial migration for self-hosted deployment
-- Converted from PostgreSQL with the following changes:
-- - UUID → TEXT (stored as string)
-- - TEXT[] → TEXT (stored as JSON array)
-- - JSONB → TEXT (stored as JSON string)
-- - TIMESTAMPTZ → TEXT (stored as ISO8601 string)
-- - gen_random_uuid() → generated in application code
-- - NOW() → datetime('now')
-- - plpgsql triggers → pure SQL triggers

-- Enable foreign keys
PRAGMA foreign_keys = ON;

-- ============================================================================
-- Users and Authentication
-- ============================================================================

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE COLLATE NOCASE,
    email TEXT UNIQUE COLLATE NOCASE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('admin', 'member', 'guest')),
    display_name TEXT,
    avatar_url TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    last_login_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email) WHERE email IS NOT NULL;
CREATE INDEX idx_users_role ON users(role);

-- User library access control
CREATE TABLE IF NOT EXISTS user_library_access (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    access_level TEXT NOT NULL DEFAULT 'read' CHECK (access_level IN ('read', 'write', 'admin')),
    granted_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, library_id)
);

CREATE INDEX idx_user_library_access_user ON user_library_access(user_id);
CREATE INDEX idx_user_library_access_library ON user_library_access(library_id);

-- User content restrictions (for parental controls)
CREATE TABLE IF NOT EXISTS user_restrictions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Rating restrictions (JSON array of allowed ratings, e.g., ["G", "PG", "PG-13"])
    allowed_ratings TEXT NOT NULL DEFAULT '[]',
    -- Content type restrictions (JSON array, e.g., ["movies", "tv"])
    allowed_content_types TEXT NOT NULL DEFAULT '[]',
    -- Time-based restrictions (optional)
    viewing_start_time TEXT,  -- e.g., "06:00"
    viewing_end_time TEXT,    -- e.g., "21:00"
    -- Pin for bypassing restrictions
    bypass_pin_hash TEXT,
    -- Metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id)
);

CREATE INDEX idx_user_restrictions_user ON user_restrictions(user_id);

-- Invite tokens for sharing (Plex-style invites)
CREATE TABLE IF NOT EXISTS invite_tokens (
    id TEXT PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,
    created_by TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- JSON array of library IDs this invite grants access to
    library_ids TEXT NOT NULL DEFAULT '[]',
    role TEXT NOT NULL DEFAULT 'guest' CHECK (role IN ('member', 'guest')),
    access_level TEXT NOT NULL DEFAULT 'read' CHECK (access_level IN ('read', 'write')),
    -- Usage limits
    expires_at TEXT,
    max_uses INTEGER,
    use_count INTEGER NOT NULL DEFAULT 0,
    -- Optional restrictions to apply to users created via this invite
    apply_restrictions INTEGER NOT NULL DEFAULT 0,
    restrictions_template TEXT,  -- JSON object with restriction settings
    -- Status
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_invite_tokens_token ON invite_tokens(token);
CREATE INDEX idx_invite_tokens_creator ON invite_tokens(created_by);
CREATE INDEX idx_invite_tokens_active ON invite_tokens(is_active) WHERE is_active = 1;

-- Refresh tokens for session management
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    device_info TEXT,
    ip_address TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at TEXT
);

CREATE INDEX idx_refresh_tokens_user ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_hash ON refresh_tokens(token_hash);
CREATE INDEX idx_refresh_tokens_expires ON refresh_tokens(expires_at);

-- ============================================================================
-- Application Settings
-- ============================================================================

CREATE TABLE IF NOT EXISTS app_settings (
    id TEXT PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    value TEXT NOT NULL,  -- JSON value
    description TEXT,
    category TEXT NOT NULL DEFAULT 'general',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_app_settings_category ON app_settings(category);
CREATE INDEX idx_app_settings_key ON app_settings(key);

-- Default torrent settings
INSERT OR IGNORE INTO app_settings (id, key, value, description, category) VALUES
    (lower(hex(randomblob(16))), 'torrent.download_dir', '"/data/downloads"', 'Directory where torrents are downloaded to', 'torrent'),
    (lower(hex(randomblob(16))), 'torrent.session_dir', '"/data/session"', 'Directory for torrent session data', 'torrent'),
    (lower(hex(randomblob(16))), 'torrent.enable_dht', 'true', 'Enable DHT for peer discovery', 'torrent'),
    (lower(hex(randomblob(16))), 'torrent.listen_port', '6881', 'Port for incoming torrent connections', 'torrent'),
    (lower(hex(randomblob(16))), 'torrent.max_concurrent', '5', 'Maximum concurrent downloads', 'torrent'),
    (lower(hex(randomblob(16))), 'torrent.upload_limit', '0', 'Upload speed limit in bytes/sec (0 = unlimited)', 'torrent'),
    (lower(hex(randomblob(16))), 'torrent.download_limit', '0', 'Download speed limit in bytes/sec (0 = unlimited)', 'torrent');

-- ============================================================================
-- Application Logs
-- ============================================================================

CREATE TABLE IF NOT EXISTS app_logs (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    level TEXT NOT NULL CHECK (level IN ('TRACE', 'DEBUG', 'INFO', 'WARN', 'ERROR')),
    target TEXT NOT NULL,
    message TEXT NOT NULL,
    fields TEXT,  -- JSON object
    span_name TEXT,
    span_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_app_logs_timestamp ON app_logs(timestamp DESC);
CREATE INDEX idx_app_logs_level ON app_logs(level);
CREATE INDEX idx_app_logs_target ON app_logs(target);
CREATE INDEX idx_app_logs_created_at ON app_logs(created_at DESC);
CREATE INDEX idx_app_logs_level_timestamp ON app_logs(level, timestamp DESC);

-- ============================================================================
-- Libraries
-- ============================================================================

CREATE TABLE IF NOT EXISTS libraries (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    library_type TEXT NOT NULL CHECK (library_type IN ('movies', 'tv', 'music', 'audiobooks', 'other')),
    -- Display settings
    icon TEXT DEFAULT 'folder',
    color TEXT DEFAULT 'blue',
    -- Scan settings
    auto_scan INTEGER NOT NULL DEFAULT 1,
    scan_interval_minutes INTEGER NOT NULL DEFAULT 60,
    watch_for_changes INTEGER NOT NULL DEFAULT 0,
    last_scanned_at TEXT,
    scanning INTEGER NOT NULL DEFAULT 0,
    -- Post-download behavior
    post_download_action TEXT NOT NULL DEFAULT 'copy' CHECK (post_download_action IN ('copy', 'move', 'hardlink')),
    organize_files INTEGER NOT NULL DEFAULT 1,
    rename_style TEXT NOT NULL DEFAULT 'none' CHECK (rename_style IN ('none', 'clean', 'preserve_info')),
    naming_pattern TEXT DEFAULT '{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}',
    -- Automation settings
    auto_download INTEGER NOT NULL DEFAULT 1,
    auto_hunt INTEGER NOT NULL DEFAULT 0,
    auto_add_discovered INTEGER NOT NULL DEFAULT 0,
    -- Inline quality settings (JSON arrays, empty = any)
    allowed_resolutions TEXT NOT NULL DEFAULT '[]',
    allowed_video_codecs TEXT NOT NULL DEFAULT '[]',
    allowed_audio_formats TEXT NOT NULL DEFAULT '[]',
    require_hdr INTEGER NOT NULL DEFAULT 0,
    allowed_hdr_types TEXT NOT NULL DEFAULT '[]',
    allowed_sources TEXT NOT NULL DEFAULT '[]',
    release_group_blacklist TEXT NOT NULL DEFAULT '[]',
    release_group_whitelist TEXT NOT NULL DEFAULT '[]',
    -- Subtitle settings
    auto_download_subtitles INTEGER DEFAULT 0,
    preferred_subtitle_languages TEXT DEFAULT '[]',
    -- Metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_libraries_user ON libraries(user_id);
CREATE INDEX idx_libraries_scanning ON libraries(scanning) WHERE scanning = 1;

-- ============================================================================
-- TV Shows
-- ============================================================================

CREATE TABLE IF NOT EXISTS tv_shows (
    id TEXT PRIMARY KEY,
    library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    -- Basic info
    name TEXT NOT NULL,
    sort_name TEXT,
    year INTEGER,
    status TEXT DEFAULT 'unknown' CHECK (status IN ('continuing', 'ended', 'upcoming', 'cancelled', 'unknown')),
    -- External IDs
    tvmaze_id INTEGER,
    tmdb_id INTEGER,
    tvdb_id INTEGER,
    imdb_id TEXT,
    -- Metadata
    overview TEXT,
    network TEXT,
    runtime INTEGER,
    genres TEXT DEFAULT '[]',  -- JSON array
    -- Artwork URLs
    poster_url TEXT,
    backdrop_url TEXT,
    -- Content rating (for restrictions)
    content_rating TEXT,  -- e.g., "TV-14", "TV-MA"
    -- Monitoring settings
    monitored INTEGER NOT NULL DEFAULT 1,
    monitor_type TEXT NOT NULL DEFAULT 'all' CHECK (monitor_type IN ('all', 'future', 'none')),
    -- Path within library
    path TEXT,
    -- Statistics
    episode_count INTEGER DEFAULT 0,
    episode_file_count INTEGER DEFAULT 0,
    size_bytes INTEGER DEFAULT 0,
    -- Override settings (NULL = inherit from library)
    auto_download_override INTEGER DEFAULT NULL,
    backfill_existing INTEGER NOT NULL DEFAULT 1,
    organize_files_override INTEGER DEFAULT NULL,
    rename_style_override TEXT DEFAULT NULL CHECK (rename_style_override IS NULL OR rename_style_override IN ('none', 'clean', 'preserve_info')),
    auto_hunt_override INTEGER DEFAULT NULL,
    -- Inline quality overrides (NULL = inherit, JSON arrays)
    allowed_resolutions_override TEXT DEFAULT NULL,
    allowed_video_codecs_override TEXT DEFAULT NULL,
    allowed_audio_formats_override TEXT DEFAULT NULL,
    require_hdr_override INTEGER DEFAULT NULL,
    allowed_hdr_types_override TEXT DEFAULT NULL,
    allowed_sources_override TEXT DEFAULT NULL,
    release_group_blacklist_override TEXT DEFAULT NULL,
    release_group_whitelist_override TEXT DEFAULT NULL,
    -- Subtitle settings override (JSON object)
    subtitle_settings_override TEXT,
    -- Metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
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
CREATE INDEX idx_tv_shows_monitored ON tv_shows(library_id, monitored) WHERE monitored = 1;

-- ============================================================================
-- Episodes
-- ============================================================================

CREATE TABLE IF NOT EXISTS episodes (
    id TEXT PRIMARY KEY,
    tv_show_id TEXT NOT NULL REFERENCES tv_shows(id) ON DELETE CASCADE,
    -- Episode identification
    season INTEGER NOT NULL,
    episode INTEGER NOT NULL,
    absolute_number INTEGER,
    -- Metadata
    title TEXT,
    overview TEXT,
    air_date TEXT,  -- DATE as TEXT
    runtime INTEGER,
    -- External IDs
    tvmaze_id INTEGER,
    tmdb_id INTEGER,
    tvdb_id INTEGER,
    -- Media file link (bidirectional)
    media_file_id TEXT REFERENCES media_files(id) ON DELETE SET NULL,
    -- Metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    -- Constraints
    UNIQUE(tv_show_id, season, episode)
);

CREATE INDEX idx_episodes_show ON episodes(tv_show_id);
CREATE INDEX idx_episodes_show_season ON episodes(tv_show_id, season);
CREATE INDEX idx_episodes_air_date ON episodes(air_date) WHERE air_date IS NOT NULL;
CREATE INDEX idx_episodes_media_file ON episodes(media_file_id) WHERE media_file_id IS NOT NULL;

-- ============================================================================
-- Movies
-- ============================================================================

CREATE TABLE IF NOT EXISTS movies (
    id TEXT PRIMARY KEY,
    library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    -- Basic info
    title TEXT NOT NULL,
    sort_title TEXT,
    original_title TEXT,
    year INTEGER,
    -- External IDs
    tmdb_id INTEGER,
    imdb_id TEXT,
    -- Metadata
    overview TEXT,
    tagline TEXT,
    runtime INTEGER,
    genres TEXT DEFAULT '[]',  -- JSON array
    production_countries TEXT DEFAULT '[]',  -- JSON array
    spoken_languages TEXT DEFAULT '[]',  -- JSON array
    -- Credits
    director TEXT,
    cast_names TEXT DEFAULT '[]',  -- JSON array
    -- Ratings
    tmdb_rating REAL,
    tmdb_vote_count INTEGER,
    -- Artwork URLs
    poster_url TEXT,
    backdrop_url TEXT,
    -- Collection info
    collection_id INTEGER,
    collection_name TEXT,
    collection_poster_url TEXT,
    -- Release info
    release_date TEXT,  -- DATE as TEXT
    certification TEXT,  -- Content rating: G, PG, PG-13, R, etc.
    -- Status/monitoring
    status TEXT DEFAULT 'unknown' CHECK (status IN ('released', 'upcoming', 'announced', 'in_production', 'unknown')),
    monitored INTEGER NOT NULL DEFAULT 1,
    -- Media file link (bidirectional)
    media_file_id TEXT REFERENCES media_files(id) ON DELETE SET NULL,
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    -- Constraints
    UNIQUE(library_id, tmdb_id),
    UNIQUE(library_id, imdb_id)
);

CREATE INDEX idx_movies_library ON movies(library_id);
CREATE INDEX idx_movies_user ON movies(user_id);
CREATE INDEX idx_movies_tmdb ON movies(tmdb_id) WHERE tmdb_id IS NOT NULL;
CREATE INDEX idx_movies_imdb ON movies(imdb_id) WHERE imdb_id IS NOT NULL;
CREATE INDEX idx_movies_year ON movies(year) WHERE year IS NOT NULL;
CREATE INDEX idx_movies_media_file ON movies(media_file_id) WHERE media_file_id IS NOT NULL;
CREATE INDEX idx_movies_certification ON movies(certification) WHERE certification IS NOT NULL;

-- ============================================================================
-- Artists (Music)
-- ============================================================================

CREATE TABLE IF NOT EXISTS artists (
    id TEXT PRIMARY KEY,
    library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    -- Basic info
    name TEXT NOT NULL,
    sort_name TEXT,
    -- External IDs
    musicbrainz_id TEXT,
    -- Metadata
    bio TEXT,
    disambiguation TEXT,
    -- Artwork
    image_url TEXT,
    -- Stats
    album_count INTEGER DEFAULT 0,
    track_count INTEGER DEFAULT 0,
    total_duration_secs INTEGER DEFAULT 0,
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    -- Constraints
    UNIQUE(library_id, musicbrainz_id)
);

CREATE INDEX idx_artists_library ON artists(library_id);
CREATE INDEX idx_artists_user ON artists(user_id);
CREATE INDEX idx_artists_musicbrainz ON artists(musicbrainz_id) WHERE musicbrainz_id IS NOT NULL;

-- ============================================================================
-- Albums (Music)
-- ============================================================================

CREATE TABLE IF NOT EXISTS albums (
    id TEXT PRIMARY KEY,
    artist_id TEXT NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    -- Basic info
    name TEXT NOT NULL,
    sort_name TEXT,
    year INTEGER,
    -- External IDs
    musicbrainz_id TEXT,
    -- Metadata
    album_type TEXT DEFAULT 'album' CHECK (album_type IN ('album', 'single', 'ep', 'compilation', 'soundtrack', 'live', 'remix', 'other')),
    genres TEXT DEFAULT '[]',  -- JSON array
    label TEXT,
    country TEXT,
    release_date TEXT,
    -- Artwork
    cover_url TEXT,
    -- Stats
    track_count INTEGER DEFAULT 0,
    disc_count INTEGER DEFAULT 1,
    total_duration_secs INTEGER DEFAULT 0,
    -- File status
    has_files INTEGER NOT NULL DEFAULT 0,
    size_bytes INTEGER DEFAULT 0,
    -- Path within library
    path TEXT,
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    -- Constraints
    UNIQUE(artist_id, musicbrainz_id)
);

CREATE INDEX idx_albums_library ON albums(library_id);
CREATE INDEX idx_albums_artist ON albums(artist_id);
CREATE INDEX idx_albums_user ON albums(user_id);
CREATE INDEX idx_albums_musicbrainz ON albums(musicbrainz_id) WHERE musicbrainz_id IS NOT NULL;

-- ============================================================================
-- Tracks (Music)
-- ============================================================================

CREATE TABLE IF NOT EXISTS tracks (
    id TEXT PRIMARY KEY,
    album_id TEXT NOT NULL REFERENCES albums(id) ON DELETE CASCADE,
    library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    -- Basic info
    title TEXT NOT NULL,
    track_number INTEGER NOT NULL,
    disc_number INTEGER DEFAULT 1,
    -- External IDs
    musicbrainz_id TEXT,
    isrc TEXT,
    -- Metadata
    duration_secs INTEGER,
    explicit INTEGER DEFAULT 0,
    -- Artist info
    artist_name TEXT,
    artist_id TEXT REFERENCES artists(id) ON DELETE SET NULL,
    -- File link
    media_file_id TEXT REFERENCES media_files(id) ON DELETE SET NULL,
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_tracks_album ON tracks(album_id);
CREATE INDEX idx_tracks_library ON tracks(library_id);
CREATE INDEX idx_tracks_media_file ON tracks(media_file_id) WHERE media_file_id IS NOT NULL;
CREATE INDEX idx_tracks_album_order ON tracks(album_id, disc_number, track_number);

-- ============================================================================
-- Audiobooks
-- ============================================================================

CREATE TABLE IF NOT EXISTS audiobooks (
    id TEXT PRIMARY KEY,
    library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    -- Basic info
    title TEXT NOT NULL,
    sort_title TEXT,
    -- Author info
    author_name TEXT,
    narrator_name TEXT,
    narrators TEXT DEFAULT '[]',  -- JSON array
    -- Metadata
    description TEXT,
    publisher TEXT,
    published_date TEXT,
    language TEXT,
    isbn TEXT,
    asin TEXT,
    -- External IDs
    audible_id TEXT,
    goodreads_id TEXT,
    -- Stats
    total_duration_secs INTEGER DEFAULT 0,
    chapter_count INTEGER DEFAULT 0,
    -- Artwork
    cover_url TEXT,
    -- File status
    has_files INTEGER NOT NULL DEFAULT 0,
    size_bytes INTEGER DEFAULT 0,
    -- Path within library
    path TEXT,
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    -- Constraints
    UNIQUE(library_id, asin),
    UNIQUE(library_id, isbn)
);

CREATE INDEX idx_audiobooks_library ON audiobooks(library_id);
CREATE INDEX idx_audiobooks_user ON audiobooks(user_id);

-- ============================================================================
-- Chapters (Audiobook chapters)
-- ============================================================================

CREATE TABLE IF NOT EXISTS chapters (
    id TEXT PRIMARY KEY,
    audiobook_id TEXT NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
    -- Chapter info
    chapter_number INTEGER NOT NULL,
    title TEXT,
    start_time_secs REAL NOT NULL DEFAULT 0,
    end_time_secs REAL,
    duration_secs INTEGER,
    -- File link
    media_file_id TEXT REFERENCES media_files(id) ON DELETE SET NULL,
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    -- Constraints
    UNIQUE(audiobook_id, chapter_number)
);

CREATE INDEX idx_chapters_audiobook ON chapters(audiobook_id);
CREATE INDEX idx_chapters_media_file ON chapters(media_file_id) WHERE media_file_id IS NOT NULL;

-- ============================================================================
-- Media Files (Central registry for all media files)
-- ============================================================================

CREATE TABLE IF NOT EXISTS media_files (
    id TEXT PRIMARY KEY,
    library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    -- Content type links (only one should be set)
    episode_id TEXT REFERENCES episodes(id) ON DELETE SET NULL,
    movie_id TEXT REFERENCES movies(id) ON DELETE SET NULL,
    track_id TEXT REFERENCES tracks(id) ON DELETE SET NULL,
    album_id TEXT REFERENCES albums(id) ON DELETE SET NULL,
    audiobook_id TEXT REFERENCES audiobooks(id) ON DELETE SET NULL,
    chapter_id TEXT REFERENCES chapters(id) ON DELETE SET NULL,
    -- File info
    path TEXT NOT NULL UNIQUE,
    relative_path TEXT,
    original_name TEXT,
    size INTEGER NOT NULL,
    -- Basic media properties
    container TEXT,
    video_codec TEXT,
    audio_codec TEXT,
    width INTEGER,
    height INTEGER,
    duration INTEGER,
    bitrate INTEGER,
    -- Enhanced video properties
    resolution TEXT,
    video_bitrate INTEGER,
    is_hdr INTEGER DEFAULT 0,
    hdr_type TEXT,
    -- Enhanced audio properties
    audio_channels TEXT,
    audio_language TEXT,
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
    analyzed_at TEXT,
    analysis_data TEXT,  -- JSON object
    -- File tracking
    file_hash TEXT,
    added_at TEXT NOT NULL DEFAULT (datetime('now')),
    modified_at TEXT,
    -- Organization tracking
    organized INTEGER NOT NULL DEFAULT 0,
    organized_at TEXT,
    original_path TEXT
);

CREATE INDEX idx_media_files_library ON media_files(library_id);
CREATE INDEX idx_media_files_episode ON media_files(episode_id) WHERE episode_id IS NOT NULL;
CREATE INDEX idx_media_files_movie ON media_files(movie_id) WHERE movie_id IS NOT NULL;
CREATE INDEX idx_media_files_track ON media_files(track_id) WHERE track_id IS NOT NULL;
CREATE INDEX idx_media_files_album ON media_files(album_id) WHERE album_id IS NOT NULL;
CREATE INDEX idx_media_files_audiobook ON media_files(audiobook_id) WHERE audiobook_id IS NOT NULL;
CREATE INDEX idx_media_files_chapter ON media_files(chapter_id) WHERE chapter_id IS NOT NULL;
CREATE INDEX idx_media_files_resolution ON media_files(resolution);
CREATE INDEX idx_media_files_unorganized ON media_files(library_id, organized) WHERE organized = 0;

-- ============================================================================
-- Video Streams
-- ============================================================================

CREATE TABLE IF NOT EXISTS video_streams (
    id TEXT PRIMARY KEY,
    media_file_id TEXT NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    stream_index INTEGER NOT NULL,
    codec TEXT NOT NULL,
    codec_long_name TEXT,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    aspect_ratio TEXT,
    frame_rate TEXT,
    avg_frame_rate TEXT,
    bitrate INTEGER,
    pixel_format TEXT,
    color_space TEXT,
    color_transfer TEXT,
    color_primaries TEXT,
    hdr_type TEXT,
    bit_depth INTEGER,
    language TEXT,
    title TEXT,
    is_default INTEGER DEFAULT 0,
    metadata TEXT DEFAULT '{}',  -- JSON object
    created_at TEXT DEFAULT (datetime('now')),
    UNIQUE(media_file_id, stream_index)
);

CREATE INDEX idx_video_streams_media_file ON video_streams(media_file_id);

-- ============================================================================
-- Audio Streams
-- ============================================================================

CREATE TABLE IF NOT EXISTS audio_streams (
    id TEXT PRIMARY KEY,
    media_file_id TEXT NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    stream_index INTEGER NOT NULL,
    codec TEXT NOT NULL,
    codec_long_name TEXT,
    channels INTEGER NOT NULL,
    channel_layout TEXT,
    sample_rate INTEGER,
    bitrate INTEGER,
    bit_depth INTEGER,
    language TEXT,
    title TEXT,
    is_default INTEGER DEFAULT 0,
    is_commentary INTEGER DEFAULT 0,
    metadata TEXT DEFAULT '{}',  -- JSON object
    created_at TEXT DEFAULT (datetime('now')),
    UNIQUE(media_file_id, stream_index)
);

CREATE INDEX idx_audio_streams_media_file ON audio_streams(media_file_id);

-- ============================================================================
-- Subtitles
-- ============================================================================

CREATE TABLE IF NOT EXISTS subtitles (
    id TEXT PRIMARY KEY,
    media_file_id TEXT NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
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
    is_default INTEGER DEFAULT 0,
    is_forced INTEGER DEFAULT 0,
    is_hearing_impaired INTEGER DEFAULT 0,
    -- Download info
    opensubtitles_id TEXT,
    downloaded_at TEXT,
    -- Stream metadata
    metadata TEXT DEFAULT '{}',  -- JSON object
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_subtitles_media_file ON subtitles(media_file_id);
CREATE INDEX idx_subtitles_language ON subtitles(language);

-- ============================================================================
-- Media Chapters (embedded chapter markers)
-- ============================================================================

CREATE TABLE IF NOT EXISTS media_chapters (
    id TEXT PRIMARY KEY,
    media_file_id TEXT NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    chapter_index INTEGER NOT NULL,
    start_secs REAL NOT NULL,
    end_secs REAL NOT NULL,
    title TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    UNIQUE(media_file_id, chapter_index)
);

CREATE INDEX idx_media_chapters_media_file ON media_chapters(media_file_id);

-- ============================================================================
-- Torrents
-- ============================================================================

CREATE TABLE IF NOT EXISTS torrents (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    -- Torrent identification
    info_hash TEXT NOT NULL,
    magnet_uri TEXT,
    -- Display info
    name TEXT NOT NULL,
    -- Status tracking
    state TEXT NOT NULL DEFAULT 'queued',
    progress REAL NOT NULL DEFAULT 0,
    -- Size info
    total_bytes INTEGER NOT NULL DEFAULT 0,
    downloaded_bytes INTEGER NOT NULL DEFAULT 0,
    uploaded_bytes INTEGER NOT NULL DEFAULT 0,
    -- Path info
    save_path TEXT NOT NULL,
    download_path TEXT,
    source_url TEXT,
    -- Library/content links
    library_id TEXT REFERENCES libraries(id) ON DELETE SET NULL,
    -- Post-processing
    post_process_status TEXT DEFAULT 'pending' CHECK (post_process_status IN ('pending', 'processing', 'completed', 'failed', 'skipped')),
    post_process_error TEXT,
    processed_at TEXT,
    -- Excluded files (JSON array of file indices)
    excluded_files TEXT DEFAULT '[]',
    -- Timestamps
    added_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    -- Unique constraint
    UNIQUE(user_id, info_hash)
);

CREATE INDEX idx_torrents_user ON torrents(user_id);
CREATE INDEX idx_torrents_state ON torrents(state);
CREATE INDEX idx_torrents_info_hash ON torrents(info_hash);
CREATE INDEX idx_torrents_library ON torrents(library_id) WHERE library_id IS NOT NULL;
CREATE INDEX idx_torrents_post_process ON torrents(post_process_status) WHERE post_process_status = 'pending';

-- ============================================================================
-- Torrent Files
-- ============================================================================

CREATE TABLE IF NOT EXISTS torrent_files (
    id TEXT PRIMARY KEY,
    torrent_id TEXT NOT NULL REFERENCES torrents(id) ON DELETE CASCADE,
    file_index INTEGER NOT NULL,
    path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    -- Download status
    downloaded_bytes INTEGER NOT NULL DEFAULT 0,
    is_selected INTEGER NOT NULL DEFAULT 1,
    -- Parsed metadata
    parsed_show_name TEXT,
    parsed_season INTEGER,
    parsed_episode INTEGER,
    parsed_resolution TEXT,
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(torrent_id, file_index)
);

CREATE INDEX idx_torrent_files_torrent ON torrent_files(torrent_id);

-- ============================================================================
-- Pending File Matches
-- ============================================================================

CREATE TABLE IF NOT EXISTS pending_file_matches (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    
    -- Source file info (works for any source: torrent, usenet, scan, manual)
    source_path TEXT NOT NULL,
    source_type TEXT NOT NULL,  -- 'torrent', 'usenet', 'irc', 'scan', 'manual'
    source_id TEXT,             -- Optional: torrent_id, usenet_download_id, etc.
    source_file_index INTEGER,  -- For multi-file sources (e.g., torrent file index)
    file_size INTEGER NOT NULL,
    
    -- Match target (only one should be set per row)
    episode_id TEXT REFERENCES episodes(id) ON DELETE CASCADE,
    movie_id TEXT REFERENCES movies(id) ON DELETE CASCADE,
    track_id TEXT REFERENCES tracks(id) ON DELETE CASCADE,
    chapter_id TEXT REFERENCES chapters(id) ON DELETE CASCADE,
    
    -- Match metadata
    match_type TEXT DEFAULT 'auto',  -- 'auto', 'manual'
    match_confidence REAL,
    
    -- Parsed quality info (from filename)
    parsed_resolution TEXT,
    parsed_codec TEXT,
    parsed_source TEXT,
    parsed_audio TEXT,
    
    -- Processing status
    copied_at TEXT,               -- null = not yet copied to library
    copy_error TEXT,              -- error message if copy failed
    
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for common queries
CREATE INDEX idx_pending_file_matches_source ON pending_file_matches(source_type, source_id);
CREATE INDEX idx_pending_file_matches_user ON pending_file_matches(user_id);
CREATE INDEX idx_pending_file_matches_episode ON pending_file_matches(episode_id);
CREATE INDEX idx_pending_file_matches_movie ON pending_file_matches(movie_id);
CREATE INDEX idx_pending_file_matches_track ON pending_file_matches(track_id);
CREATE INDEX idx_pending_file_matches_chapter ON pending_file_matches(chapter_id);

-- ============================================================================
-- RSS Feeds
-- ============================================================================

CREATE TABLE IF NOT EXISTS rss_feeds (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    library_id TEXT REFERENCES libraries(id) ON DELETE CASCADE,
    -- Feed info
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    -- Settings
    enabled INTEGER NOT NULL DEFAULT 1,
    poll_interval_minutes INTEGER NOT NULL DEFAULT 15,
    -- Tracking
    last_polled_at TEXT,
    last_successful_at TEXT,
    last_error TEXT,
    consecutive_failures INTEGER DEFAULT 0,
    -- Metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_rss_feeds_user ON rss_feeds(user_id);
CREATE INDEX idx_rss_feeds_library ON rss_feeds(library_id) WHERE library_id IS NOT NULL;
CREATE INDEX idx_rss_feeds_enabled ON rss_feeds(enabled, last_polled_at) WHERE enabled = 1;

-- ============================================================================
-- RSS Feed Items
-- ============================================================================

CREATE TABLE IF NOT EXISTS rss_feed_items (
    id TEXT PRIMARY KEY,
    feed_id TEXT NOT NULL REFERENCES rss_feeds(id) ON DELETE CASCADE,
    -- Item identification
    guid TEXT,
    link_hash TEXT NOT NULL,
    title_hash TEXT NOT NULL,
    -- Parsed data
    title TEXT NOT NULL,
    link TEXT NOT NULL,
    pub_date TEXT,
    description TEXT,
    -- Parsed metadata
    parsed_show_name TEXT,
    parsed_season INTEGER,
    parsed_episode INTEGER,
    parsed_resolution TEXT,
    parsed_codec TEXT,
    parsed_source TEXT,
    parsed_audio TEXT,
    parsed_hdr TEXT,
    -- Processing
    processed INTEGER NOT NULL DEFAULT 0,
    torrent_id TEXT REFERENCES torrents(id) ON DELETE SET NULL,
    skipped_reason TEXT,
    -- Metadata
    seen_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(feed_id, link_hash)
);

CREATE INDEX idx_rss_items_feed ON rss_feed_items(feed_id);
CREATE INDEX idx_rss_items_processed ON rss_feed_items(feed_id, processed) WHERE processed = 0;

-- ============================================================================
-- Indexer Configuration
-- ============================================================================

CREATE TABLE IF NOT EXISTS indexer_configs (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    indexer_type TEXT NOT NULL,
    definition_id TEXT,
    name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 50,
    -- Site configuration
    site_url TEXT,
    -- Torznab capabilities
    supports_search INTEGER DEFAULT 1,
    supports_tv_search INTEGER DEFAULT 1,
    supports_movie_search INTEGER DEFAULT 1,
    supports_music_search INTEGER DEFAULT 0,
    supports_book_search INTEGER DEFAULT 0,
    supports_imdb_search INTEGER DEFAULT 0,
    supports_tvdb_search INTEGER DEFAULT 0,
    capabilities TEXT,  -- JSON object
    -- Health tracking
    last_error TEXT,
    error_count INTEGER NOT NULL DEFAULT 0,
    last_success_at TEXT,
    last_error_at TEXT,
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_indexer_configs_user_id ON indexer_configs(user_id);
CREATE INDEX idx_indexer_configs_type ON indexer_configs(indexer_type);
CREATE INDEX idx_indexer_configs_enabled ON indexer_configs(enabled) WHERE enabled = 1;

-- ============================================================================
-- Indexer Credentials (encrypted)
-- ============================================================================

CREATE TABLE IF NOT EXISTS indexer_credentials (
    id TEXT PRIMARY KEY,
    indexer_config_id TEXT NOT NULL REFERENCES indexer_configs(id) ON DELETE CASCADE,
    credential_type TEXT NOT NULL,
    encrypted_value TEXT NOT NULL,
    nonce TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(indexer_config_id, credential_type)
);

CREATE INDEX idx_indexer_credentials_config ON indexer_credentials(indexer_config_id);

-- ============================================================================
-- Indexer Settings
-- ============================================================================

CREATE TABLE IF NOT EXISTS indexer_settings (
    id TEXT PRIMARY KEY,
    indexer_config_id TEXT NOT NULL REFERENCES indexer_configs(id) ON DELETE CASCADE,
    setting_key TEXT NOT NULL,
    setting_value TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(indexer_config_id, setting_key)
);

CREATE INDEX idx_indexer_settings_config ON indexer_settings(indexer_config_id);

-- ============================================================================
-- Indexer Search Cache
-- ============================================================================

CREATE TABLE IF NOT EXISTS indexer_search_cache (
    id TEXT PRIMARY KEY,
    indexer_config_id TEXT NOT NULL REFERENCES indexer_configs(id) ON DELETE CASCADE,
    query_hash TEXT NOT NULL,
    query_type TEXT NOT NULL,
    results TEXT NOT NULL,  -- JSON array
    result_count INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(indexer_config_id, query_hash)
);

CREATE INDEX idx_indexer_search_cache_expires ON indexer_search_cache(expires_at);
CREATE INDEX idx_indexer_search_cache_lookup ON indexer_search_cache(indexer_config_id, query_hash);

-- ============================================================================
-- Cast Devices
-- ============================================================================

CREATE TABLE IF NOT EXISTS cast_devices (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    address TEXT NOT NULL UNIQUE,
    port INTEGER NOT NULL DEFAULT 8009,
    model TEXT,
    device_type TEXT NOT NULL DEFAULT 'chromecast',
    is_favorite INTEGER NOT NULL DEFAULT 0,
    is_manual INTEGER NOT NULL DEFAULT 0,
    last_seen_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_cast_devices_address ON cast_devices(address);

-- ============================================================================
-- Cast Sessions
-- ============================================================================

CREATE TABLE IF NOT EXISTS cast_sessions (
    id TEXT PRIMARY KEY,
    device_id TEXT REFERENCES cast_devices(id) ON DELETE SET NULL,
    media_file_id TEXT REFERENCES media_files(id) ON DELETE SET NULL,
    episode_id TEXT REFERENCES episodes(id) ON DELETE SET NULL,
    stream_url TEXT NOT NULL,
    player_state TEXT NOT NULL DEFAULT 'idle',
    current_position REAL NOT NULL DEFAULT 0,
    duration REAL,
    volume REAL NOT NULL DEFAULT 1.0,
    is_muted INTEGER NOT NULL DEFAULT 0,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    last_position REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_cast_sessions_device_active ON cast_sessions(device_id) WHERE ended_at IS NULL;

-- ============================================================================
-- Cast Settings
-- ============================================================================

CREATE TABLE IF NOT EXISTS cast_settings (
    id TEXT PRIMARY KEY,
    auto_discovery_enabled INTEGER NOT NULL DEFAULT 1,
    discovery_interval_seconds INTEGER NOT NULL DEFAULT 30,
    default_volume REAL NOT NULL DEFAULT 1.0,
    transcode_incompatible INTEGER NOT NULL DEFAULT 1,
    preferred_quality TEXT DEFAULT '1080p',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Insert default cast settings
INSERT OR IGNORE INTO cast_settings (id, auto_discovery_enabled, discovery_interval_seconds, default_volume, transcode_incompatible, preferred_quality)
VALUES (lower(hex(randomblob(16))), 1, 30, 1.0, 1, '1080p');

-- ============================================================================
-- Playback Sessions
-- ============================================================================

CREATE TABLE IF NOT EXISTS playback_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    episode_id TEXT REFERENCES episodes(id) ON DELETE CASCADE,
    media_file_id TEXT REFERENCES media_files(id) ON DELETE CASCADE,
    tv_show_id TEXT REFERENCES tv_shows(id) ON DELETE CASCADE,
    movie_id TEXT REFERENCES movies(id) ON DELETE CASCADE,
    -- Playback state
    current_position REAL NOT NULL DEFAULT 0,
    duration REAL,
    volume REAL NOT NULL DEFAULT 1.0,
    is_muted INTEGER NOT NULL DEFAULT 0,
    is_playing INTEGER NOT NULL DEFAULT 0,
    -- Timestamps
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    -- One active session per user
    UNIQUE(user_id)
);

CREATE INDEX idx_playback_sessions_user ON playback_sessions(user_id);
CREATE INDEX idx_playback_sessions_incomplete ON playback_sessions(user_id) WHERE completed_at IS NULL;

-- ============================================================================
-- Watch Progress
-- ============================================================================

CREATE TABLE IF NOT EXISTS watch_progress (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    -- Content identification (only one should be set)
    content_type TEXT NOT NULL CHECK (content_type IN ('episode', 'movie', 'track', 'chapter')),
    episode_id TEXT REFERENCES episodes(id) ON DELETE CASCADE,
    movie_id TEXT REFERENCES movies(id) ON DELETE CASCADE,
    track_id TEXT REFERENCES tracks(id) ON DELETE CASCADE,
    chapter_id TEXT REFERENCES chapters(id) ON DELETE CASCADE,
    -- Progress
    position_secs REAL NOT NULL DEFAULT 0,
    duration_secs REAL,
    progress_percent REAL GENERATED ALWAYS AS (
        CASE WHEN duration_secs > 0 THEN (position_secs / duration_secs) * 100 ELSE 0 END
    ) STORED,
    completed INTEGER NOT NULL DEFAULT 0,
    -- Timestamps
    last_watched_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_watch_progress_user ON watch_progress(user_id);
CREATE INDEX idx_watch_progress_episode ON watch_progress(user_id, episode_id) WHERE episode_id IS NOT NULL;
CREATE INDEX idx_watch_progress_movie ON watch_progress(user_id, movie_id) WHERE movie_id IS NOT NULL;
CREATE INDEX idx_watch_progress_recent ON watch_progress(user_id, last_watched_at DESC);
CREATE UNIQUE INDEX idx_watch_progress_user_episode ON watch_progress(user_id, episode_id) WHERE content_type = 'episode';
CREATE UNIQUE INDEX idx_watch_progress_user_movie ON watch_progress(user_id, movie_id) WHERE content_type = 'movie';

-- ============================================================================
-- Schedule Cache (TVMaze)
-- ============================================================================

CREATE TABLE IF NOT EXISTS schedule_cache (
    id TEXT PRIMARY KEY,
    -- Episode identification
    tvmaze_episode_id INTEGER NOT NULL,
    episode_name TEXT NOT NULL,
    season INTEGER NOT NULL,
    episode_number INTEGER NOT NULL,
    episode_type TEXT,
    -- Air date/time
    air_date TEXT NOT NULL,
    air_time TEXT,
    air_stamp TEXT,
    -- Episode metadata
    runtime INTEGER,
    episode_image_url TEXT,
    summary TEXT,
    -- Show information (denormalized)
    tvmaze_show_id INTEGER NOT NULL,
    show_name TEXT NOT NULL,
    show_network TEXT,
    show_poster_url TEXT,
    show_genres TEXT DEFAULT '[]',  -- JSON array
    -- Cache metadata
    country_code TEXT NOT NULL DEFAULT 'US',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(tvmaze_episode_id, country_code)
);

CREATE INDEX idx_schedule_cache_air_date ON schedule_cache(air_date, country_code);
CREATE INDEX idx_schedule_cache_country ON schedule_cache(country_code);
CREATE INDEX idx_schedule_cache_show ON schedule_cache(tvmaze_show_id);

-- ============================================================================
-- Schedule Sync State
-- ============================================================================

CREATE TABLE IF NOT EXISTS schedule_sync_state (
    id TEXT PRIMARY KEY,
    country_code TEXT NOT NULL UNIQUE,
    last_synced_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_sync_days INTEGER NOT NULL DEFAULT 7,
    sync_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================================
-- Naming Patterns
-- ============================================================================

CREATE TABLE IF NOT EXISTS naming_patterns (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    library_type TEXT NOT NULL CHECK (library_type IN ('movies', 'tv', 'music', 'audiobooks', 'other')),
    name TEXT NOT NULL,
    pattern TEXT NOT NULL,
    description TEXT,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_naming_patterns_user ON naming_patterns(user_id);
CREATE INDEX idx_naming_patterns_type ON naming_patterns(library_type);

-- ============================================================================
-- Priority Rules
-- ============================================================================

CREATE TABLE IF NOT EXISTS priority_rules (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    library_type TEXT,
    library_id TEXT REFERENCES libraries(id) ON DELETE CASCADE,
    -- Rule definition
    name TEXT NOT NULL,
    priority_order TEXT NOT NULL DEFAULT '[]',  -- JSON array of source preferences
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_priority_rules_user ON priority_rules(user_id);
CREATE INDEX idx_priority_rules_library ON priority_rules(library_id);

-- ============================================================================
-- Usenet Servers
-- ============================================================================

CREATE TABLE IF NOT EXISTS usenet_servers (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 563,
    use_ssl INTEGER NOT NULL DEFAULT 1,
    username TEXT,
    password_encrypted TEXT,
    password_nonce TEXT,
    connections INTEGER NOT NULL DEFAULT 10,
    priority INTEGER NOT NULL DEFAULT 50,
    enabled INTEGER NOT NULL DEFAULT 1,
    -- Health tracking
    last_error TEXT,
    error_count INTEGER NOT NULL DEFAULT 0,
    last_success_at TEXT,
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_usenet_servers_user ON usenet_servers(user_id);
CREATE INDEX idx_usenet_servers_enabled ON usenet_servers(enabled) WHERE enabled = 1;

-- ============================================================================
-- Usenet Downloads
-- ============================================================================

CREATE TABLE IF NOT EXISTS usenet_downloads (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    -- Download identification
    name TEXT NOT NULL,
    nzb_url TEXT,
    nzb_data TEXT,  -- Base64 encoded NZB content
    -- Status
    state TEXT NOT NULL DEFAULT 'queued',
    progress REAL NOT NULL DEFAULT 0,
    -- Size info
    total_bytes INTEGER NOT NULL DEFAULT 0,
    downloaded_bytes INTEGER NOT NULL DEFAULT 0,
    -- Path info
    save_path TEXT NOT NULL,
    download_path TEXT,
    -- Library link
    library_id TEXT REFERENCES libraries(id) ON DELETE SET NULL,
    -- Post-processing
    post_process_status TEXT DEFAULT 'pending',
    post_process_error TEXT,
    -- Timestamps
    added_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE INDEX idx_usenet_downloads_user ON usenet_downloads(user_id);
CREATE INDEX idx_usenet_downloads_state ON usenet_downloads(state);
CREATE INDEX idx_usenet_downloads_library ON usenet_downloads(library_id);

-- ============================================================================
-- Notifications
-- ============================================================================

CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    -- Notification content
    notification_type TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'info',
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    -- Related content
    entity_type TEXT,
    entity_id TEXT,
    -- Action info (JSON object)
    action_type TEXT,
    action_data TEXT,
    -- Status
    is_read INTEGER NOT NULL DEFAULT 0,
    is_dismissed INTEGER NOT NULL DEFAULT 0,
    -- Resolution
    resolution TEXT,
    resolved_at TEXT,
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT
);

CREATE INDEX idx_notifications_user ON notifications(user_id);
CREATE INDEX idx_notifications_unread ON notifications(user_id, is_read) WHERE is_read = 0;
CREATE INDEX idx_notifications_category ON notifications(user_id, category);
CREATE INDEX idx_notifications_created ON notifications(created_at DESC);

-- ============================================================================
-- Artwork Cache (BLOB storage)
-- ============================================================================

CREATE TABLE IF NOT EXISTS artwork_cache (
    id TEXT PRIMARY KEY,
    -- Entity identification
    entity_type TEXT NOT NULL CHECK (entity_type IN ('show', 'movie', 'episode', 'album', 'artist', 'audiobook')),
    entity_id TEXT NOT NULL,
    artwork_type TEXT NOT NULL CHECK (artwork_type IN ('poster', 'backdrop', 'thumbnail', 'banner', 'cover')),
    -- Image data
    content_hash TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    data BLOB NOT NULL,
    size_bytes INTEGER NOT NULL,
    -- Original source
    source_url TEXT,
    -- Metadata
    width INTEGER,
    height INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    -- Unique per entity/type
    UNIQUE(entity_type, entity_id, artwork_type)
);

CREATE INDEX idx_artwork_cache_lookup ON artwork_cache(entity_type, entity_id);
CREATE INDEX idx_artwork_cache_hash ON artwork_cache(content_hash);

-- ============================================================================
-- Torznab Categories (static reference data)
-- ============================================================================

CREATE TABLE IF NOT EXISTS torznab_categories (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    parent_id INTEGER REFERENCES torznab_categories(id),
    description TEXT
);

INSERT OR IGNORE INTO torznab_categories (id, name, parent_id, description) VALUES
    (1000, 'Console', NULL, 'Console games'),
    (2000, 'Movies', NULL, 'Movies'),
    (3000, 'Audio', NULL, 'Audio/Music'),
    (4000, 'PC', NULL, 'PC software and games'),
    (5000, 'TV', NULL, 'TV shows'),
    (6000, 'XXX', NULL, 'Adult content'),
    (7000, 'Books', NULL, 'Books and comics'),
    (8000, 'Other', NULL, 'Other/Misc'),
    (2010, 'Movies/Foreign', 2000, 'Foreign movies'),
    (2020, 'Movies/Other', 2000, 'Other movies'),
    (2030, 'Movies/SD', 2000, 'SD quality movies'),
    (2040, 'Movies/HD', 2000, 'HD quality movies'),
    (2045, 'Movies/UHD', 2000, '4K/UHD movies'),
    (2050, 'Movies/BluRay', 2000, 'BluRay movies'),
    (2060, 'Movies/3D', 2000, '3D movies'),
    (2070, 'Movies/DVD', 2000, 'DVD movies'),
    (2080, 'Movies/WEB-DL', 2000, 'WEB-DL movies'),
    (5010, 'TV/WEB-DL', 5000, 'WEB-DL TV shows'),
    (5020, 'TV/Foreign', 5000, 'Foreign TV shows'),
    (5030, 'TV/SD', 5000, 'SD TV shows'),
    (5040, 'TV/HD', 5000, 'HD TV shows'),
    (5045, 'TV/UHD', 5000, '4K/UHD TV shows'),
    (5050, 'TV/Other', 5000, 'Other TV shows'),
    (5060, 'TV/Sport', 5000, 'Sports'),
    (5070, 'TV/Anime', 5000, 'Anime'),
    (5080, 'TV/Documentary', 5000, 'Documentaries'),
    (3010, 'Audio/MP3', 3000, 'MP3 audio'),
    (3020, 'Audio/Video', 3000, 'Music videos'),
    (3030, 'Audio/Audiobook', 3000, 'Audiobooks'),
    (3040, 'Audio/Lossless', 3000, 'Lossless audio'),
    (3050, 'Audio/Other', 3000, 'Other audio'),
    (3060, 'Audio/Foreign', 3000, 'Foreign audio'),
    (7010, 'Books/Mags', 7000, 'Magazines'),
    (7020, 'Books/EBook', 7000, 'E-Books'),
    (7030, 'Books/Comics', 7000, 'Comics'),
    (7040, 'Books/Technical', 7000, 'Technical books'),
    (7050, 'Books/Other', 7000, 'Other books'),
    (7060, 'Books/Foreign', 7000, 'Foreign books'),
    (4010, 'PC/0day', 4000, '0-day releases'),
    (4020, 'PC/ISO', 4000, 'ISO images'),
    (4030, 'PC/Mac', 4000, 'Mac software'),
    (4040, 'PC/Mobile-Other', 4000, 'Mobile software'),
    (4050, 'PC/Games', 4000, 'PC games'),
    (4060, 'PC/Mobile-iOS', 4000, 'iOS apps'),
    (4070, 'PC/Mobile-Android', 4000, 'Android apps'),
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
    (1180, 'Console/Switch', 1000, 'Nintendo Switch');
