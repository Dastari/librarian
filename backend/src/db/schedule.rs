//! Schedule cache database operations

use anyhow::Result;
use time::OffsetDateTime;
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "sqlite")]
use crate::db::sqlite_helpers::{
    json_to_vec, str_to_uuid, uuid_to_str, vec_to_json,
};

#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

/// A cached schedule entry in the database
#[derive(Debug, Clone)]
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


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for ScheduleCacheRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let air_date_str: String = row.try_get("air_date")?;
        let air_stamp_str: Option<String> = row.try_get("air_stamp")?;
        let show_genres_str: String = row.try_get("show_genres")?;
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            tvmaze_episode_id: row.try_get("tvmaze_episode_id")?,
            episode_name: row.try_get("episode_name")?,
            season: row.try_get("season")?,
            episode_number: row.try_get("episode_number")?,
            episode_type: row.try_get("episode_type")?,
            air_date: time::Date::parse(
                &air_date_str,
                time::macros::format_description!("[year]-[month]-[day]"),
            )
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            air_time: row.try_get("air_time")?,
            air_stamp: air_stamp_str
                .map(|s| {
                    OffsetDateTime::parse(&s, &time::format_description::well_known::Rfc3339)
                        .or_else(|_| {
                            // Try SQLite datetime format
                            let format = time::macros::format_description!(
                                "[year]-[month]-[day] [hour]:[minute]:[second]"
                            );
                            time::PrimitiveDateTime::parse(&s, format)
                                .map(|pdt| pdt.assume_utc())
                        })
                })
                .transpose()
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            runtime: row.try_get("runtime")?,
            episode_image_url: row.try_get("episode_image_url")?,
            summary: row.try_get("summary")?,
            tvmaze_show_id: row.try_get("tvmaze_show_id")?,
            show_name: row.try_get("show_name")?,
            show_network: row.try_get("show_network")?,
            show_poster_url: row.try_get("show_poster_url")?,
            show_genres: json_to_vec(&show_genres_str),
            country_code: row.try_get("country_code")?,
            created_at: {
                OffsetDateTime::parse(&created_at_str, &time::format_description::well_known::Rfc3339)
                    .or_else(|_| {
                        let format = time::macros::format_description!(
                            "[year]-[month]-[day] [hour]:[minute]:[second]"
                        );
                        time::PrimitiveDateTime::parse(&created_at_str, format)
                            .map(|pdt| pdt.assume_utc())
                    })
                    .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
            },
            updated_at: {
                OffsetDateTime::parse(&updated_at_str, &time::format_description::well_known::Rfc3339)
                    .or_else(|_| {
                        let format = time::macros::format_description!(
                            "[year]-[month]-[day] [hour]:[minute]:[second]"
                        );
                        time::PrimitiveDateTime::parse(&updated_at_str, format)
                            .map(|pdt| pdt.assume_utc())
                    })
                    .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
            },
        })
    }
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
#[derive(Debug, Clone)]
pub struct ScheduleSyncStateRecord {
    pub id: Uuid,
    pub country_code: String,
    pub last_synced_at: OffsetDateTime,
    pub last_sync_days: i32,
    pub sync_error: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for ScheduleSyncStateRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let last_synced_at_str: String = row.try_get("last_synced_at")?;
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;

        fn parse_datetime(s: &str) -> Result<OffsetDateTime, time::error::Parse> {
            OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).or_else(|_| {
                let format = time::macros::format_description!(
                    "[year]-[month]-[day] [hour]:[minute]:[second]"
                );
                time::PrimitiveDateTime::parse(s, format).map(|pdt| pdt.assume_utc())
            })
        }

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            country_code: row.try_get("country_code")?,
            last_synced_at: parse_datetime(&last_synced_at_str)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            last_sync_days: row.try_get("last_sync_days")?,
            sync_error: row.try_get("sync_error")?,
            created_at: parse_datetime(&created_at_str)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            updated_at: parse_datetime(&updated_at_str)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        })
    }
}

/// Schedule repository for database operations
pub struct ScheduleRepository {
    pool: DbPool,
}

