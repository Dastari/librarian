-- Migration: Rename audiobook_chapters -> chapters and audiobook_authors -> authors
-- This simplifies the table names for consistency
-- Note: Existing data in chapters will be dropped as requested

-- ============================================================================
-- Drop and recreate authors table (replacing audiobook_authors)
-- ============================================================================

-- First, we need to update audiobooks to remove the foreign key
ALTER TABLE audiobooks DROP CONSTRAINT IF EXISTS audiobooks_author_id_fkey;

-- Make author_id nullable (it may have been NOT NULL)
ALTER TABLE audiobooks ALTER COLUMN author_id DROP NOT NULL;

-- Clear author_id references since we're dropping the old table
-- (existing author data will need to be re-fetched from metadata sources)
UPDATE audiobooks SET author_id = NULL WHERE author_id IS NOT NULL;

-- Drop old table and create new one
DROP TABLE IF EXISTS audiobook_authors CASCADE;

CREATE TABLE IF NOT EXISTS authors (
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

CREATE INDEX idx_authors_library ON authors(library_id);
CREATE INDEX idx_authors_user ON authors(user_id);
CREATE INDEX idx_authors_audible ON authors(audible_id) WHERE audible_id IS NOT NULL;
CREATE INDEX idx_authors_name ON authors(library_id, LOWER(name));

CREATE TRIGGER set_updated_at_authors
    BEFORE UPDATE ON authors
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE authors IS 'Audiobook authors in a library';

-- Re-add the foreign key to audiobooks
ALTER TABLE audiobooks 
ADD CONSTRAINT audiobooks_author_id_fkey 
FOREIGN KEY (author_id) REFERENCES authors(id) ON DELETE CASCADE;

-- ============================================================================
-- Drop and recreate chapters table (replacing audiobook_chapters)
-- ============================================================================

-- First, update torrent_file_matches to remove the foreign key
ALTER TABLE torrent_file_matches DROP CONSTRAINT IF EXISTS torrent_file_matches_audiobook_chapter_id_fkey;
ALTER TABLE torrent_file_matches DROP CONSTRAINT IF EXISTS torrent_file_matches_chapter_id_fkey;

-- Rename the column in torrent_file_matches (only if old name exists)
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns 
               WHERE table_name = 'torrent_file_matches' 
               AND column_name = 'audiobook_chapter_id') THEN
        ALTER TABLE torrent_file_matches RENAME COLUMN audiobook_chapter_id TO chapter_id;
    END IF;
END $$;

-- Drop old table and create new one
DROP TABLE IF EXISTS audiobook_chapters CASCADE;

CREATE TABLE IF NOT EXISTS chapters (
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
    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'missing' 
    CHECK (status IN ('missing', 'wanted', 'downloading', 'downloaded', 'ignored')),
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_chapters_audiobook ON chapters(audiobook_id);
CREATE INDEX idx_chapters_order ON chapters(audiobook_id, chapter_number);
CREATE INDEX idx_chapters_media_file ON chapters(media_file_id) WHERE media_file_id IS NOT NULL;
CREATE INDEX idx_chapters_status ON chapters(audiobook_id, status);

COMMENT ON TABLE chapters IS 'Chapter markers for audiobooks';
COMMENT ON COLUMN chapters.start_secs IS 'Start time in seconds from beginning';
COMMENT ON COLUMN chapters.status IS 'Download status for chapter-based audiobooks';

-- Re-add the foreign key to torrent_file_matches
ALTER TABLE torrent_file_matches 
ADD CONSTRAINT torrent_file_matches_chapter_id_fkey 
FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE SET NULL;

-- Update the index
DROP INDEX IF EXISTS idx_torrent_file_matches_chapter;
CREATE INDEX idx_torrent_file_matches_chapter ON torrent_file_matches(chapter_id) WHERE chapter_id IS NOT NULL;
