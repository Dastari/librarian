-- Migration: Add naming pattern presets table
-- Allows users to select from predefined or custom file naming patterns

-- Naming pattern presets table
CREATE TABLE IF NOT EXISTS naming_patterns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    pattern TEXT NOT NULL,
    description TEXT,
    is_default BOOLEAN DEFAULT false,
    is_system BOOLEAN DEFAULT true,  -- true = built-in, false = user-created
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index for quick default lookup
CREATE INDEX IF NOT EXISTS idx_naming_patterns_default ON naming_patterns(is_default) WHERE is_default = true;

-- Seed with default presets
INSERT INTO naming_patterns (name, pattern, description, is_default, is_system) VALUES
('Standard', '{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}', 'Show/Season 01/Show - S01E01 - Title.ext', true, true),
('Plex Style', '{show}/Season {season:02}/{show} - s{season:02}e{episode:02} - {title}.{ext}', 'Lowercase season/episode (Plex compatible)', false, true),
('Compact', '{show}/S{season:02}/{show}.S{season:02}E{episode:02}.{ext}', 'Compact format without episode title', false, true),
('Scene Style', '{show}/Season {season:02}/{show}.S{season:02}E{episode:02}.{title}.{ext}', 'Dots instead of spaces (scene style)', false, true),
('Jellyfin', '{show}/Season {season}/{show} S{season:02}E{episode:02} {title}.{ext}', 'Jellyfin recommended format', false, true),
('Simple', '{show}/Season {season:02}/{season:02}x{episode:02} - {title}.{ext}', 'Simple 01x01 format', false, true),
('Flat', '{show} - S{season:02}E{episode:02} - {title}.{ext}', 'All files in show folder (no season folders)', false, true);

COMMENT ON TABLE naming_patterns IS 'File naming pattern presets for library organization';
COMMENT ON COLUMN naming_patterns.is_system IS 'True for built-in patterns, false for user-created';
COMMENT ON COLUMN naming_patterns.is_default IS 'The default pattern used for new libraries';
