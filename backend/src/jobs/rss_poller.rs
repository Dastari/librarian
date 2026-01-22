//! RSS/Torznab feed poller
//!
//! NOTE: This job is deprecated. The new media pipeline uses auto-hunt and
//! pending_file_matches for tracking downloads instead of storing torrent links
//! on episodes.
//!
//! This job:
//! 1. Gets feeds that are due for polling
//! 2. Fetches and parses each feed (with bounded concurrency)
//! 3. Stores new items in the database
//!
//! Episode matching is no longer performed here - use auto-hunt instead.

use anyhow::Result;
use chrono::Datelike;
use regex::Regex;
#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
use sqlx::SqlitePool;
use std::sync::Arc;

#[cfg(feature = "postgres")]
type DbPool = PgPool;
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
type DbPool = SqlitePool;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::libraries::LibraryRecord;
use crate::db::tv_shows::TvShowRecord;
use crate::db::{CreateRssFeedItem, Database, RssFeedRecord};
use crate::services::ParsedRssItem;
use crate::services::RssService;
use crate::services::text_utils::{normalize_quality, normalize_show_name};

/// Maximum concurrent feed fetches
const MAX_CONCURRENT_FEEDS: usize = 5;

/// Delay between feed fetch batches (ms)
const FEED_BATCH_DELAY_MS: u64 = 200;

/// Poll all RSS feeds that are due for polling
pub async fn poll_feeds() -> Result<()> {
    // RSS episode matching is deprecated - episodes now derive status from media_file_id
    // and downloads are tracked via pending_file_matches. Use auto-hunt instead.
    warn!("RSS feed polling with episode matching is deprecated. Use auto-hunt for automated downloads.");
    
    info!("Starting RSS feed poll job (fetch-only mode)");

    // Get database path/URL from environment
    #[cfg(feature = "sqlite")]
    let database_url = std::env::var("DATABASE_PATH")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "./data/librarian.db".to_string());

    #[cfg(feature = "postgres")]
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/librarian".to_string());

    let pool = DbPool::connect(&database_url).await?;
    let db = Database::new(pool);

    poll_feeds_with_db(&db).await
}

/// Poll feeds with a provided database connection
///
/// Uses bounded concurrency to prevent overwhelming external RSS servers.
pub async fn poll_feeds_with_db(db: &Database) -> Result<()> {
    let rss_service = Arc::new(RssService::new());

    // Get feeds that need polling
    let feeds = db.rss_feeds().list_due_for_poll().await?;

    if feeds.is_empty() {
        debug!(job = "rss_poller", "No feeds due for polling");
        return Ok(());
    }

    info!(
        job = "rss_poller",
        feed_count = feeds.len(),
        max_concurrent = MAX_CONCURRENT_FEEDS,
        "Found feeds due for polling"
    );

    // Semaphore to limit concurrent feed fetches
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_FEEDS));

    // Process feeds in chunks
    let chunk_size = MAX_CONCURRENT_FEEDS;

    for chunk in feeds.chunks(chunk_size) {
        let mut handles = Vec::with_capacity(chunk.len());

        for feed in chunk {
            let db = db.clone();
            let rss_service = rss_service.clone();
            let semaphore = semaphore.clone();
            let feed = feed.clone();

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.expect("Semaphore closed");

                if let Err(e) = poll_single_feed(&db, &rss_service, &feed).await {
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
            });

            handles.push(handle);
        }

        // Wait for all feeds in this chunk to complete
        for handle in handles {
            if let Err(e) = handle.await {
                error!(job = "rss_poller", error = %e, "Feed poll task panicked");
            }
        }

        // Small delay between chunks
        if FEED_BATCH_DELAY_MS > 0 {
            tokio::time::sleep(Duration::from_millis(FEED_BATCH_DELAY_MS)).await;
        }
    }

    debug!("RSS polling job completed");
    Ok(())
}

