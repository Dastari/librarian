-- Migration: Add library_type to naming patterns
-- Different library types need different naming pattern variables

-- Add library_type column to naming_patterns
ALTER TABLE naming_patterns ADD COLUMN IF NOT EXISTS library_type VARCHAR(50) DEFAULT 'tv';

-- Update existing patterns to be TV patterns
UPDATE naming_patterns SET library_type = 'tv' WHERE library_type IS NULL;

-- Create index for library type lookup
CREATE INDEX IF NOT EXISTS idx_naming_patterns_type ON naming_patterns(library_type);

-- Create composite index for type + default lookup
CREATE INDEX IF NOT EXISTS idx_naming_patterns_type_default ON naming_patterns(library_type, is_default) WHERE is_default = true;

-- ============================================================================
-- Movie Naming Patterns
-- Variables: {title}, {year}, {quality}, {ext}
-- ============================================================================

INSERT INTO naming_patterns (name, pattern, description, is_default, is_system, library_type) VALUES
('Movie Standard', '{title} ({year})/{title} ({year}).{ext}', 'Title (Year)/Title (Year).ext', true, true, 'movies'),
('Movie with Quality', '{title} ({year})/{title} ({year}) - {quality}.{ext}', 'Include quality in filename', false, true, 'movies'),
('Flat Movies', '{title} ({year}).{ext}', 'All movies in root folder', false, true, 'movies'),
('Plex Movie', '{title} ({year})/{title} ({year}) [{quality}].{ext}', 'Plex style with quality', false, true, 'movies'),
('Jellyfin Movie', '{title} ({year})/{title}.{ext}', 'Jellyfin recommended format', false, true, 'movies');

-- ============================================================================
-- Music Naming Patterns
-- Variables: {artist}, {album}, {year}, {track}, {title}, {disc}, {ext}
-- ============================================================================

INSERT INTO naming_patterns (name, pattern, description, is_default, is_system, library_type) VALUES
('Music Standard', '{artist}/{album} ({year})/{track:02} - {title}.{ext}', 'Artist/Album (Year)/01 - Title.ext', true, true, 'music'),
('Music with Disc', '{artist}/{album} ({year})/Disc {disc}/{track:02} - {title}.{ext}', 'Include disc number folder', false, true, 'music'),
('Artist Only', '{artist}/{track:02} - {title}.{ext}', 'All tracks in artist folder', false, true, 'music'),
('Album Only', '{album} ({year})/{track:02} - {title}.{ext}', 'Albums in root, no artist folder', false, true, 'music'),
('Full Track Info', '{artist}/{album} ({year})/{track:02} - {artist} - {title}.{ext}', 'Include artist in track filename', false, true, 'music');

-- ============================================================================
-- Audiobook Naming Patterns
-- Variables: {author}, {title}, {series}, {series_position}, {chapter}, {chapter_title}, {ext}
-- ============================================================================

INSERT INTO naming_patterns (name, pattern, description, is_default, is_system, library_type) VALUES
('Audiobook Standard', '{author}/{title}/{chapter:02} - {chapter_title}.{ext}', 'Author/Title/01 - Chapter.ext', true, true, 'audiobooks'),
('Audiobook Series', '{author}/{series} {series_position} - {title}/{chapter:02} - {chapter_title}.{ext}', 'Include series info', false, true, 'audiobooks'),
('Audiobook Simple', '{author}/{title}/{chapter:02}.{ext}', 'Simple chapter numbering', false, true, 'audiobooks'),
('Audiobook Flat', '{author}/{title}.{ext}', 'Single file audiobooks', false, true, 'audiobooks'),
('Plex Audiobook', '{author}/{title}/{title} - Chapter {chapter:02}.{ext}', 'Plex audiobook format', false, true, 'audiobooks');

-- ============================================================================
-- Other/Generic Naming Patterns
-- Variables: {name}, {ext}
-- ============================================================================

INSERT INTO naming_patterns (name, pattern, description, is_default, is_system, library_type) VALUES
('Generic Preserve', '{name}.{ext}', 'Keep original filename', true, true, 'other'),
('Generic Folder', '{name}/{name}.{ext}', 'Each file in its own folder', false, true, 'other');

-- Add comment for library_type
COMMENT ON COLUMN naming_patterns.library_type IS 'Which library type this pattern applies to: tv, movies, music, audiobooks, other';
