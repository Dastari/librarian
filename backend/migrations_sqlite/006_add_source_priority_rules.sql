-- Add source_priority_rules table for managing indexer/usenet priority ordering
-- This table was missing from the initial SQLite schema

CREATE TABLE IF NOT EXISTS source_priority_rules (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Scope: NULL = user default, library_type = type-wide, library_id = specific library
    library_type TEXT,  -- e.g., 'tv', 'movies', 'music', 'audiobooks'
    library_id TEXT REFERENCES libraries(id) ON DELETE CASCADE,
    -- JSON array of source references: [{"source_type": "torrent_indexer", "id": "xxx"}, ...]
    priority_order TEXT NOT NULL DEFAULT '[]',
    -- Whether to search all sources or stop at first result
    search_all_sources INTEGER NOT NULL DEFAULT 0,
    -- Whether this rule is active
    enabled INTEGER NOT NULL DEFAULT 1,
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    -- Unique constraint on scope (user + type + library)
    UNIQUE(user_id, library_type, library_id)
);

-- Indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_source_priority_rules_user ON source_priority_rules(user_id);
CREATE INDEX IF NOT EXISTS idx_source_priority_rules_type ON source_priority_rules(library_type);
CREATE INDEX IF NOT EXISTS idx_source_priority_rules_library ON source_priority_rules(library_id);
CREATE INDEX IF NOT EXISTS idx_source_priority_rules_enabled ON source_priority_rules(enabled);

-- Trigger to update updated_at on changes
CREATE TRIGGER IF NOT EXISTS update_source_priority_rules_updated_at
AFTER UPDATE ON source_priority_rules
FOR EACH ROW
BEGIN
    UPDATE source_priority_rules SET updated_at = datetime('now') WHERE id = NEW.id;
END;
