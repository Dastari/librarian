-- Application settings stored in the database
-- These settings control various aspects of the application behavior

CREATE TABLE IF NOT EXISTS app_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key VARCHAR(255) NOT NULL UNIQUE,
    value JSONB NOT NULL,
    description TEXT,
    category VARCHAR(100) NOT NULL DEFAULT 'general',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create updated_at trigger
CREATE TRIGGER set_updated_at_app_settings
    BEFORE UPDATE ON app_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Insert default torrent client settings
INSERT INTO app_settings (key, value, description, category) VALUES
    ('torrent.download_dir', '"/data/downloads"', 'Directory where torrents are downloaded to', 'torrent'),
    ('torrent.session_dir', '"/data/session"', 'Directory for torrent session data (resume info, DHT)', 'torrent'),
    ('torrent.enable_dht', 'true', 'Enable DHT for peer discovery', 'torrent'),
    ('torrent.listen_port', '6881', 'Port to listen for incoming torrent connections (0 = random)', 'torrent'),
    ('torrent.max_concurrent', '5', 'Maximum number of concurrent downloads', 'torrent'),
    ('torrent.upload_limit', '0', 'Upload speed limit in bytes/sec (0 = unlimited)', 'torrent'),
    ('torrent.download_limit', '0', 'Download speed limit in bytes/sec (0 = unlimited)', 'torrent')
ON CONFLICT (key) DO NOTHING;

-- Index for faster lookups
CREATE INDEX idx_app_settings_category ON app_settings(category);
CREATE INDEX idx_app_settings_key ON app_settings(key);
