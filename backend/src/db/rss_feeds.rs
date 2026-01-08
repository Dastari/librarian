//! RSS Feed database repository

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// RSS Feed record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RssFeedRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub library_id: Option<Uuid>,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub poll_interval_minutes: i32,
    pub last_polled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_successful_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_error: Option<String>,
    pub consecutive_failures: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// RSS Feed item record
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RssFeedItemRecord {
    pub id: Uuid,
    pub feed_id: Uuid,
    pub guid: Option<String>,
    pub link_hash: String,
    pub title_hash: String,
    pub title: String,
    pub link: String,
    pub pub_date: Option<chrono::DateTime<chrono::Utc>>,
    pub description: Option<String>,
    pub parsed_show_name: Option<String>,
    pub parsed_season: Option<i32>,
    pub parsed_episode: Option<i32>,
    pub parsed_resolution: Option<String>,
    pub parsed_codec: Option<String>,
    pub parsed_source: Option<String>,
    pub processed: bool,
    pub matched_episode_id: Option<Uuid>,
    pub torrent_id: Option<Uuid>,
    pub skipped_reason: Option<String>,
    pub seen_at: chrono::DateTime<chrono::Utc>,
}

/// Input for creating an RSS feed
#[derive(Debug)]
pub struct CreateRssFeed {
    pub user_id: Uuid,
    pub library_id: Option<Uuid>,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub poll_interval_minutes: i32,
}

/// Input for updating an RSS feed
#[derive(Debug, Default)]
pub struct UpdateRssFeed {
    pub name: Option<String>,
    pub url: Option<String>,
    pub library_id: Option<Uuid>,
    pub enabled: Option<bool>,
    pub poll_interval_minutes: Option<i32>,
}

/// Input for creating an RSS feed item
#[derive(Debug)]
pub struct CreateRssFeedItem {
    pub feed_id: Uuid,
    pub guid: Option<String>,
    pub link_hash: String,
    pub title_hash: String,
    pub title: String,
    pub link: String,
    pub pub_date: Option<chrono::DateTime<chrono::Utc>>,
    pub description: Option<String>,
    pub parsed_show_name: Option<String>,
    pub parsed_season: Option<i32>,
    pub parsed_episode: Option<i32>,
    pub parsed_resolution: Option<String>,
    pub parsed_codec: Option<String>,
    pub parsed_source: Option<String>,
}

pub struct RssFeedRepository {
    pool: PgPool,
}

