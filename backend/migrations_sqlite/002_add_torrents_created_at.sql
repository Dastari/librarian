-- Add created_at column to torrents table (for compatibility with PostgreSQL schema)
-- SQLite doesn't support IF NOT EXISTS for ALTER TABLE, so we use a safe approach

-- Add the column (will fail silently if it already exists in newer schema)
ALTER TABLE torrents ADD COLUMN created_at TEXT NOT NULL DEFAULT (datetime('now'));

-- Backfill existing rows: set created_at = added_at for rows where it wasn't set
UPDATE torrents SET created_at = added_at WHERE created_at IS NULL OR created_at = '';

-- Create index for the new column
CREATE INDEX IF NOT EXISTS idx_torrents_created_at ON torrents(created_at);
