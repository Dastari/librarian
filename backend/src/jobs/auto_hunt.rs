//! Auto-hunt job for finding missing content across all library types
//!
//! This job:
//! 1. Finds libraries with auto_hunt enabled
//! 2. Searches indexers for missing content (movies, TV episodes, music, audiobooks)
//! 3. Downloads releases that match quality settings
//! 4. Uses bounded concurrency to avoid overwhelming indexers

use anyhow::Result;
use regex::Regex;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::db::libraries::LibraryRecord;
use crate::db::movies::MovieRecord;
use crate::db::tv_shows::TvShowRecord;
use crate::indexer::manager::IndexerManager;
use crate::indexer::{ReleaseInfo, TorznabQuery};
use crate::services::hunt::HuntService;
use crate::services::TorrentService;
use crate::services::text_utils::normalize_quality;
use crate::services::torrent::TorrentInfo;
use crate::services::torrent_file_matcher::TorrentFileMatcher;
use crate::services::torrent_metadata::{
    extract_audio_files, is_single_file_album, parse_torrent_files,
};
use crate::services::track_matcher::{TrackMatchResult, match_tracks};

/// Maximum concurrent searches to indexers
const MAX_CONCURRENT_SEARCHES: usize = 2;

/// Delay between search batches (ms) to avoid rate limiting
const SEARCH_BATCH_DELAY_MS: u64 = 2000;

/// Maximum items to hunt per run (to avoid overwhelming the system)
const MAX_HUNT_PER_RUN: usize = 20;

/// Torznab category IDs
mod categories {
    pub const MOVIES: i32 = 2000;
    pub const MOVIES_FOREIGN: i32 = 2010;
    pub const MOVIES_SD: i32 = 2030;
    pub const MOVIES_HD: i32 = 2040;
    pub const MOVIES_UHD: i32 = 2045;
    pub const MOVIES_BLURAY: i32 = 2050;
    pub const MOVIES_3D: i32 = 2060;

    pub const TV: i32 = 5000;
    pub const TV_FOREIGN: i32 = 5020;
    pub const TV_SD: i32 = 5030;
    pub const TV_HD: i32 = 5040;
    pub const TV_UHD: i32 = 5045;
    pub const TV_SPORT: i32 = 5060;
    pub const TV_ANIME: i32 = 5070;
    pub const TV_DOCUMENTARY: i32 = 5080;

    pub const AUDIO: i32 = 3000;
    pub const AUDIO_MP3: i32 = 3010;
    pub const AUDIO_VIDEO: i32 = 3020;
    pub const AUDIO_AUDIOBOOK: i32 = 3030;
    pub const AUDIO_LOSSLESS: i32 = 3040;

    pub const BOOKS: i32 = 7000;
    pub const BOOKS_MAGS: i32 = 7010;
    pub const BOOKS_EBOOK: i32 = 7020;
    pub const BOOKS_COMICS: i32 = 7030;
    pub const BOOKS_TECHNICAL: i32 = 7040;
    pub const BOOKS_FOREIGN: i32 = 7060;

    pub fn movies_all() -> Vec<i32> {
        vec![
            MOVIES,
            MOVIES_FOREIGN,
            MOVIES_SD,
            MOVIES_HD,
            MOVIES_UHD,
            MOVIES_BLURAY,
            MOVIES_3D,
        ]
    }

    pub fn tv_all() -> Vec<i32> {
        vec![
            TV,
            TV_FOREIGN,
            TV_SD,
            TV_HD,
            TV_UHD,
            TV_SPORT,
            TV_ANIME,
            TV_DOCUMENTARY,
        ]
    }

    pub fn music_all() -> Vec<i32> {
        vec![AUDIO, AUDIO_MP3, AUDIO_LOSSLESS]
    }

    pub fn audiobooks() -> Vec<i32> {
        vec![AUDIO_AUDIOBOOK]
    }

    pub fn ebooks() -> Vec<i32> {
        vec![BOOKS, BOOKS_EBOOK, BOOKS_FOREIGN]
    }
}

/// Download a release using the indexer's authentication
///
/// This function handles both magnet URIs (no auth needed) and torrent file URLs
/// (requires downloading via the indexer with proper cookies/headers).
async fn download_release(
    release: &ReleaseInfo,
    torrent_service: &Arc<TorrentService>,
    indexer_manager: &Arc<IndexerManager>,
    user_id: Option<Uuid>,
) -> Result<TorrentInfo> {
    // Prefer magnet URI if available (no authentication needed)
    if let Some(ref magnet) = release.magnet_uri {
        debug!(
            release_title = %release.title,
            "Downloading via magnet URI"
        );
        return torrent_service.add_magnet(magnet, user_id).await;
    }

    // For torrent file URLs, we need to download via the indexer to get proper auth
    if let Some(ref link) = release.link {
        if let Some(ref indexer_id) = release.indexer_id {
            debug!(
                release_title = %release.title,
                indexer_id = %indexer_id,
                "Downloading torrent file via indexer"
            );

            // Download the torrent file bytes using the indexer's authentication
            let torrent_bytes = indexer_manager.download_torrent(indexer_id, link).await?;

            // Add the torrent from bytes
            return torrent_service
                .add_torrent_bytes(&torrent_bytes, user_id)
                .await;
        } else {
            // No indexer_id - try direct download (might fail for private trackers)
            warn!(
                release_title = %release.title,
                "Release has no indexer_id, attempting direct download"
            );
            return torrent_service.add_torrent_url(link, user_id).await;
        }
    }

    Err(anyhow::anyhow!(
        "Release has no download link or magnet URI"
    ))
}

/// Create file-level matches after a torrent download starts
///
/// This replaces the legacy torrent-level linking (link_to_movie, link_to_episode, etc.)
/// with the new file-level matching system.
async fn create_file_matches_for_movie(
    db: &Database,
    torrent_info: &TorrentInfo,
    movie_id: Uuid,
) -> Result<()> {
    // Get the torrent database record
    let torrent_record = db
        .torrents()
        .get_by_info_hash(&torrent_info.info_hash)
        .await?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Torrent record not found for info_hash: {}",
                torrent_info.info_hash
            )
        })?;

    // Create file-level matches
    let matcher = TorrentFileMatcher::new(db.clone());
    matcher
        .create_matches_for_movie_torrent(torrent_record.id, &torrent_info.files, movie_id)
        .await?;

    debug!(
        job = "auto_hunt",
        torrent_id = %torrent_record.id,
        movie_id = %movie_id,
        files = torrent_info.files.len(),
        "Created file-level matches for movie"
    );

    Ok(())
}

/// Create file-level matches for an episode torrent
async fn create_file_matches_for_episode(
    db: &Database,
    torrent_info: &TorrentInfo,
    episode_id: Uuid,
) -> Result<()> {
    let torrent_record = db
        .torrents()
        .get_by_info_hash(&torrent_info.info_hash)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Torrent record not found"))?;

    let matcher = TorrentFileMatcher::new(db.clone());
    matcher
        .create_matches_for_episode_torrent(torrent_record.id, &torrent_info.files, episode_id)
        .await?;

    debug!(
        job = "auto_hunt",
        torrent_id = %torrent_record.id,
        episode_id = %episode_id,
        files = torrent_info.files.len(),
        "Created file-level matches for episode"
    );

    Ok(())
}

/// Create file-level matches for an album torrent
async fn create_file_matches_for_album(
    db: &Database,
    torrent_info: &TorrentInfo,
    album_id: Uuid,
) -> Result<()> {
    info!(
        job = "auto_hunt",
        info_hash = %torrent_info.info_hash,
        album_id = %album_id,
        file_count = torrent_info.files.len(),
        "Looking up torrent record to create file matches"
    );

    let torrent_record = match db.torrents().get_by_info_hash(&torrent_info.info_hash).await {
        Ok(Some(record)) => {
            info!(
                job = "auto_hunt",
                torrent_id = %record.id,
                torrent_name = %record.name,
                "Found torrent record in database"
            );
            record
        }
        Ok(None) => {
            warn!(
                job = "auto_hunt",
                info_hash = %torrent_info.info_hash,
                "Torrent record not found in database - was it saved with user_id?"
            );
            return Err(anyhow::anyhow!(
                "Torrent record not found for info_hash {}",
                torrent_info.info_hash
            ));
        }
        Err(e) => {
            warn!(
                job = "auto_hunt",
                info_hash = %torrent_info.info_hash,
                error = %e,
                "Database error looking up torrent record"
            );
            return Err(e);
        }
    };

    let matcher = TorrentFileMatcher::new(db.clone());
    match matcher
        .create_matches_for_album(torrent_record.id, &torrent_info.files, album_id)
        .await
    {
        Ok(matches) => {
            info!(
                job = "auto_hunt",
                torrent_id = %torrent_record.id,
                album_id = %album_id,
                files = torrent_info.files.len(),
                matches_created = matches.len(),
                "Successfully created file-level matches for album"
            );
            Ok(())
        }
        Err(e) => {
            warn!(
                job = "auto_hunt",
                torrent_id = %torrent_record.id,
                album_id = %album_id,
                error = %e,
                "Failed to create file matches via TorrentFileMatcher"
            );
            Err(e)
        }
    }
}

