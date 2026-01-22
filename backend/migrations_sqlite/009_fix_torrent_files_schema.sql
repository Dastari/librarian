-- Fix torrent_files table schema to match PostgreSQL and Rust code
-- SQLite requires table recreation for complex schema changes

-- Step 1: Create the new table with correct schema
CREATE TABLE IF NOT EXISTS torrent_files_new (
    id TEXT PRIMARY KEY,
    torrent_id TEXT NOT NULL REFERENCES torrents(id) ON DELETE CASCADE,
    file_index INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    downloaded_bytes INTEGER NOT NULL DEFAULT 0,
    progress REAL NOT NULL DEFAULT 0,
    media_file_id TEXT,
    is_excluded INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(torrent_id, file_index)
);

-- Step 2: Copy data from old table (if it exists and has data)
-- Map: path -> file_path, size_bytes -> file_size, is_selected -> NOT is_excluded
INSERT OR IGNORE INTO torrent_files_new (
    id, torrent_id, file_index, file_path, relative_path, file_size,
    downloaded_bytes, progress, is_excluded, created_at, updated_at
)
SELECT 
    id, 
    torrent_id, 
    file_index, 
    path,           -- old 'path' becomes 'file_path'
    path,           -- use same value for 'relative_path' (best effort)
    size_bytes,     -- old 'size_bytes' becomes 'file_size'
    downloaded_bytes,
    CAST(downloaded_bytes AS REAL) / CASE WHEN size_bytes > 0 THEN size_bytes ELSE 1 END,  -- calculate progress
    CASE WHEN is_selected = 1 THEN 0 ELSE 1 END,  -- invert: is_selected=1 means is_excluded=0
    created_at,
    updated_at
FROM torrent_files;

-- Step 3: Drop old table
DROP TABLE IF EXISTS torrent_files;

-- Step 4: Rename new table
ALTER TABLE torrent_files_new RENAME TO torrent_files;

-- Step 5: Create indexes
CREATE INDEX IF NOT EXISTS idx_torrent_files_torrent ON torrent_files(torrent_id);
CREATE INDEX IF NOT EXISTS idx_torrent_files_media_file ON torrent_files(media_file_id);