impl RssFeedRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all RSS feeds for a user
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<RssFeedRecord>> {
        let records = sqlx::query_as::<_, RssFeedRecord>(
            r#"
            SELECT id, user_id, library_id, name, url, enabled,
                   poll_interval_minutes, last_polled_at, last_successful_at,
                   last_error, consecutive_failures, created_at, updated_at
            FROM rss_feeds
            WHERE user_id = $1
            ORDER BY name
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get RSS feeds for a specific library
    pub async fn list_by_library(&self, library_id: Uuid) -> Result<Vec<RssFeedRecord>> {
        let records = sqlx::query_as::<_, RssFeedRecord>(
            r#"
            SELECT id, user_id, library_id, name, url, enabled,
                   poll_interval_minutes, last_polled_at, last_successful_at,
                   last_error, consecutive_failures, created_at, updated_at
            FROM rss_feeds
            WHERE library_id = $1 OR library_id IS NULL
            ORDER BY name
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get enabled feeds that need polling
    pub async fn list_due_for_poll(&self) -> Result<Vec<RssFeedRecord>> {
        let records = sqlx::query_as::<_, RssFeedRecord>(
            r#"
            SELECT id, user_id, library_id, name, url, enabled,
                   poll_interval_minutes, last_polled_at, last_successful_at,
                   last_error, consecutive_failures, created_at, updated_at
            FROM rss_feeds
            WHERE enabled = true
              AND (last_polled_at IS NULL 
                   OR last_polled_at < NOW() - (poll_interval_minutes || ' minutes')::interval)
            ORDER BY last_polled_at NULLS FIRST
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get an RSS feed by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<RssFeedRecord>> {
        let record = sqlx::query_as::<_, RssFeedRecord>(
            r#"
            SELECT id, user_id, library_id, name, url, enabled,
                   poll_interval_minutes, last_polled_at, last_successful_at,
                   last_error, consecutive_failures, created_at, updated_at
            FROM rss_feeds
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create a new RSS feed
    pub async fn create(&self, input: CreateRssFeed) -> Result<RssFeedRecord> {
        let record = sqlx::query_as::<_, RssFeedRecord>(
            r#"
            INSERT INTO rss_feeds (
                user_id, library_id, name, url, enabled, poll_interval_minutes
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, user_id, library_id, name, url, enabled,
                      poll_interval_minutes, last_polled_at, last_successful_at,
                      last_error, consecutive_failures, created_at, updated_at
            "#,
        )
        .bind(input.user_id)
        .bind(input.library_id)
        .bind(&input.name)
        .bind(&input.url)
        .bind(input.enabled)
        .bind(input.poll_interval_minutes)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update an RSS feed
    pub async fn update(&self, id: Uuid, input: UpdateRssFeed) -> Result<Option<RssFeedRecord>> {
        let record = sqlx::query_as::<_, RssFeedRecord>(
            r#"
            UPDATE rss_feeds SET
                name = COALESCE($2, name),
                url = COALESCE($3, url),
                library_id = COALESCE($4, library_id),
                enabled = COALESCE($5, enabled),
                poll_interval_minutes = COALESCE($6, poll_interval_minutes),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, user_id, library_id, name, url, enabled,
                      poll_interval_minutes, last_polled_at, last_successful_at,
                      last_error, consecutive_failures, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&input.name)
        .bind(&input.url)
        .bind(input.library_id)
        .bind(input.enabled)
        .bind(input.poll_interval_minutes)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete an RSS feed
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM rss_feeds WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update poll status (success)
    pub async fn mark_poll_success(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE rss_feeds SET
                last_polled_at = NOW(),
                last_successful_at = NOW(),
                last_error = NULL,
                consecutive_failures = 0,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update poll status (failure)
    pub async fn mark_poll_failure(&self, id: Uuid, error: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE rss_feeds SET
                last_polled_at = NOW(),
                last_error = $2,
                consecutive_failures = COALESCE(consecutive_failures, 0) + 1,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ========== RSS Feed Items ==========

    /// Check if an item already exists (for deduplication)
    pub async fn item_exists(&self, feed_id: Uuid, link_hash: &str) -> Result<bool> {
        let exists: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM rss_feed_items 
                WHERE feed_id = $1 AND link_hash = $2
            )
            "#,
        )
        .bind(feed_id)
        .bind(link_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists.0)
    }

    /// Create a new RSS feed item
    pub async fn create_item(&self, input: CreateRssFeedItem) -> Result<RssFeedItemRecord> {
        let record = sqlx::query_as::<_, RssFeedItemRecord>(
            r#"
            INSERT INTO rss_feed_items (
                feed_id, guid, link_hash, title_hash, title, link,
                pub_date, description, parsed_show_name, parsed_season,
                parsed_episode, parsed_resolution, parsed_codec, parsed_source
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (feed_id, link_hash) DO UPDATE SET
                seen_at = NOW()
            RETURNING id, feed_id, guid, link_hash, title_hash, title, link,
                      pub_date, description, parsed_show_name, parsed_season,
                      parsed_episode, parsed_resolution, parsed_codec, parsed_source,
                      processed, matched_episode_id, torrent_id, skipped_reason, seen_at
            "#,
        )
        .bind(input.feed_id)
        .bind(&input.guid)
        .bind(&input.link_hash)
        .bind(&input.title_hash)
        .bind(&input.title)
        .bind(&input.link)
        .bind(input.pub_date)
        .bind(&input.description)
        .bind(&input.parsed_show_name)
        .bind(input.parsed_season)
        .bind(input.parsed_episode)
        .bind(&input.parsed_resolution)
        .bind(&input.parsed_codec)
        .bind(&input.parsed_source)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get unprocessed items for a feed
    pub async fn get_unprocessed_items(&self, feed_id: Uuid) -> Result<Vec<RssFeedItemRecord>> {
        let records = sqlx::query_as::<_, RssFeedItemRecord>(
            r#"
            SELECT id, feed_id, guid, link_hash, title_hash, title, link,
                   pub_date, description, parsed_show_name, parsed_season,
                   parsed_episode, parsed_resolution, parsed_codec, parsed_source,
                   processed, matched_episode_id, torrent_id, skipped_reason, seen_at
            FROM rss_feed_items
            WHERE feed_id = $1 AND processed = false
            ORDER BY pub_date DESC NULLS LAST
            "#,
        )
        .bind(feed_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Mark item as processed
    pub async fn mark_item_processed(
        &self,
        id: Uuid,
        matched_episode_id: Option<Uuid>,
        torrent_id: Option<Uuid>,
        skipped_reason: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE rss_feed_items SET
                processed = true,
                matched_episode_id = $2,
                torrent_id = $3,
                skipped_reason = $4
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(matched_episode_id)
        .bind(torrent_id)
        .bind(skipped_reason)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Clean up old items (keep last N days)
    pub async fn cleanup_old_items(&self, days: i32) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM rss_feed_items
            WHERE seen_at < NOW() - ($1 || ' days')::interval
            "#,
        )
        .bind(days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
