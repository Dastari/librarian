-- Fix usenet_downloads table schema to match PostgreSQL and Rust code
-- SQLite 3.25.0+ supports ALTER TABLE RENAME COLUMN

-- Rename 'name' to 'nzb_name'
ALTER TABLE usenet_downloads RENAME COLUMN name TO nzb_name;

-- Rename 'total_bytes' to 'size_bytes'
ALTER TABLE usenet_downloads RENAME COLUMN total_bytes TO size_bytes;

-- Add missing columns
ALTER TABLE usenet_downloads ADD COLUMN nzb_hash TEXT;
ALTER TABLE usenet_downloads ADD COLUMN download_speed INTEGER DEFAULT 0;
ALTER TABLE usenet_downloads ADD COLUMN eta_seconds INTEGER;
ALTER TABLE usenet_downloads ADD COLUMN error_message TEXT;
ALTER TABLE usenet_downloads ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE usenet_downloads ADD COLUMN episode_id TEXT;
ALTER TABLE usenet_downloads ADD COLUMN movie_id TEXT;
ALTER TABLE usenet_downloads ADD COLUMN album_id TEXT;
ALTER TABLE usenet_downloads ADD COLUMN audiobook_id TEXT;
ALTER TABLE usenet_downloads ADD COLUMN indexer_id TEXT;
ALTER TABLE usenet_downloads ADD COLUMN updated_at TEXT NOT NULL DEFAULT (datetime('now'));

-- Rename 'added_at' to 'created_at' if it exists (match PostgreSQL naming)
-- Note: SQLite errors on missing columns are non-fatal for this operation
ALTER TABLE usenet_downloads RENAME COLUMN added_at TO created_at;

-- Create indexes for the new columns
CREATE INDEX IF NOT EXISTS idx_usenet_downloads_episode ON usenet_downloads(episode_id);
CREATE INDEX IF NOT EXISTS idx_usenet_downloads_movie ON usenet_downloads(movie_id);
CREATE INDEX IF NOT EXISTS idx_usenet_downloads_album ON usenet_downloads(album_id);
CREATE INDEX IF NOT EXISTS idx_usenet_downloads_audiobook ON usenet_downloads(audiobook_id);
CREATE INDEX IF NOT EXISTS idx_usenet_downloads_indexer ON usenet_downloads(indexer_id);
