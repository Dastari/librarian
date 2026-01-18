-- Migration: Add music tables for music library support
-- Artists, Albums, and Tracks

-- ============================================================================
-- Artists Table
-- ============================================================================

CREATE TABLE IF NOT EXISTS artists (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    -- Basic info
    name VARCHAR(500) NOT NULL,
    sort_name VARCHAR(500),
    -- External IDs
    musicbrainz_id UUID,
    -- Metadata
    bio TEXT,
    disambiguation VARCHAR(500),
    -- Artwork
    image_url TEXT,
    -- Stats (cached)
    album_count INTEGER DEFAULT 0,
    track_count INTEGER DEFAULT 0,
    total_duration_secs INTEGER DEFAULT 0,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Constraints
    UNIQUE(library_id, musicbrainz_id)
);

CREATE INDEX idx_artists_library ON artists(library_id);
CREATE INDEX idx_artists_user ON artists(user_id);
CREATE INDEX idx_artists_musicbrainz ON artists(musicbrainz_id) WHERE musicbrainz_id IS NOT NULL;
CREATE INDEX idx_artists_name ON artists(library_id, LOWER(name));

CREATE TRIGGER set_updated_at_artists
    BEFORE UPDATE ON artists
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE artists IS 'Music artists in a library';
COMMENT ON COLUMN artists.musicbrainz_id IS 'MusicBrainz artist MBID';
COMMENT ON COLUMN artists.disambiguation IS 'MusicBrainz disambiguation text to distinguish artists with same name';

-- ============================================================================
-- Albums Table
-- ============================================================================

CREATE TABLE IF NOT EXISTS albums (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    artist_id UUID NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    -- Basic info
    name VARCHAR(500) NOT NULL,
    sort_name VARCHAR(500),
    year INTEGER,
    -- External IDs
    musicbrainz_id UUID,
    -- Metadata
    album_type VARCHAR(50) DEFAULT 'album' CHECK (album_type IN ('album', 'single', 'ep', 'compilation', 'soundtrack', 'live', 'remix', 'other')),
    genres TEXT[] DEFAULT '{}',
    label VARCHAR(255),
    country VARCHAR(10),
    release_date DATE,
    -- Artwork
    cover_url TEXT,
    -- Stats (cached)
    track_count INTEGER DEFAULT 0,
    disc_count INTEGER DEFAULT 1,
    total_duration_secs INTEGER DEFAULT 0,
    -- File status
    has_files BOOLEAN NOT NULL DEFAULT false,
    size_bytes BIGINT DEFAULT 0,
    -- Path within library
    path TEXT,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Constraints
    UNIQUE(artist_id, musicbrainz_id)
);

CREATE INDEX idx_albums_library ON albums(library_id);
CREATE INDEX idx_albums_artist ON albums(artist_id);
CREATE INDEX idx_albums_user ON albums(user_id);
CREATE INDEX idx_albums_musicbrainz ON albums(musicbrainz_id) WHERE musicbrainz_id IS NOT NULL;
CREATE INDEX idx_albums_year ON albums(year) WHERE year IS NOT NULL;
CREATE INDEX idx_albums_name ON albums(library_id, LOWER(name));

CREATE TRIGGER set_updated_at_albums
    BEFORE UPDATE ON albums
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE albums IS 'Music albums in a library';
COMMENT ON COLUMN albums.musicbrainz_id IS 'MusicBrainz release group or release MBID';
COMMENT ON COLUMN albums.album_type IS 'Type of release: album, single, ep, compilation, soundtrack, live, remix, other';

-- ============================================================================
-- Tracks Table
-- ============================================================================

CREATE TABLE IF NOT EXISTS tracks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    album_id UUID NOT NULL REFERENCES albums(id) ON DELETE CASCADE,
    library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    -- Basic info
    title VARCHAR(500) NOT NULL,
    track_number INTEGER NOT NULL,
    disc_number INTEGER DEFAULT 1,
    -- External IDs
    musicbrainz_id UUID,
    isrc VARCHAR(20),
    -- Metadata
    duration_secs INTEGER,
    explicit BOOLEAN DEFAULT false,
    -- Artist info (for featured artists, differs from album artist)
    artist_name VARCHAR(500),
    artist_id UUID REFERENCES artists(id) ON DELETE SET NULL,
    -- File link
    media_file_id UUID REFERENCES media_files(id) ON DELETE SET NULL,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tracks_album ON tracks(album_id);
CREATE INDEX idx_tracks_library ON tracks(library_id);
CREATE INDEX idx_tracks_musicbrainz ON tracks(musicbrainz_id) WHERE musicbrainz_id IS NOT NULL;
CREATE INDEX idx_tracks_media_file ON tracks(media_file_id) WHERE media_file_id IS NOT NULL;
CREATE INDEX idx_tracks_album_order ON tracks(album_id, disc_number, track_number);

CREATE TRIGGER set_updated_at_tracks
    BEFORE UPDATE ON tracks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE tracks IS 'Individual tracks/songs in an album';
COMMENT ON COLUMN tracks.artist_name IS 'Track artist (may differ from album artist for compilations)';
COMMENT ON COLUMN tracks.isrc IS 'International Standard Recording Code';

-- ============================================================================
-- Add track_id and album_id to media_files
-- ============================================================================

ALTER TABLE media_files ADD COLUMN IF NOT EXISTS track_id UUID REFERENCES tracks(id) ON DELETE SET NULL;
ALTER TABLE media_files ADD COLUMN IF NOT EXISTS album_id UUID REFERENCES albums(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_media_files_track ON media_files(track_id) WHERE track_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_media_files_album ON media_files(album_id) WHERE album_id IS NOT NULL;

COMMENT ON COLUMN media_files.track_id IS 'Link to track record if this file is a music track';
COMMENT ON COLUMN media_files.album_id IS 'Link to album for grouping music files';
