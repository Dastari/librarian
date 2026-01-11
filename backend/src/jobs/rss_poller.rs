//! RSS/Torznab feed poller
//!
//! This job:
//! 1. Gets feeds that are due for polling
//! 2. Fetches and parses each feed
//! 3. Stores new items in the database
//! 4. Matches items to wanted episodes
//! 5. Updates episode status to "available" when matched

use anyhow::Result;
use chrono::Datelike;
use regex::Regex;
use sqlx::PgPool;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::db::{CreateRssFeedItem, Database, RssFeedRecord};
use crate::db::quality_profiles::QualityProfileRecord;
use crate::services::RssService;
use crate::services::ParsedRssItem;

/// Poll all RSS feeds that are due for polling
pub async fn poll_feeds() -> Result<()> {
    info!(job = "rss_poller", "Starting RSS feed poll job");

    // Get database pool from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/librarian".to_string());

    let pool = PgPool::connect(&database_url).await?;
    let db = Database::new(pool);

    poll_feeds_with_db(&db).await
}

/// Poll feeds with a provided database connection
pub async fn poll_feeds_with_db(db: &Database) -> Result<()> {
    let rss_service = RssService::new();

    // Get feeds that need polling
    let feeds = db.rss_feeds().list_due_for_poll().await?;
    info!(
        job = "rss_poller",
        feed_count = feeds.len(),
        "Found feeds due for polling"
    );

    for feed in feeds {
        if let Err(e) = poll_single_feed(db, &rss_service, &feed).await {
            error!(
                job = "rss_poller",
                feed_name = %feed.name,
                feed_id = %feed.id,
                error = %e,
                "Failed to poll feed: {}",
                feed.name
            );
            // Mark the feed as failed
            if let Err(mark_err) = db
                .rss_feeds()
                .mark_poll_failure(feed.id, &e.to_string())
                .await
            {
                error!(
                    job = "rss_poller",
                    feed_name = %feed.name,
                    error = %mark_err,
                    "Failed to mark feed poll failure"
                );
            }
        }
    }

    info!(job = "rss_poller", "RSS polling job completed");
    Ok(())
}

/// Poll a single RSS feed by ID (for manual polling via GraphQL)
pub async fn poll_single_feed_by_id(
    db: &Database,
    feed_id: Uuid,
) -> Result<(i32, i32)> {
    let feed = db
        .rss_feeds()
        .get_by_id(feed_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("RSS feed not found"))?;
    
    let rss_service = RssService::new();
    poll_single_feed(db, &rss_service, &feed).await
}

