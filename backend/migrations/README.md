# Database Migrations

## Overview

Migrations are numbered sequentially (001_, 002_, etc.) and run in order by sqlx.

## Table Status

### Active Tables (in use)

| Table | Purpose |
|-------|---------|
| `libraries` | User library configurations (path, settings, quality filters) |
| `tv_shows` | TV shows tracked in libraries with monitoring settings |
| `episodes` | Individual episodes with air dates and download status |
| `media_files` | Media files discovered in libraries |
| `torrents` | Active and completed torrent downloads |
| `rss_feeds` | RSS feed configurations for episode discovery |
| `rss_feed_items` | Parsed RSS items for deduplication and tracking |
| `unmatched_files` | Media files that couldn't be auto-matched |
| `app_settings` | Application-wide settings (torrent client, etc.) |
| `app_logs` | Backend tracing logs for debugging |
| `quality_profiles` | Quality preference profiles |
| `indexer_configs` | Torrent indexer configurations |
| `indexer_credentials` | Encrypted indexer credentials |
| `indexer_settings` | Non-sensitive indexer settings |
| `indexer_search_cache` | Cached search results |
| `torznab_categories` | Standard Torznab category definitions |

### Removed Tables (Migration 014)

These tables were removed in migration 014 as they were never used or replaced:

| Table | Reason for Removal |
|-------|-------------------|
| `subscriptions` | Replaced by `tv_shows` monitoring system |
| `media_items` | Replaced by `tv_shows` + `episodes` |
| `events` | Audit log never implemented |
| `artwork` | Replaced by URL fields on `tv_shows` |
| `jobs` | Replaced by in-memory job scheduling |

### Deprecated (Pending Removal)

| Table/Column | Status | Replacement |
|--------------|--------|-------------|
| `quality_profiles` table | **DEPRECATED** | Use inline quality settings on `libraries`/`tv_shows` |
| `libraries.default_quality_profile_id` | **DEPRECATED** | Use `allowed_resolutions`, `allowed_video_codecs`, etc. |
| `tv_shows.quality_profile_id` | **DEPRECATED** | Use `allowed_resolutions_override`, `allowed_video_codecs_override`, etc. |

## Migration Guidelines

1. Always use numbered prefixes: `014_description.sql`, `015_description.sql`, etc.
2. Use `TIMESTAMPTZ` for all timestamp columns (not `TIMESTAMP`)
3. Use `UUID` for primary keys via `gen_random_uuid()`
4. Add `created_at` and `updated_at` columns to new tables
5. Create appropriate indexes for common query patterns
6. Enable RLS for user-scoped tables (though currently disabled for backend access)
