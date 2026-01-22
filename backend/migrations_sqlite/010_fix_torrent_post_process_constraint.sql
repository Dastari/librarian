-- Fix torrents table post_process_status CHECK constraint
-- SQLite requires table recreation to modify constraints

-- Step 1: Create new table with updated constraint
CREATE TABLE IF NOT EXISTS torrents_new (
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
    -- Source tracking
    source_feed_id TEXT,
    source_indexer_id TEXT,
    -- Library/content links
    library_id TEXT REFERENCES libraries(id) ON DELETE SET NULL,
    -- Post-processing (updated constraint with all valid values)
    post_process_status TEXT DEFAULT 'pending' CHECK (post_process_status IN ('pending', 'processing', 'completed', 'failed', 'skipped', 'error', 'matched', 'unmatched', 'partial')),
    post_process_error TEXT,
    processed_at TEXT,
    -- Excluded files (JSON array of file indices)
    excluded_files TEXT DEFAULT '[]',
    -- Timestamps
    added_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    -- Unique constraint
    UNIQUE(info_hash)
);

-- Step 2: Copy existing data
INSERT OR IGNORE INTO torrents_new (
    id, user_id, info_hash, magnet_uri, name, state, progress,
    total_bytes, downloaded_bytes, uploaded_bytes,
    save_path, download_path, source_url, source_feed_id, source_indexer_id,
    library_id, post_process_status, post_process_error, processed_at,
    excluded_files, added_at, completed_at
)
SELECT 
    id, user_id, info_hash, magnet_uri, name, state, progress,
    total_bytes, downloaded_bytes, uploaded_bytes,
    save_path, download_path, source_url, source_feed_id, source_indexer_id,
    library_id, post_process_status, post_process_error, processed_at,
    excluded_files, added_at, completed_at
FROM torrents;

-- Step 3: Drop old table (this will cascade to torrent_files due to FK)
DROP TABLE IF EXISTS torrents;

-- Step 4: Rename new table
ALTER TABLE torrents_new RENAME TO torrents;

-- Step 5: Recreate indexes
CREATE INDEX IF NOT EXISTS idx_torrents_user ON torrents(user_id);
CREATE INDEX IF NOT EXISTS idx_torrents_info_hash ON torrents(info_hash);
CREATE INDEX IF NOT EXISTS idx_torrents_state ON torrents(state);
CREATE INDEX IF NOT EXISTS idx_torrents_library ON torrents(library_id);
CREATE INDEX IF NOT EXISTS idx_torrents_source_feed ON torrents(source_feed_id);
CREATE INDEX IF NOT EXISTS idx_torrents_source_indexer ON torrents(source_indexer_id);
CREATE INDEX IF NOT EXISTS idx_torrents_post_process ON torrents(post_process_status);
