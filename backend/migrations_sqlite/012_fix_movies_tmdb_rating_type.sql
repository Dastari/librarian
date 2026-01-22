-- Fix movies table tmdb_rating column type from REAL to TEXT
-- SQLite requires table recreation to change column types

-- Step 1: Create new table with correct schema
CREATE TABLE IF NOT EXISTS movies_new (
    id TEXT PRIMARY KEY,
    library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    -- Basic info
    title TEXT NOT NULL,
    sort_title TEXT,
    original_title TEXT,
    year INTEGER,
    -- External IDs
    tmdb_id INTEGER,
    imdb_id TEXT,
    -- Metadata
    overview TEXT,
    tagline TEXT,
    runtime INTEGER,
    genres TEXT DEFAULT '[]',
    production_countries TEXT DEFAULT '[]',
    spoken_languages TEXT DEFAULT '[]',
    -- Credits
    director TEXT,
    cast_names TEXT DEFAULT '[]',
    -- Ratings (TEXT for Decimal storage in SQLite)
    tmdb_rating TEXT,
    tmdb_vote_count INTEGER,
    -- Artwork URLs
    poster_url TEXT,
    backdrop_url TEXT,
    -- Collection info
    collection_id INTEGER,
    collection_name TEXT,
    collection_poster_url TEXT,
    -- Release info
    release_date TEXT,
    certification TEXT,
    -- Status/monitoring
    status TEXT DEFAULT 'unknown' CHECK (status IN ('released', 'upcoming', 'announced', 'in_production', 'unknown')),
    monitored INTEGER NOT NULL DEFAULT 1,
    -- Download status
    download_status TEXT DEFAULT 'wanted' CHECK (download_status IN ('wanted', 'downloading', 'downloaded', 'ignored')),
    has_file INTEGER NOT NULL DEFAULT 0,
    -- Media file link
    media_file_id TEXT REFERENCES media_files(id) ON DELETE SET NULL,
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Step 2: Copy existing data (convert REAL to TEXT)
INSERT OR IGNORE INTO movies_new (
    id, library_id, user_id, title, sort_title, original_title, year,
    tmdb_id, imdb_id, overview, tagline, runtime, genres,
    production_countries, spoken_languages, director, cast_names,
    tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
    collection_id, collection_name, collection_poster_url,
    release_date, certification, status, monitored, media_file_id,
    created_at, updated_at
)
SELECT 
    id, library_id, user_id, title, sort_title, original_title, year,
    tmdb_id, imdb_id, overview, tagline, runtime, genres,
    production_countries, spoken_languages, director, cast_names,
    CAST(tmdb_rating AS TEXT), tmdb_vote_count, poster_url, backdrop_url,
    collection_id, collection_name, collection_poster_url,
    release_date, certification, status, monitored, media_file_id,
    created_at, updated_at
FROM movies;

-- Step 3: Drop old table
DROP TABLE IF EXISTS movies;

-- Step 4: Rename new table
ALTER TABLE movies_new RENAME TO movies;

-- Step 5: Recreate indexes
CREATE INDEX IF NOT EXISTS idx_movies_library ON movies(library_id);
CREATE INDEX IF NOT EXISTS idx_movies_user ON movies(user_id);
CREATE INDEX IF NOT EXISTS idx_movies_tmdb ON movies(tmdb_id);
CREATE INDEX IF NOT EXISTS idx_movies_imdb ON movies(imdb_id);
CREATE INDEX IF NOT EXISTS idx_movies_download_status ON movies(download_status);

-- Also create the UNIQUE constraints as indexes (SQLite doesn't preserve them during recreation)
CREATE UNIQUE INDEX IF NOT EXISTS idx_movies_library_tmdb ON movies(library_id, tmdb_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_movies_library_imdb ON movies(library_id, imdb_id);
