-- Migration 041: Media Files Central Refactor
--
-- This migration makes media_files the single source of truth for all file metadata.
-- Content tables (episodes, movies, tracks, chapters) now only contain content metadata
-- with a bidirectional link to media_files. Status is derived from media_file_id presence.
--
-- BREAKING CHANGE: Accepts full data loss as confirmed by user.

-- ============================================================================
-- Phase 1: Drop Legacy Tables (completely unused)
-- ============================================================================

-- Drop tables that were created but never actually used
DROP TABLE IF EXISTS quality_profiles CASCADE;
DROP TABLE IF EXISTS unmatched_files CASCADE;
DROP TABLE IF EXISTS movie_releases CASCADE;
DROP TABLE IF EXISTS movie_collections CASCADE;
DROP TABLE IF EXISTS usenet_file_matches CASCADE;

-- Remove deprecated FK columns that referenced dropped tables
ALTER TABLE libraries DROP COLUMN IF EXISTS default_quality_profile_id;
ALTER TABLE tv_shows DROP COLUMN IF EXISTS quality_profile_id;

-- ============================================================================
-- Phase 2: Clean Up Existing Data (fresh start)
-- ============================================================================

-- Truncate download-related tables
TRUNCATE media_files CASCADE;
TRUNCATE torrents CASCADE;
TRUNCATE torrent_files CASCADE;
TRUNCATE pending_file_matches CASCADE;

-- ============================================================================
-- Phase 3: Update Episodes Table
-- ============================================================================

