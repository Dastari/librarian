-- Add is_system column to naming_patterns if it doesn't exist
-- and seed default naming patterns for all library types

-- First, add the is_system column (SQLite doesn't support IF NOT EXISTS for columns)
-- We use a workaround: try to add it and ignore if it fails
ALTER TABLE naming_patterns ADD COLUMN is_system INTEGER NOT NULL DEFAULT 0;

-- ============================================================================
-- TV Show Naming Patterns
-- Variables: {show}, {season}, {episode}, {title}, {ext}
-- ============================================================================

INSERT OR IGNORE INTO naming_patterns (id, user_id, library_type, name, pattern, description, is_default, is_system, created_at, updated_at) VALUES
('00000000-0000-0000-0000-000000000001', 'system', 'tv', 'Standard', '{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}', 'Show/Season 01/Show - S01E01 - Title.ext', 1, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000002', 'system', 'tv', 'Plex Style', '{show}/Season {season:02}/{show} - s{season:02}e{episode:02} - {title}.{ext}', 'Lowercase season/episode (Plex compatible)', 0, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000003', 'system', 'tv', 'Compact', '{show}/S{season:02}/{show}.S{season:02}E{episode:02}.{ext}', 'Compact format without episode title', 0, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000004', 'system', 'tv', 'Scene Style', '{show}/Season {season:02}/{show}.S{season:02}E{episode:02}.{title}.{ext}', 'Dots instead of spaces (scene style)', 0, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000005', 'system', 'tv', 'Jellyfin', '{show}/Season {season}/{show} S{season:02}E{episode:02} {title}.{ext}', 'Jellyfin recommended format', 0, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000006', 'system', 'tv', 'Simple', '{show}/Season {season:02}/{season:02}x{episode:02} - {title}.{ext}', 'Simple 01x01 format', 0, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000007', 'system', 'tv', 'Flat', '{show} - S{season:02}E{episode:02} - {title}.{ext}', 'All files in show folder (no season folders)', 0, 1, datetime('now'), datetime('now'));

-- ============================================================================
-- Movie Naming Patterns
-- Variables: {title}, {year}, {quality}, {ext}
-- ============================================================================

INSERT OR IGNORE INTO naming_patterns (id, user_id, library_type, name, pattern, description, is_default, is_system, created_at, updated_at) VALUES
('00000000-0000-0000-0000-000000000011', 'system', 'movies', 'Movie Standard', '{title} ({year})/{title} ({year}).{ext}', 'Title (Year)/Title (Year).ext', 1, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000012', 'system', 'movies', 'Movie with Quality', '{title} ({year})/{title} ({year}) - {quality}.{ext}', 'Include quality in filename', 0, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000013', 'system', 'movies', 'Flat Movies', '{title} ({year}).{ext}', 'All movies in root folder', 0, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000014', 'system', 'movies', 'Plex Movie', '{title} ({year})/{title} ({year}) [{quality}].{ext}', 'Plex style with quality', 0, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000015', 'system', 'movies', 'Jellyfin Movie', '{title} ({year})/{title}.{ext}', 'Jellyfin recommended format', 0, 1, datetime('now'), datetime('now'));

-- ============================================================================
-- Music Naming Patterns
-- Variables: {artist}, {album}, {year}, {track}, {title}, {disc}, {ext}
-- ============================================================================

INSERT OR IGNORE INTO naming_patterns (id, user_id, library_type, name, pattern, description, is_default, is_system, created_at, updated_at) VALUES
('00000000-0000-0000-0000-000000000021', 'system', 'music', 'Music Standard', '{artist}/{album} ({year})/{track:02} - {title}.{ext}', 'Artist/Album (Year)/01 - Title.ext', 1, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000022', 'system', 'music', 'Music with Disc', '{artist}/{album} ({year})/Disc {disc}/{track:02} - {title}.{ext}', 'Include disc number folder', 0, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000023', 'system', 'music', 'Artist Only', '{artist}/{track:02} - {title}.{ext}', 'All tracks in artist folder', 0, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000024', 'system', 'music', 'Album Only', '{album} ({year})/{track:02} - {title}.{ext}', 'Albums in root, no artist folder', 0, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000025', 'system', 'music', 'Full Track Info', '{artist}/{album} ({year})/{track:02} - {artist} - {title}.{ext}', 'Include artist in track filename', 0, 1, datetime('now'), datetime('now'));

-- ============================================================================
-- Audiobook Naming Patterns
-- Variables: {author}, {title}, {series}, {series_position}, {chapter}, {chapter_title}, {ext}
-- ============================================================================

INSERT OR IGNORE INTO naming_patterns (id, user_id, library_type, name, pattern, description, is_default, is_system, created_at, updated_at) VALUES
('00000000-0000-0000-0000-000000000031', 'system', 'audiobooks', 'Audiobook Standard', '{author}/{title}/{chapter:02} - {chapter_title}.{ext}', 'Author/Title/01 - Chapter.ext', 1, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000032', 'system', 'audiobooks', 'Audiobook Series', '{author}/{series} {series_position} - {title}/{chapter:02} - {chapter_title}.{ext}', 'Include series info', 0, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000033', 'system', 'audiobooks', 'Audiobook Simple', '{author}/{title}/{chapter:02}.{ext}', 'Simple chapter numbering', 0, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000034', 'system', 'audiobooks', 'Audiobook Flat', '{author}/{title}.{ext}', 'Single file audiobooks', 0, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000035', 'system', 'audiobooks', 'Plex Audiobook', '{author}/{title}/{title} - Chapter {chapter:02}.{ext}', 'Plex audiobook format', 0, 1, datetime('now'), datetime('now'));

-- ============================================================================
-- Other/Generic Naming Patterns
-- Variables: {name}, {ext}
-- ============================================================================

INSERT OR IGNORE INTO naming_patterns (id, user_id, library_type, name, pattern, description, is_default, is_system, created_at, updated_at) VALUES
('00000000-0000-0000-0000-000000000041', 'system', 'other', 'Generic Preserve', '{name}.{ext}', 'Keep original filename', 1, 1, datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000042', 'system', 'other', 'Generic Folder', '{name}/{name}.{ext}', 'Each file in its own folder', 0, 1, datetime('now'), datetime('now'));
