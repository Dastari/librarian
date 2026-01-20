-- Migration: Drop legacy torrent linking columns
-- 
-- These columns have been replaced by the file-level matching system
-- in the torrent_file_matches table. The new system allows:
-- - Individual files to be matched to different episodes/movies/tracks
-- - Multi-episode and season pack torrents to work correctly
-- - Better tracking of match confidence and processing status
--
-- Before running this migration, ensure all code has been updated to use
-- torrent_file_matches instead of these legacy columns.

-- Drop the legacy linking columns from torrents table
ALTER TABLE torrents DROP COLUMN IF EXISTS library_id;
ALTER TABLE torrents DROP COLUMN IF EXISTS episode_id;
ALTER TABLE torrents DROP COLUMN IF EXISTS movie_id;
ALTER TABLE torrents DROP COLUMN IF EXISTS track_id;
ALTER TABLE torrents DROP COLUMN IF EXISTS album_id;
ALTER TABLE torrents DROP COLUMN IF EXISTS audiobook_id;

-- Note: source_feed_id is kept as it's used for RSS feed tracking
-- Note: source_indexer_id is kept as it's used for indexer tracking
