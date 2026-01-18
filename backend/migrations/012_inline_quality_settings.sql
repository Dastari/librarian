-- Inline Quality Settings Migration
-- Adds granular quality filtering directly on libraries and tv_shows tables
-- Replaces reliance on quality_profiles for filtering with inline checkbox-style settings
-- Empty arrays mean "any" (accept all), non-empty arrays filter to only those values

-- ============================================================================
-- Libraries: Add Inline Quality Settings
-- ============================================================================

-- Resolution filtering (empty = any)
ALTER TABLE libraries
ADD COLUMN IF NOT EXISTS allowed_resolutions TEXT[] NOT NULL DEFAULT '{}';

-- Video codec filtering (empty = any)
ALTER TABLE libraries
ADD COLUMN IF NOT EXISTS allowed_video_codecs TEXT[] NOT NULL DEFAULT '{}';

-- Audio format filtering (empty = any)
ALTER TABLE libraries
ADD COLUMN IF NOT EXISTS allowed_audio_formats TEXT[] NOT NULL DEFAULT '{}';

-- HDR settings
ALTER TABLE libraries
ADD COLUMN IF NOT EXISTS require_hdr BOOLEAN NOT NULL DEFAULT false;

-- HDR types (empty with require_hdr=true means any HDR, non-empty filters to specific types)
ALTER TABLE libraries
ADD COLUMN IF NOT EXISTS allowed_hdr_types TEXT[] NOT NULL DEFAULT '{}';

-- Source/release type filtering (empty = any)
ALTER TABLE libraries
ADD COLUMN IF NOT EXISTS allowed_sources TEXT[] NOT NULL DEFAULT '{}';

-- Release group filtering
ALTER TABLE libraries
ADD COLUMN IF NOT EXISTS release_group_blacklist TEXT[] NOT NULL DEFAULT '{}';

ALTER TABLE libraries
ADD COLUMN IF NOT EXISTS release_group_whitelist TEXT[] NOT NULL DEFAULT '{}';

-- ============================================================================
-- TV Shows: Add Quality Override Fields
-- NULL = inherit from library, non-NULL = override (empty array = any)
-- ============================================================================

-- Resolution override
ALTER TABLE tv_shows
ADD COLUMN IF NOT EXISTS allowed_resolutions_override TEXT[] DEFAULT NULL;

-- Video codec override
ALTER TABLE tv_shows
ADD COLUMN IF NOT EXISTS allowed_video_codecs_override TEXT[] DEFAULT NULL;

-- Audio format override
ALTER TABLE tv_shows
ADD COLUMN IF NOT EXISTS allowed_audio_formats_override TEXT[] DEFAULT NULL;

-- HDR requirement override
ALTER TABLE tv_shows
ADD COLUMN IF NOT EXISTS require_hdr_override BOOLEAN DEFAULT NULL;

-- HDR types override
ALTER TABLE tv_shows
ADD COLUMN IF NOT EXISTS allowed_hdr_types_override TEXT[] DEFAULT NULL;

-- Source/release type override
ALTER TABLE tv_shows
ADD COLUMN IF NOT EXISTS allowed_sources_override TEXT[] DEFAULT NULL;

-- Release group blacklist override
ALTER TABLE tv_shows
ADD COLUMN IF NOT EXISTS release_group_blacklist_override TEXT[] DEFAULT NULL;

-- Release group whitelist override
ALTER TABLE tv_shows
ADD COLUMN IF NOT EXISTS release_group_whitelist_override TEXT[] DEFAULT NULL;

-- ============================================================================
-- Add parsed_audio to RSS feed items for audio format matching
-- ============================================================================

ALTER TABLE rss_feed_items
ADD COLUMN IF NOT EXISTS parsed_audio VARCHAR(50);

ALTER TABLE rss_feed_items
ADD COLUMN IF NOT EXISTS parsed_hdr VARCHAR(50);

-- ============================================================================
-- Comments for documentation
-- ============================================================================

COMMENT ON COLUMN libraries.allowed_resolutions IS 'Allowed resolutions: 2160p, 1080p, 720p, 480p. Empty = any.';
COMMENT ON COLUMN libraries.allowed_video_codecs IS 'Allowed video codecs: hevc, h264, av1, xvid. Empty = any.';
COMMENT ON COLUMN libraries.allowed_audio_formats IS 'Allowed audio formats: atmos, truehd, dtshd, dts, dd51, aac. Empty = any.';
COMMENT ON COLUMN libraries.require_hdr IS 'If true, only accept releases with HDR.';
COMMENT ON COLUMN libraries.allowed_hdr_types IS 'Allowed HDR types: hdr10, hdr10plus, dolbyvision, hlg. Empty with require_hdr=true = any HDR.';
COMMENT ON COLUMN libraries.allowed_sources IS 'Allowed sources: webdl, webrip, bluray, hdtv. Empty = any.';

COMMENT ON COLUMN tv_shows.allowed_resolutions_override IS 'Override library resolution filter. NULL = inherit.';
COMMENT ON COLUMN tv_shows.allowed_video_codecs_override IS 'Override library codec filter. NULL = inherit.';
COMMENT ON COLUMN tv_shows.allowed_audio_formats_override IS 'Override library audio filter. NULL = inherit.';
COMMENT ON COLUMN tv_shows.require_hdr_override IS 'Override library HDR requirement. NULL = inherit.';
COMMENT ON COLUMN tv_shows.allowed_hdr_types_override IS 'Override library HDR type filter. NULL = inherit.';
COMMENT ON COLUMN tv_shows.allowed_sources_override IS 'Override library source filter. NULL = inherit.';