/// Poll a single RSS feed
async fn poll_single_feed(
    db: &Database,
    rss_service: &RssService,
    feed: &RssFeedRecord,
) -> Result<(i32, i32)> {
    info!(
        job = "rss_poller",
        feed_name = %feed.name,
        feed_id = %feed.id,
        "Polling feed: {}",
        feed.name
    );

    // Fetch and parse the feed
    let items = rss_service.fetch_feed(&feed.url).await?;
    info!(
        job = "rss_poller",
        feed_name = %feed.name,
        item_count = items.len(),
        "Fetched items from feed: {}",
        feed.name
    );

    let mut new_items = 0;
    let mut matched_episodes = 0;

    for item in items {
        // Check if we've already seen this item
        let exists = db
            .rss_feeds()
            .item_exists(feed.id, &item.link_hash)
            .await?;

        if exists {
            debug!("Skipping existing item: {}", item.title);
            continue;
        }

        // Transform the link if needed (e.g., IPTorrents page URL -> download URL)
        let download_link = transform_torrent_link(&item, &feed.url);
        
        // Clone resolution before moving item fields
        let parsed_resolution = item.parsed_resolution.clone();

        // Store the new item with the transformed download link
        let rss_item = db
            .rss_feeds()
            .create_item(CreateRssFeedItem {
                feed_id: feed.id,
                guid: item.guid,
                link_hash: item.link_hash,
                title_hash: item.title_hash,
                title: item.title.clone(),
                link: download_link.clone(),
                pub_date: item.pub_date,
                description: item.description,
                parsed_show_name: item.parsed_show_name.clone(),
                parsed_season: item.parsed_season,
                parsed_episode: item.parsed_episode,
                parsed_resolution: item.parsed_resolution,
                parsed_codec: item.parsed_codec,
                parsed_source: item.parsed_source,
            })
            .await?;

        new_items += 1;

        // Try to match this item to a wanted episode
        if let Some(ref show_name) = item.parsed_show_name {
            if let (Some(season), Some(episode)) = (item.parsed_season, item.parsed_episode) {
                if let Some(matched) =
                    try_match_episode(
                        db,
                        show_name,
                        season,
                        episode,
                        &download_link,
                        rss_item.id,
                        parsed_resolution.as_deref(),
                    )
                        .await?
                {
                    info!(
                        job = "rss_poller",
                        feed_name = %feed.name,
                        show_name = %show_name,
                        season = season,
                        episode = episode,
                        episode_id = %matched,
                        "Matched RSS item to episode: {} S{:02}E{:02}",
                        show_name, season, episode
                    );
                    matched_episodes += 1;

                    // Mark the RSS item as processed with the matched episode
                    db.rss_feeds()
                        .mark_item_processed(rss_item.id, Some(matched), None, None)
                        .await?;
                } else {
                    // No match found, mark as processed with reason
                    db.rss_feeds()
                        .mark_item_processed(
                            rss_item.id,
                            None,
                            None,
                            Some("No matching wanted episode"),
                        )
                        .await?;
                }
            } else {
                // Couldn't parse season/episode, mark as skipped
                db.rss_feeds()
                    .mark_item_processed(
                        rss_item.id,
                        None,
                        None,
                        Some("Could not parse season/episode from title"),
                    )
                    .await?;
            }
        } else {
            // Couldn't parse show name, mark as skipped
            db.rss_feeds()
                .mark_item_processed(
                    rss_item.id,
                    None,
                    None,
                    Some("Could not parse show name from title"),
                )
                .await?;
        }
    }

    // Mark the feed as successfully polled
    db.rss_feeds().mark_poll_success(feed.id).await?;

    info!(
        job = "rss_poller",
        feed_name = %feed.name,
        new_items = new_items,
        matched_episodes = matched_episodes,
        "Feed poll complete for {}: {} new items, {} matched episodes",
        feed.name, new_items, matched_episodes
    );

    Ok((new_items, matched_episodes))
}

/// Get the effective quality profile for a show (show's profile or library default)
async fn get_effective_quality_profile(
    db: &Database,
    show_id: Uuid,
) -> Result<Option<QualityProfileRecord>> {
    // Get the show
    let show = db.tv_shows().get_by_id(show_id).await?;
    let show = match show {
        Some(s) => s,
        None => return Ok(None),
    };

    // First try show's quality profile
    if let Some(profile_id) = show.quality_profile_id {
        if let Some(profile) = db.quality_profiles().get_by_id(profile_id).await? {
            return Ok(Some(profile));
        }
    }

    // Fall back to library's default quality profile
    let library = db.libraries().get_by_id(show.library_id).await?;
    if let Some(library) = library {
        if let Some(profile_id) = library.default_quality_profile_id {
            if let Some(profile) = db.quality_profiles().get_by_id(profile_id).await? {
                return Ok(Some(profile));
            }
        }
    }

    Ok(None)
}

/// Check if an RSS item's resolution matches the quality profile
fn matches_quality_profile(
    parsed_resolution: Option<&str>,
    profile: &QualityProfileRecord,
) -> bool {
    // If profile allows "any" resolution, always match
    if profile.preferred_resolution.as_deref() == Some("any")
        || profile.min_resolution.as_deref() == Some("any")
    {
        return true;
    }

    // If we couldn't parse resolution from the RSS item, allow it (conservative)
    let resolution = match parsed_resolution {
        Some(r) => r.to_lowercase(),
        None => return true,
    };

    // Convert resolution strings to numeric values for comparison
    fn resolution_value(res: &str) -> i32 {
        match res.to_lowercase().as_str() {
            "2160p" | "4k" | "uhd" => 2160,
            "1080p" | "fullhd" | "fhd" => 1080,
            "720p" | "hd" => 720,
            "480p" | "sd" => 480,
            "any" => 0,
            _ => 0,
        }
    }

    let item_res = resolution_value(&resolution);
    
    // Check minimum resolution
    if let Some(min_res) = &profile.min_resolution {
        let min_val = resolution_value(min_res);
        if min_val > 0 && item_res > 0 && item_res < min_val {
            debug!(
                "Quality filter: {} < min resolution {} - rejecting",
                resolution, min_res
            );
            return false;
        }
    }

    // Check preferred resolution - if set, prefer matching it but don't reject
    // unless it's way off (more than one tier below)
    if let Some(pref_res) = &profile.preferred_resolution {
        let pref_val = resolution_value(pref_res);
        if pref_val > 0 && item_res > 0 {
            // Calculate tier difference (each tier is roughly 2x)
            let tier_diff = if pref_val > item_res {
                (pref_val / item_res.max(1)) as f32
            } else {
                1.0
            };
            
            // Reject if more than 2 tiers below preferred
            if tier_diff > 2.5 {
                debug!(
                    "Quality filter: {} is significantly below preferred {} - rejecting",
                    resolution, pref_res
                );
                return false;
            }
        }
    }

    true
}