-- Add media_file_id for bidirectional linking
ALTER TABLE episodes 
    ADD COLUMN IF NOT EXISTS media_file_id UUID REFERENCES media_files(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_episodes_media_file 
    ON episodes(media_file_id) WHERE media_file_id IS NOT NULL;

-- Drop legacy columns from episodes
ALTER TABLE episodes DROP COLUMN IF EXISTS torrent_link;
ALTER TABLE episodes DROP COLUMN IF EXISTS torrent_link_added_at;
ALTER TABLE episodes DROP COLUMN IF EXISTS matched_rss_item_id;
ALTER TABLE episodes DROP COLUMN IF EXISTS active_download_id;
ALTER TABLE episodes DROP COLUMN IF EXISTS status;

-- Drop the constraint that references the dropped column
ALTER TABLE episodes DROP CONSTRAINT IF EXISTS fk_episodes_matched_rss_item;
ALTER TABLE episodes DROP CONSTRAINT IF EXISTS episodes_status_check;

-- ============================================================================
-- Phase 4: Update Movies Table
-- ============================================================================

-- Add media_file_id for bidirectional linking
ALTER TABLE movies 
    ADD COLUMN IF NOT EXISTS media_file_id UUID REFERENCES media_files(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_movies_media_file 
    ON movies(media_file_id) WHERE media_file_id IS NOT NULL;

-- Drop legacy columns from movies
ALTER TABLE movies DROP COLUMN IF EXISTS allowed_resolutions_override;
ALTER TABLE movies DROP COLUMN IF EXISTS allowed_video_codecs_override;
ALTER TABLE movies DROP COLUMN IF EXISTS allowed_audio_formats_override;
ALTER TABLE movies DROP COLUMN IF EXISTS require_hdr_override;
ALTER TABLE movies DROP COLUMN IF EXISTS allowed_hdr_types_override;
ALTER TABLE movies DROP COLUMN IF EXISTS allowed_sources_override;
ALTER TABLE movies DROP COLUMN IF EXISTS release_group_blacklist_override;
ALTER TABLE movies DROP COLUMN IF EXISTS release_group_whitelist_override;
ALTER TABLE movies DROP COLUMN IF EXISTS has_file;
ALTER TABLE movies DROP COLUMN IF EXISTS size_bytes;
ALTER TABLE movies DROP COLUMN IF EXISTS path;
ALTER TABLE movies DROP COLUMN IF EXISTS download_status;
ALTER TABLE movies DROP COLUMN IF EXISTS active_download_id;

-- Drop the constraint for download_status
ALTER TABLE movies DROP CONSTRAINT IF EXISTS movies_download_status_check;

-- ============================================================================
-- Phase 5: Update Tracks Table
-- ============================================================================

-- tracks already has media_file_id, just remove legacy columns
ALTER TABLE tracks DROP COLUMN IF EXISTS status;
ALTER TABLE tracks DROP COLUMN IF EXISTS active_download_id;

-- Drop the constraint for status
ALTER TABLE tracks DROP CONSTRAINT IF EXISTS tracks_status_check;

-- Drop related indexes
DROP INDEX IF EXISTS idx_tracks_status;
DROP INDEX IF EXISTS idx_tracks_wanted;
DROP INDEX IF EXISTS idx_tracks_active_download;

-- ============================================================================
-- Phase 6: Update Chapters Table (audiobook chapters)
-- ============================================================================

-- chapters already has media_file_id, just remove legacy columns
ALTER TABLE chapters DROP COLUMN IF EXISTS status;
ALTER TABLE chapters DROP COLUMN IF EXISTS active_download_id;

-- Drop the constraint for status
ALTER TABLE chapters DROP CONSTRAINT IF EXISTS chapters_status_check;

-- Drop related indexes
DROP INDEX IF EXISTS idx_chapters_status;
DROP INDEX IF EXISTS idx_chapters_active_download;

-- ============================================================================
-- Phase 7: Update Albums Table
-- ============================================================================

-- Remove download_status from albums (status derived from track media_file_ids)
ALTER TABLE albums DROP COLUMN IF EXISTS download_status;
ALTER TABLE albums DROP CONSTRAINT IF EXISTS albums_download_status_check;
DROP INDEX IF EXISTS idx_albums_download_status;

-- ============================================================================
-- Phase 8: Update Audiobooks Table
-- ============================================================================

-- Remove download_status from audiobooks (status derived from chapter media_file_ids)
ALTER TABLE audiobooks DROP COLUMN IF EXISTS download_status;
ALTER TABLE audiobooks DROP CONSTRAINT IF EXISTS audiobooks_download_status_check;
DROP INDEX IF EXISTS idx_audiobooks_download_status;

-- ============================================================================
-- Phase 9: Update Media Files Table
-- ============================================================================

-- Add chapter_id for bidirectional linking with audiobook chapters
ALTER TABLE media_files 
    ADD COLUMN IF NOT EXISTS chapter_id UUID REFERENCES chapters(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_media_files_chapter 
    ON media_files(chapter_id) WHERE chapter_id IS NOT NULL;

-- Ensure all content type FKs exist
-- episode_id and movie_id already exist from initial schema
-- track_id and album_id already exist from migration 019
-- audiobook_id already exists from migration 020
-- chapter_id added above

-- ============================================================================
-- Phase 10: Create Helper Views for Status Computation
-- ============================================================================

-- Episode status view (status derived from media_file_id and pending_file_matches)
CREATE OR REPLACE VIEW episode_status_v AS
SELECT 
    e.id,
    e.tv_show_id,
    e.media_file_id,
    CASE 
        WHEN e.media_file_id IS NOT NULL THEN 'downloaded'
        WHEN pfm.id IS NOT NULL AND pfm.copied_at IS NULL THEN 'downloading'
        WHEN e.air_date IS NOT NULL AND e.air_date <= CURRENT_DATE THEN 'wanted'
        ELSE 'missing'
    END as computed_status
FROM episodes e
LEFT JOIN pending_file_matches pfm ON pfm.episode_id = e.id AND pfm.copied_at IS NULL;

-- Movie status view
CREATE OR REPLACE VIEW movie_status_v AS
SELECT 
    m.id,
    m.library_id,
    m.media_file_id,
    CASE 
        WHEN m.media_file_id IS NOT NULL THEN 'downloaded'
        WHEN pfm.id IS NOT NULL AND pfm.copied_at IS NULL THEN 'downloading'
        WHEN m.monitored = true THEN 'wanted'
        ELSE 'missing'
    END as computed_status
FROM movies m
LEFT JOIN pending_file_matches pfm ON pfm.movie_id = m.id AND pfm.copied_at IS NULL;

-- Track status view
CREATE OR REPLACE VIEW track_status_v AS
SELECT 
    t.id,
    t.album_id,
    t.media_file_id,
    CASE 
        WHEN t.media_file_id IS NOT NULL THEN 'downloaded'
        WHEN pfm.id IS NOT NULL AND pfm.copied_at IS NULL THEN 'downloading'
        ELSE 'wanted'
    END as computed_status
FROM tracks t
LEFT JOIN pending_file_matches pfm ON pfm.track_id = t.id AND pfm.copied_at IS NULL;

-- Chapter status view
CREATE OR REPLACE VIEW chapter_status_v AS
SELECT 
    c.id,
    c.audiobook_id,
    c.media_file_id,
    CASE 
        WHEN c.media_file_id IS NOT NULL THEN 'downloaded'
        WHEN pfm.id IS NOT NULL AND pfm.copied_at IS NULL THEN 'downloading'
        ELSE 'wanted'
    END as computed_status
FROM chapters c
LEFT JOIN pending_file_matches pfm ON pfm.chapter_id = c.id AND pfm.copied_at IS NULL;

-- ============================================================================
-- Phase 11: Add Comments
-- ============================================================================

COMMENT ON COLUMN episodes.media_file_id IS 'Link to the media file for this episode (bidirectional with media_files.episode_id)';
COMMENT ON COLUMN movies.media_file_id IS 'Link to the media file for this movie (bidirectional with media_files.movie_id)';
COMMENT ON COLUMN media_files.chapter_id IS 'Link to audiobook chapter if this file is a chapter (bidirectional with chapters.media_file_id)';

COMMENT ON VIEW episode_status_v IS 'Computed episode status based on media_file_id and pending_file_matches';
COMMENT ON VIEW movie_status_v IS 'Computed movie status based on media_file_id and pending_file_matches';
COMMENT ON VIEW track_status_v IS 'Computed track status based on media_file_id and pending_file_matches';
COMMENT ON VIEW chapter_status_v IS 'Computed chapter status based on media_file_id and pending_file_matches';
