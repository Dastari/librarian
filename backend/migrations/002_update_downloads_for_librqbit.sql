-- Migration: Update downloads table for librqbit integration
-- This replaces the qBittorrent-based schema with native librqbit support

-- Drop the old downloads table if it exists and recreate with new schema
DROP TABLE IF EXISTS downloads;

-- Torrents table - tracks active and completed downloads
CREATE TABLE torrents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    
    -- Torrent identification
    info_hash VARCHAR(40) NOT NULL,  -- 40-char hex string
    magnet_uri TEXT,                  -- Original magnet link for re-adding
    
    -- Display info
    name VARCHAR(500) NOT NULL,
    
    -- Status tracking
    state VARCHAR(50) NOT NULL DEFAULT 'queued',
    progress REAL NOT NULL DEFAULT 0,  -- 0.0 to 1.0
    
    -- Size info
    total_bytes BIGINT NOT NULL DEFAULT 0,
    downloaded_bytes BIGINT NOT NULL DEFAULT 0,
    uploaded_bytes BIGINT NOT NULL DEFAULT 0,
    
    -- Path info
    save_path TEXT NOT NULL,
    
    -- Link to media if identified
    media_item_id UUID REFERENCES media_items(id) ON DELETE SET NULL,
    subscription_id UUID REFERENCES subscriptions(id) ON DELETE SET NULL,
    
    -- Timestamps
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    
    -- Unique constraint on info_hash per user
    CONSTRAINT torrents_user_infohash_unique UNIQUE (user_id, info_hash)
);

-- Indexes
CREATE INDEX idx_torrents_user ON torrents(user_id);
CREATE INDEX idx_torrents_state ON torrents(state);
CREATE INDEX idx_torrents_info_hash ON torrents(info_hash);

-- Enable RLS
ALTER TABLE torrents ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view own torrents"
    ON torrents FOR SELECT
    USING (auth.uid() = user_id);

CREATE POLICY "Users can insert own torrents"
    ON torrents FOR INSERT
    WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can update own torrents"
    ON torrents FOR UPDATE
    USING (auth.uid() = user_id);

CREATE POLICY "Users can delete own torrents"
    ON torrents FOR DELETE
    USING (auth.uid() = user_id);

-- Service role can do everything (for backend operations)
CREATE POLICY "Service role has full access"
    ON torrents FOR ALL
    USING (auth.role() = 'service_role');
