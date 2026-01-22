//! RSS Feed database repository

use anyhow::Result;
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "sqlite")]
use crate::db::sqlite_helpers::{
    bool_to_int, int_to_bool, str_to_datetime, str_to_datetime_opt, str_to_uuid, str_to_uuid_opt,
    uuid_to_str,
};

#[cfg(feature = "postgres")]
type DbPool = PgPool;
#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

/// RSS Feed record from database
#[derive(Debug, Clone)]
pub struct RssFeedRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub library_id: Option<Uuid>,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub poll_interval_minutes: i32,
    /// Post-download action override (copy, move, hardlink) - NULL uses library setting
    pub post_download_action: Option<String>,
    pub last_polled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_successful_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_error: Option<String>,
    pub consecutive_failures: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "postgres")]
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for RssFeedRecord {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            library_id: row.try_get("library_id")?,
            name: row.try_get("name")?,
            url: row.try_get("url")?,
            enabled: row.try_get("enabled")?,
            poll_interval_minutes: row.try_get("poll_interval_minutes")?,
            post_download_action: row.try_get("post_download_action")?,
            last_polled_at: row.try_get("last_polled_at")?,
            last_successful_at: row.try_get("last_successful_at")?,
            last_error: row.try_get("last_error")?,
            consecutive_failures: row.try_get("consecutive_failures")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for RssFeedRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let library_id_str: Option<String> = row.try_get("library_id")?;
        let enabled_int: i32 = row.try_get("enabled")?;
        let last_polled_str: Option<String> = row.try_get("last_polled_at")?;
        let last_successful_str: Option<String> = row.try_get("last_successful_at")?;
        let created_str: String = row.try_get("created_at")?;
        let updated_str: String = row.try_get("updated_at")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            library_id: str_to_uuid_opt(library_id_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            name: row.try_get("name")?,
            url: row.try_get("url")?,
            enabled: int_to_bool(enabled_int),
            poll_interval_minutes: row.try_get("poll_interval_minutes")?,
            post_download_action: row.try_get("post_download_action")?,
            last_polled_at: str_to_datetime_opt(last_polled_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            last_successful_at: str_to_datetime_opt(last_successful_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            last_error: row.try_get("last_error")?,
            consecutive_failures: row.try_get("consecutive_failures")?,
            created_at: str_to_datetime(&created_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

/// RSS Feed item record
#[derive(Debug, Clone)]
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
    pub parsed_audio: Option<String>,
    pub parsed_hdr: Option<String>,
    pub processed: bool,
    pub matched_episode_id: Option<Uuid>,
    pub torrent_id: Option<Uuid>,
    pub skipped_reason: Option<String>,
    pub seen_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "postgres")]
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for RssFeedItemRecord {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            feed_id: row.try_get("feed_id")?,
            guid: row.try_get("guid")?,
            link_hash: row.try_get("link_hash")?,
            title_hash: row.try_get("title_hash")?,
            title: row.try_get("title")?,
            link: row.try_get("link")?,
            pub_date: row.try_get("pub_date")?,
            description: row.try_get("description")?,
            parsed_show_name: row.try_get("parsed_show_name")?,
            parsed_season: row.try_get("parsed_season")?,
            parsed_episode: row.try_get("parsed_episode")?,
            parsed_resolution: row.try_get("parsed_resolution")?,
            parsed_codec: row.try_get("parsed_codec")?,
            parsed_source: row.try_get("parsed_source")?,
            parsed_audio: row.try_get("parsed_audio")?,
            parsed_hdr: row.try_get("parsed_hdr")?,
            processed: row.try_get("processed")?,
            matched_episode_id: row.try_get("matched_episode_id")?,
            torrent_id: row.try_get("torrent_id")?,
            skipped_reason: row.try_get("skipped_reason")?,
            seen_at: row.try_get("seen_at")?,
        })
    }
}

