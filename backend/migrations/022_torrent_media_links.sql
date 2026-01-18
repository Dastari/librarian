-- Add foreign keys to torrents table for linking to different media types
-- This allows the download monitor to know what type of content a torrent contains
-- and how to process/organize it after download completes.

-- Add movie_id for linking torrents to movies
ALTER TABLE torrents 
ADD COLUMN IF NOT EXISTS movie_id UUID REFERENCES movies(id) ON DELETE SET NULL;

-- Add track_id for linking torrents to music tracks
ALTER TABLE torrents 
ADD COLUMN IF NOT EXISTS track_id UUID REFERENCES tracks(id) ON DELETE SET NULL;

-- Add album_id for linking torrents to music albums (full album downloads)
ALTER TABLE torrents 
ADD COLUMN IF NOT EXISTS album_id UUID REFERENCES albums(id) ON DELETE SET NULL;

-- Add audiobook_id for linking torrents to audiobooks
ALTER TABLE torrents 
ADD COLUMN IF NOT EXISTS audiobook_id UUID REFERENCES audiobooks(id) ON DELETE SET NULL;

-- Create indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_torrents_movie_id ON torrents(movie_id) WHERE movie_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_torrents_track_id ON torrents(track_id) WHERE track_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_torrents_album_id ON torrents(album_id) WHERE album_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_torrents_audiobook_id ON torrents(audiobook_id) WHERE audiobook_id IS NOT NULL;

-- Comment on columns for documentation
COMMENT ON COLUMN torrents.movie_id IS 'Foreign key to movies table for movie downloads';
COMMENT ON COLUMN torrents.track_id IS 'Foreign key to tracks table for single track downloads';
COMMENT ON COLUMN torrents.album_id IS 'Foreign key to albums table for full album downloads';
COMMENT ON COLUMN torrents.audiobook_id IS 'Foreign key to audiobooks table for audiobook downloads';
