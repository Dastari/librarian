-- Re-add created_at column to torrents table
-- This was lost when migration 010 recreated the table without including it

-- Add the column with a constant default (SQLite doesn't allow non-constant defaults in ALTER TABLE)
ALTER TABLE torrents ADD COLUMN created_at TEXT DEFAULT '';

-- Backfill all rows: set created_at = added_at
UPDATE torrents SET created_at = added_at;

-- Create index for the new column
CREATE INDEX IF NOT EXISTS idx_torrents_created_at ON torrents(created_at);
