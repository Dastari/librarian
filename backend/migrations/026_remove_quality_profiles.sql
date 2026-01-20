-- ============================================================================
-- Migration: Remove deprecated quality_profiles system
-- ============================================================================
-- Quality filtering is now handled via inline settings on libraries and tv_shows:
-- - libraries.allowed_resolutions, allowed_video_codecs, etc.
-- - tv_shows.*_override fields for per-show customization
--
-- The quality_profiles table and related foreign keys are no longer used.

-- Remove foreign key columns first
ALTER TABLE libraries DROP COLUMN IF EXISTS default_quality_profile_id;
ALTER TABLE tv_shows DROP COLUMN IF EXISTS quality_profile_id;

-- Drop the deprecated quality_profiles table
DROP TABLE IF EXISTS quality_profiles CASCADE;