/// Try to match an RSS item to a wanted episode
/// Uses multiple strategies to handle different numbering schemes
async fn try_match_episode(
    db: &Database,
    show_name: &str,
    season: i32,
    episode: i32,
    torrent_link: &str,
    rss_item_id: Uuid,
    parsed_resolution: Option<&str>,
) -> Result<Option<Uuid>> {
    // Normalize the show name for matching
    let normalized_name = normalize_show_name(show_name);

    // Find TV shows that match this name
    let shows = find_matching_shows(db, &normalized_name).await?;

    if shows.is_empty() {
        debug!(
            "No monitored shows match name: {} (normalized: {})",
            show_name, normalized_name
        );
        return Ok(None);
    }

    info!(
        job = "rss_poller",
        show_name = %show_name,
        match_count = shows.len(),
        season = season,
        episode = episode,
        "Found matching show(s) for '{}', looking for S{:02}E{:02}",
        show_name, season, episode
    );

    // Strategy 1: Try exact season/episode match
    for show_id in &shows {
        // Check quality profile before matching
        if let Some(profile) = get_effective_quality_profile(db, *show_id).await? {
            if !matches_quality_profile(parsed_resolution, &profile) {
                info!(
                    job = "rss_poller",
                    show_name = %show_name,
                    resolution = ?parsed_resolution,
                    profile_name = %profile.name,
                    "Skipping: quality doesn't match profile '{}'",
                    profile.name
                );
                continue;
            }
        }

        if let Some(ep_id) = try_exact_match(db, *show_id, season, episode, torrent_link, rss_item_id).await? {
            info!("Strategy 1 (exact match) succeeded for S{:02}E{:02}", season, episode);
            return Ok(Some(ep_id));
        }
    }

    // Strategy 2: Try year-based season matching
    // Some shows (like talk shows, soaps) use the year as season number
    // e.g., S02E04 in scene naming might be Season 2026, Episode 4
    let current_year = chrono::Utc::now().year();
    let year_seasons = [current_year, current_year - 1, current_year - 2];
    
    for show_id in &shows {
        // Check quality profile before matching
        if let Some(profile) = get_effective_quality_profile(db, *show_id).await? {
            if !matches_quality_profile(parsed_resolution, &profile) {
                continue; // Already logged in Strategy 1
            }
        }

        for &year_season in &year_seasons {
            if let Some(ep_id) = try_exact_match(db, *show_id, year_season, episode, torrent_link, rss_item_id).await? {
                info!(
                    "Strategy 2 (year-based season) succeeded: S{:02}E{:02} matched as Season {} Episode {}",
                    season, episode, year_season, episode
                );
                return Ok(Some(ep_id));
            }
        }
    }

    // Strategy 3: Try using absolute episode number
    // Scene S02E04 might be absolute episode number calculated from season start
    // This is a rough heuristic for shows that don't reset episode numbers per season
    for show_id in &shows {
        // Check quality profile before matching
        if let Some(profile) = get_effective_quality_profile(db, *show_id).await? {
            if !matches_quality_profile(parsed_resolution, &profile) {
                continue; // Already logged in Strategy 1
            }
        }

        if let Some(ep_id) = try_absolute_match(db, *show_id, season, episode, torrent_link, rss_item_id).await? {
            info!(
                "Strategy 3 (absolute episode) succeeded for S{:02}E{:02}",
                season, episode
            );
            return Ok(Some(ep_id));
        }
    }

    debug!(
        "No matching episode found for {} S{:02}E{:02} after all strategies",
        show_name, season, episode
    );
    Ok(None)
}