/// Effective quality settings for filtering releases
#[derive(Debug, Default, Clone)]
pub struct EffectiveQualitySettings {
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
    /// Create from library settings only
    pub fn from_library(library: &LibraryRecord) -> Self {
        Self {
            allowed_resolutions: library.allowed_resolutions.clone(),
            allowed_video_codecs: library.allowed_video_codecs.clone(),
            allowed_audio_formats: library.allowed_audio_formats.clone(),
            require_hdr: library.require_hdr,
            allowed_hdr_types: library.allowed_hdr_types.clone(),
            allowed_sources: library.allowed_sources.clone(),
            release_group_blacklist: library.release_group_blacklist.clone(),
            release_group_whitelist: library.release_group_whitelist.clone(),
        }
    }

    /// Merge library settings with movie overrides
    pub fn from_library_and_movie(library: &LibraryRecord, movie: &MovieRecord) -> Self {
        Self {
            allowed_resolutions: movie
                .allowed_resolutions_override
                .clone()
                .unwrap_or_else(|| library.allowed_resolutions.clone()),
            allowed_video_codecs: movie
                .allowed_video_codecs_override
                .clone()
                .unwrap_or_else(|| library.allowed_video_codecs.clone()),
            allowed_audio_formats: movie
                .allowed_audio_formats_override
                .clone()
                .unwrap_or_else(|| library.allowed_audio_formats.clone()),
            require_hdr: movie.require_hdr_override.unwrap_or(library.require_hdr),
            allowed_hdr_types: movie
                .allowed_hdr_types_override
                .clone()
                .unwrap_or_else(|| library.allowed_hdr_types.clone()),
            allowed_sources: movie
                .allowed_sources_override
                .clone()
                .unwrap_or_else(|| library.allowed_sources.clone()),
            release_group_blacklist: movie
                .release_group_blacklist_override
                .clone()
                .unwrap_or_else(|| library.release_group_blacklist.clone()),
            release_group_whitelist: movie
                .release_group_whitelist_override
                .clone()
                .unwrap_or_else(|| library.release_group_whitelist.clone()),
        }
    }