impl ScheduleRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Upsert a schedule cache entry

    #[cfg(feature = "sqlite")]
    pub async fn upsert_entry(&self, entry: UpsertScheduleEntry) -> Result<ScheduleCacheRecord> {
        let id = uuid_to_str(Uuid::new_v4());
        let air_date_str = entry
            .air_date
            .format(time::macros::format_description!("[year]-[month]-[day]"))
            .unwrap_or_default();
        let air_stamp_str = entry.air_stamp.map(|dt| {
            dt.format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default()
        });
        let show_genres_json = vec_to_json(&entry.show_genres);

        sqlx::query(
            r#"
            INSERT INTO schedule_cache (
                id, tvmaze_episode_id, episode_name, season, episode_number, episode_type,
                air_date, air_time, air_stamp, runtime, episode_image_url, summary,
                tvmaze_show_id, show_name, show_network, show_poster_url, show_genres,
                country_code, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, datetime('now'), datetime('now'))
            ON CONFLICT (tvmaze_episode_id, country_code)
            DO UPDATE SET
                episode_name = excluded.episode_name,
                season = excluded.season,
                episode_number = excluded.episode_number,
                episode_type = excluded.episode_type,
                air_date = excluded.air_date,
                air_time = excluded.air_time,
                air_stamp = excluded.air_stamp,
                runtime = excluded.runtime,
                episode_image_url = excluded.episode_image_url,
                summary = excluded.summary,
                tvmaze_show_id = excluded.tvmaze_show_id,
                show_name = excluded.show_name,
                show_network = excluded.show_network,
                show_poster_url = excluded.show_poster_url,
                show_genres = excluded.show_genres,
                updated_at = datetime('now')
            "#,
        )
        .bind(&id)
        .bind(entry.tvmaze_episode_id)
        .bind(&entry.episode_name)
        .bind(entry.season)
        .bind(entry.episode_number)
        .bind(&entry.episode_type)
        .bind(&air_date_str)
        .bind(&entry.air_time)
        .bind(&air_stamp_str)
        .bind(entry.runtime)
        .bind(&entry.episode_image_url)
        .bind(&entry.summary)
        .bind(entry.tvmaze_show_id)
        .bind(&entry.show_name)
        .bind(&entry.show_network)
        .bind(&entry.show_poster_url)
        .bind(&show_genres_json)
        .bind(&entry.country_code)
        .execute(&self.pool)
        .await?;

        // Fetch the record back
        let record = sqlx::query_as::<_, ScheduleCacheRecord>(
            "SELECT * FROM schedule_cache WHERE tvmaze_episode_id = ?1 AND country_code = ?2",
        )
        .bind(entry.tvmaze_episode_id)
        .bind(&entry.country_code)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Upsert multiple entries in a batch

    #[cfg(feature = "sqlite")]
    pub async fn upsert_batch(&self, entries: Vec<UpsertScheduleEntry>) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await?;
        let mut count = 0;

        for entry in entries {
            let id = uuid_to_str(Uuid::new_v4());
            let air_date_str = entry
                .air_date
                .format(time::macros::format_description!("[year]-[month]-[day]"))
                .unwrap_or_default();
            let air_stamp_str = entry.air_stamp.map(|dt| {
                dt.format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default()
            });
            let show_genres_json = vec_to_json(&entry.show_genres);

            sqlx::query(
                r#"
                INSERT INTO schedule_cache (
                    id, tvmaze_episode_id, episode_name, season, episode_number, episode_type,
                    air_date, air_time, air_stamp, runtime, episode_image_url, summary,
                    tvmaze_show_id, show_name, show_network, show_poster_url, show_genres,
                    country_code, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, datetime('now'), datetime('now'))
                ON CONFLICT (tvmaze_episode_id, country_code)
                DO UPDATE SET
                    episode_name = excluded.episode_name,
                    season = excluded.season,
                    episode_number = excluded.episode_number,
                    episode_type = excluded.episode_type,
                    air_date = excluded.air_date,
                    air_time = excluded.air_time,
                    air_stamp = excluded.air_stamp,
                    runtime = excluded.runtime,
                    episode_image_url = excluded.episode_image_url,
                    summary = excluded.summary,
                    tvmaze_show_id = excluded.tvmaze_show_id,
                    show_name = excluded.show_name,
                    show_network = excluded.show_network,
                    show_poster_url = excluded.show_poster_url,
                    show_genres = excluded.show_genres,
                    updated_at = datetime('now')
                "#,
            )
            .bind(&id)
            .bind(entry.tvmaze_episode_id)
            .bind(&entry.episode_name)
            .bind(entry.season)
            .bind(entry.episode_number)
            .bind(&entry.episode_type)
            .bind(&air_date_str)
            .bind(&entry.air_time)
            .bind(&air_stamp_str)
            .bind(entry.runtime)
            .bind(&entry.episode_image_url)
            .bind(&entry.summary)
            .bind(entry.tvmaze_show_id)
            .bind(&entry.show_name)
            .bind(&entry.show_network)
            .bind(&entry.show_poster_url)
            .bind(&show_genres_json)
            .bind(&entry.country_code)
            .execute(&mut *tx)
            .await?;
            count += 1;
        }

        tx.commit().await?;
        Ok(count)
    }

    /// Get upcoming episodes for the next N days from cache

    #[cfg(feature = "sqlite")]
    pub async fn get_upcoming(
        &self,
        days: i32,
        country_code: Option<&str>,
    ) -> Result<Vec<ScheduleCacheRecord>> {
        let today = time::OffsetDateTime::now_utc().date();
        let end_date = today + time::Duration::days(days as i64);
        let country = country_code.unwrap_or("US");

        let today_str = today
            .format(time::macros::format_description!("[year]-[month]-[day]"))
            .unwrap_or_default();
        let end_date_str = end_date
            .format(time::macros::format_description!("[year]-[month]-[day]"))
            .unwrap_or_default();

        let records = sqlx::query_as::<_, ScheduleCacheRecord>(
            r#"
            SELECT * FROM schedule_cache
            WHERE country_code = ?1
              AND air_date >= ?2
              AND air_date <= ?3
            ORDER BY air_date, air_time
            "#,
        )
        .bind(country)
        .bind(&today_str)
        .bind(&end_date_str)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get schedule entries for a specific date

    #[cfg(feature = "sqlite")]
    pub async fn get_by_date(
        &self,
        date: time::Date,
        country_code: Option<&str>,
    ) -> Result<Vec<ScheduleCacheRecord>> {
        let country = country_code.unwrap_or("US");
        let date_str = date
            .format(time::macros::format_description!("[year]-[month]-[day]"))
            .unwrap_or_default();

        let records = sqlx::query_as::<_, ScheduleCacheRecord>(
            r#"
            SELECT * FROM schedule_cache
            WHERE country_code = ?1 AND air_date = ?2
            ORDER BY air_time
            "#,
        )
        .bind(country)
        .bind(&date_str)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Delete old schedule entries (cleanup)

    #[cfg(feature = "sqlite")]
    pub async fn delete_before(&self, before_date: time::Date) -> Result<u64> {
        let date_str = before_date
            .format(time::macros::format_description!("[year]-[month]-[day]"))
            .unwrap_or_default();

        let result = sqlx::query("DELETE FROM schedule_cache WHERE air_date < ?1")
            .bind(&date_str)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Delete all entries for a country (for full refresh)

    #[cfg(feature = "sqlite")]
    pub async fn delete_by_country(&self, country_code: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM schedule_cache WHERE country_code = ?1")
            .bind(country_code)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Get sync state for a country

    #[cfg(feature = "sqlite")]
    pub async fn get_sync_state(
        &self,
        country_code: &str,
    ) -> Result<Option<ScheduleSyncStateRecord>> {
        let record = sqlx::query_as::<_, ScheduleSyncStateRecord>(
            "SELECT * FROM schedule_sync_state WHERE country_code = ?1",
        )
        .bind(country_code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update sync state

    #[cfg(feature = "sqlite")]
    pub async fn update_sync_state(
        &self,
        country_code: &str,
        days: i32,
        error: Option<&str>,
    ) -> Result<ScheduleSyncStateRecord> {
        let id = uuid_to_str(Uuid::new_v4());

        sqlx::query(
            r#"
            INSERT INTO schedule_sync_state (id, country_code, last_synced_at, last_sync_days, sync_error, created_at, updated_at)
            VALUES (?1, ?2, datetime('now'), ?3, ?4, datetime('now'), datetime('now'))
            ON CONFLICT (country_code)
            DO UPDATE SET
                last_synced_at = datetime('now'),
                last_sync_days = excluded.last_sync_days,
                sync_error = excluded.sync_error,
                updated_at = datetime('now')
            "#,
        )
        .bind(&id)
        .bind(country_code)
        .bind(days)
        .bind(error)
        .execute(&self.pool)
        .await?;

        // Fetch the record back
        self.get_sync_state(country_code)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve sync state after upsert"))
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

    #[cfg(feature = "sqlite")]
    pub async fn get_stats(&self) -> Result<Vec<(String, i64, Option<OffsetDateTime>)>> {
        // For SQLite, we need to manually parse the results due to type differences
        let rows = sqlx::query_as::<_, (String, i64, Option<String>)>(
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

        let stats: Vec<(String, i64, Option<OffsetDateTime>)> = rows
            .into_iter()
            .map(|(country, count, last_synced)| {
                let dt = last_synced.and_then(|s| {
                    OffsetDateTime::parse(&s, &time::format_description::well_known::Rfc3339)
                        .or_else(|_| {
                            let format = time::macros::format_description!(
                                "[year]-[month]-[day] [hour]:[minute]:[second]"
                            );
                            time::PrimitiveDateTime::parse(&s, format).map(|pdt| pdt.assume_utc())
                        })
                        .ok()
                });
                (country, count, dt)
            })
            .collect();

        Ok(stats)
    }
}