/// Try to match by exact season/episode number
async fn try_exact_match(
    db: &Database,
    show_id: Uuid,
    season: i32,
    episode: i32,
    torrent_link: &str,
    rss_item_id: Uuid,
) -> Result<Option<Uuid>> {
    let episode_record = db
        .episodes()
        .get_by_show_season_episode(show_id, season, episode)
        .await?;

    if let Some(ep) = episode_record {
        if ep.status == "missing" || ep.status == "wanted" {
            db.episodes()
                .mark_available(ep.id, torrent_link, Some(rss_item_id))
                .await?;
            return Ok(Some(ep.id));
        } else if ep.status == "available" {
            // Update torrent_link if we have a different/better one
            let current_link = ep.torrent_link.as_deref().unwrap_or("");
            if current_link != torrent_link {
                info!(
                    "Updating torrent link for already-available episode (old: {}, new: {})",
                    current_link, torrent_link
                );
                db.episodes()
                    .mark_available(ep.id, torrent_link, Some(rss_item_id))
                    .await?;
                return Ok(Some(ep.id));
            } else {
                debug!("Episode already available with same link, skipping");
            }
        } else {
            debug!(
                "Episode found but status is '{}', not matching",
                ep.status
            );
        }
    }

    Ok(None)
}

/// Try to match using absolute episode number calculation
/// For shows where scene uses S02E04 format but metadata has different numbering
async fn try_absolute_match(
    db: &Database,
    show_id: Uuid,
    scene_season: i32,
    scene_episode: i32,
    torrent_link: &str,
    rss_item_id: Uuid,
) -> Result<Option<Uuid>> {
    // For shows with year-based seasons (season > 2000) and absolute episode numbers,
    // we need to find the nth episode of that "scene season"
    
    // First, get all seasons for this show to understand the numbering
    let seasons: Vec<(i32,)> = sqlx::query_as(
        "SELECT DISTINCT season FROM episodes WHERE tv_show_id = $1 ORDER BY season"
    )
    .bind(show_id)
    .fetch_all(db.pool())
    .await?;

    let season_numbers: Vec<i32> = seasons.into_iter().map(|(s,)| s).collect();
    
    // Check if this show uses year-based seasons
    let uses_year_seasons = season_numbers.iter().any(|&s| s > 2000);
    
    if uses_year_seasons && scene_season > 0 && scene_season <= season_numbers.len() as i32 {
        // Map scene season to actual year-based season
        // Scene S01 = first year season, S02 = second year season, etc.
        let target_year_season = season_numbers.get((scene_season - 1) as usize);
        
        if let Some(&year_season) = target_year_season {
            // Now find the nth episode of that year season
            let episodes: Vec<(Uuid, i32, i32, String)> = sqlx::query_as(
                r#"
                SELECT id, season, episode, status FROM episodes
                WHERE tv_show_id = $1 AND season = $2
                ORDER BY episode
                "#,
            )
            .bind(show_id)
            .bind(year_season)
            .fetch_all(db.pool())
            .await?;

            // Scene episode N is the Nth episode in this season
            if let Some((ep_id, found_season, found_episode, status)) = episodes.get((scene_episode - 1) as usize) {
                if status == "missing" || status == "wanted" || status == "available" {
                    info!(
                        "Year-based absolute match: S{:02}E{:02} -> Season {} Episode {} (year season mapping)",
                        scene_season, scene_episode, found_season, found_episode
                    );
                    db.episodes()
                        .mark_available(*ep_id, torrent_link, Some(rss_item_id))
                        .await?;
                    return Ok(Some(*ep_id));
                } else {
                    debug!(
                        "Found matching episode but status is '{}', skipping",
                        status
                    );
                }
            }
        }
    }

    // Fallback: Calculate rough absolute episode number for traditional shows
    let estimated_absolute = if scene_season > 1 {
        let episodes_per_season = 22;
        ((scene_season - 1) * episodes_per_season) + scene_episode
    } else {
        scene_episode
    };

    // Try to find episodes by absolute_number field
    let rows: Vec<(Uuid, i32, i32, String)> = sqlx::query_as(
        r#"
        SELECT id, season, episode, status FROM episodes
        WHERE tv_show_id = $1
          AND (status = 'missing' OR status = 'wanted' OR status = 'available')
          AND absolute_number = $2
        LIMIT 1
        "#,
    )
    .bind(show_id)
    .bind(estimated_absolute)
    .fetch_all(db.pool())
    .await?;

    if let Some((ep_id, found_season, found_episode, status)) = rows.into_iter().next()
        && (status == "missing" || status == "wanted" || status == "available") {
            info!(
                "Absolute number match: S{:02}E{:02} -> Season {} Episode {} (abs: {})",
                scene_season, scene_episode, found_season, found_episode, estimated_absolute
            );
            db.episodes()
                .mark_available(ep_id, torrent_link, Some(rss_item_id))
                .await?;
            return Ok(Some(ep_id));
        }

    Ok(None)
}

