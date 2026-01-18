//! Schedule cache database operations

use anyhow::Result;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// A cached schedule entry in the database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScheduleCacheRecord {
    pub id: Uuid,
    pub tvmaze_episode_id: i32,
    pub episode_name: String,
    pub season: i32,
    pub episode_number: i32,
    pub episode_type: Option<String>,
    pub air_date: time::Date,
    pub air_time: Option<String>,
    pub air_stamp: Option<OffsetDateTime>,
    pub runtime: Option<i32>,
    pub episode_image_url: Option<String>,
    pub summary: Option<String>,
    pub tvmaze_show_id: i32,
    pub show_name: String,
    pub show_network: Option<String>,
    pub show_poster_url: Option<String>,
    pub show_genres: Vec<String>,
    pub country_code: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Input for creating/upserting a schedule cache entry
#[derive(Debug, Clone)]
pub struct UpsertScheduleEntry {
    pub tvmaze_episode_id: i32,
    pub episode_name: String,
    pub season: i32,
    pub episode_number: i32,
    pub episode_type: Option<String>,
    pub air_date: time::Date,
    pub air_time: Option<String>,
    pub air_stamp: Option<OffsetDateTime>,
    pub runtime: Option<i32>,
    pub episode_image_url: Option<String>,
    pub summary: Option<String>,
    pub tvmaze_show_id: i32,
    pub show_name: String,
    pub show_network: Option<String>,
    pub show_poster_url: Option<String>,
    pub show_genres: Vec<String>,
    pub country_code: String,
}

