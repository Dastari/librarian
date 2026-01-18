-- Cast devices and sessions for Chromecast/AirPlay support
-- Migration: 014_cast_devices.sql

-- Saved cast devices (for manual entries and favorites)
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

-- Cast session history (for analytics/resume and active sessions)
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

-- Cast settings (global settings for casting)
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

-- Insert default cast settings
INSERT INTO cast_settings (id, auto_discovery_enabled, discovery_interval_seconds, default_volume, transcode_incompatible, preferred_quality)
VALUES (gen_random_uuid(), true, 30, 1.0, true, '1080p')
ON CONFLICT DO NOTHING;

-- Index for finding active sessions
CREATE INDEX IF NOT EXISTS idx_cast_sessions_device_active ON cast_sessions(device_id) WHERE ended_at IS NULL;

-- Index for device lookup by address
CREATE INDEX IF NOT EXISTS idx_cast_devices_address ON cast_devices(address);

-- Unique constraint on address handled above

-- Trigger to update updated_at timestamp
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

CREATE TRIGGER cast_sessions_updated_at
    BEFORE UPDATE ON cast_sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_cast_updated_at();

CREATE TRIGGER cast_settings_updated_at
    BEFORE UPDATE ON cast_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_cast_updated_at();
