-- Add 'available' status to episodes for RSS-matched episodes
-- Add torrent_link column to store where to download from

-- Update the status check constraint to include 'available'
ALTER TABLE episodes DROP CONSTRAINT IF EXISTS episodes_status_check;
ALTER TABLE episodes ADD CONSTRAINT episodes_status_check 
    CHECK (status IN ('missing', 'wanted', 'available', 'downloading', 'downloaded', 'ignored'));

-- Add torrent_link column to store the download URL
ALTER TABLE episodes 
ADD COLUMN IF NOT EXISTS torrent_link TEXT,
ADD COLUMN IF NOT EXISTS torrent_link_added_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS matched_rss_item_id UUID REFERENCES rss_feed_items(id) ON DELETE SET NULL;

-- Index for finding available episodes that need downloading
CREATE INDEX IF NOT EXISTS idx_episodes_available 
    ON episodes(status) 
    WHERE status = 'available';

-- Comment for documentation
COMMENT ON COLUMN episodes.torrent_link IS 'URL/magnet link to download this episode from RSS feed match';
COMMENT ON COLUMN episodes.torrent_link_added_at IS 'When the torrent link was found in RSS feed';
COMMENT ON COLUMN episodes.matched_rss_item_id IS 'Reference to the RSS feed item that matched this episode';