/// Transform a torrent link from page URL to download URL for specific trackers
///
/// Some trackers (like IPTorrents) use page URLs in their RSS feeds instead of
/// direct download links. This function transforms those URLs into usable download links.
fn transform_torrent_link(item: &ParsedRssItem, feed_url: &str) -> String {
    let link = &item.link;
    
    // IPTorrents: Transform /t/{id} page URLs to /download.php/{id}/name.torrent?torrent_pass=...
    if link.contains("iptorrents.com/t/")
        && let Some(download_url) = transform_iptorrents_link(link, &item.title, feed_url) {
            debug!(
                "Transformed IPTorrents link: {} -> {}",
                link, download_url
            );
            return download_url;
        }
    
    // For other trackers or if transformation fails, return the original link
    link.clone()
}

/// Transform IPTorrents page URL to download URL
///
/// Input:  https://iptorrents.com/t/7109574
/// Output: https://iptorrents.com/download.php/7109574/Girl.Taken.S01E01.720p.HEVC.x265-MeGusta.torrent?torrent_pass=...
fn transform_iptorrents_link(page_url: &str, title: &str, feed_url: &str) -> Option<String> {
    // Extract torrent ID from page URL
    let id_regex = Regex::new(r"iptorrents\.com/t/(\d+)").ok()?;
    let torrent_id = id_regex.captures(page_url)?
        .get(1)?
        .as_str();
    
    // Extract torrent_pass from feed URL (it's in the tp= parameter)
    // Feed URL format: https://iptorrents.com/t.rss?u=...;tp=PASSKEY;...
    let pass_regex = Regex::new(r"[;&?]tp=([a-f0-9]+)").ok()?;
    let torrent_pass = pass_regex.captures(feed_url)?
        .get(1)?
        .as_str();
    
    // Create a safe filename from the title (replace spaces with dots, remove special chars)
    let safe_filename = title
        .replace(' ', ".")
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "");
    
    // Construct the download URL
    Some(format!(
        "https://iptorrents.com/download.php/{}/{}.torrent?torrent_pass={}",
        torrent_id,
        safe_filename,
        torrent_pass
    ))
}

/// Find TV shows with names matching the given normalized name
async fn find_matching_shows(db: &Database, normalized_name: &str) -> Result<Vec<Uuid>> {
    // Query for shows with matching names (case-insensitive, normalized)
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT id FROM tv_shows
        WHERE monitored = true
          AND (
            LOWER(REPLACE(REPLACE(REPLACE(name, '.', ' '), '-', ' '), '_', ' ')) 
            LIKE '%' || $1 || '%'
            OR LOWER(name) LIKE '%' || $1 || '%'
          )
        "#,
    )
    .bind(normalized_name.to_lowercase())
    .fetch_all(db.pool())
    .await?;

    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Normalize a show name for matching
fn normalize_show_name(name: &str) -> String {
    name.to_lowercase()
        .replace(['.', '-', '_'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_show_name() {
        assert_eq!(normalize_show_name("Chicago Fire"), "chicago fire");
        assert_eq!(normalize_show_name("Chicago.Fire"), "chicago fire");
        assert_eq!(normalize_show_name("The-Daily-Show"), "the daily show");
        assert_eq!(
            normalize_show_name("Power Book III Raising Kanan"),
            "power book iii raising kanan"
        );
    }
}
