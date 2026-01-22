-- Add missing source columns to torrents table for schema parity with PostgreSQL
-- These columns track the source of torrent downloads (RSS feed or indexer)

-- Add source_feed_id column (references RSS feed that added this torrent)
ALTER TABLE torrents ADD COLUMN source_feed_id TEXT;

-- Add source_indexer_id column (references indexer used to download this torrent)
ALTER TABLE torrents ADD COLUMN source_indexer_id TEXT;

-- Create indexes for source lookups
CREATE INDEX IF NOT EXISTS idx_torrents_source_feed ON torrents(source_feed_id);
CREATE INDEX IF NOT EXISTS idx_torrents_source_indexer ON torrents(source_indexer_id);