    /// Merge library settings with show overrides
    pub fn from_library_and_show(library: &LibraryRecord, show: &TvShowRecord) -> Self {
        Self {
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

/// Parsed quality info from a release title
#[derive(Debug, Default)]
struct ParsedQualityInfo {
    pub resolution: Option<String>,
    pub codec: Option<String>,
    pub audio: Option<String>,
    pub hdr: Option<String>,
    pub source: Option<String>,
    pub release_group: Option<String>,
}

impl ParsedQualityInfo {
    /// Parse quality info from a release title
    fn from_title(title: &str) -> Self {
        let title_lower = title.to_lowercase();

        // Resolution
        let resolution = if title_lower.contains("2160p")
            || title_lower.contains("4k")
            || title_lower.contains("uhd")
        {
            Some("2160p".to_string())
        } else if title_lower.contains("1080p") {
            Some("1080p".to_string())
        } else if title_lower.contains("720p") {
            Some("720p".to_string())
        } else if title_lower.contains("480p") || title_lower.contains("sd") {
            Some("480p".to_string())
        } else {
            None
        };

        // Codec
        let codec = if title_lower.contains("x265")
            || title_lower.contains("hevc")
            || title_lower.contains("h.265")
        {
            Some("x265".to_string())
        } else if title_lower.contains("x264")
            || title_lower.contains("h.264")
            || title_lower.contains("avc")
        {
            Some("x264".to_string())
        } else if title_lower.contains("av1") {
            Some("AV1".to_string())
        } else {
            None
        };

        // Audio
        let audio = if title_lower.contains("truehd") {
            Some("TrueHD".to_string())
        } else if title_lower.contains("dts-hd") || title_lower.contains("dts hd") {
            Some("DTS-HD".to_string())
        } else if title_lower.contains("atmos") {
            Some("Atmos".to_string())
        } else if title_lower.contains("dd+")
            || title_lower.contains("ddp")
            || title_lower.contains("eac3")
        {
            Some("DD+".to_string())
        } else if title_lower.contains("dd5.1") || title_lower.contains("ac3") {
            Some("DD5.1".to_string())
        } else if title_lower.contains("aac") {
            Some("AAC".to_string())
        } else if title_lower.contains("flac") {
            Some("FLAC".to_string())
        } else {
            None
        };

        // HDR
        let hdr = if title_lower.contains("dv")
            || title_lower.contains("dolby vision")
            || title_lower.contains("dolbyvision")
        {
            Some("DV".to_string())
        } else if title_lower.contains("hdr10+") {
            Some("HDR10+".to_string())
        } else if title_lower.contains("hdr10") || title_lower.contains("hdr") {
            Some("HDR10".to_string())
        } else {
            None
        };

        // Source
        let source = if title_lower.contains("bluray")
            || title_lower.contains("blu-ray")
            || title_lower.contains("bdrip")
        {
            Some("BluRay".to_string())
        } else if title_lower.contains("webdl")
            || title_lower.contains("web-dl")
            || title_lower.contains("web dl")
        {
            Some("WEB-DL".to_string())
        } else if title_lower.contains("webrip") || title_lower.contains("web-rip") {
            Some("WEBRip".to_string())
        } else if title_lower.contains("hdtv") {
            Some("HDTV".to_string())
        } else if title_lower.contains("dvdrip") || title_lower.contains("dvd") {
            Some("DVDRip".to_string())
        } else {
            None
        };

        // Release group (usually after last dash, before extension)
        let release_group = extract_release_group(title);

        Self {
            resolution,
            codec,
            audio,
            hdr,
            source,
            release_group,
        }
    }
}

/// Extract release group from a title
fn extract_release_group(title: &str) -> Option<String> {
    let group_re = Regex::new(r"-([A-Za-z0-9]+)(?:\.[A-Za-z0-9]+)?$").ok()?;
    group_re
        .captures(title)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

/// Check if a release matches the quality settings
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

    // Check HDR requirement
    if settings.require_hdr {
        let hdr = parsed.hdr.as_deref().unwrap_or("");
        if hdr.is_empty() {
            debug!("Quality filter: HDR required but not present");
            return false;
        }

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

/// Score a release for ranking (higher is better)
fn score_release(
    release: &ReleaseInfo,
    parsed: &ParsedQualityInfo,
    settings: &EffectiveQualitySettings,
) -> i32 {
    let mut score = 0;

    // Prefer releases with more seeders
    if let Some(seeders) = release.seeders {
        score += (seeders.min(100) as i32) * 2;
    }

    // Prefer freeleech
    if release.is_freeleech() {
        score += 50;
    }

    // Resolution preference (higher res = higher score, but only if allowed)
    if let Some(ref res) = parsed.resolution {
        match res.as_str() {
            "2160p" => score += 40,
            "1080p" => score += 30,
            "720p" => score += 20,
            "480p" => score += 10,
            _ => {}
        }
    }

    // Preferred resolution boost
    for (i, pref_res) in settings.allowed_resolutions.iter().enumerate() {
        if let Some(ref res) = parsed.resolution {
            if res.to_lowercase() == pref_res.to_lowercase() {
                score += ((settings.allowed_resolutions.len() - i) * 10) as i32;
                break;
            }
        }
    }

    // Codec preference
    if let Some(ref codec) = parsed.codec {
        let codec_lower = codec.to_lowercase();
        if codec_lower.contains("x265") || codec_lower.contains("hevc") {
            score += 15; // Prefer x265 for efficiency
        }
    }

    // HDR bonus
    if parsed.hdr.is_some() && settings.require_hdr {
        score += 25;
    }

    score
}

/// Select the best release from a list of matching releases
fn select_best_release<'a>(
    releases: &'a [ReleaseInfo],
    settings: &EffectiveQualitySettings,
) -> Option<&'a ReleaseInfo> {
    let mut scored: Vec<(&ReleaseInfo, i32)> = releases
        .iter()
        .filter_map(|r| {
            let parsed = ParsedQualityInfo::from_title(&r.title);
            if matches_quality_settings(&parsed, settings) {
                let score = score_release(r, &parsed, settings);
                Some((r, score))
            } else {
                None
            }
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.1.cmp(&a.1));

    scored.first().map(|(r, _)| *r)
}

/// Result of a hunt operation
#[derive(Debug, Default)]
pub struct HuntResult {
    pub searched: i32,
    pub matched: i32,
    pub downloaded: i32,
    pub skipped: i32,
    pub failed: i32,
}

/// Main auto-hunt job entry point
pub async fn run_auto_hunt(
    pool: PgPool,
    torrent_service: Arc<TorrentService>,
    indexer_manager: Arc<IndexerManager>,
) -> Result<()> {
    info!("Starting auto-hunt job");

    let db = Database::new(pool);

    // Get all libraries that need auto-hunt processing:
    // 1. Libraries with auto_hunt = true (library-level setting)
    // 2. Libraries that have at least one TV show with auto_hunt_override = true
    // Note: Movies don't support auto_hunt_override yet
    let libraries: Vec<LibraryRecord> = sqlx::query_as(
        r#"
        SELECT DISTINCT l.id, l.user_id, l.name, l.path, l.library_type, l.icon, l.color,
               l.auto_scan, l.scan_interval_minutes, l.watch_for_changes,
               l.post_download_action, l.organize_files, l.rename_style, l.naming_pattern,
               l.auto_add_discovered, l.auto_download, l.auto_hunt,
               l.scanning, l.last_scanned_at, l.created_at, l.updated_at,
               l.allowed_resolutions, l.allowed_video_codecs, l.allowed_audio_formats,
               l.require_hdr, l.allowed_hdr_types, l.allowed_sources,
               l.release_group_blacklist, l.release_group_whitelist,
               l.auto_download_subtitles, l.preferred_subtitle_languages
        FROM libraries l
        WHERE l.auto_hunt = true
           OR EXISTS (SELECT 1 FROM tv_shows s WHERE s.library_id = l.id AND s.auto_hunt_override = true AND s.monitored = true)
        "#,
    )
    .fetch_all(db.pool())
    .await?;

    if libraries.is_empty() {
        debug!(job = "auto_hunt", "No libraries need auto-hunt processing");
        return Ok(());
    }

    info!(
        job = "auto_hunt",
        library_count = libraries.len(),
        "Found libraries with auto-hunt enabled"
    );

    let mut total_result = HuntResult::default();

    for library in libraries {
        // Load indexers for this library's user
        if let Err(e) = indexer_manager.load_user_indexers(library.user_id).await {
            warn!(
                job = "auto_hunt",
                library_id = %library.id,
                user_id = %library.user_id,
                error = %e,
                "Failed to load indexers for user"
            );
            continue;
        }

        let result = match library.library_type.to_lowercase().as_str() {
            "movies" => hunt_movies(&db, &library, &torrent_service, &indexer_manager).await,
            "tv" => hunt_tv_episodes(&db, &library, &torrent_service, &indexer_manager).await,
            "music" => hunt_music(&db, &library, &torrent_service, &indexer_manager).await,
            "audiobooks" => {
                hunt_audiobooks(&db, &library, &torrent_service, &indexer_manager).await
            }
            _ => {
                debug!(
                    job = "auto_hunt",
                    library_type = %library.library_type,
                    "Unsupported library type for auto-hunt"
                );
                continue;
            }
        };

        match result {
            Ok(r) => {
                total_result.searched += r.searched;
                total_result.matched += r.matched;
                total_result.downloaded += r.downloaded;
                total_result.skipped += r.skipped;
                total_result.failed += r.failed;
            }
            Err(e) => {
                error!(
                    job = "auto_hunt",
                    library_id = %library.id,
                    library_name = %library.name,
                    error = %e,
                    "Failed to hunt for library"
                );
            }
        }
    }

    info!(
        job = "auto_hunt",
        searched = total_result.searched,
        matched = total_result.matched,
        downloaded = total_result.downloaded,
        skipped = total_result.skipped,
        failed = total_result.failed,
        "Auto-hunt job complete"
    );

    Ok(())
}

/// Hunt for missing movies (internal)
async fn hunt_movies(
    db: &Database,
    library: &LibraryRecord,
    torrent_service: &Arc<TorrentService>,
    indexer_manager: &Arc<IndexerManager>,
) -> Result<HuntResult> {
    hunt_movies_impl(db, library, torrent_service, indexer_manager).await
}

/// Public function to hunt for movies in a specific library
/// Called from GraphQL mutation for manual triggering
pub async fn hunt_movies_for_library(
    db: &Database,
    library: &LibraryRecord,
    torrent_service: &Arc<TorrentService>,
    indexer_manager: &Arc<IndexerManager>,
) -> Result<HuntResult> {
    hunt_movies_impl(db, library, torrent_service, indexer_manager).await
}

/// Hunt for a single specific movie immediately
/// Called when a new movie is added to trigger instant search
pub async fn hunt_single_movie(
    db: &Database,
    movie: &MovieRecord,
    library: &LibraryRecord,
    torrent_service: &Arc<TorrentService>,
    indexer_manager: &Arc<IndexerManager>,
) -> Result<HuntResult> {
    // Skip if movie already has a file
    if movie.has_file {
        debug!(
            job = "auto_hunt",
            movie_title = %movie.title,
            "Movie already has file, skipping hunt"
        );
        return Ok(HuntResult::default());
    }

    // Skip if not monitored
    if !movie.monitored {
        debug!(
            job = "auto_hunt",
            movie_title = %movie.title,
            "Movie not monitored, skipping hunt"
        );
        return Ok(HuntResult::default());
    }

    info!(
        job = "auto_hunt",
        movie_id = %movie.id,
        movie_title = %movie.title,
        movie_year = ?movie.year,
        "Hunting for specific movie"
    );

    let mut result = HuntResult::default();
    result.searched = 1;

    let quality_settings = EffectiveQualitySettings::from_library_and_movie(library, movie);

    // Build search query
    let search_term = if let Some(year) = movie.year {
        format!("{} {}", movie.title, year)
    } else {
        movie.title.clone()
    };

    let mut query = TorznabQuery::movie_search(&search_term);
    query.categories = categories::movies_all();
    query.year = movie.year;

    // Add IMDB ID if available for more accurate search
    if let Some(ref imdb_id) = movie.imdb_id {
        query.imdb_id = Some(imdb_id.clone());
    }

    // Add TMDB ID if available
    if let Some(tmdb_id) = movie.tmdb_id {
        query.tmdb_id = Some(tmdb_id);
    }

    info!(
        job = "auto_hunt",
        movie_title = %movie.title,
        movie_year = ?movie.year,
        imdb_id = ?movie.imdb_id,
        "Searching for movie"
    );

    // Search all indexers
    let search_results = indexer_manager.search_all(&query).await;

    let mut all_releases: Vec<ReleaseInfo> = Vec::new();
    for indexer_result in search_results {
        if let Some(ref error) = indexer_result.error {
            warn!(
                job = "auto_hunt",
                indexer_name = %indexer_result.indexer_name,
                error = %error,
                "Indexer search failed"
            );
            continue;
        }
        all_releases.extend(indexer_result.releases);
    }

    if all_releases.is_empty() {
        debug!(
            job = "auto_hunt",
            movie_title = %movie.title,
            "No releases found"
        );
        result.skipped = 1;
        return Ok(result);
    }

    info!(
        job = "auto_hunt",
        movie_title = %movie.title,
        release_count = all_releases.len(),
        "Found releases, selecting best match"
    );

    // Select best release based on quality settings
    if let Some(best) = select_best_release(&all_releases, &quality_settings) {
        result.matched = 1;

        info!(
            job = "auto_hunt",
            movie_title = %movie.title,
            release_title = %best.title,
            seeders = ?best.seeders,
            indexer = ?best.indexer_name,
            "Downloading best match via indexer"
        );

        // Download using the indexer's authentication
        match download_release(best, torrent_service, indexer_manager, Some(library.user_id)).await {
            Ok(torrent_info) => {
                info!(
                    job = "auto_hunt",
                    movie_title = %movie.title,
                    torrent_id = torrent_info.id,
                    torrent_name = %torrent_info.name,
                    library_organize_files = library.organize_files,
                    "Successfully started download"
                );

                // Create file-level matches for the movie
                if let Err(e) = create_file_matches_for_movie(db, &torrent_info, movie.id).await {
                    error!(
                        job = "auto_hunt",
                        movie_title = %movie.title,
                        error = %e,
                        "Failed to create file matches for movie"
                    );
                }

                result.downloaded = 1;
            }
            Err(e) => {
                error!(
                    job = "auto_hunt",
                    movie_title = %movie.title,
                    error = %e,
                    "Failed to start download"
                );
                result.failed = 1;
            }
        }
    } else {
        debug!(
            job = "auto_hunt",
            movie_title = %movie.title,
            "No releases matched quality settings"
        );
        result.skipped = 1;
    }

    Ok(result)
}

/// Hunt for missing movies implementation
async fn hunt_movies_impl(
    db: &Database,
    library: &LibraryRecord,
    torrent_service: &Arc<TorrentService>,
    indexer_manager: &Arc<IndexerManager>,
) -> Result<HuntResult> {
    info!(
        job = "auto_hunt",
        library_id = %library.id,
        library_name = %library.name,
        "Hunting for missing movies in '{}'",
        library.name
    );

    // First, check if there are any active downloads for movies in this library
    // Uses torrent_file_matches to find active downloads (file-level matching)
    let active_movie_downloads: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(DISTINCT tfm.movie_id) FROM torrent_file_matches tfm
           JOIN torrents t ON t.id = tfm.torrent_id
           JOIN movies m ON m.id = tfm.movie_id
           WHERE m.library_id = $1 
           AND tfm.movie_id IS NOT NULL 
           AND NOT tfm.processed
           AND t.state NOT IN ('removed', 'error')"#,
    )
    .bind(library.id)
    .fetch_one(db.pool())
    .await
    .unwrap_or(0);

    if active_movie_downloads > 0 {
        debug!(
            job = "auto_hunt",
            library_name = %library.name,
            active_downloads = active_movie_downloads,
            "Found active downloads for movies in this library"
        );
    }

    // Debug: Get all movies in this library to understand state
    let all_movies: Vec<(String, bool, bool)> =
        sqlx::query_as(r#"SELECT title, monitored, has_file FROM movies WHERE library_id = $1"#)
            .bind(library.id)
            .fetch_all(db.pool())
            .await
            .unwrap_or_default();

    for (title, monitored, has_file) in &all_movies {
        debug!(
            job = "auto_hunt",
            library_name = %library.name,
            movie_title = %title,
            monitored = monitored,
            has_file = has_file,
            "Movie state"
        );
    }

    // Get monitored movies without files
    let movies: Vec<MovieRecord> = sqlx::query_as(
        r#"
        SELECT id, library_id, user_id, title, sort_title, original_title, year,
               tmdb_id, imdb_id, overview, tagline, runtime, genres,
               production_countries, spoken_languages, director, cast_names,
               tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
               collection_id, collection_name, collection_poster_url,
               release_date, certification, status, monitored,
               allowed_resolutions_override, allowed_video_codecs_override,
               allowed_audio_formats_override, require_hdr_override,
               allowed_hdr_types_override, allowed_sources_override,
               release_group_blacklist_override, release_group_whitelist_override,
               has_file, size_bytes, path, created_at, updated_at, download_status
        FROM movies
        WHERE library_id = $1
          AND monitored = true
          AND (download_status IN ('missing', 'wanted', 'suboptimal') OR (download_status IS NULL AND has_file = false))
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(library.id)
    .bind(MAX_HUNT_PER_RUN as i32)
    .fetch_all(db.pool())
    .await?;

    if movies.is_empty() {
        info!(
            job = "auto_hunt",
            library_name = %library.name,
            total_movies = all_movies.len(),
            "No missing monitored movies found (all movies either have files or are not monitored)"
        );
        return Ok(HuntResult::default());
    }

    info!(
        job = "auto_hunt",
        library_name = %library.name,
        movie_count = movies.len(),
        "Found missing movies to hunt"
    );

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_SEARCHES));
    let mut result = HuntResult::default();

    for movie in movies {
        let _permit = semaphore.acquire().await?;
        result.searched += 1;

        let quality_settings = EffectiveQualitySettings::from_library_and_movie(library, &movie);

        // Build search query
        let search_term = if let Some(year) = movie.year {
            format!("{} {}", movie.title, year)
        } else {
            movie.title.clone()
        };

        let mut query = TorznabQuery::movie_search(&search_term);
        query.categories = categories::movies_all();
        query.year = movie.year;

        // Add IMDB ID if available for more accurate search
        if let Some(ref imdb_id) = movie.imdb_id {
            query.imdb_id = Some(imdb_id.clone());
        }

        // Add TMDB ID if available
        if let Some(tmdb_id) = movie.tmdb_id {
            query.tmdb_id = Some(tmdb_id);
        }

        info!(
            job = "auto_hunt",
            movie_title = %movie.title,
            movie_year = ?movie.year,
            imdb_id = ?movie.imdb_id,
            "Searching for movie"
        );

        // Search all indexers
        let search_results = indexer_manager.search_all(&query).await;

        let mut all_releases: Vec<ReleaseInfo> = Vec::new();
        for indexer_result in search_results {
            if let Some(ref error) = indexer_result.error {
                warn!(
                    job = "auto_hunt",
                    indexer_name = %indexer_result.indexer_name,
                    error = %error,
                    "Indexer search failed"
                );
                continue;
            }
            all_releases.extend(indexer_result.releases);
        }

        if all_releases.is_empty() {
            debug!(
                job = "auto_hunt",
                movie_title = %movie.title,
                "No releases found"
            );
            result.skipped += 1;
            continue;
        }

        info!(
            job = "auto_hunt",
            movie_title = %movie.title,
            release_count = all_releases.len(),
            "Found releases, selecting best match"
        );

        // Select best release based on quality settings
        if let Some(best) = select_best_release(&all_releases, &quality_settings) {
            result.matched += 1;

            info!(
                job = "auto_hunt",
                movie_title = %movie.title,
                release_title = %best.title,
                seeders = ?best.seeders,
                indexer = ?best.indexer_name,
                "Downloading best match via indexer"
            );

            // Download using the indexer's authentication
            match download_release(best, torrent_service, indexer_manager, Some(library.user_id)).await {
                Ok(torrent_info) => {
                    info!(
                        job = "auto_hunt",
                        movie_title = %movie.title,
                        torrent_id = torrent_info.id,
                        torrent_name = %torrent_info.name,
                        "Successfully started download"
                    );

                    // Create file-level matches for the movie
                    if let Err(e) = create_file_matches_for_movie(db, &torrent_info, movie.id).await
                    {
                        error!(
                            job = "auto_hunt",
                            movie_title = %movie.title,
                            error = %e,
                            "Failed to create file matches for movie"
                        );
                    }

                    result.downloaded += 1;
                }
                Err(e) => {
                    error!(
                        job = "auto_hunt",
                        movie_title = %movie.title,
                        error = %e,
                        "Failed to start download"
                    );
                    result.failed += 1;
                }
            }
        } else {
            debug!(
                job = "auto_hunt",
                movie_title = %movie.title,
                "No releases matched quality settings"
            );
            result.skipped += 1;
        }

        // Delay between searches
        tokio::time::sleep(Duration::from_millis(SEARCH_BATCH_DELAY_MS)).await;
    }

    Ok(result)
}

/// Hunt for missing TV episodes (internal)
async fn hunt_tv_episodes(
    db: &Database,
    library: &LibraryRecord,
    torrent_service: &Arc<TorrentService>,
    indexer_manager: &Arc<IndexerManager>,
) -> Result<HuntResult> {
    hunt_tv_episodes_impl(db, library, torrent_service, indexer_manager).await
}

/// Public function to hunt for TV episodes in a specific library
/// Called from GraphQL mutation for manual triggering
pub async fn hunt_tv_for_library(
    db: &Database,
    library: &LibraryRecord,
    torrent_service: &Arc<TorrentService>,
    indexer_manager: &Arc<IndexerManager>,
) -> Result<HuntResult> {
    hunt_tv_episodes_impl(db, library, torrent_service, indexer_manager).await
}

/// Hunt for missing TV episodes implementation
async fn hunt_tv_episodes_impl(
    db: &Database,
    library: &LibraryRecord,
    torrent_service: &Arc<TorrentService>,
    indexer_manager: &Arc<IndexerManager>,
) -> Result<HuntResult> {
    info!(
        job = "auto_hunt",
        library_id = %library.id,
        library_name = %library.name,
        "Hunting for missing TV episodes"
    );

    // Get monitored shows with auto_hunt enabled (via override or library default)
    // - If show has auto_hunt_override = true, always include it
    // - If show has auto_hunt_override = NULL, use library setting
    // - If show has auto_hunt_override = false, exclude it
    let shows: Vec<TvShowRecord> = sqlx::query_as(
        r#"
        SELECT id, library_id, user_id, name, sort_name, year, status,
               tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network, runtime,
               genres, poster_url, backdrop_url, monitored, monitor_type, path,
               auto_download_override, backfill_existing, organize_files_override,
               rename_style_override, auto_hunt_override, episode_count,
               episode_file_count, size_bytes, created_at, updated_at,
               allowed_resolutions_override, allowed_video_codecs_override,
               allowed_audio_formats_override, require_hdr_override,
               allowed_hdr_types_override, allowed_sources_override,
               release_group_blacklist_override, release_group_whitelist_override
        FROM tv_shows
        WHERE library_id = $1
          AND monitored = true
          AND (auto_hunt_override = true OR (auto_hunt_override IS NULL AND $2 = true))
        "#,
    )
    .bind(library.id)
    .bind(library.auto_hunt)
    .fetch_all(db.pool())
    .await?;

    let mut result = HuntResult::default();
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_SEARCHES));

    for show in shows {
        // Get missing/wanted/suboptimal episodes for this show
        let episodes: Vec<(Uuid, i32, i32, String)> = sqlx::query_as(
            r#"
            SELECT id, season, episode, status
            FROM episodes
            WHERE tv_show_id = $1
              AND status IN ('missing', 'wanted', 'suboptimal')
            ORDER BY season, episode
            LIMIT $2
            "#,
        )
        .bind(show.id)
        .bind(MAX_HUNT_PER_RUN as i32)
        .fetch_all(db.pool())
        .await?;

        if episodes.is_empty() {
            continue;
        }

        info!(
            job = "auto_hunt",
            show_name = %show.name,
            episode_count = episodes.len(),
            "Found {} missing/suboptimal episodes to hunt for '{}'",
            episodes.len(), show.name
        );

        let quality_settings = EffectiveQualitySettings::from_library_and_show(library, &show);

        for (episode_id, season, episode, _status) in episodes {
            let _permit = semaphore.acquire().await?;
            result.searched += 1;

            // Build search query
            let search_term = format!("{} S{:02}E{:02}", show.name, season, episode);
            let mut query = TorznabQuery::tv_search(&show.name);
            query.season = Some(season);
            query.episode = Some(format!("{:02}", episode));
            query.categories = categories::tv_all();

            // Add IDs if available
            if let Some(imdb_id) = &show.imdb_id {
                query.imdb_id = Some(imdb_id.clone());
            }
            if let Some(tvdb_id) = show.tvdb_id {
                query.tvdb_id = Some(tvdb_id);
            }
            if let Some(tmdb_id) = show.tmdb_id {
                query.tmdb_id = Some(tmdb_id);
            }

            info!(
                job = "auto_hunt",
                show_name = %show.name,
                search_term = %search_term,
                "Searching for episode '{}'",
                search_term
            );

            let search_results = indexer_manager.search_all(&query).await;

            let mut all_releases: Vec<ReleaseInfo> = Vec::new();
            for indexer_result in search_results {
                if indexer_result.error.is_none() {
                    all_releases.extend(indexer_result.releases);
                }
            }

            if all_releases.is_empty() {
                result.skipped += 1;
                continue;
            }

            if let Some(best) = select_best_release(&all_releases, &quality_settings) {
                result.matched += 1;

                // Use the download_release helper which handles indexer authentication
                let add_result = download_release(best, torrent_service, indexer_manager, Some(library.user_id)).await;

                match add_result {
                    Ok(torrent_info) => {
                        info!(
                            job = "auto_hunt",
                            show_name = %show.name,
                            season = season,
                            episode = episode,
                            torrent_name = %torrent_info.name,
                            "Successfully started episode download"
                        );

                        // Create file-level matches for the episode
                        if let Err(e) =
                            create_file_matches_for_episode(db, &torrent_info, episode_id).await
                        {
                            error!(job = "auto_hunt", error = %e, "Failed to create file matches for episode");
                        }

                        result.downloaded += 1;
                    }
                    Err(e) => {
                        error!(
                            job = "auto_hunt",
                            show_name = %show.name,
                            error = %e,
                            "Failed to start download"
                        );
                        result.failed += 1;
                    }
                }
            } else {
                result.skipped += 1;
            }

            tokio::time::sleep(Duration::from_millis(SEARCH_BATCH_DELAY_MS)).await;
        }
    }

    Ok(result)
}

/// Hunt for missing music albums
///
/// This function validates torrents against expected track listings before downloading.
/// For releases with .torrent files, it downloads and parses the torrent metadata to
/// ensure the files match the expected tracks from MusicBrainz.
async fn hunt_music(
    db: &Database,
    library: &LibraryRecord,
    torrent_service: &Arc<TorrentService>,
    indexer_manager: &Arc<IndexerManager>,
) -> Result<HuntResult> {
    info!(
        job = "auto_hunt",
        library_id = %library.id,
        library_name = %library.name,
        "Hunting for missing music"
    );

    // Get albums without files
    let albums = db
        .albums()
        .list_needing_files(library.id, MAX_HUNT_PER_RUN as i64)
        .await?;

    if albums.is_empty() {
        debug!(
            job = "auto_hunt",
            library_name = %library.name,
            "No missing albums found"
        );
        return Ok(HuntResult::default());
    }

    info!(
        job = "auto_hunt",
        library_name = %library.name,
        album_count = albums.len(),
        "Found missing albums to hunt"
    );

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_SEARCHES));
    let mut result = HuntResult::default();

    for album in albums {
        let _permit = semaphore.acquire().await?;
        result.searched += 1;

        // Get artist name
        let artist_name = db
            .albums()
            .get_artist_by_id(album.artist_id)
            .await?
            .map(|a| a.name)
            .unwrap_or_else(|| "Unknown Artist".to_string());

        // Get expected tracks for this album (for track matching)
        let expected_tracks = db.tracks().list_by_album(album.id).await?;
        let has_track_info = !expected_tracks.is_empty();

        // Search with just the album name to get more results
        // We'll score results by how well they match artist and year
        let search_term = album.name.clone();

        let mut query = TorznabQuery::music_search(&search_term);
        query.categories = categories::music_all();

        info!(
            job = "auto_hunt",
            album_name = %album.name,
            artist = %artist_name,
            album_year = ?album.year,
            expected_tracks = expected_tracks.len(),
            "Searching for album with query: '{}'",
            search_term
        );

        // Search all indexers
        let search_results = indexer_manager.search_all(&query).await;

        let mut all_releases: Vec<ReleaseInfo> = Vec::new();
        let mut indexer_count = 0;
        for indexer_result in search_results {
            indexer_count += 1;
            if let Some(ref error) = indexer_result.error {
                warn!(
                    job = "auto_hunt",
                    indexer_name = %indexer_result.indexer_name,
                    error = %error,
                    "Indexer search failed"
                );
                continue;
            }
            info!(
                job = "auto_hunt",
                indexer_name = %indexer_result.indexer_name,
                release_count = indexer_result.releases.len(),
                "Indexer '{}' returned {} results",
                indexer_result.indexer_name, indexer_result.releases.len()
            );
            all_releases.extend(indexer_result.releases);
        }

        if all_releases.is_empty() {
            info!(
                job = "auto_hunt",
                album_name = %album.name,
                artist = %artist_name,
                indexers_queried = indexer_count,
                "No releases found for album (searched: '{}')",
                search_term
            );
            result.skipped += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(SEARCH_BATCH_DELAY_MS)).await;
            continue;
        }

        // Score and sort releases by artist match, year match, and quality
        let scored_releases =
            score_music_releases_with_context(&all_releases, &artist_name, &album.name, album.year);

        // Separate releases with .torrent files (can validate) from magnet-only
        let (torrent_releases, magnet_releases): (Vec<_>, Vec<_>) = scored_releases
            .into_iter()
            .partition(|(r, _)| r.link.is_some() && r.indexer_id.is_some());

        info!(
            job = "auto_hunt",
            album_name = %album.name,
            torrent_count = torrent_releases.len(),
            magnet_count = magnet_releases.len(),
            "Separated releases by type"
        );

        let mut downloaded = false;

        // Try .torrent releases first (can validate track listing)
        if has_track_info {
            for (release, score) in &torrent_releases {
                // Download the .torrent file (metadata only, not the content)
                let indexer_id = release.indexer_id.as_ref().unwrap();
                let link = release.link.as_ref().unwrap();

                match indexer_manager.download_torrent(indexer_id, link).await {
                    Ok(torrent_bytes) => {
                        // Parse torrent to get file list
                        match parse_torrent_files(&torrent_bytes) {
                            Ok(files) => {
                                let audio_files = extract_audio_files(&files);

                                // Check if it's a single-file album (needs splitting)
                                if is_single_file_album(&files) {
                                    debug!(
                                        job = "auto_hunt",
                                        album_name = %album.name,
                                        release_title = %release.title,
                                        "Skipping single-file album (needs splitting)"
                                    );
                                    continue;
                                }

                                // Match tracks
                                let match_result = match_tracks(&expected_tracks, &files);

                                info!(
                                    job = "auto_hunt",
                                    album_name = %album.name,
                                    release_title = %release.title,
                                    matched = match_result.matched_count,
                                    expected = match_result.expected_count,
                                    match_pct = format!("{:.1}%", match_result.match_percentage * 100.0),
                                    audio_files = audio_files.len(),
                                    score = score,
                                    "Track matching result"
                                );

                                // Check if match meets threshold (80%)
                                if match_result.meets_threshold(TrackMatchResult::DEFAULT_THRESHOLD)
                                {
                                    // Good match! Add the torrent
                                    result.matched += 1;

                                    match torrent_service
                                        .add_torrent_bytes(&torrent_bytes, Some(library.user_id))
                                        .await
                                    {
                                        Ok(info) => {
                                            info!(
                                                job = "auto_hunt",
                                                album_name = %album.name,
                                                release_title = %release.title,
                                                info_hash = %info.info_hash,
                                                matched_tracks = match_result.matched_count,
                                                "Music download started (validated)"
                                            );

                                            // Create file-level matches for the album
                                            if let Err(e) =
                                                create_file_matches_for_album(db, &info, album.id)
                                                    .await
                                            {
                                                warn!(job = "auto_hunt", error = %e, "Failed to create file matches for album");
                                            }

                                            result.downloaded += 1;
                                            downloaded = true;
                                            break;
                                        }
                                        Err(e) => {
                                            error!(
                                                job = "auto_hunt",
                                                album_name = %album.name,
                                                error = %e,
                                                "Failed to add validated torrent"
                                            );
                                        }
                                    }
                                } else {
                                    debug!(
                                        job = "auto_hunt",
                                        album_name = %album.name,
                                        release_title = %release.title,
                                        match_pct = format!("{:.1}%", match_result.match_percentage * 100.0),
                                        unmatched = ?match_result.unmatched_tracks,
                                        "Release rejected - insufficient track match"
                                    );
                                }
                            }
                            Err(e) => {
                                warn!(
                                    job = "auto_hunt",
                                    album_name = %album.name,
                                    release_title = %release.title,
                                    error = %e,
                                    "Failed to parse torrent file"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            job = "auto_hunt",
                            album_name = %album.name,
                            release_title = %release.title,
                            error = %e,
                            "Failed to download torrent file for validation"
                        );
                    }
                }
            }
        }

        // Fall back to magnet releases (no validation possible) or if no track info
        if !downloaded {
            // Use the best magnet release, or best torrent if no magnets
            let fallback_release = magnet_releases
                .first()
                .or_else(|| torrent_releases.first())
                .map(|(r, _)| r);

            if let Some(release) = fallback_release {
                if !has_track_info {
                    info!(
                        job = "auto_hunt",
                        album_name = %album.name,
                        "No track info available, downloading without validation"
                    );
                } else {
                    info!(
                        job = "auto_hunt",
                        album_name = %album.name,
                        "No validated releases, falling back to unvalidated download"
                    );
                }

                result.matched += 1;

                match download_release(release, torrent_service, indexer_manager, Some(library.user_id)).await {
                    Ok(info) => {
                        info!(
                            job = "auto_hunt",
                            album_name = %album.name,
                            info_hash = %info.info_hash,
                            "Music download started (unvalidated fallback)"
                        );

                        // Create file-level matches for the album
                        if let Err(e) = create_file_matches_for_album(db, &info, album.id).await {
                            warn!(job = "auto_hunt", error = %e, "Failed to create file matches for album");
                        }

                        result.downloaded += 1;
                        downloaded = true;
                    }
                    Err(e) => {
                        error!(
                            job = "auto_hunt",
                            album_name = %album.name,
                            error = %e,
                            "Failed to start fallback download"
                        );
                        result.failed += 1;
                    }
                }
            }
        }

        if !downloaded {
            debug!(
                job = "auto_hunt",
                album_name = %album.name,
                releases_found = all_releases.len(),
                "No suitable releases found after validation"
            );
            result.skipped += 1;
        }

        // Small delay between searches
        tokio::time::sleep(tokio::time::Duration::from_millis(SEARCH_BATCH_DELAY_MS)).await;
    }

    Ok(result)
}