/// Poll a single RSS feed by ID (for manual polling via GraphQL)
pub async fn poll_single_feed_by_id(db: &Database, feed_id: Uuid) -> Result<(i32, i32)> {
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
        let exists = db.rss_feeds().item_exists(feed.id, &item.link_hash).await?;

        if exists {
            debug!("Skipping existing item: {}", item.title);
            continue;
        }

        // Transform the link if needed (e.g., IPTorrents page URL -> download URL)
        let download_link = transform_torrent_link(&item, &feed.url);

        // Build quality info struct for matching
        let quality_info = ParsedQualityInfo {
            resolution: item.parsed_resolution.clone(),
            codec: item.parsed_codec.clone(),
            audio: item.parsed_audio.clone(),
            hdr: item.parsed_hdr.clone(),
            source: item.parsed_source.clone(),
            release_group: extract_release_group(&item.title),
        };

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
                parsed_audio: item.parsed_audio,
                parsed_hdr: item.parsed_hdr,
            })
            .await?;

        new_items += 1;

        // Try to match this item to a wanted episode
        if let Some(ref show_name) = item.parsed_show_name {
            if let (Some(season), Some(episode)) = (item.parsed_season, item.parsed_episode) {
                if let Some(matched) = try_match_episode(
                    db,
                    show_name,
                    season,
                    episode,
                    &download_link,
                    rss_item.id,
                    &quality_info,
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

/// Effective quality settings for a show (merged from library + show overrides)
#[derive(Debug, Default)]
struct EffectiveQualitySettings {
    pub allowed_resolutions: Vec<String>,
    pub allowed_video_codecs: Vec<String>,
    pub allowed_audio_formats: Vec<String>,
    pub require_hdr: bool,
    pub allowed_hdr_types: Vec<String>,
    pub allowed_sources: Vec<String>,
    pub release_group_blacklist: Vec<String>,
    pub release_group_whitelist: Vec<String>,
}

impl EffectiveQualitySettings {
    /// Create effective settings by merging library defaults with show overrides
    fn from_library_and_show(library: &LibraryRecord, show: &TvShowRecord) -> Self {
        Self {
            // Use show override if set, otherwise use library default
            allowed_resolutions: show
                .allowed_resolutions_override
                .clone()
                .unwrap_or_else(|| library.allowed_resolutions.clone()),
            allowed_video_codecs: show
                .allowed_video_codecs_override
                .clone()
                .unwrap_or_else(|| library.allowed_video_codecs.clone()),
            allowed_audio_formats: show
                .allowed_audio_formats_override
                .clone()
                .unwrap_or_else(|| library.allowed_audio_formats.clone()),
            require_hdr: show.require_hdr_override.unwrap_or(library.require_hdr),
            allowed_hdr_types: show
                .allowed_hdr_types_override
                .clone()
                .unwrap_or_else(|| library.allowed_hdr_types.clone()),
            allowed_sources: show
                .allowed_sources_override
                .clone()
                .unwrap_or_else(|| library.allowed_sources.clone()),
            release_group_blacklist: show
                .release_group_blacklist_override
                .clone()
                .unwrap_or_else(|| library.release_group_blacklist.clone()),
            release_group_whitelist: show
                .release_group_whitelist_override
                .clone()
                .unwrap_or_else(|| library.release_group_whitelist.clone()),
        }
    }
}

/// Get the effective quality settings for a show (merged library + show overrides)
async fn get_effective_quality_settings(
    db: &Database,
    show_id: Uuid,
) -> Result<Option<EffectiveQualitySettings>> {
    // Get the show
    let show = match db.tv_shows().get_by_id(show_id).await? {
        Some(s) => s,
        None => return Ok(None),
    };

    // Get the library
    let library = match db.libraries().get_by_id(show.library_id).await? {
        Some(l) => l,
        None => return Ok(None),
    };

    Ok(Some(EffectiveQualitySettings::from_library_and_show(
        &library, &show,
    )))
}

/// Parsed quality info from an RSS item for matching
#[derive(Debug, Default)]
struct ParsedQualityInfo {
    pub resolution: Option<String>,
    pub codec: Option<String>,
    pub audio: Option<String>,
    pub hdr: Option<String>,
    pub source: Option<String>,
    pub release_group: Option<String>,
}

/// Check if an RSS item matches the quality settings
/// Returns true if the item passes all quality filters
fn matches_quality_settings(
    parsed: &ParsedQualityInfo,
    settings: &EffectiveQualitySettings,
) -> bool {
    // Check resolution (empty = any)
    if !settings.allowed_resolutions.is_empty() {
        let resolution = parsed.resolution.as_deref().unwrap_or("");
        let normalized_res = normalize_quality(resolution);

        let matches = settings.allowed_resolutions.iter().any(|allowed| {
            let allowed_norm = normalize_quality(allowed);
            normalized_res.contains(&allowed_norm)
                || allowed_norm.contains(&normalized_res)
                || (allowed_norm == "2160p" && (normalized_res == "4k" || normalized_res == "uhd"))
                || (normalized_res == "2160p" && (allowed_norm == "4k" || allowed_norm == "uhd"))
        });

        if !matches && !resolution.is_empty() {
            debug!(
                "Quality filter: resolution '{}' not in allowed list {:?}",
                resolution, settings.allowed_resolutions
            );
            return false;
        }
    }

    // Check video codec (empty = any)
    if !settings.allowed_video_codecs.is_empty() {
        let codec = parsed.codec.as_deref().unwrap_or("");
        let normalized_codec = normalize_quality(codec);

        let matches = settings.allowed_video_codecs.iter().any(|allowed| {
            let allowed_norm = normalize_quality(allowed);
            normalized_codec.contains(&allowed_norm)
                || allowed_norm.contains(&normalized_codec)
                || (normalized_codec == "hevc" && allowed_norm == "h265")
                || (normalized_codec == "h265" && allowed_norm == "hevc")
                || (normalized_codec == "h264" && (allowed_norm == "x264" || allowed_norm == "avc"))
        });

        if !matches && !codec.is_empty() {
            debug!(
                "Quality filter: codec '{}' not in allowed list {:?}",
                codec, settings.allowed_video_codecs
            );
            return false;
        }
    }

    // Check audio format (empty = any)
    if !settings.allowed_audio_formats.is_empty() {
        let audio = parsed.audio.as_deref().unwrap_or("");
        let normalized_audio = normalize_quality(audio);

        let matches = settings.allowed_audio_formats.iter().any(|allowed| {
            let allowed_norm = normalize_quality(allowed);
            normalized_audio.contains(&allowed_norm)
                || allowed_norm.contains(&normalized_audio)
                || (normalized_audio == "dd" && allowed_norm == "dd51")
                || (normalized_audio == "ddplus" && allowed_norm == "dd+")
        });

        if !matches && !audio.is_empty() {
            debug!(
                "Quality filter: audio '{}' not in allowed list {:?}",
                audio, settings.allowed_audio_formats
            );
            return false;
        }
    }

    // Check HDR requirement
    if settings.require_hdr {
        let hdr = parsed.hdr.as_deref().unwrap_or("");
        if hdr.is_empty() {
            debug!("Quality filter: HDR required but not present");
            return false;
        }

        // If specific HDR types are specified, check those
        if !settings.allowed_hdr_types.is_empty() {
            let normalized_hdr = normalize_quality(hdr);
            let matches = settings.allowed_hdr_types.iter().any(|allowed| {
                let allowed_norm = normalize_quality(allowed);
                normalized_hdr.contains(&allowed_norm) || allowed_norm.contains(&normalized_hdr)
            });

            if !matches {
                debug!(
                    "Quality filter: HDR type '{}' not in allowed list {:?}",
                    hdr, settings.allowed_hdr_types
                );
                return false;
            }
        }
    }

    // Check source (empty = any)
    if !settings.allowed_sources.is_empty() {
        let source = parsed.source.as_deref().unwrap_or("");
        let normalized_source = normalize_quality(source);

        let matches = settings.allowed_sources.iter().any(|allowed| {
            let allowed_norm = normalize_quality(allowed);
            normalized_source.contains(&allowed_norm) || allowed_norm.contains(&normalized_source)
        });

        if !matches && !source.is_empty() {
            debug!(
                "Quality filter: source '{}' not in allowed list {:?}",
                source, settings.allowed_sources
            );
            return false;
        }
    }

    // Check release group blacklist
    if !settings.release_group_blacklist.is_empty() {
        if let Some(group) = &parsed.release_group {
            let normalized_group = normalize_quality(group);
            if settings
                .release_group_blacklist
                .iter()
                .any(|blocked| normalize_quality(blocked) == normalized_group)
            {
                debug!("Quality filter: release group '{}' is blacklisted", group);
                return false;
            }
        }
    }

    // Check release group whitelist (if set, only allow listed groups)
    if !settings.release_group_whitelist.is_empty() {
        if let Some(group) = &parsed.release_group {
            let normalized_group = normalize_quality(group);
            if !settings
                .release_group_whitelist
                .iter()
                .any(|allowed| normalize_quality(allowed) == normalized_group)
            {
                debug!(
                    "Quality filter: release group '{}' not in whitelist {:?}",
                    group, settings.release_group_whitelist
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
    quality_info: &ParsedQualityInfo,
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
        // Check quality settings before matching
        if let Some(settings) = get_effective_quality_settings(db, *show_id).await? {
            if !matches_quality_settings(quality_info, &settings) {
                info!(
                    job = "rss_poller",
                    show_name = %show_name,
                    resolution = ?quality_info.resolution,
                    codec = ?quality_info.codec,
                    "Skipping: quality doesn't match settings"
                );
                continue;
            }
        }

        if let Some(ep_id) =
            try_exact_match(db, *show_id, season, episode, torrent_link, rss_item_id).await?
        {
            info!(
                "Strategy 1 (exact match) succeeded for S{:02}E{:02}",
                season, episode
            );
            return Ok(Some(ep_id));
        }
    }

    // Strategy 2: Try year-based season matching
    // Some shows (like talk shows, soaps) use the year as season number
    // e.g., S02E04 in scene naming might be Season 2026, Episode 4
    let current_year = chrono::Utc::now().year();
    let year_seasons = [current_year, current_year - 1, current_year - 2];

    for show_id in &shows {
        // Check quality settings before matching
        if let Some(settings) = get_effective_quality_settings(db, *show_id).await? {
            if !matches_quality_settings(quality_info, &settings) {
                continue; // Already logged in Strategy 1
            }
        }

        for &year_season in &year_seasons {
            if let Some(ep_id) = try_exact_match(
                db,
                *show_id,
                year_season,
                episode,
                torrent_link,
                rss_item_id,
            )
            .await?
            {
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
        // Check quality settings before matching
        if let Some(settings) = get_effective_quality_settings(db, *show_id).await? {
            if !matches_quality_settings(quality_info, &settings) {
                continue; // Already logged in Strategy 1
            }
        }

        if let Some(ep_id) =
            try_absolute_match(db, *show_id, season, episode, torrent_link, rss_item_id).await?
        {
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
///
/// NOTE: This function no longer stores torrent_link on episodes.
/// Episode status is now derived from media_file_id presence.
/// Returns the episode ID if found and wanted (no media file).
async fn try_exact_match(
    db: &Database,
    show_id: Uuid,
    season: i32,
    episode: i32,
    _torrent_link: &str,
    _rss_item_id: Uuid,
) -> Result<Option<Uuid>> {
    let episode_record = db
        .episodes()
        .get_by_show_season_episode(show_id, season, episode)
        .await?;

    if let Some(ep) = episode_record {
        // Status is now derived from media_file_id:
        // - No media_file_id = wanted/missing
        // - Has media_file_id = downloaded
        if ep.media_file_id.is_none() {
            // Episode is wanted (no media file linked)
            // Note: We no longer store torrent_link on episodes.
            // The download would need to be triggered via auto-hunt or manual action.
            debug!(
                episode_id = %ep.id,
                season = ep.season,
                episode = ep.episode,
                "Found wanted episode (no media file)"
            );
            return Ok(Some(ep.id));
        } else {
            debug!(
                episode_id = %ep.id,
                "Episode already has media file, skipping"
            );
        }
    }

    Ok(None)
}

/// Try to match using absolute episode number calculation
/// For shows where scene uses S02E04 format but metadata has different numbering
///
/// NOTE: This function no longer stores torrent_link on episodes.
/// Episode status is now derived from media_file_id presence.
async fn try_absolute_match(
    db: &Database,
    show_id: Uuid,
    scene_season: i32,
    scene_episode: i32,
    _torrent_link: &str,
    _rss_item_id: Uuid,
) -> Result<Option<Uuid>> {
    // For shows with year-based seasons (season > 2000) and absolute episode numbers,
    // we need to find the nth episode of that "scene season"

    // First, get all seasons for this show to understand the numbering
    let seasons: Vec<(i32,)> = sqlx::query_as(
        "SELECT DISTINCT season FROM episodes WHERE tv_show_id = $1 ORDER BY season",
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
            // Status is derived from media_file_id: NULL = wanted, NOT NULL = downloaded
            let episodes: Vec<(Uuid, i32, i32, Option<Uuid>)> = sqlx::query_as(
                r#"
                SELECT id, season, episode, media_file_id FROM episodes
                WHERE tv_show_id = $1 AND season = $2
                ORDER BY episode
                "#,
            )
            .bind(show_id)
            .bind(year_season)
            .fetch_all(db.pool())
            .await?;

            // Scene episode N is the Nth episode in this season
            if let Some((ep_id, found_season, found_episode, media_file_id)) =
                episodes.get((scene_episode - 1) as usize)
            {
                // Episode is wanted if no media file is linked
                if media_file_id.is_none() {
                    info!(
                        "Year-based absolute match: S{:02}E{:02} -> Season {} Episode {} (year season mapping)",
                        scene_season, scene_episode, found_season, found_episode
                    );
                    return Ok(Some(*ep_id));
                } else {
                    debug!(
                        "Found matching episode but already has media file, skipping"
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
    // Status is derived from media_file_id: NULL = wanted
    let rows: Vec<(Uuid, i32, i32)> = sqlx::query_as(
        r#"
        SELECT id, season, episode FROM episodes
        WHERE tv_show_id = $1
          AND media_file_id IS NULL
          AND absolute_number = $2
        LIMIT 1
        "#,
    )
    .bind(show_id)
    .bind(estimated_absolute)
    .fetch_all(db.pool())
    .await?;

    if let Some((ep_id, found_season, found_episode)) = rows.into_iter().next() {
        info!(
            "Absolute number match: S{:02}E{:02} -> Season {} Episode {} (abs: {})",
            scene_season, scene_episode, found_season, found_episode, estimated_absolute
        );
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
        && let Some(download_url) = transform_iptorrents_link(link, &item.title, feed_url)
    {
        debug!("Transformed IPTorrents link: {} -> {}", link, download_url);
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
    let torrent_id = id_regex.captures(page_url)?.get(1)?.as_str();

    // Extract torrent_pass from feed URL (it's in the tp= parameter)
    // Feed URL format: https://iptorrents.com/t.rss?u=...;tp=PASSKEY;...
    let pass_regex = Regex::new(r"[;&?]tp=([a-f0-9]+)").ok()?;
    let torrent_pass = pass_regex.captures(feed_url)?.get(1)?.as_str();

    // Create a safe filename from the title (replace spaces with dots, remove special chars)
    let safe_filename = title
        .replace(' ', ".")
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "");

    // Construct the download URL
    Some(format!(
        "https://iptorrents.com/download.php/{}/{}.torrent?torrent_pass={}",
        torrent_id, safe_filename, torrent_pass
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

/// Extract release group from a title (usually after the last dash)
fn extract_release_group(title: &str) -> Option<String> {
    let group_re = Regex::new(r"-([A-Za-z0-9]+)(?:\.[A-Za-z0-9]+)?$").ok()?;
    group_re
        .captures(title)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
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
