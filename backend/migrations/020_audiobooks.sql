-- Migration: Add audiobook tables for audiobook library support
-- Authors, Audiobooks, and Chapters

-- ============================================================================
-- Audiobook Authors Table
-- ============================================================================

CREATE TABLE IF NOT EXISTS audiobook_authors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    -- Basic info
    name VARCHAR(500) NOT NULL,
    sort_name VARCHAR(500),
    -- External IDs
    audible_id VARCHAR(50),
    openlibrary_id VARCHAR(50),
    goodreads_id VARCHAR(50),
    -- Metadata
    bio TEXT,
    image_url TEXT,
    -- Stats (cached)
    book_count INTEGER DEFAULT 0,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audiobook_authors_library ON audiobook_authors(library_id);
CREATE INDEX idx_audiobook_authors_user ON audiobook_authors(user_id);
CREATE INDEX idx_audiobook_authors_audible ON audiobook_authors(audible_id) WHERE audible_id IS NOT NULL;
CREATE INDEX idx_audiobook_authors_name ON audiobook_authors(library_id, LOWER(name));

CREATE TRIGGER set_updated_at_audiobook_authors
    BEFORE UPDATE ON audiobook_authors
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE audiobook_authors IS 'Audiobook authors in a library';

-- ============================================================================
-- Audiobooks Table
-- ============================================================================

CREATE TABLE IF NOT EXISTS audiobooks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    author_id UUID NOT NULL REFERENCES audiobook_authors(id) ON DELETE CASCADE,
    library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    -- Basic info
    title VARCHAR(500) NOT NULL,
    sort_title VARCHAR(500),
    subtitle VARCHAR(500),
    -- External IDs
    audible_id VARCHAR(50),
    asin VARCHAR(20),
    isbn VARCHAR(20),
    openlibrary_id VARCHAR(50),
    goodreads_id VARCHAR(50),
    -- Metadata
    description TEXT,
    publisher VARCHAR(255),
    publish_date DATE,
    language VARCHAR(10),
    -- Narrator(s)
    narrators TEXT[] DEFAULT '{}',
    -- Series info
    series_name VARCHAR(255),
    series_position DECIMAL(5, 2), -- Allows for x.5 positions
    -- Duration
    duration_secs INTEGER,
    -- Ratings
    audible_rating DECIMAL(3, 2),
    audible_rating_count INTEGER,
    -- Artwork
    cover_url TEXT,
    -- File status
    has_files BOOLEAN NOT NULL DEFAULT false,
    size_bytes BIGINT DEFAULT 0,
    -- Listening progress
    is_finished BOOLEAN DEFAULT false,
    last_played_at TIMESTAMPTZ,
    -- Path within library
    path TEXT,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Constraints
    UNIQUE(library_id, audible_id),
    UNIQUE(library_id, asin)
);

CREATE INDEX idx_audiobooks_library ON audiobooks(library_id);
CREATE INDEX idx_audiobooks_author ON audiobooks(author_id);
CREATE INDEX idx_audiobooks_user ON audiobooks(user_id);
CREATE INDEX idx_audiobooks_audible ON audiobooks(audible_id) WHERE audible_id IS NOT NULL;
CREATE INDEX idx_audiobooks_asin ON audiobooks(asin) WHERE asin IS NOT NULL;
CREATE INDEX idx_audiobooks_series ON audiobooks(series_name, series_position) WHERE series_name IS NOT NULL;
CREATE INDEX idx_audiobooks_has_files ON audiobooks(library_id, has_files);

CREATE TRIGGER set_updated_at_audiobooks
    BEFORE UPDATE ON audiobooks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE audiobooks IS 'Audiobooks in a library';
COMMENT ON COLUMN audiobooks.series_position IS 'Position in series (decimal allows for x.5 entries)';

-- ============================================================================
-- Audiobook Chapters Table
-- ============================================================================

CREATE TABLE IF NOT EXISTS audiobook_chapters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    audiobook_id UUID NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
    -- Chapter info
    chapter_number INTEGER NOT NULL,
    title VARCHAR(500),
    -- Timing
    start_secs INTEGER NOT NULL,
    end_secs INTEGER NOT NULL,
    duration_secs INTEGER GENERATED ALWAYS AS (end_secs - start_secs) STORED,
    -- If chapter is in a separate file
    media_file_id UUID REFERENCES media_files(id) ON DELETE SET NULL,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audiobook_chapters_audiobook ON audiobook_chapters(audiobook_id);
CREATE INDEX idx_audiobook_chapters_order ON audiobook_chapters(audiobook_id, chapter_number);
CREATE INDEX idx_audiobook_chapters_media_file ON audiobook_chapters(media_file_id) WHERE media_file_id IS NOT NULL;

COMMENT ON TABLE audiobook_chapters IS 'Chapter markers for audiobooks';
COMMENT ON COLUMN audiobook_chapters.start_secs IS 'Start time in seconds from beginning';

-- ============================================================================
-- Audiobook Listening Progress Table
-- ============================================================================

CREATE TABLE IF NOT EXISTS audiobook_progress (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    audiobook_id UUID NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    -- Progress
    current_position_secs INTEGER NOT NULL DEFAULT 0,
    chapter_number INTEGER,
    -- Speed preference
    playback_speed DECIMAL(3, 2) DEFAULT 1.0,
    -- Timestamps
    last_played_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- One progress record per user per audiobook
    UNIQUE(audiobook_id, user_id)
);

CREATE INDEX idx_audiobook_progress_user ON audiobook_progress(user_id);
CREATE INDEX idx_audiobook_progress_audiobook ON audiobook_progress(audiobook_id);
CREATE INDEX idx_audiobook_progress_last_played ON audiobook_progress(user_id, last_played_at DESC);

CREATE TRIGGER set_updated_at_audiobook_progress
    BEFORE UPDATE ON audiobook_progress
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE audiobook_progress IS 'User listening progress for audiobooks';
COMMENT ON COLUMN audiobook_progress.playback_speed IS 'User-preferred playback speed (1.0 = normal)';

-- ============================================================================
-- Add audiobook_id to media_files
-- ============================================================================

ALTER TABLE media_files ADD COLUMN IF NOT EXISTS audiobook_id UUID REFERENCES audiobooks(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_media_files_audiobook ON media_files(audiobook_id) WHERE audiobook_id IS NOT NULL;

COMMENT ON COLUMN media_files.audiobook_id IS 'Link to audiobook record if this file is an audiobook';
