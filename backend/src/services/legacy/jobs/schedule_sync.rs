//! TV Schedule sync job
//!
//! Periodically fetches the TV schedule from TVMaze and caches it in the database.
//! This reduces API calls and ensures fast schedule queries.

use anyhow::Result;
use tracing::{debug, error, info, warn};

type DbPool = crate::db::Pool;

use crate::db::{ScheduleRepository, UpsertScheduleEntry};
use crate::services::tvmaze::TvMazeClient;

/// Default number of days to fetch
const DEFAULT_SYNC_DAYS: u32 = 14;

/// Countries to sync by default
const DEFAULT_COUNTRIES: &[&str] = &["US", "GB"];

/// Maximum cache age in minutes before auto-refresh
const CACHE_MAX_AGE_MINUTES: i64 = 60 * 6; // 6 hours

/// Sync TV schedule from TVMaze for configured countries
pub async fn sync_schedule(pool: DbPool) -> Result<()> {
    info!("Starting TV schedule sync");

    let client = TvMazeClient::new();
    let repo = ScheduleRepository::new(pool.clone());

    // Sync each configured country
    for &country in DEFAULT_COUNTRIES {
        match sync_country(&client, &repo, country, DEFAULT_SYNC_DAYS).await {
            Ok(count) => {
                info!("Schedule sync completed for {}: {} entries", country, count);
            }
            Err(e) => {
                error!(country = country, error = %e, "Failed to sync schedule for country");
                // Update sync state with error
                if let Err(e2) = repo
                    .update_sync_state(country, 0, Some(&e.to_string()))
                    .await
                {
                    warn!(error = %e2, "Failed to update sync state with error");
                }
            }
        }
    }

    // Cleanup old entries (older than 7 days ago)
    let cleanup_date = time::OffsetDateTime::now_utc().date() - time::Duration::days(7);
    match repo.delete_before(cleanup_date).await {
        Ok(deleted) if deleted > 0 => {
            info!("Cleaned up {} old schedule entries", deleted);
        }
        Ok(_) => {}
        Err(e) => {
            warn!(error = %e, "Failed to cleanup old schedule entries");
        }
    }

    info!("TV schedule sync finished");
    Ok(())
}

/// Sync schedule for a specific country
async fn sync_country(
    client: &TvMazeClient,
    repo: &ScheduleRepository,
    country: &str,
    days: u32,
) -> Result<usize> {
    debug!("Syncing {} day TV schedule for {}", days, country);

    // Fetch schedule from TVMaze
    let schedule = client.get_upcoming_schedule(days, Some(country)).await?;
    debug!(
        country = country,
        entries = schedule.len(),
        "Fetched schedule from TVMaze"
    );

    // Convert to database entries
    let entries: Vec<UpsertScheduleEntry> = schedule
        .into_iter()
        .filter_map(|entry| {
            // Skip entries without season/episode numbers (specials, etc.)
            let season = entry.season?;
            let episode_number = entry.number?;

            // Parse air_date
            let air_date = entry.airdate.as_ref().and_then(|d| {
                time::Date::parse(d, time::macros::format_description!("[year]-[month]-[day]")).ok()
            })?;

            // Parse air_stamp
            let air_stamp = entry.air_stamp.as_ref().and_then(|s| {
                time::OffsetDateTime::parse(
                    s,
                    &time::format_description::well_known::Iso8601::DEFAULT,
                )
                .ok()
            });

            // Strip HTML from summary
            let summary = entry.summary.map(|s| {
                let re = regex::Regex::new(r"<[^>]+>").unwrap();
                re.replace_all(&s, "").trim().to_string()
            });

            // Get network name
            let show_network = entry
                .show
                .network
                .as_ref()
                .map(|n| n.name.clone())
                .or_else(|| entry.show.web_channel.as_ref().map(|w| w.name.clone()));

            Some(UpsertScheduleEntry {
                tvmaze_episode_id: entry.id as i32,
                episode_name: entry.name,
                season: season as i32,
                episode_number: episode_number as i32,
                episode_type: entry.episode_type,
                air_date,
                air_time: entry.airtime,
                air_stamp,
                runtime: entry.runtime.map(|r| r as i32),
                episode_image_url: entry.image.as_ref().and_then(|i| i.medium.clone()),
                summary,
                tvmaze_show_id: entry.show.id as i32,
                show_name: entry.show.name,
                show_network,
                show_poster_url: entry.show.image.as_ref().and_then(|i| i.medium.clone()),
                show_genres: entry.show.genres,
                country_code: country.to_string(),
            })
        })
        .collect();

    let count = entries.len();

    // Upsert to database
    repo.upsert_batch(entries).await?;

    // Update sync state
    repo.update_sync_state(country, days as i32, None).await?;

    Ok(count)
}

/// Check if any country needs a sync (cache is stale)
pub async fn needs_sync(pool: DbPool) -> Result<bool> {
    let repo = ScheduleRepository::new(pool);

    for &country in DEFAULT_COUNTRIES {
        if repo.is_cache_stale(country, CACHE_MAX_AGE_MINUTES).await? {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Sync a specific country on demand
pub async fn sync_country_on_demand(pool: DbPool, country: &str, days: u32) -> Result<usize> {
    let client = TvMazeClient::new();
    let repo = ScheduleRepository::new(pool);

    sync_country(&client, &repo, country, days).await
}

/// Get the configured countries for sync
pub fn get_sync_countries() -> &'static [&'static str] {
    DEFAULT_COUNTRIES
}