/// Schedule sync state record
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScheduleSyncStateRecord {
    pub id: Uuid,
    pub country_code: String,
    pub last_synced_at: OffsetDateTime,
    pub last_sync_days: i32,
    pub sync_error: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Schedule repository for database operations
pub struct ScheduleRepository {
    pool: PgPool,
}

impl ScheduleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert a schedule cache entry
    pub async fn upsert_entry(&self, entry: UpsertScheduleEntry) -> Result<ScheduleCacheRecord> {
        let record = sqlx::query_as::<_, ScheduleCacheRecord>(
            r#"
            INSERT INTO schedule_cache (
                tvmaze_episode_id, episode_name, season, episode_number, episode_type,
                air_date, air_time, air_stamp, runtime, episode_image_url, summary,
                tvmaze_show_id, show_name, show_network, show_poster_url, show_genres,
                country_code
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            ON CONFLICT (tvmaze_episode_id, country_code)
            DO UPDATE SET
                episode_name = EXCLUDED.episode_name,
                season = EXCLUDED.season,
                episode_number = EXCLUDED.episode_number,
                episode_type = EXCLUDED.episode_type,
                air_date = EXCLUDED.air_date,
                air_time = EXCLUDED.air_time,
                air_stamp = EXCLUDED.air_stamp,
                runtime = EXCLUDED.runtime,
                episode_image_url = EXCLUDED.episode_image_url,
                summary = EXCLUDED.summary,
                tvmaze_show_id = EXCLUDED.tvmaze_show_id,
                show_name = EXCLUDED.show_name,
                show_network = EXCLUDED.show_network,
                show_poster_url = EXCLUDED.show_poster_url,
                show_genres = EXCLUDED.show_genres,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(entry.tvmaze_episode_id)
        .bind(&entry.episode_name)
        .bind(entry.season)
        .bind(entry.episode_number)
        .bind(&entry.episode_type)
        .bind(entry.air_date)
        .bind(&entry.air_time)
        .bind(entry.air_stamp)
        .bind(entry.runtime)
        .bind(&entry.episode_image_url)
        .bind(&entry.summary)
        .bind(entry.tvmaze_show_id)
        .bind(&entry.show_name)
        .bind(&entry.show_network)
        .bind(&entry.show_poster_url)
        .bind(&entry.show_genres)
        .bind(&entry.country_code)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Upsert multiple entries in a batch
    pub async fn upsert_batch(&self, entries: Vec<UpsertScheduleEntry>) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await?;
        let mut count = 0;

        for entry in entries {
            sqlx::query(
                r#"
                INSERT INTO schedule_cache (
                    tvmaze_episode_id, episode_name, season, episode_number, episode_type,
                    air_date, air_time, air_stamp, runtime, episode_image_url, summary,
                    tvmaze_show_id, show_name, show_network, show_poster_url, show_genres,
                    country_code
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
                ON CONFLICT (tvmaze_episode_id, country_code)
                DO UPDATE SET
                    episode_name = EXCLUDED.episode_name,
                    season = EXCLUDED.season,
                    episode_number = EXCLUDED.episode_number,
                    episode_type = EXCLUDED.episode_type,
                    air_date = EXCLUDED.air_date,
                    air_time = EXCLUDED.air_time,
                    air_stamp = EXCLUDED.air_stamp,
                    runtime = EXCLUDED.runtime,
                    episode_image_url = EXCLUDED.episode_image_url,
                    summary = EXCLUDED.summary,
                    tvmaze_show_id = EXCLUDED.tvmaze_show_id,
                    show_name = EXCLUDED.show_name,
                    show_network = EXCLUDED.show_network,
                    show_poster_url = EXCLUDED.show_poster_url,
                    show_genres = EXCLUDED.show_genres,
                    updated_at = NOW()
                "#,
            )
            .bind(entry.tvmaze_episode_id)
            .bind(&entry.episode_name)
            .bind(entry.season)
            .bind(entry.episode_number)
            .bind(&entry.episode_type)
            .bind(entry.air_date)
            .bind(&entry.air_time)
            .bind(entry.air_stamp)
            .bind(entry.runtime)
            .bind(&entry.episode_image_url)
            .bind(&entry.summary)
            .bind(entry.tvmaze_show_id)
            .bind(&entry.show_name)
            .bind(&entry.show_network)
            .bind(&entry.show_poster_url)
            .bind(&entry.show_genres)
            .bind(&entry.country_code)
            .execute(&mut *tx)
            .await?;
            count += 1;
        }

        tx.commit().await?;
        Ok(count)
    }

    /// Get upcoming episodes for the next N days from cache
    pub async fn get_upcoming(
        &self,
        days: i32,
        country_code: Option<&str>,
    ) -> Result<Vec<ScheduleCacheRecord>> {
        let today = time::OffsetDateTime::now_utc().date();
        let end_date = today + time::Duration::days(days as i64);
        let country = country_code.unwrap_or("US");

        let records = sqlx::query_as::<_, ScheduleCacheRecord>(
            r#"
            SELECT * FROM schedule_cache
            WHERE country_code = $1
              AND air_date >= $2
              AND air_date <= $3
            ORDER BY air_date, air_time
            "#,
        )
        .bind(country)
        .bind(today)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get schedule entries for a specific date
    pub async fn get_by_date(
        &self,
        date: time::Date,
        country_code: Option<&str>,
    ) -> Result<Vec<ScheduleCacheRecord>> {
        let country = country_code.unwrap_or("US");

        let records = sqlx::query_as::<_, ScheduleCacheRecord>(
            r#"
            SELECT * FROM schedule_cache
            WHERE country_code = $1 AND air_date = $2
            ORDER BY air_time
            "#,
        )
        .bind(country)
        .bind(date)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Delete old schedule entries (cleanup)
    pub async fn delete_before(&self, before_date: time::Date) -> Result<u64> {
        let result = sqlx::query("DELETE FROM schedule_cache WHERE air_date < $1")
            .bind(before_date)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Delete all entries for a country (for full refresh)
    pub async fn delete_by_country(&self, country_code: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM schedule_cache WHERE country_code = $1")
            .bind(country_code)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Get sync state for a country
    pub async fn get_sync_state(&self, country_code: &str) -> Result<Option<ScheduleSyncStateRecord>> {
        let record = sqlx::query_as::<_, ScheduleSyncStateRecord>(
            "SELECT * FROM schedule_sync_state WHERE country_code = $1",
        )
        .bind(country_code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update sync state
    pub async fn update_sync_state(
        &self,
        country_code: &str,
        days: i32,
        error: Option<&str>,
    ) -> Result<ScheduleSyncStateRecord> {
        let record = sqlx::query_as::<_, ScheduleSyncStateRecord>(
            r#"
            INSERT INTO schedule_sync_state (country_code, last_synced_at, last_sync_days, sync_error)
            VALUES ($1, NOW(), $2, $3)
            ON CONFLICT (country_code)
            DO UPDATE SET
                last_synced_at = NOW(),
                last_sync_days = EXCLUDED.last_sync_days,
                sync_error = EXCLUDED.sync_error,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(country_code)
        .bind(days)
        .bind(error)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Check if cache is stale (needs refresh)
    /// Returns true if cache is older than max_age_minutes
    pub async fn is_cache_stale(&self, country_code: &str, max_age_minutes: i64) -> Result<bool> {
        let state = self.get_sync_state(country_code).await?;

        match state {
            None => Ok(true), // No sync state = needs sync
            Some(s) => {
                let now = OffsetDateTime::now_utc();
                let age = now - s.last_synced_at;
                let max_age = time::Duration::minutes(max_age_minutes);
                Ok(age > max_age)
            }
        }
    }

    /// Get cache stats
    pub async fn get_stats(&self) -> Result<Vec<(String, i64, Option<OffsetDateTime>)>> {
        let stats = sqlx::query_as::<_, (String, i64, Option<OffsetDateTime>)>(
            r#"
            SELECT 
                sc.country_code,
                COUNT(*) as entry_count,
                ss.last_synced_at
            FROM schedule_cache sc
            LEFT JOIN schedule_sync_state ss ON sc.country_code = ss.country_code
            GROUP BY sc.country_code, ss.last_synced_at
            ORDER BY sc.country_code
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(stats)
    }
}