#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for RssFeedItemRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let feed_id_str: String = row.try_get("feed_id")?;
        let pub_date_str: Option<String> = row.try_get("pub_date")?;
        let processed_int: i32 = row.try_get("processed")?;
        let matched_episode_id_str: Option<String> = row.try_get("matched_episode_id")?;
        let torrent_id_str: Option<String> = row.try_get("torrent_id")?;
        let seen_at_str: String = row.try_get("seen_at")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            feed_id: str_to_uuid(&feed_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            guid: row.try_get("guid")?,
            link_hash: row.try_get("link_hash")?,
            title_hash: row.try_get("title_hash")?,
            title: row.try_get("title")?,
            link: row.try_get("link")?,
            pub_date: str_to_datetime_opt(pub_date_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            description: row.try_get("description")?,
            parsed_show_name: row.try_get("parsed_show_name")?,
            parsed_season: row.try_get("parsed_season")?,
            parsed_episode: row.try_get("parsed_episode")?,
            parsed_resolution: row.try_get("parsed_resolution")?,
            parsed_codec: row.try_get("parsed_codec")?,
            parsed_source: row.try_get("parsed_source")?,
            parsed_audio: row.try_get("parsed_audio")?,
            parsed_hdr: row.try_get("parsed_hdr")?,
            processed: int_to_bool(processed_int),
            matched_episode_id: str_to_uuid_opt(matched_episode_id_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            torrent_id: str_to_uuid_opt(torrent_id_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            skipped_reason: row.try_get("skipped_reason")?,
            seen_at: str_to_datetime(&seen_at_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
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
    pub parsed_audio: Option<String>,
    pub parsed_hdr: Option<String>,
}

pub struct RssFeedRepository {
    pool: DbPool,
}

impl RssFeedRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get all RSS feeds for a user
    #[cfg(feature = "postgres")]
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<RssFeedRecord>> {
        let records = sqlx::query_as::<_, RssFeedRecord>(
            r#"
            SELECT id, user_id, library_id, name, url, enabled,
                   poll_interval_minutes, post_download_action, last_polled_at, last_successful_at,
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

    #[cfg(feature = "sqlite")]
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<RssFeedRecord>> {
        let records = sqlx::query_as::<_, RssFeedRecord>(
            r#"
            SELECT id, user_id, library_id, name, url, enabled,
                   poll_interval_minutes, post_download_action, last_polled_at, last_successful_at,
                   last_error, consecutive_failures, created_at, updated_at
            FROM rss_feeds
            WHERE user_id = ?1
            ORDER BY name
            "#,
        )
        .bind(uuid_to_str(user_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get RSS feeds for a specific library
    #[cfg(feature = "postgres")]
    pub async fn list_by_library(&self, library_id: Uuid) -> Result<Vec<RssFeedRecord>> {
        let records = sqlx::query_as::<_, RssFeedRecord>(
            r#"
            SELECT id, user_id, library_id, name, url, enabled,
                   poll_interval_minutes, post_download_action, last_polled_at, last_successful_at,
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

    #[cfg(feature = "sqlite")]
    pub async fn list_by_library(&self, library_id: Uuid) -> Result<Vec<RssFeedRecord>> {
        let records = sqlx::query_as::<_, RssFeedRecord>(
            r#"
            SELECT id, user_id, library_id, name, url, enabled,
                   poll_interval_minutes, post_download_action, last_polled_at, last_successful_at,
                   last_error, consecutive_failures, created_at, updated_at
            FROM rss_feeds
            WHERE library_id = ?1 OR library_id IS NULL
            ORDER BY name
            "#,
        )
        .bind(uuid_to_str(library_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get enabled feeds that need polling
    #[cfg(feature = "postgres")]
    pub async fn list_due_for_poll(&self) -> Result<Vec<RssFeedRecord>> {
        let records = sqlx::query_as::<_, RssFeedRecord>(
            r#"
            SELECT id, user_id, library_id, name, url, enabled,
                   poll_interval_minutes, post_download_action, last_polled_at, last_successful_at,
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

    #[cfg(feature = "sqlite")]
    pub async fn list_due_for_poll(&self) -> Result<Vec<RssFeedRecord>> {
        let records = sqlx::query_as::<_, RssFeedRecord>(
            r#"
            SELECT id, user_id, library_id, name, url, enabled,
                   poll_interval_minutes, post_download_action, last_polled_at, last_successful_at,
                   last_error, consecutive_failures, created_at, updated_at
            FROM rss_feeds
            WHERE enabled = 1
              AND (last_polled_at IS NULL 
                   OR datetime(last_polled_at, '+' || poll_interval_minutes || ' minutes') < datetime('now'))
            ORDER BY last_polled_at IS NOT NULL, last_polled_at
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get an RSS feed by ID
    #[cfg(feature = "postgres")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<RssFeedRecord>> {
        let record = sqlx::query_as::<_, RssFeedRecord>(
            r#"
            SELECT id, user_id, library_id, name, url, enabled,
                   poll_interval_minutes, post_download_action, last_polled_at, last_successful_at,
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

    #[cfg(feature = "sqlite")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<RssFeedRecord>> {
        let record = sqlx::query_as::<_, RssFeedRecord>(
            r#"
            SELECT id, user_id, library_id, name, url, enabled,
                   poll_interval_minutes, post_download_action, last_polled_at, last_successful_at,
                   last_error, consecutive_failures, created_at, updated_at
            FROM rss_feeds
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create a new RSS feed
    #[cfg(feature = "postgres")]
    pub async fn create(&self, input: CreateRssFeed) -> Result<RssFeedRecord> {
        let record = sqlx::query_as::<_, RssFeedRecord>(
            r#"
            INSERT INTO rss_feeds (
                user_id, library_id, name, url, enabled, poll_interval_minutes
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, user_id, library_id, name, url, enabled,
                      poll_interval_minutes, post_download_action, last_polled_at, last_successful_at,
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

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, input: CreateRssFeed) -> Result<RssFeedRecord> {
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            INSERT INTO rss_feeds (
                id, user_id, library_id, name, url, enabled, poll_interval_minutes,
                created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(uuid_to_str(input.user_id))
        .bind(input.library_id.map(uuid_to_str))
        .bind(&input.name)
        .bind(&input.url)
        .bind(bool_to_int(input.enabled))
        .bind(input.poll_interval_minutes)
        .execute(&self.pool)
        .await?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve RSS feed after insert"))
    }

    /// Update an RSS feed
    #[cfg(feature = "postgres")]
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
                      poll_interval_minutes, post_download_action, last_polled_at, last_successful_at,
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

    #[cfg(feature = "sqlite")]
    pub async fn update(&self, id: Uuid, input: UpdateRssFeed) -> Result<Option<RssFeedRecord>> {
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            UPDATE rss_feeds SET
                name = COALESCE(?2, name),
                url = COALESCE(?3, url),
                library_id = COALESCE(?4, library_id),
                enabled = COALESCE(?5, enabled),
                poll_interval_minutes = COALESCE(?6, poll_interval_minutes),
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(&id_str)
        .bind(&input.name)
        .bind(&input.url)
        .bind(input.library_id.map(uuid_to_str))
        .bind(input.enabled.map(bool_to_int))
        .bind(input.poll_interval_minutes)
        .execute(&self.pool)
        .await?;

        self.get_by_id(id).await
    }

    /// Delete an RSS feed
    #[cfg(feature = "postgres")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM rss_feeds WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM rss_feeds WHERE id = ?1")
            .bind(uuid_to_str(id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update poll status (success)
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn mark_poll_success(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE rss_feeds SET
                last_polled_at = datetime('now'),
                last_successful_at = datetime('now'),
                last_error = NULL,
                consecutive_failures = 0,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update poll status (failure)
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn mark_poll_failure(&self, id: Uuid, error: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE rss_feeds SET
                last_polled_at = datetime('now'),
                last_error = ?2,
                consecutive_failures = COALESCE(consecutive_failures, 0) + 1,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ========== RSS Feed Items ==========

    /// Check if an item already exists (for deduplication)
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn item_exists(&self, feed_id: Uuid, link_hash: &str) -> Result<bool> {
        let count: (i32,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM rss_feed_items 
            WHERE feed_id = ?1 AND link_hash = ?2
            "#,
        )
        .bind(uuid_to_str(feed_id))
        .bind(link_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0 > 0)
    }

    /// Create a new RSS feed item
    #[cfg(feature = "postgres")]
    pub async fn create_item(&self, input: CreateRssFeedItem) -> Result<RssFeedItemRecord> {
        let record = sqlx::query_as::<_, RssFeedItemRecord>(
            r#"
            INSERT INTO rss_feed_items (
                feed_id, guid, link_hash, title_hash, title, link,
                pub_date, description, parsed_show_name, parsed_season,
                parsed_episode, parsed_resolution, parsed_codec, parsed_source,
                parsed_audio, parsed_hdr
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            ON CONFLICT (feed_id, link_hash) DO UPDATE SET
                seen_at = NOW()
            RETURNING id, feed_id, guid, link_hash, title_hash, title, link,
                      pub_date, description, parsed_show_name, parsed_season,
                      parsed_episode, parsed_resolution, parsed_codec, parsed_source,
                      parsed_audio, parsed_hdr,
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
        .bind(&input.parsed_audio)
        .bind(&input.parsed_hdr)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn create_item(&self, input: CreateRssFeedItem) -> Result<RssFeedItemRecord> {
        use crate::db::sqlite_helpers::datetime_to_str;

        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);
        let feed_id_str = uuid_to_str(input.feed_id);

        // Try to insert, on conflict just update seen_at
        sqlx::query(
            r#"
            INSERT INTO rss_feed_items (
                id, feed_id, guid, link_hash, title_hash, title, link,
                pub_date, description, parsed_show_name, parsed_season,
                parsed_episode, parsed_resolution, parsed_codec, parsed_source,
                parsed_audio, parsed_hdr, processed, seen_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, 0, datetime('now'))
            ON CONFLICT (feed_id, link_hash) DO UPDATE SET
                seen_at = datetime('now')
            "#,
        )
        .bind(&id_str)
        .bind(&feed_id_str)
        .bind(&input.guid)
        .bind(&input.link_hash)
        .bind(&input.title_hash)
        .bind(&input.title)
        .bind(&input.link)
        .bind(input.pub_date.map(datetime_to_str))
        .bind(&input.description)
        .bind(&input.parsed_show_name)
        .bind(input.parsed_season)
        .bind(input.parsed_episode)
        .bind(&input.parsed_resolution)
        .bind(&input.parsed_codec)
        .bind(&input.parsed_source)
        .bind(&input.parsed_audio)
        .bind(&input.parsed_hdr)
        .execute(&self.pool)
        .await?;

        // Fetch the record (could be existing or new)
        let record = sqlx::query_as::<_, RssFeedItemRecord>(
            r#"
            SELECT id, feed_id, guid, link_hash, title_hash, title, link,
                   pub_date, description, parsed_show_name, parsed_season,
                   parsed_episode, parsed_resolution, parsed_codec, parsed_source,
                   parsed_audio, parsed_hdr,
                   processed, matched_episode_id, torrent_id, skipped_reason, seen_at
            FROM rss_feed_items
            WHERE feed_id = ?1 AND link_hash = ?2
            "#,
        )
        .bind(&feed_id_str)
        .bind(&input.link_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get unprocessed items for a feed
    #[cfg(feature = "postgres")]
    pub async fn get_unprocessed_items(&self, feed_id: Uuid) -> Result<Vec<RssFeedItemRecord>> {
        let records = sqlx::query_as::<_, RssFeedItemRecord>(
            r#"
            SELECT id, feed_id, guid, link_hash, title_hash, title, link,
                   pub_date, description, parsed_show_name, parsed_season,
                   parsed_episode, parsed_resolution, parsed_codec, parsed_source,
                   parsed_audio, parsed_hdr,
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

    #[cfg(feature = "sqlite")]
    pub async fn get_unprocessed_items(&self, feed_id: Uuid) -> Result<Vec<RssFeedItemRecord>> {
        let records = sqlx::query_as::<_, RssFeedItemRecord>(
            r#"
            SELECT id, feed_id, guid, link_hash, title_hash, title, link,
                   pub_date, description, parsed_show_name, parsed_season,
                   parsed_episode, parsed_resolution, parsed_codec, parsed_source,
                   parsed_audio, parsed_hdr,
                   processed, matched_episode_id, torrent_id, skipped_reason, seen_at
            FROM rss_feed_items
            WHERE feed_id = ?1 AND processed = 0
            ORDER BY pub_date IS NULL, pub_date DESC
            "#,
        )
        .bind(uuid_to_str(feed_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Mark item as processed
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
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
                processed = 1,
                matched_episode_id = ?2,
                torrent_id = ?3,
                skipped_reason = ?4
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(matched_episode_id.map(uuid_to_str))
        .bind(torrent_id.map(uuid_to_str))
        .bind(skipped_reason)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Clean up old items (keep last N days)
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn cleanup_old_items(&self, days: i32) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM rss_feed_items
            WHERE datetime(seen_at, '+' || ?1 || ' days') < datetime('now')
            "#,
        )
        .bind(days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
