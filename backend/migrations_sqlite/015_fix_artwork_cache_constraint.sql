-- Fix artwork_cache CHECK constraint to accept plural artwork types
-- The code stores 'posters', 'backdrops', etc. but the original constraint only allowed singular values

-- SQLite requires recreating the table to change CHECK constraints
-- Create new table with corrected constraint
CREATE TABLE IF NOT EXISTS artwork_cache_new (
    id TEXT PRIMARY KEY,
    -- Entity identification
    entity_type TEXT NOT NULL CHECK (entity_type IN ('show', 'movie', 'episode', 'album', 'artist', 'audiobook')),
    entity_id TEXT NOT NULL,
    artwork_type TEXT NOT NULL CHECK (artwork_type IN ('poster', 'posters', 'backdrop', 'backdrops', 'thumbnail', 'thumbnails', 'banner', 'banners', 'cover')),
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

-- Copy any existing data (there likely isn't any due to the constraint bug)
INSERT OR IGNORE INTO artwork_cache_new 
SELECT * FROM artwork_cache;

-- Drop old table and rename new one
DROP TABLE IF EXISTS artwork_cache;
ALTER TABLE artwork_cache_new RENAME TO artwork_cache;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_artwork_cache_lookup ON artwork_cache(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_artwork_cache_hash ON artwork_cache(content_hash);