/// Score music releases and return sorted by score (highest first)
/// Score music releases based on artist/year match and quality
///
/// Scoring prioritizes:
/// 1. Artist name word matches (most important)
/// 2. Year match
/// 3. Quality indicators (FLAC, 24bit, etc.)
/// 4. Seeders and freeleech
///
/// TODO: Integrate library quality settings to boost scores for matching formats.
/// If the library has quality preferences configured (e.g., prefers FLAC over MP3,
/// or requires 24bit), we should:
/// - Add bonus points when release matches preferred quality (FLAC, 24bit, 320kbps, etc.)
/// - Add bonus points when release matches preferred codec (AAC, MP3, FLAC, ALAC, etc.)
/// - Potentially filter out releases that don't meet minimum quality requirements
/// - Consider bitrate preferences (lossless vs lossy, specific bitrates)
fn score_music_releases_with_context<'a>(
    releases: &'a [ReleaseInfo],
    artist_name: &str,
    album_name: &str,
    year: Option<i32>,
) -> Vec<(&'a ReleaseInfo, i32)> {
    // Extract words from artist name for matching (lowercase, filter short words)
    let artist_words: Vec<String> = artist_name
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 2)
        .map(|w| w.to_string())
        .collect();

    // Extract words from album name for matching
    let album_words: Vec<String> = album_name
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 2)
        .map(|w| w.to_string())
        .collect();

    let year_str = year.map(|y| y.to_string());

    let mut scored: Vec<_> = releases
        .iter()
        .filter(|r| r.seeders.unwrap_or(0) >= 1)
        .map(|r| {
            let mut score = 0i32;
            let title_lower = r.title.to_lowercase();

            // Count how many artist words appear in the release title
            let artist_matches = artist_words
                .iter()
                .filter(|word| title_lower.contains(word.as_str()))
                .count();

            // Artist match is very important (50 points per matching word)
            score += (artist_matches as i32) * 50;

            // Bonus if ALL artist words match
            if artist_matches == artist_words.len() && !artist_words.is_empty() {
                score += 100; // Full artist match bonus
            }

            // Count album name word matches
            let album_matches = album_words
                .iter()
                .filter(|word| title_lower.contains(word.as_str()))
                .count();

            // Album match (30 points per word, but album name was our search term so most should match)
            score += (album_matches as i32) * 30;

            // Year match (important for avoiding wrong versions)
            if let Some(ref y) = year_str {
                if title_lower.contains(y) {
                    score += 150 // Year match bonus
                }
            }

            // // Quality indicators (higher is better)
            // if title_lower.contains("24bit") || title_lower.contains("24-bit") {
            //     score += 40; // Hi-res
            // } else if title_lower.contains("flac") {
            //     score += 30;
            // } else if title_lower.contains("320") || title_lower.contains("v0") {
            //     score += 20; // High bitrate MP3
            // } else if title_lower.contains("mp3") {
            //     score += 10;
            // }

            // Prefer web releases over rips
            if title_lower.contains("web") {
                score += 5;
            }

            // Add seeders to score (capped at 30)
            let seeders = r.seeders.unwrap_or(0);
            score += seeders.min(30);

            // Prefer freeleech
            if r.download_volume_factor == 0.0 {
                score += 15;
            }

            (r, score)
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.1.cmp(&a.1));

    // Log the top results for debugging
    for (i, (release, score)) in scored.iter().take(3).enumerate() {
        tracing::debug!(
            rank = i + 1,
            score = score,
            title = %release.title,
            seeders = ?release.seeders,
            "Top album release candidate"
        );
    }

    scored
}

