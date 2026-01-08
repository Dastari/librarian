-- Initial database schema for Librarian
-- Run with: sqlx migrate run

-- Libraries table
-- Supports multiple library types: movies, tv, music, audiobooks, etc.
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
    scan_interval_hours INTEGER NOT NULL DEFAULT 24,
    last_scanned_at TIMESTAMPTZ,
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for faster library lookups by user
CREATE INDEX idx_libraries_user ON libraries(user_id);

-- Enable RLS on libraries
ALTER TABLE libraries ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view own libraries"
    ON libraries FOR SELECT
    USING (auth.uid() = user_id);

CREATE POLICY "Users can create own libraries"
    ON libraries FOR INSERT
    WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can update own libraries"
    ON libraries FOR UPDATE
    USING (auth.uid() = user_id);

CREATE POLICY "Users can delete own libraries"
    ON libraries FOR DELETE
    USING (auth.uid() = user_id);

-- Media items table (movies and episodes)
CREATE TABLE IF NOT EXISTS media_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_type VARCHAR(50) NOT NULL CHECK (media_type IN ('movie', 'episode')),
    title VARCHAR(500) NOT NULL,
    year INTEGER,
    -- TV-specific fields
    show_id UUID REFERENCES media_items(id) ON DELETE SET NULL,
    season INTEGER,
    episode INTEGER,
    -- External IDs
    tvdb_id INTEGER,
    tmdb_id INTEGER,
    imdb_id VARCHAR(20),
    -- Metadata
    overview TEXT,
    runtime INTEGER, -- in minutes
    rating DECIMAL(3, 1),
    aired_date DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_media_items_tvdb ON media_items(tvdb_id) WHERE tvdb_id IS NOT NULL;
CREATE INDEX idx_media_items_tmdb ON media_items(tmdb_id) WHERE tmdb_id IS NOT NULL;
CREATE INDEX idx_media_items_show ON media_items(show_id) WHERE show_id IS NOT NULL;
CREATE INDEX idx_media_items_type ON media_items(media_type);

-- Media files table
CREATE TABLE IF NOT EXISTS media_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_item_id UUID REFERENCES media_items(id) ON DELETE CASCADE,
    library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    path TEXT NOT NULL UNIQUE,
    size BIGINT NOT NULL,
    -- Media properties from ffprobe
    container VARCHAR(50),
    video_codec VARCHAR(50),
    audio_codec VARCHAR(50),
    width INTEGER,
    height INTEGER,
    duration INTEGER, -- in seconds
    bitrate INTEGER,
    -- File tracking
    file_hash VARCHAR(64),
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    modified_at TIMESTAMPTZ
);

CREATE INDEX idx_media_files_library ON media_files(library_id);
CREATE INDEX idx_media_files_media_item ON media_files(media_item_id) WHERE media_item_id IS NOT NULL;

-- Artwork table
CREATE TABLE IF NOT EXISTS artwork (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_item_id UUID NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    kind VARCHAR(50) NOT NULL CHECK (kind IN ('poster', 'backdrop', 'thumb', 'banner')),
    storage_key TEXT NOT NULL,
    width INTEGER,
    height INTEGER,
    source VARCHAR(50), -- 'tvdb', 'tmdb', 'local'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_artwork_media_item ON artwork(media_item_id);

-- Quality profiles table
CREATE TABLE IF NOT EXISTS quality_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    rules JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE quality_profiles ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can manage own quality profiles"
    ON quality_profiles FOR ALL
    USING (auth.uid() = user_id);

-- Subscriptions table
CREATE TABLE IF NOT EXISTS subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    show_name VARCHAR(500) NOT NULL,
    show_tvdb_id INTEGER NOT NULL,
    quality_profile_id UUID REFERENCES quality_profiles(id) ON DELETE SET NULL,
    monitored BOOLEAN NOT NULL DEFAULT true,
    -- Track what we're looking for
    latest_wanted_season INTEGER,
    latest_wanted_episode INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE subscriptions ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can manage own subscriptions"
    ON subscriptions FOR ALL
    USING (auth.uid() = user_id);

CREATE INDEX idx_subscriptions_user ON subscriptions(user_id);
CREATE INDEX idx_subscriptions_tvdb ON subscriptions(show_tvdb_id);

-- Downloads table (tracks qBittorrent torrents)
CREATE TABLE IF NOT EXISTS downloads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    qbittorrent_hash VARCHAR(64) NOT NULL UNIQUE,
    name VARCHAR(500),
    state VARCHAR(50) NOT NULL,
    progress DECIMAL(5, 2) NOT NULL DEFAULT 0,
    size BIGINT,
    -- Link to media if identified
    media_item_id UUID REFERENCES media_items(id) ON DELETE SET NULL,
    subscription_id UUID REFERENCES subscriptions(id) ON DELETE SET NULL,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

ALTER TABLE downloads ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can manage own downloads"
    ON downloads FOR ALL
    USING (auth.uid() = user_id);

CREATE INDEX idx_downloads_user ON downloads(user_id);
CREATE INDEX idx_downloads_state ON downloads(state);

-- Jobs table (for background tasks)
CREATE TABLE IF NOT EXISTS jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kind VARCHAR(100) NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}',
    state VARCHAR(50) NOT NULL DEFAULT 'pending' CHECK (state IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    priority INTEGER NOT NULL DEFAULT 0,
    run_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_jobs_state_run_at ON jobs(state, run_at) WHERE state = 'pending';
CREATE INDEX idx_jobs_kind ON jobs(kind);

-- Events table (audit log)
CREATE TABLE IF NOT EXISTS events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID,
    event_type VARCHAR(100) NOT NULL,
    entity_type VARCHAR(100),
    entity_id UUID,
    payload JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_events_user ON events(user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_events_type ON events(event_type);
CREATE INDEX idx_events_entity ON events(entity_type, entity_id) WHERE entity_type IS NOT NULL;

-- Updated at trigger function
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply updated_at triggers
CREATE TRIGGER set_updated_at_libraries
    BEFORE UPDATE ON libraries
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER set_updated_at_media_items
    BEFORE UPDATE ON media_items
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER set_updated_at_quality_profiles
    BEFORE UPDATE ON quality_profiles
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER set_updated_at_subscriptions
    BEFORE UPDATE ON subscriptions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
