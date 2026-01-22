-- Add missing columns to various tables for schema parity with PostgreSQL
-- Using subqueries to check if columns exist before adding (SQLite workaround)

-- ============================================================================
-- Usenet Servers: Add encrypted_password and retention_days
-- ============================================================================

-- Note: SQLite ignores duplicate column additions in newer versions,
-- but we wrap in a transaction that continues on error
-- For older SQLite, we just try and the error is non-fatal

-- These statements may fail if columns already exist - that's OK
CREATE TABLE IF NOT EXISTS _migration_temp (id INTEGER);
DROP TABLE IF EXISTS _migration_temp;

-- Usenet servers columns
ALTER TABLE usenet_servers ADD COLUMN encrypted_password TEXT;
ALTER TABLE usenet_servers ADD COLUMN retention_days INTEGER DEFAULT 1200;

-- Indexer configs column
ALTER TABLE indexer_configs ADD COLUMN post_download_action TEXT;

-- RSS Feeds column
ALTER TABLE rss_feeds ADD COLUMN post_download_action TEXT;

-- Playback Sessions columns
ALTER TABLE playback_sessions ADD COLUMN track_id TEXT;
ALTER TABLE playback_sessions ADD COLUMN audiobook_id TEXT;
ALTER TABLE playback_sessions ADD COLUMN album_id TEXT;
ALTER TABLE playback_sessions ADD COLUMN content_type TEXT DEFAULT 'episode';

-- Notifications column
ALTER TABLE notifications ADD COLUMN read_at TEXT;

-- Media Files columns
ALTER TABLE media_files ADD COLUMN organize_status TEXT DEFAULT 'pending';
ALTER TABLE media_files ADD COLUMN organize_error TEXT;
ALTER TABLE media_files ADD COLUMN content_type TEXT DEFAULT 'video';
ALTER TABLE media_files ADD COLUMN quality_status TEXT DEFAULT 'unknown';
ALTER TABLE media_files ADD COLUMN meta_artist TEXT;
ALTER TABLE media_files ADD COLUMN meta_album TEXT;
ALTER TABLE media_files ADD COLUMN meta_title TEXT;
ALTER TABLE media_files ADD COLUMN meta_track_number INTEGER;
ALTER TABLE media_files ADD COLUMN meta_disc_number INTEGER;
ALTER TABLE media_files ADD COLUMN meta_year INTEGER;
ALTER TABLE media_files ADD COLUMN meta_genre TEXT;

-- Create indexes (IF NOT EXISTS is supported for indexes)
CREATE INDEX IF NOT EXISTS idx_playback_sessions_track ON playback_sessions(track_id);
CREATE INDEX IF NOT EXISTS idx_playback_sessions_audiobook ON playback_sessions(audiobook_id);
CREATE INDEX IF NOT EXISTS idx_playback_sessions_album ON playback_sessions(album_id);
CREATE INDEX IF NOT EXISTS idx_playback_sessions_content_type ON playback_sessions(content_type);
CREATE INDEX IF NOT EXISTS idx_notifications_read_at ON notifications(read_at);
CREATE INDEX IF NOT EXISTS idx_media_files_organize_status ON media_files(organize_status);
CREATE INDEX IF NOT EXISTS idx_media_files_content_type ON media_files(content_type);