/// Legacy scoring function for backwards compatibility
fn score_music_releases(releases: &[ReleaseInfo]) -> Vec<(&ReleaseInfo, i32)> {
    let mut scored: Vec<_> = releases
        .iter()
        .filter(|r| r.seeders.unwrap_or(0) >= 1)
        .map(|r| {
            let mut score = 0i32;
            let title_lower = r.title.to_lowercase();

            // Quality indicators (higher is better)
            if title_lower.contains("24bit") || title_lower.contains("24-bit") {
                score += 120; // Hi-res
            } else if title_lower.contains("flac") {
                score += 100;
            } else if title_lower.contains("320") || title_lower.contains("v0") {
                score += 50; // High bitrate MP3
            } else if title_lower.contains("mp3") {
                score += 20;
            }

            // Prefer web releases over rips
            if title_lower.contains("web") {
                score += 10;
            }

            // Add seeders to score (capped at 50)
            let seeders = r.seeders.unwrap_or(0);
            score += seeders.min(50);

            // Prefer freeleech
            if r.download_volume_factor == 0.0 {
                score += 25;
            }

            (r, score)
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored
}

/// Hunt for a single specific album immediately
/// Called when a new album is added to trigger instant search
pub async fn hunt_single_album(
    db: &Database,
    album: &crate::db::AlbumRecord,
    library: &LibraryRecord,
    torrent_service: &Arc<TorrentService>,
    indexer_manager: &Arc<IndexerManager>,
) -> Result<HuntResult> {
    // Skip if album already has files
    if album.has_files {
        debug!(
            job = "auto_hunt",
            album_name = %album.name,
            "Album already has files, skipping hunt"
        );
        return Ok(HuntResult::default());
    }

    info!(
        job = "auto_hunt",
        album_id = %album.id,
        album_name = %album.name,
        "Hunting for specific album"
    );

    let mut result = HuntResult::default();
    result.searched = 1;

    // Get artist name
    let artist_name = db
        .albums()
        .get_artist_by_id(album.artist_id)
        .await?
        .map(|a| a.name)
        .unwrap_or_else(|| "Unknown Artist".to_string());

    // Get expected tracks for this album (for track matching)
    let expected_tracks = db.tracks().list_by_album(album.id).await?;
    let has_track_info = !expected_tracks.is_empty();

    // Search with just the album name to get more results
    // We'll score results by how well they match artist and year
    let search_term = album.name.clone();

    let mut query = TorznabQuery::music_search(&search_term);
    query.categories = categories::music_all();

    info!(
        job = "auto_hunt",
        album_name = %album.name,
        artist = %artist_name,
        album_year = ?album.year,
        expected_tracks = expected_tracks.len(),
        "Searching indexers for album with query: '{}'",
        search_term
    );

    // Search all indexers
    let search_results = indexer_manager.search_all(&query).await;

    let mut all_releases: Vec<ReleaseInfo> = Vec::new();
    for indexer_result in search_results {
        if let Some(ref error) = indexer_result.error {
            warn!(
                job = "auto_hunt",
                indexer_name = %indexer_result.indexer_name,
                error = %error,
                "Indexer search failed"
            );
            continue;
        }
        if indexer_result.releases.is_empty() {
            debug!(
                job = "auto_hunt",
                indexer_name = %indexer_result.indexer_name,
                "Indexer returned 0 results for album"
            );
        } else {
            info!(
                job = "auto_hunt",
                indexer_name = %indexer_result.indexer_name,
                release_count = indexer_result.releases.len(),
                "Indexer returned {} results for album",
                indexer_result.releases.len()
            );
        }
        all_releases.extend(indexer_result.releases);
    }

    if all_releases.is_empty() {
        info!(
            job = "auto_hunt",
            album_name = %album.name,
            artist = %artist_name,
            "No releases found for album across all indexers (searched: '{}')",
            search_term
        );
        result.skipped = 1;
        return Ok(result);
    }

    info!(
        job = "auto_hunt",
        album_name = %album.name,
        release_count = all_releases.len(),
        "Found {} releases for album, scoring by artist/year match",
        all_releases.len()
    );

    // Score and sort releases by artist match, year match, and quality
    let scored_releases =
        score_music_releases_with_context(&all_releases, &artist_name, &album.name, album.year);

    // Separate releases with .torrent files (can validate) from magnet-only
    let (torrent_releases, magnet_releases): (Vec<_>, Vec<_>) = scored_releases
        .into_iter()
        .partition(|(r, _)| r.link.is_some() && r.indexer_id.is_some());

    let mut downloaded = false;

    // Try .torrent releases first (can validate track listing)
    if has_track_info {
        for (release, score) in &torrent_releases {
            let indexer_id = release.indexer_id.as_ref().unwrap();
            let link = release.link.as_ref().unwrap();

            match indexer_manager.download_torrent(indexer_id, link).await {
                Ok(torrent_bytes) => {
                    match parse_torrent_files(&torrent_bytes) {
                        Ok(files) => {
                            let audio_files = extract_audio_files(&files);

                            // Check if it's a single-file album (needs splitting)
                            if is_single_file_album(&files) {
                                debug!(
                                    job = "auto_hunt",
                                    album_name = %album.name,
                                    release_title = %release.title,
                                    "Skipping single-file album (needs splitting)"
                                );
                                continue;
                            }

                            // Match tracks
                            let match_result = match_tracks(&expected_tracks, &files);

                            info!(
                                job = "auto_hunt",
                                album_name = %album.name,
                                release_title = %release.title,
                                matched = match_result.matched_count,
                                expected = match_result.expected_count,
                                match_pct = format!("{:.1}%", match_result.match_percentage * 100.0),
                                audio_files = audio_files.len(),
                                score = score,
                                "Track matching result for album"
                            );

                            // Check if match meets threshold (80%)
                            if match_result.meets_threshold(TrackMatchResult::DEFAULT_THRESHOLD) {
                                result.matched += 1;

                                match torrent_service
                                    .add_torrent_bytes(&torrent_bytes, Some(library.user_id))
                                    .await
                                {
                                    Ok(info) => {
                                        info!(
                                            job = "auto_hunt",
                                            album_name = %album.name,
                                            release_title = %release.title,
                                            info_hash = %info.info_hash,
                                            matched_tracks = match_result.matched_count,
                                            "Album download started (validated)"
                                        );

                                        // Create file-level matches for the album
                                        if let Err(e) =
                                            create_file_matches_for_album(db, &info, album.id).await
                                        {
                                            warn!(job = "auto_hunt", error = %e, "Failed to create file matches for album");
                                        }

                                        result.downloaded += 1;
                                        downloaded = true;
                                        break;
                                    }
                                    Err(e) => {
                                        error!(
                                            job = "auto_hunt",
                                            album_name = %album.name,
                                            error = %e,
                                            "Failed to add validated torrent for album"
                                        );
                                    }
                                }
                            } else {
                                debug!(
                                    job = "auto_hunt",
                                    album_name = %album.name,
                                    release_title = %release.title,
                                    match_pct = format!("{:.1}%", match_result.match_percentage * 100.0),
                                    "Release rejected - insufficient track match"
                                );
                            }
                        }
                        Err(e) => {
                            warn!(
                                job = "auto_hunt",
                                album_name = %album.name,
                                release_title = %release.title,
                                error = %e,
                                "Failed to parse torrent file for album"
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        job = "auto_hunt",
                        album_name = %album.name,
                        release_title = %release.title,
                        error = %e,
                        "Failed to download torrent file for validation"
                    );
                }
            }
        }
    }

    // Fall back to magnet releases (no validation possible) or if no track info
    if !downloaded {
        let fallback_release = magnet_releases
            .first()
            .or_else(|| torrent_releases.first())
            .map(|(r, _)| r);

        if let Some(release) = fallback_release {
            if !has_track_info {
                info!(
                    job = "auto_hunt",
                    album_name = %album.name,
                    "No track info available, downloading album without validation"
                );
            } else {
                info!(
                    job = "auto_hunt",
                    album_name = %album.name,
                    "No validated releases, falling back to unvalidated download for album"
                );
            }

            result.matched += 1;

            match download_release(release, torrent_service, indexer_manager, Some(library.user_id)).await {
                Ok(info) => {
                    info!(
                        job = "auto_hunt",
                        album_name = %album.name,
                        info_hash = %info.info_hash,
                        "Album download started (unvalidated fallback)"
                    );

                    // Create file-level matches for the album
                    if let Err(e) = create_file_matches_for_album(db, &info, album.id).await {
                        warn!(job = "auto_hunt", error = %e, "Failed to create file matches for album");
                    }

                    result.downloaded += 1;
                }
                Err(e) => {
                    error!(
                        job = "auto_hunt",
                        album_name = %album.name,
                        error = %e,
                        "Failed to start fallback download for album"
                    );
                    result.failed += 1;
                }
            }
        } else {
            result.skipped += 1;
        }
    }

    Ok(result)
}

/// Hunt for missing audiobooks
async fn hunt_audiobooks(
    db: &Database,
    library: &LibraryRecord,
    torrent_service: &Arc<TorrentService>,
    indexer_manager: &Arc<IndexerManager>,
) -> Result<HuntResult> {
    hunt_audiobooks_impl(db, library, torrent_service, indexer_manager).await
}

/// Public function to hunt for audiobooks in a specific library
/// Called from GraphQL mutation for manual triggering
pub async fn hunt_audiobooks_for_library(
    db: &Database,
    library: &LibraryRecord,
    torrent_service: &Arc<TorrentService>,
    indexer_manager: &Arc<IndexerManager>,
) -> Result<HuntResult> {
    hunt_audiobooks_impl(db, library, torrent_service, indexer_manager).await
}

/// Hunt for a single specific audiobook immediately
/// Called when a new audiobook is added to trigger instant search
pub async fn hunt_single_audiobook(
    db: &Database,
    audiobook: &crate::db::AudiobookRecord,
    library: &LibraryRecord,
    torrent_service: &Arc<TorrentService>,
    indexer_manager: &Arc<IndexerManager>,
) -> Result<HuntResult> {
    // Skip if audiobook already has files
    if audiobook.has_files {
        debug!(
            job = "auto_hunt",
            audiobook_title = %audiobook.title,
            "Audiobook already has files, skipping hunt"
        );
        return Ok(HuntResult::default());
    }

    info!(
        job = "auto_hunt",
        audiobook_id = %audiobook.id,
        audiobook_title = %audiobook.title,
        "Hunting for specific audiobook"
    );

    let mut result = HuntResult::default();
    result.searched = 1;

    // Get author name for search
    let author_name = if let Some(author_id) = audiobook.author_id {
        db.audiobooks()
            .get_author_by_id(author_id)
            .await?
            .map(|a| a.name)
            .unwrap_or_else(|| "Unknown Author".to_string())
    } else {
        "Unknown Author".to_string()
    };

    // Build search query - author + title
    let search_term = format!("{} {}", author_name, audiobook.title);

    let mut query = TorznabQuery::search(&search_term);
    query.categories = categories::audiobooks();

    info!(
        job = "auto_hunt",
        audiobook_title = %audiobook.title,
        author = %author_name,
        search_term = %search_term,
        "Searching indexers for audiobook"
    );

    // Search all enabled indexers
    let search_results = indexer_manager.search_all(&query).await;

    let mut all_releases: Vec<ReleaseInfo> = Vec::new();
    for indexer_result in search_results {
        if let Some(ref error) = indexer_result.error {
            warn!(
                job = "auto_hunt",
                indexer_name = %indexer_result.indexer_name,
                error = %error,
                "Indexer search failed"
            );
        }
        all_releases.extend(indexer_result.releases);
    }

    if all_releases.is_empty() {
        debug!(
            job = "auto_hunt",
            audiobook_title = %audiobook.title,
            "No releases found for audiobook"
        );
        return Ok(result);
    }

    info!(
        job = "auto_hunt",
        audiobook_title = %audiobook.title,
        release_count = all_releases.len(),
        "Found releases for audiobook"
    );

    // Score and sort releases
    let scored_releases = score_audiobook_releases(&all_releases, &audiobook.title, &author_name);

    // Try to download the best release
    for (release, score) in scored_releases {
        if score < 10 {
            debug!(
                job = "auto_hunt",
                audiobook_title = %audiobook.title,
                release_title = %release.title,
                score = score,
                "Release score too low, skipping"
            );
            continue;
        }

        info!(
            job = "auto_hunt",
            audiobook_title = %audiobook.title,
            release_title = %release.title,
            release_size = ?release.size,
            score = score,
            "Attempting to download audiobook release"
        );

        // Download the release
        match download_release(release, torrent_service, indexer_manager, Some(library.user_id)).await {
            Ok(_) => {
                info!(
                    job = "auto_hunt",
                    audiobook_title = %audiobook.title,
                    release_title = %release.title,
                    "Successfully started download for audiobook"
                );
                result.downloaded += 1;
                break;
            }
            Err(e) => {
                warn!(
                    job = "auto_hunt",
                    audiobook_title = %audiobook.title,
                    release_title = %release.title,
                    error = %e,
                    "Failed to download audiobook release, trying next"
                );
                continue;
            }
        }
    }

    Ok(result)
}

/// Hunt for missing audiobooks implementation
async fn hunt_audiobooks_impl(
    db: &Database,
    library: &LibraryRecord,
    torrent_service: &Arc<TorrentService>,
    indexer_manager: &Arc<IndexerManager>,
) -> Result<HuntResult> {
    info!(
        job = "auto_hunt",
        library_id = %library.id,
        library_name = %library.name,
        "Hunting for missing audiobooks"
    );

    // Get audiobooks without files
    let audiobooks = db
        .audiobooks()
        .list_needing_files(library.id, MAX_HUNT_PER_RUN as i64)
        .await?;

    if audiobooks.is_empty() {
        debug!(
            job = "auto_hunt",
            library_name = %library.name,
            "No missing audiobooks found"
        );
        return Ok(HuntResult::default());
    }

    info!(
        job = "auto_hunt",
        library_name = %library.name,
        audiobook_count = audiobooks.len(),
        "Found missing audiobooks to hunt"
    );

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_SEARCHES));
    let mut result = HuntResult::default();

    for audiobook in audiobooks {
        let _permit = semaphore.acquire().await?;
        result.searched += 1;

        // Get author name
        let author_name = if let Some(author_id) = audiobook.author_id {
            db.audiobooks()
                .get_author_by_id(author_id)
                .await?
                .map(|a| a.name)
                .unwrap_or_else(|| "Unknown Author".to_string())
        } else {
            "Unknown Author".to_string()
        };

        // Build search query - author + title
        let search_term = format!("{} {}", author_name, audiobook.title);

        let mut query = TorznabQuery::search(&search_term);
        query.categories = categories::audiobooks();

        debug!(
            job = "auto_hunt",
            audiobook_title = %audiobook.title,
            author = %author_name,
            search_term = %search_term,
            "Searching for audiobook"
        );

        // Search all enabled indexers
        let search_results = indexer_manager.search_all(&query).await;

        let mut all_releases: Vec<ReleaseInfo> = Vec::new();
        for indexer_result in search_results {
            if indexer_result.error.is_none() {
                all_releases.extend(indexer_result.releases);
            }
        }

        if all_releases.is_empty() {
            debug!(
                job = "auto_hunt",
                audiobook_title = %audiobook.title,
                "No releases found"
            );
            result.skipped += 1;
            continue;
        }

        debug!(
            job = "auto_hunt",
            audiobook_title = %audiobook.title,
            release_count = all_releases.len(),
            "Found releases for audiobook"
        );

        // Score and sort releases
        let scored_releases = score_audiobook_releases(&all_releases, &audiobook.title, &author_name);

        // Try to download the best release
        for (release, score) in scored_releases {
            if score < 10 {
                continue;
            }

            info!(
                job = "auto_hunt",
                audiobook_title = %audiobook.title,
                release_title = %release.title,
                score = score,
                "Attempting to download audiobook"
            );

            match download_release(release, torrent_service, indexer_manager, Some(library.user_id)).await {
                Ok(_) => {
                    info!(
                        job = "auto_hunt",
                        audiobook_title = %audiobook.title,
                        release_title = %release.title,
                        "Successfully started download for audiobook"
                    );
                    result.downloaded += 1;
                    break;
                }
                Err(e) => {
                    warn!(
                        job = "auto_hunt",
                        audiobook_title = %audiobook.title,
                        error = %e,
                        "Failed to download, trying next release"
                    );
                    result.failed += 1;
                    continue;
                }
            }
        }
    }

    info!(
        job = "auto_hunt",
        library_name = %library.name,
        searched = result.searched,
        downloaded = result.downloaded,
        "Audiobook hunt complete"
    );

    Ok(result)
}

/// Score audiobook releases based on title/author match and quality indicators
fn score_audiobook_releases<'a>(
    releases: &'a [ReleaseInfo],
    title: &str,
    author: &str,
) -> Vec<(&'a ReleaseInfo, i32)> {
    let title_lower = title.to_lowercase();
    let author_lower = author.to_lowercase();

    let mut scored: Vec<(&ReleaseInfo, i32)> = releases
        .iter()
        .map(|r| {
            let mut score = 0i32;
            let release_lower = r.title.to_lowercase();

            // Title match
            if release_lower.contains(&title_lower) {
                score += 50;
            } else {
                // Partial title match - check word overlap
                let title_words: std::collections::HashSet<_> = title_lower
                    .split_whitespace()
                    .filter(|w| w.len() > 2)
                    .collect();
                let release_words: std::collections::HashSet<_> = release_lower
                    .split_whitespace()
                    .filter(|w| w.len() > 2)
                    .collect();
                let overlap = title_words.intersection(&release_words).count();
                score += (overlap * 10) as i32;
            }

            // Author match
            if release_lower.contains(&author_lower) {
                score += 30;
            } else {
                // Check author last name
                if let Some(last_name) = author_lower.split_whitespace().last() {
                    if release_lower.contains(last_name) {
                        score += 15;
                    }
                }
            }

            // Prefer M4B format (common for audiobooks)
            if release_lower.contains("m4b") {
                score += 10;
            }

            // Prefer unabridged
            if release_lower.contains("unabridged") {
                score += 15;
            }

            // Penalize abridged
            if release_lower.contains("abridged") && !release_lower.contains("unabridged") {
                score -= 20;
            }

            // Prefer larger files (likely higher quality)
            if let Some(size) = r.size {
                if size > 500_000_000 {
                    // > 500MB
                    score += 10;
                }
                if size > 1_000_000_000 {
                    // > 1GB
                    score += 5;
                }
            }

            // Bonus for seeders
            if let Some(seeders) = r.seeders {
                if seeders > 5 {
                    score += 5;
                }
                if seeders > 20 {
                    score += 5;
                }
            }

            (r, score)
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored
}

/// Run auto-hunt for a single library
///
/// This is called after library scans complete or can be triggered manually.
/// It respects the library's auto_hunt setting.
pub async fn run_auto_hunt_for_library(
    pool: PgPool,
    library_id: Uuid,
    torrent_service: Arc<TorrentService>,
    indexer_manager: Arc<IndexerManager>,
) -> Result<HuntResult> {
    let db = Database::new(pool);

    // Get the library
    let library: Option<LibraryRecord> = sqlx::query_as(
        r#"
        SELECT id, user_id, name, path, library_type, icon, color,
               auto_scan, scan_interval_minutes, watch_for_changes,
               post_download_action, organize_files, rename_style, naming_pattern,
               auto_add_discovered, auto_download, auto_hunt,
               scanning, last_scanned_at, created_at, updated_at,
               allowed_resolutions, allowed_video_codecs, allowed_audio_formats,
               require_hdr, allowed_hdr_types, allowed_sources,
               release_group_blacklist, release_group_whitelist,
               auto_download_subtitles, preferred_subtitle_languages
        FROM libraries
        WHERE id = $1
        "#,
    )
    .bind(library_id)
    .fetch_optional(db.pool())
    .await?;

    let Some(library) = library else {
        warn!(
            job = "auto_hunt",
            library_id = %library_id,
            "Library not found"
        );
        return Ok(HuntResult::default());
    };

    // Check if auto_hunt should run (library-level or show-level overrides)
    let has_show_overrides = if library.library_type.to_lowercase() == "tv" {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM tv_shows WHERE library_id = $1 AND auto_hunt_override = true AND monitored = true)"
        )
        .bind(library_id)
        .fetch_one(db.pool())
        .await
        .unwrap_or(false)
    } else {
        false
    };

    if !library.auto_hunt && !has_show_overrides {
        debug!(
            job = "auto_hunt",
            library_id = %library_id,
            library_name = %library.name,
            "Library does not have auto_hunt enabled and no show overrides, skipping"
        );
        return Ok(HuntResult::default());
    }

    info!(
        job = "auto_hunt",
        library_id = %library_id,
        library_name = %library.name,
        library_type = %library.library_type,
        has_show_overrides = has_show_overrides,
        "Running auto-hunt for library after scan"
    );

    // Load indexers for this library's user
    if let Err(e) = indexer_manager.load_user_indexers(library.user_id).await {
        warn!(
            job = "auto_hunt",
            library_id = %library_id,
            user_id = %library.user_id,
            error = %e,
            "Failed to load indexers for user"
        );
        return Ok(HuntResult::default());
    }

    // Run hunt based on library type (case-insensitive)
    let result = match library.library_type.to_lowercase().as_str() {
        "movies" => hunt_movies(&db, &library, &torrent_service, &indexer_manager).await,
        "tv" => hunt_tv_episodes(&db, &library, &torrent_service, &indexer_manager).await,
        "music" => hunt_music(&db, &library, &torrent_service, &indexer_manager).await,
        "audiobooks" => hunt_audiobooks(&db, &library, &torrent_service, &indexer_manager).await,
        _ => {
            debug!(
                job = "auto_hunt",
                library_type = %library.library_type,
                "Unsupported library type for auto-hunt"
            );
            Ok(HuntResult::default())
        }
    };

    match &result {
        Ok(r) => {
            info!(
                job = "auto_hunt",
                library_id = %library_id,
                library_name = %library.name,
                searched = r.searched,
                matched = r.matched,
                downloaded = r.downloaded,
                skipped = r.skipped,
                failed = r.failed,
                "Auto-hunt complete for '{}': {} searched, {} matched, {} downloaded",
                library.name, r.searched, r.matched, r.downloaded
            );
        }
        Err(e) => {
            error!(
                job = "auto_hunt",
                library_id = %library_id,
                library_name = %library.name,
                error = %e,
                "Auto-hunt for library failed"
            );
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_quality_info() {
        let parsed = ParsedQualityInfo::from_title(
            "Movie.Name.2024.2160p.UHD.BluRay.x265.HDR.DTS-HD.MA-GROUP",
        );
        assert_eq!(parsed.resolution, Some("2160p".to_string()));
        assert_eq!(parsed.codec, Some("x265".to_string()));
        assert!(parsed.hdr.is_some());
        assert_eq!(parsed.source, Some("BluRay".to_string()));
        assert_eq!(parsed.release_group, Some("GROUP".to_string()));
    }

    #[test]
    fn test_parse_quality_1080p_web() {
        let parsed = ParsedQualityInfo::from_title("Show.S01E01.1080p.WEB-DL.x264.DD5.1-TEAM");
        assert_eq!(parsed.resolution, Some("1080p".to_string()));
        assert_eq!(parsed.codec, Some("x264".to_string()));
        assert_eq!(parsed.source, Some("WEB-DL".to_string()));
        assert_eq!(parsed.audio, Some("DD5.1".to_string()));
    }

    #[test]
    fn test_quality_matching() {
        let parsed = ParsedQualityInfo::from_title("Movie.2024.1080p.BluRay.x264-GROUP");

        // Empty settings = match anything
        let settings = EffectiveQualitySettings::default();
        assert!(matches_quality_settings(&parsed, &settings));

        // Specific resolution required
        let settings = EffectiveQualitySettings {
            allowed_resolutions: vec!["1080p".to_string()],
            ..Default::default()
        };
        assert!(matches_quality_settings(&parsed, &settings));

        // Wrong resolution
        let settings = EffectiveQualitySettings {
            allowed_resolutions: vec!["2160p".to_string()],
            ..Default::default()
        };
        assert!(!matches_quality_settings(&parsed, &settings));
    }
}
