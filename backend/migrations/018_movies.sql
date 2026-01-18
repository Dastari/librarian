-- Migration: Add movies table for movie library support
-- Movies are single-file items (unlike TV shows with episodes)

-- ============================================================================
-- Movies Table
-- ============================================================================

CREATE TABLE IF NOT EXISTS movies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    -- Basic info
    title VARCHAR(500) NOT NULL,
    sort_title VARCHAR(500),
    original_title VARCHAR(500),
    year INTEGER,
    -- External IDs
    tmdb_id INTEGER,
    imdb_id VARCHAR(20),
    -- Metadata
    overview TEXT,
    tagline TEXT,
    runtime INTEGER, -- minutes
    genres TEXT[] DEFAULT '{}',
    production_countries TEXT[] DEFAULT '{}',
    spoken_languages TEXT[] DEFAULT '{}',
    -- Credits (simplified)
    director VARCHAR(255),
    cast_names TEXT[] DEFAULT '{}', -- Top billed cast
    -- Ratings
    tmdb_rating DECIMAL(3, 1),
    tmdb_vote_count INTEGER,
    -- Artwork URLs (cached to Supabase storage)
    poster_url TEXT,
    backdrop_url TEXT,
    -- Collection info (for movie series like "The Matrix Collection")
    collection_id INTEGER,
    collection_name VARCHAR(255),
    collection_poster_url TEXT,
    -- Release info
    release_date DATE,
    certification VARCHAR(20), -- PG-13, R, etc.
    -- Status/monitoring
    status VARCHAR(50) DEFAULT 'unknown' CHECK (status IN ('released', 'upcoming', 'announced', 'in_production', 'unknown')),
    monitored BOOLEAN NOT NULL DEFAULT true,
    -- Quality settings (mirrors TV show quality overrides)
    allowed_resolutions_override TEXT[] DEFAULT NULL,
    allowed_video_codecs_override TEXT[] DEFAULT NULL,
    allowed_audio_formats_override TEXT[] DEFAULT NULL,
    require_hdr_override BOOLEAN DEFAULT NULL,
    allowed_hdr_types_override TEXT[] DEFAULT NULL,
    allowed_sources_override TEXT[] DEFAULT NULL,
    release_group_blacklist_override TEXT[] DEFAULT NULL,
    release_group_whitelist_override TEXT[] DEFAULT NULL,
    -- File/download status
    has_file BOOLEAN NOT NULL DEFAULT false,
    size_bytes BIGINT DEFAULT 0,
    -- Path within library (optional subfolder)
    path TEXT,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Constraints
    UNIQUE(library_id, tmdb_id),
    UNIQUE(library_id, imdb_id)
);

CREATE INDEX idx_movies_library ON movies(library_id);
CREATE INDEX idx_movies_user ON movies(user_id);
CREATE INDEX idx_movies_tmdb ON movies(tmdb_id) WHERE tmdb_id IS NOT NULL;
CREATE INDEX idx_movies_imdb ON movies(imdb_id) WHERE imdb_id IS NOT NULL;
CREATE INDEX idx_movies_year ON movies(year) WHERE year IS NOT NULL;
CREATE INDEX idx_movies_monitored ON movies(library_id, monitored) WHERE monitored = true;
CREATE INDEX idx_movies_collection ON movies(collection_id) WHERE collection_id IS NOT NULL;
CREATE INDEX idx_movies_has_file ON movies(library_id, has_file);

CREATE TRIGGER set_updated_at_movies
    BEFORE UPDATE ON movies
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE movies IS 'Movies tracked in a library with monitoring settings';
COMMENT ON COLUMN movies.collection_id IS 'TMDB collection ID for movie series (e.g., The Matrix Collection)';
COMMENT ON COLUMN movies.has_file IS 'Whether a media file exists for this movie';

-- ============================================================================
-- Add movie_id to media_files
-- ============================================================================

ALTER TABLE media_files ADD COLUMN IF NOT EXISTS movie_id UUID REFERENCES movies(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_media_files_movie ON media_files(movie_id) WHERE movie_id IS NOT NULL;

COMMENT ON COLUMN media_files.movie_id IS 'Link to movie record if this file is a movie';

-- ============================================================================
-- Movie Collections (for grouping movies in a series)
-- ============================================================================

CREATE TABLE IF NOT EXISTS movie_collections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    -- TMDB collection info
    tmdb_collection_id INTEGER NOT NULL,
    name VARCHAR(255) NOT NULL,
    overview TEXT,
    poster_url TEXT,
    backdrop_url TEXT,
    -- Cached movie count
    movie_count INTEGER DEFAULT 0,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Constraints
    UNIQUE(library_id, tmdb_collection_id)
);

CREATE INDEX idx_movie_collections_library ON movie_collections(library_id);
CREATE INDEX idx_movie_collections_tmdb ON movie_collections(tmdb_collection_id);

CREATE TRIGGER set_updated_at_movie_collections
    BEFORE UPDATE ON movie_collections
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE movie_collections IS 'Movie collections/series (e.g., The Matrix Collection, MCU)';

-- ============================================================================
-- Movie Download History (for tracking available releases)
-- ============================================================================

CREATE TABLE IF NOT EXISTS movie_releases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    movie_id UUID NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    -- Release info (from indexer search)
    title TEXT NOT NULL,
    guid VARCHAR(255) NOT NULL,
    indexer_id UUID,
    -- Download link
    link TEXT,
    magnet_uri TEXT,
    info_hash VARCHAR(40),
    -- Release metadata
    size_bytes BIGINT,
    seeders INTEGER,
    leechers INTEGER,
    publish_date TIMESTAMPTZ,
    -- Parsed quality info
    resolution VARCHAR(20),
    source VARCHAR(50),
    video_codec VARCHAR(20),
    audio_codec VARCHAR(50),
    is_hdr BOOLEAN DEFAULT false,
    hdr_type VARCHAR(20),
    release_group VARCHAR(100),
    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'available' CHECK (status IN ('available', 'downloading', 'downloaded', 'rejected')),
    rejected_reason TEXT,
    -- Timestamps
    found_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    grabbed_at TIMESTAMPTZ,
    -- Unique per movie
    UNIQUE(movie_id, guid)
);

CREATE INDEX idx_movie_releases_movie ON movie_releases(movie_id);
CREATE INDEX idx_movie_releases_status ON movie_releases(status);
CREATE INDEX idx_movie_releases_found ON movie_releases(found_at DESC);

COMMENT ON TABLE movie_releases IS 'Available torrent releases for movies from indexer searches';
