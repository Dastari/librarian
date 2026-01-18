-- Migration 014: Clean up legacy/unused tables
-- 
-- These tables were created in earlier migrations but are no longer used:
-- - subscriptions: Replaced by tv_shows monitoring system
-- - media_items: Replaced by tv_shows + episodes
-- - events: Audit log never implemented
-- - artwork: Replaced by poster_url/backdrop_url fields on tv_shows
-- - jobs: Background job queue replaced by in-memory scheduling
--
-- Also deprecates quality_profiles in favor of inline quality settings on libraries/tv_shows

-- Drop unused tables (cascade to remove any foreign key dependencies)
DROP TABLE IF EXISTS subscriptions CASCADE;
DROP TABLE IF EXISTS media_items CASCADE;
DROP TABLE IF EXISTS events CASCADE;
DROP TABLE IF EXISTS artwork CASCADE;
DROP TABLE IF EXISTS jobs CASCADE;

-- Add deprecation comment to quality_profiles table
-- Note: We keep the table for now as it may still be referenced, but mark it deprecated
COMMENT ON TABLE quality_profiles IS 'DEPRECATED: Use inline quality settings on libraries and tv_shows instead (allowedResolutions, allowedVideoCodecs, etc.). This table will be removed in a future migration.';

-- Remove deprecated foreign key columns from libraries and tv_shows
-- First, drop the constraints if they exist
DO $$ 
BEGIN
    -- Drop foreign key from libraries if it exists
    IF EXISTS (
        SELECT 1 FROM information_schema.table_constraints 
        WHERE constraint_name = 'libraries_default_quality_profile_id_fkey' 
        AND table_name = 'libraries'
    ) THEN
        ALTER TABLE libraries DROP CONSTRAINT libraries_default_quality_profile_id_fkey;
    END IF;
    
    -- Drop foreign key from tv_shows if it exists
    IF EXISTS (
        SELECT 1 FROM information_schema.table_constraints 
        WHERE constraint_name = 'tv_shows_quality_profile_id_fkey' 
        AND table_name = 'tv_shows'
    ) THEN
        ALTER TABLE tv_shows DROP CONSTRAINT tv_shows_quality_profile_id_fkey;
    END IF;
END $$;

-- Add deprecation comments to the columns (but don't remove them yet for backwards compatibility)
COMMENT ON COLUMN libraries.default_quality_profile_id IS 'DEPRECATED: Use inline quality settings (allowed_resolutions, allowed_video_codecs, etc.) instead.';
COMMENT ON COLUMN tv_shows.quality_profile_id IS 'DEPRECATED: Use inline quality settings (allowed_resolutions_override, allowed_video_codecs_override, etc.) instead.';
