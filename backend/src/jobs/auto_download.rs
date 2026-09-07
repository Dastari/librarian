//! Auto-download candidate discovery and grab (the "hunt").
//!
//! For each of the four media types, finds items that should be hunted for,
//! builds one or more `SourceQuery` variants, searches all configured sources,
//! filters candidates against the item's resolved `QualityProfile`, ranks the
//! survivors with `services::quality::profile::filter_and_rank`, and grabs the
//! best release via `TorrentService::add_magnet_with_metadata`, stamping the
//! wanted-target linkage columns on the created `Torrent` row.
//!
//! What is hunted is decided by `auto_download_mode` on the parent
//! Show/Album/Audiobook (movies use `monitored` + `wanted`):
//!
//! - `NONE`  — never hunt.
//! - `WANTED`— hunt only children already flagged `wanted`.
//! - `ALL`   — every child that is missing, not `ignored` and already
//!   aired/released is treated as wanted; the job flips `wanted = true` on
//!   those rows so the UI status matches reality (a future episode becomes
//!   wanted by itself once its air date passes).
//!
//! Three behaviours keep the hunt from hammering indexers or re-grabbing junk:
//!
//! - **Season packs**: when a season is mostly missing and the profile allows
//!   packs, the pack is searched first ("Show S02" and "Show Season 2"); only
//!   if no pack is grabbed does the job fall back to per-episode searches.
//! - **Search backoff**: a target is not re-searched more often than every
//!   [`SEARCH_BACKOFF_HOURS`] hours. The timestamps live in the
//!   `auto_download.search_state` app setting so a restart does not reset
//!   them. Manual searches (`searchMissing`) ignore the backoff.
//! - **Release blocklist**: releases recorded in `ReleaseBlocklist` (added by
//!   the retry sweep for failed/stalled grabs) are never grabbed again.
//!
//! See `docs/tier1-features-plan.md` §1/§2 for the original design and
//! `docs/design.md`'s Media Pipeline Decision Guide (Q1, Q23, Q25, Q37, Q39,
//! Q50, Q52).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use graphql_orm::graphql::filters::{BoolFilter, IntFilter, StringFilter};
use tracing::{info, warn};

use crate::db::Database;
use crate::graphql::entities::{
    Album, AlbumWhereInput, AppSetting, AppSettingWhereInput, Artist, ArtistWhereInput, Audiobook,
    AudiobookWhereInput, AutoDownloadMode, Chapter, ChapterWhereInput, CreateAppSettingInput,
    Episode, EpisodeWhereInput, Movie, MovieWhereInput, QualityProfile, ReleaseBlocklist,
    ReleaseBlocklistWhereInput, Show, ShowWhereInput, Torrent, TorrentGrabDetails,
    TorrentWhereInput, Track, TrackWhereInput, UpdateAppSettingInput, UpdateChapterInput,
    UpdateEpisodeInput, UpdateTrackInput, stamp_torrent_grab_details,
};
use crate::services::ServicesManager;
use crate::services::quality::profile;
use crate::services::sources::{QueryType, SourceQuery, SourceRelease};
use crate::services::torrent::{TorrentAddMetadata, get_default_user_id};

/// App setting holding the per-target "last searched at" map (JSON object of
/// `target key -> RFC3339 timestamp`).
const SEARCH_STATE_SETTING: &str = "auto_download.search_state";

/// A target is not re-searched more often than this, unless the search was
/// triggered manually through `searchMissing`.
pub const SEARCH_BACKOFF_HOURS: i64 = 6;

/// Search-state entries older than this are dropped when the map is written
/// back, so the setting cannot grow without bound.
const SEARCH_STATE_RETENTION_DAYS: i64 = 30;

/// A season is hunted as a pack when at least this many of its episodes are
/// missing...
const SEASON_PACK_MIN_MISSING: usize = 3;

/// ...or when at least this share of its aired episodes are missing.
const SEASON_PACK_MISSING_RATIO: f64 = 0.5;

/// Wanted-target linkage to stamp on a torrent grabbed for a [`WantedCandidate`].
#[derive(Debug, Clone, Default)]
pub struct CandidateLinkage {
    pub episode_id: Option<String>,
    pub movie_id: Option<String>,
    pub track_id: Option<String>,
    pub chapter_id: Option<String>,
    pub show_id: Option<String>,
    pub album_id: Option<String>,
    pub audiobook_id: Option<String>,
    /// Set (with `show_id`) for season-pack grabs.
    pub season: Option<i32>,
}

/// A single hunt target resolved to one or more search queries.
#[derive(Debug, Clone)]
pub struct WantedCandidate {
    /// Human-facing label for logs.
    pub label: String,
    pub library_id: String,
    /// Query variants to try. Results from every variant are merged and
    /// de-duplicated before ranking (a season pack is searched both as
    /// "Show S02" and "Show Season 2" because indexers title them either way).
    pub queries: Vec<SourceQuery>,
    /// Fuzzy-match anchor: what an acceptable release's title must resemble.
    pub match_title: String,
    pub linkage: CandidateLinkage,
    /// Per-entity quality profile override (Show/Movie/Album/Audiobook), if
    /// set. Falls back to `Library.qualityProfileId` (then the seeded
    /// default) via `quality::profile::resolve_profile`.
    pub quality_profile_override_id: Option<String>,
    /// Stable key for the search-backoff map.
    pub search_key: String,
    /// How many episodes a grab is expected to satisfy. `1` for everything
    /// except season packs; used to interpret the profile's size limits per
    /// episode.
    pub episode_count: i32,
    /// Tried, in order, only if this candidate grabs nothing. Used for the
    /// "season pack first, per-episode second" flow. One level deep — a
    /// fallback never has fallbacks of its own.
    pub fallbacks: Vec<WantedCandidate>,
}

/// What a hunt pass should cover. `run_once` uses a library-wide scope; the
/// `searchMissing` mutation and the retry sweep use targeted scopes.
#[derive(Debug, Clone, Default)]
pub struct HuntScope {
    pub library_id: Option<String>,
    pub show_id: Option<String>,
    pub season: Option<i32>,
    pub movie_id: Option<String>,
    pub album_id: Option<String>,
    pub audiobook_id: Option<String>,
    /// Manual searches ignore the per-target backoff.
    pub ignore_backoff: bool,
}

impl HuntScope {
    /// The scheduled whole-library (or single-library) pass.
    pub fn library(library_id: Option<String>) -> Self {
        Self {
            library_id,
            ..Default::default()
        }
    }

    /// Whether the scope names at least one concrete item.
    pub fn has_target(&self) -> bool {
        self.show_id.is_some()
            || self.movie_id.is_some()
            || self.album_id.is_some()
            || self.audiobook_id.is_some()
            || self.library_id.is_some()
    }

    fn wants_shows(&self) -> bool {
        self.show_id.is_some() || !self.names_other_kind(self.show_id.is_some())
    }

    fn wants_movies(&self) -> bool {
        self.movie_id.is_some() || !self.names_other_kind(self.movie_id.is_some())
    }

    fn wants_albums(&self) -> bool {
        self.album_id.is_some() || !self.names_other_kind(self.album_id.is_some())
    }

    fn wants_audiobooks(&self) -> bool {
        self.audiobook_id.is_some() || !self.names_other_kind(self.audiobook_id.is_some())
    }

    /// True when the scope names a specific item of some *other* kind, in
    /// which case this kind must be skipped entirely.
    fn names_other_kind(&self, this_kind_named: bool) -> bool {
        let any_named = self.show_id.is_some()
            || self.movie_id.is_some()
            || self.album_id.is_some()
            || self.audiobook_id.is_some();
        any_named && !this_kind_named
    }
}

/// Outcome of a single hunt pass, returned to the caller (background loop,
/// `triggerAutoDownload`, or `searchMissing`) for logging/reporting.
#[derive(Debug, Clone, Default)]
pub struct AutoDownloadRunSummary {
    pub candidates_considered: usize,
    pub searched: usize,
    pub grabbed: usize,
    pub errors: Vec<String>,
}

fn string_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

fn bool_eq(value: bool) -> BoolFilter {
    BoolFilter {
        eq: Some(value),
        ..Default::default()
    }
}

fn is_null() -> StringFilter {
    StringFilter {
        is_null: Some(true),
        ..Default::default()
    }
}

// ===========================================================================
// Auto-download mode normalisation
// ===========================================================================

/// Normalise the (`auto_download`, `auto_download_mode`) pair on read.
///
/// `auto_download_mode` is authoritative: `NONE` means "never hunt" and
/// implies `autoDownload = false`; any other mode means "hunt" and implies
/// `autoDownload = true`. The boolean stays as the master switch the UI
/// toggles, but a stale boolean never overrides the mode.
pub fn normalize_auto_download(
    _auto_download: bool,
    mode: AutoDownloadMode,
) -> (bool, AutoDownloadMode) {
    (mode != AutoDownloadMode::None, mode)
}

// ===========================================================================
// Release blocklist
// ===========================================================================

/// Active `ReleaseBlocklist` entries, indexed for O(1) candidate filtering.
#[derive(Debug, Clone, Default)]
pub struct Blocklist {
    info_hashes: HashSet<String>,
    guids: HashSet<String>,
}

impl Blocklist {
    /// Load every entry that has not expired.
    pub async fn load(db: &Database) -> Result<Self> {
        let now = Utc::now();
        let rows = ReleaseBlocklist::query(db.pool())
            .filter(ReleaseBlocklistWhereInput::default())
            .fetch_all()
            .await
            .context("failed to query the release blocklist")?;

        let mut blocklist = Self::default();
        for row in rows {
            let expired = row
                .expires_at
                .as_deref()
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .is_some_and(|expires| expires.with_timezone(&Utc) <= now);
            if expired {
                continue;
            }
            if let Some(hash) = row.info_hash {
                blocklist.info_hashes.insert(hash.to_ascii_lowercase());
            }
            if let Some(guid) = row.guid {
                blocklist.guids.insert(guid.to_ascii_lowercase());
            }
        }
        Ok(blocklist)
    }

    pub fn blocks_info_hash(&self, info_hash: &str) -> bool {
        self.info_hashes.contains(&info_hash.to_ascii_lowercase())
    }

    pub fn blocks(&self, release: &SourceRelease) -> bool {
        release
            .info_hash
            .as_deref()
            .is_some_and(|hash| self.blocks_info_hash(hash))
            || self.guids.contains(&release.guid.to_ascii_lowercase())
    }

    pub fn is_empty(&self) -> bool {
        self.info_hashes.is_empty() && self.guids.is_empty()
    }
}

// ===========================================================================
// Search backoff state
// ===========================================================================

/// `target key -> last searched at (RFC3339)`.
pub type SearchState = HashMap<String, String>;

async fn find_search_state_setting(db: &Database) -> Result<Option<AppSetting>> {
    Ok(AppSetting::query(db.pool())
        .filter(AppSettingWhereInput {
            key: Some(string_eq(SEARCH_STATE_SETTING)),
            ..Default::default()
        })
        .fetch_all()
        .await
        .context("failed to query the auto-download search state setting")?
        .into_iter()
        .next())
}

async fn load_search_state(db: &Database) -> SearchState {
    match find_search_state_setting(db).await {
        Ok(Some(setting)) => serde_json::from_str(&setting.value).unwrap_or_else(|error| {
            warn!(
                setting = SEARCH_STATE_SETTING,
                error = %error,
                "Auto-download: search state is not valid JSON; starting from an empty map"
            );
            SearchState::new()
        }),
        Ok(None) => SearchState::new(),
        Err(error) => {
            warn!(
                setting = SEARCH_STATE_SETTING,
                error = %error,
                "Auto-download: could not read search state; backoff is disabled for this pass"
            );
            SearchState::new()
        }
    }
}

async fn save_search_state(db: &Database, state: &SearchState) -> Result<()> {
    let cutoff = Utc::now() - chrono::Duration::days(SEARCH_STATE_RETENTION_DAYS);
    let pruned: SearchState = state
        .iter()
        .filter(|(_, timestamp)| {
            DateTime::parse_from_rfc3339(timestamp)
                .map(|parsed| parsed.with_timezone(&Utc) >= cutoff)
                .unwrap_or(false)
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let value = serde_json::to_string(&pruned)?;

    match find_search_state_setting(db).await? {
        Some(existing) => {
            AppSetting::update_by_id(
                db,
                &existing.id,
                UpdateAppSettingInput {
                    value: Some(value),
                    ..Default::default()
                },
            )
            .await
            .context("failed to update the auto-download search state")?;
        }
        None => {
            AppSetting::insert(
                db,
                CreateAppSettingInput {
                    key: SEARCH_STATE_SETTING.to_string(),
                    value,
                    description: Some(
                        "Auto-download hunt: last search time per wanted target".to_string(),
                    ),
                    category: "auto_download".to_string(),
                },
            )
            .await
            .context("failed to create the auto-download search state")?;
        }
    }
    Ok(())
}

/// Whether a target may be searched again: never searched before, or the
/// backoff window has elapsed. Pure so the window logic is unit-testable.
pub fn backoff_elapsed(state: &SearchState, key: &str, now: DateTime<Utc>) -> bool {
    let Some(last) = state.get(key) else {
        return true;
    };
    let Ok(parsed) = DateTime::parse_from_rfc3339(last) else {
        return true;
    };
    now.signed_duration_since(parsed.with_timezone(&Utc))
        >= chrono::Duration::hours(SEARCH_BACKOFF_HOURS)
}

// ===========================================================================
// Shared discovery helpers
// ===========================================================================

/// Whether a torrent already exists for this wanted-target filter that isn't
/// in a terminal `failed` state and isn't blocklisted — used to avoid
/// re-grabbing an item on every tick while a previous grab is still in flight
/// or already satisfied it. A blocklisted torrent counts as resolved so the
/// retry sweep's re-hunt is not blocked by the very grab it just blocked.
async fn has_unresolved_torrent(
    db: &Database,
    filter: TorrentWhereInput,
    blocklist: &Blocklist,
) -> Result<bool> {
    let rows: Vec<Torrent> = Torrent::query(db.pool()).filter(filter).fetch_all().await?;
    Ok(rows.iter().any(|torrent| {
        torrent.post_process_status.as_deref() != Some("failed")
            && !blocklist.blocks_info_hash(&torrent.info_hash)
    }))
}

/// Whether a dated item has already been released. Items with no date at all
/// are treated as released: back-catalogue entries frequently have no air date
/// and refusing to hunt them would make `ALL` mode useless.
fn is_released(date: Option<&str>, stamp: Option<&str>, now: DateTime<Utc>) -> bool {
    for value in [stamp, date].into_iter().flatten() {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
            return parsed.with_timezone(&Utc) <= now;
        }
        if let Ok(parsed) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
            return parsed <= now.date_naive();
        }
    }
    true
}

fn is_ignored(ignored: Option<bool>) -> bool {
    ignored.unwrap_or(false)
}

/// First four-digit year in a date-ish string (`2019-04-01` -> `2019`).
fn year_from_date(value: Option<&str>) -> Option<i32> {
    let value = value?;
    value.get(0..4)?.parse::<i32>().ok()
}

// ===========================================================================
// Candidate discovery: TV
// ===========================================================================

/// Flip `wanted = true` on rows that `ALL` mode implies are wanted, so the
/// stored status matches what the hunt is actually doing.
async fn mark_episode_wanted(db: &Database, episode: &Episode, show_name: &str) {
    if let Err(error) = Episode::update_by_id(
        db,
        &episode.id,
        UpdateEpisodeInput {
            wanted: Some(true),
            ..Default::default()
        },
    )
    .await
    {
        warn!(
            show = %show_name,
            episode = %format!("S{:02}E{:02}", episode.season, episode.episode),
            error = %error,
            "Auto-download: failed to mark episode wanted for ALL mode"
        );
    } else {
        info!(
            show = %show_name,
            episode = %format!("S{:02}E{:02}", episode.season, episode.episode),
            "Auto-download: marked missing aired episode wanted (mode ALL)"
        );
    }
}

fn episode_candidate(
    show: &Show,
    episode: &Episode,
    profile_override: Option<String>,
) -> WantedCandidate {
    let term = format!(
        "{} S{:02}E{:02}",
        show.name, episode.season, episode.episode
    );
    WantedCandidate {
        label: format!("episode '{term}'"),
        library_id: show.library_id.clone(),
        match_title: term.clone(),
        queries: vec![SourceQuery {
            query_type: QueryType::TvSearch,
            search_term: Some(term),
            season: Some(episode.season),
            episode: Some(episode.episode.to_string()),
            tvmaze_id: show.tvmaze_id,
            tmdb_id: show.tmdb_id,
            tvdb_id: show.tvdb_id,
            imdb_id: show.imdb_id.clone(),
            year: show.year,
            cache: true,
            ..Default::default()
        }],
        linkage: CandidateLinkage {
            episode_id: Some(episode.id.clone()),
            show_id: Some(show.id.clone()),
            ..Default::default()
        },
        quality_profile_override_id: profile_override,
        search_key: format!("episode:{}", episode.id),
        episode_count: 1,
        fallbacks: Vec::new(),
    }
}

fn season_pack_candidate(
    show: &Show,
    season: i32,
    episode_count: i32,
    profile_override: Option<String>,
    fallbacks: Vec<WantedCandidate>,
) -> WantedCandidate {
    let padded = format!("{} S{:02}", show.name, season);
    let worded = format!("{} Season {}", show.name, season);
    let base = SourceQuery {
        query_type: QueryType::TvSearch,
        season: Some(season),
        episode: None,
        tvmaze_id: show.tvmaze_id,
        tmdb_id: show.tmdb_id,
        tvdb_id: show.tvdb_id,
        imdb_id: show.imdb_id.clone(),
        year: show.year,
        cache: true,
        ..Default::default()
    };
    WantedCandidate {
        label: format!("season pack '{padded}'"),
        library_id: show.library_id.clone(),
        match_title: padded.clone(),
        queries: vec![
            SourceQuery {
                search_term: Some(padded),
                ..base.clone()
            },
            SourceQuery {
                search_term: Some(worded),
                ..base
            },
        ],
        linkage: CandidateLinkage {
            show_id: Some(show.id.clone()),
            season: Some(season),
            ..Default::default()
        },
        quality_profile_override_id: profile_override,
        search_key: format!("show:{}:s{}", show.id, season),
        episode_count: episode_count.max(1),
        fallbacks,
    }
}

/// Whether a season should be hunted as a pack: at least
/// [`SEASON_PACK_MIN_MISSING`] missing episodes, or at least
/// [`SEASON_PACK_MISSING_RATIO`] of the season's aired episodes missing.
pub fn should_hunt_season_pack(missing: usize, aired: usize, allow_packs: bool) -> bool {
    if !allow_packs || missing == 0 {
        return false;
    }
    missing >= SEASON_PACK_MIN_MISSING
        || (aired > 0 && (missing as f64) >= (aired as f64) * SEASON_PACK_MISSING_RATIO)
}

async fn discover_episode_candidates(
    db: &Database,
    scope: &HuntScope,
    blocklist: &Blocklist,
) -> Result<Vec<WantedCandidate>> {
    let mut filter = ShowWhereInput::default();
    if let Some(library_id) = &scope.library_id {
        filter.library_id = Some(string_eq(library_id));
    }
    if let Some(show_id) = &scope.show_id {
        filter.id = Some(string_eq(show_id));
    }
    let shows = Show::query(db.pool()).filter(filter).fetch_all().await?;

    let now = Utc::now();
    let mut out = Vec::new();

    for show in shows {
        let (enabled, mode) = normalize_auto_download(show.auto_download, show.auto_download_mode);
        if !enabled {
            tracing::debug!(
                show = %show.name,
                mode = %mode,
                "Auto-download: skipping show, auto-download mode is NONE"
            );
            continue;
        }

        let episodes = Episode::query(db.pool())
            .filter(EpisodeWhereInput {
                show_id: Some(string_eq(&show.id)),
                ..Default::default()
            })
            .fetch_all()
            .await?;

        // Missing = no file, not ignored, already aired.
        let missing: Vec<Episode> = episodes
            .iter()
            .filter(|episode| {
                episode.media_file_id.is_none()
                    && !is_ignored(episode.ignored)
                    && is_released(
                        episode.air_date.as_deref(),
                        episode.air_stamp.as_deref(),
                        now,
                    )
            })
            .cloned()
            .collect();

        // ALL mode: everything missing and aired counts as wanted, and the row
        // is updated so status reflects that.
        if mode == AutoDownloadMode::All {
            for episode in missing.iter().filter(|episode| !episode.wanted) {
                mark_episode_wanted(db, episode, &show.name).await;
            }
        }

        let mut targets: Vec<Episode> = match mode {
            AutoDownloadMode::All => missing.clone(),
            _ => missing.iter().filter(|e| e.wanted).cloned().collect(),
        };
        if let Some(season) = scope.season {
            targets.retain(|episode| episode.season == season);
        }
        if targets.is_empty() {
            continue;
        }

        let quality_profile = profile::resolve_profile(
            db,
            show.quality_profile_id.as_deref(),
            &show.library_id,
        )
        .await
        .unwrap_or_else(|error| {
            warn!(
                show = %show.name,
                error = %error,
                "Auto-download: failed to resolve quality profile; falling back to 'Any Quality'"
            );
            profile::builtin_any_profile()
        });

        let mut seasons: Vec<i32> = targets.iter().map(|episode| episode.season).collect();
        seasons.sort_unstable();
        seasons.dedup();

        for season in seasons {
            // A pack for this season is already downloading: leave the whole
            // season alone until it resolves.
            if has_unresolved_torrent(
                db,
                TorrentWhereInput {
                    show_id: Some(string_eq(&show.id)),
                    season: Some(IntFilter {
                        eq: Some(season),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                blocklist,
            )
            .await?
            {
                tracing::debug!(
                    show = %show.name,
                    season,
                    "Auto-download: a season pack is already in flight; skipping the season"
                );
                continue;
            }

            let season_targets: Vec<&Episode> = targets
                .iter()
                .filter(|episode| episode.season == season)
                .collect();
            let aired_in_season = episodes
                .iter()
                .filter(|episode| {
                    episode.season == season
                        && is_released(
                            episode.air_date.as_deref(),
                            episode.air_stamp.as_deref(),
                            now,
                        )
                })
                .count();

            let mut per_episode = Vec::new();
            for episode in &season_targets {
                if has_unresolved_torrent(
                    db,
                    TorrentWhereInput {
                        episode_id: Some(string_eq(&episode.id)),
                        ..Default::default()
                    },
                    blocklist,
                )
                .await?
                {
                    continue;
                }
                per_episode.push(episode_candidate(
                    &show,
                    episode,
                    show.quality_profile_id.clone(),
                ));
            }

            if should_hunt_season_pack(
                season_targets.len(),
                aired_in_season,
                quality_profile.allow_season_packs,
            ) {
                out.push(season_pack_candidate(
                    &show,
                    season,
                    aired_in_season.max(season_targets.len()) as i32,
                    show.quality_profile_id.clone(),
                    per_episode,
                ));
            } else {
                out.extend(per_episode);
            }
        }
    }

    Ok(out)
}

// ===========================================================================
// Candidate discovery: music
// ===========================================================================

async fn resolve_artist_name(db: &Database, album: &Album) -> Result<String> {
    let artists = Artist::query(db.pool())
        .filter(ArtistWhereInput {
            id: Some(string_eq(&album.artist_id)),
            ..Default::default()
        })
        .fetch_all()
        .await?;
    Ok(artists
        .into_iter()
        .next()
        .map(|artist| artist.name)
        .unwrap_or_else(|| album.name.clone()))
}

async fn discover_track_candidates(
    db: &Database,
    scope: &HuntScope,
    blocklist: &Blocklist,
) -> Result<Vec<WantedCandidate>> {
    let mut filter = AlbumWhereInput::default();
    if let Some(library_id) = &scope.library_id {
        filter.library_id = Some(string_eq(library_id));
    }
    if let Some(album_id) = &scope.album_id {
        filter.id = Some(string_eq(album_id));
    }
    let albums = Album::query(db.pool()).filter(filter).fetch_all().await?;

    let now = Utc::now();
    let mut out = Vec::new();

    for album in albums {
        let (enabled, mode) =
            normalize_auto_download(album.auto_download, album.auto_download_mode);
        if !enabled {
            continue;
        }
        if !is_released(album.release_date.as_deref(), None, now) {
            continue;
        }

        let tracks = Track::query(db.pool())
            .filter(TrackWhereInput {
                album_id: Some(string_eq(&album.id)),
                media_file_id: Some(is_null()),
                ..Default::default()
            })
            .fetch_all()
            .await?;

        let missing: Vec<Track> = tracks
            .into_iter()
            .filter(|track| !is_ignored(track.ignored))
            .collect();

        if mode == AutoDownloadMode::All {
            for track in missing.iter().filter(|track| !track.wanted) {
                if let Err(error) = Track::update_by_id(
                    db,
                    &track.id,
                    UpdateTrackInput {
                        wanted: Some(true),
                        ..Default::default()
                    },
                )
                .await
                {
                    warn!(
                        album = %album.name,
                        track = %track.title,
                        error = %error,
                        "Auto-download: failed to mark track wanted for ALL mode"
                    );
                }
            }
        }

        let targets: Vec<Track> = match mode {
            AutoDownloadMode::All => missing,
            _ => missing.into_iter().filter(|track| track.wanted).collect(),
        };
        if targets.is_empty() {
            continue;
        }

        let fallback_artist = resolve_artist_name(db, &album).await?;
        let album_year = album
            .year
            .or_else(|| year_from_date(album.release_date.as_deref()));

        for track in targets {
            if has_unresolved_torrent(
                db,
                TorrentWhereInput {
                    track_id: Some(string_eq(&track.id)),
                    ..Default::default()
                },
                blocklist,
            )
            .await?
            {
                continue;
            }

            let artist = track
                .artist_name
                .clone()
                .unwrap_or_else(|| fallback_artist.clone());
            // Per-track query only — never a whole-album search.
            let term = format!("{} {} {}", artist, album.name, track.title);
            out.push(WantedCandidate {
                label: format!("track '{term}'"),
                library_id: track.library_id.clone(),
                match_title: term.clone(),
                queries: vec![SourceQuery {
                    query_type: QueryType::MusicSearch,
                    search_term: Some(term),
                    artist: Some(artist),
                    album: Some(album.name.clone()),
                    year: album_year,
                    cache: true,
                    ..Default::default()
                }],
                linkage: CandidateLinkage {
                    track_id: Some(track.id.clone()),
                    album_id: Some(album.id.clone()),
                    ..Default::default()
                },
                quality_profile_override_id: album.quality_profile_id.clone(),
                search_key: format!("track:{}", track.id),
                episode_count: 1,
                fallbacks: Vec::new(),
            });
        }
    }
    Ok(out)
}

// ===========================================================================
// Candidate discovery: audiobooks
// ===========================================================================

/// Audiobooks are searched (and grabbed) at the whole-book level, not
/// per-chapter: unlike TV/music, audiobook releases are essentially never
/// published as one-file-per-chapter torrents, so a per-chapter `SourceQuery`
/// would almost never match anything real. See `docs/design.md` Q52.
async fn discover_audiobook_candidates(
    db: &Database,
    scope: &HuntScope,
    blocklist: &Blocklist,
) -> Result<Vec<WantedCandidate>> {
    let mut filter = AudiobookWhereInput::default();
    if let Some(library_id) = &scope.library_id {
        filter.library_id = Some(string_eq(library_id));
    }
    if let Some(audiobook_id) = &scope.audiobook_id {
        filter.id = Some(string_eq(audiobook_id));
    }
    let audiobooks = Audiobook::query(db.pool())
        .filter(filter)
        .fetch_all()
        .await?;

    let now = Utc::now();
    let mut out = Vec::new();

    for audiobook in audiobooks {
        let (enabled, mode) =
            normalize_auto_download(audiobook.auto_download, audiobook.auto_download_mode);
        if !enabled {
            continue;
        }
        if !is_released(audiobook.published_date.as_deref(), None, now) {
            continue;
        }

        let chapters = Chapter::query(db.pool())
            .filter(ChapterWhereInput {
                audiobook_id: Some(string_eq(&audiobook.id)),
                media_file_id: Some(is_null()),
                ..Default::default()
            })
            .fetch_all()
            .await?;

        let missing: Vec<Chapter> = chapters
            .into_iter()
            .filter(|chapter| !is_ignored(chapter.ignored))
            .collect();

        if mode == AutoDownloadMode::All {
            for chapter in missing.iter().filter(|chapter| !chapter.wanted) {
                if let Err(error) = Chapter::update_by_id(
                    db,
                    &chapter.id,
                    UpdateChapterInput {
                        wanted: Some(true),
                        ..Default::default()
                    },
                )
                .await
                {
                    warn!(
                        audiobook = %audiobook.title,
                        chapter = chapter.chapter_number,
                        error = %error,
                        "Auto-download: failed to mark chapter wanted for ALL mode"
                    );
                }
            }
        }

        let has_target = match mode {
            AutoDownloadMode::All => !missing.is_empty(),
            _ => missing.iter().any(|chapter| chapter.wanted),
        };
        if !has_target {
            continue;
        }

        if has_unresolved_torrent(
            db,
            TorrentWhereInput {
                audiobook_id: Some(string_eq(&audiobook.id)),
                ..Default::default()
            },
            blocklist,
        )
        .await?
        {
            continue;
        }

        let term = match &audiobook.author_name {
            Some(author) => format!("{} {}", author, audiobook.title),
            None => audiobook.title.clone(),
        };
        out.push(WantedCandidate {
            label: format!("audiobook '{term}'"),
            library_id: audiobook.library_id.clone(),
            match_title: term.clone(),
            queries: vec![SourceQuery {
                query_type: QueryType::BookSearch,
                search_term: Some(term),
                title: Some(audiobook.title.clone()),
                author: audiobook.author_name.clone(),
                year: year_from_date(audiobook.published_date.as_deref()),
                cache: true,
                ..Default::default()
            }],
            linkage: CandidateLinkage {
                audiobook_id: Some(audiobook.id.clone()),
                ..Default::default()
            },
            quality_profile_override_id: audiobook.quality_profile_id.clone(),
            search_key: format!("audiobook:{}", audiobook.id),
            episode_count: 1,
            fallbacks: Vec::new(),
        });
    }
    Ok(out)
}

// ===========================================================================
// Candidate discovery: movies
// ===========================================================================

async fn discover_movie_candidates(
    db: &Database,
    scope: &HuntScope,
    blocklist: &Blocklist,
) -> Result<Vec<WantedCandidate>> {
    let mut filter = MovieWhereInput {
        monitored: Some(bool_eq(true)),
        wanted: Some(bool_eq(true)),
        ..Default::default()
    };
    if let Some(library_id) = &scope.library_id {
        filter.library_id = Some(string_eq(library_id));
    }
    if let Some(movie_id) = &scope.movie_id {
        filter.id = Some(string_eq(movie_id));
        // A targeted manual search must work even for a movie whose `wanted`
        // flag was cleared; `monitored` is still respected.
        filter.wanted = None;
    }
    let movies = Movie::query(db.pool()).filter(filter).fetch_all().await?;

    let now = Utc::now();
    let mut out = Vec::new();

    for movie in movies {
        if is_ignored(movie.ignored) || movie.media_file_id.is_some() {
            continue;
        }
        if !is_released(movie.release_date.as_deref(), None, now) {
            continue;
        }
        if has_unresolved_torrent(
            db,
            TorrentWhereInput {
                movie_id: Some(string_eq(&movie.id)),
                ..Default::default()
            },
            blocklist,
        )
        .await?
        {
            continue;
        }

        let term = match movie.year {
            Some(year) => format!("{} {}", movie.title, year),
            None => movie.title.clone(),
        };
        out.push(WantedCandidate {
            label: format!("movie '{term}'"),
            library_id: movie.library_id.clone(),
            match_title: movie.title.clone(),
            queries: vec![SourceQuery {
                query_type: QueryType::MovieSearch,
                search_term: Some(term),
                imdb_id: movie.imdb_id.clone(),
                tmdb_id: movie.tmdb_id,
                year: movie
                    .year
                    .or_else(|| year_from_date(movie.release_date.as_deref())),
                cache: true,
                ..Default::default()
            }],
            linkage: CandidateLinkage {
                movie_id: Some(movie.id.clone()),
                ..Default::default()
            },
            quality_profile_override_id: movie.quality_profile_id.clone(),
            search_key: format!("movie:{}", movie.id),
            episode_count: 1,
            fallbacks: Vec::new(),
        });
    }
    Ok(out)
}

async fn discover_candidates(
    db: &Database,
    scope: &HuntScope,
    blocklist: &Blocklist,
) -> Result<Vec<WantedCandidate>> {
    let mut out = Vec::new();
    if scope.wants_shows() {
        out.extend(discover_episode_candidates(db, scope, blocklist).await?);
    }
    if scope.wants_albums() {
        out.extend(discover_track_candidates(db, scope, blocklist).await?);
    }
    if scope.wants_audiobooks() {
        out.extend(discover_audiobook_candidates(db, scope, blocklist).await?);
    }
    if scope.wants_movies() {
        out.extend(discover_movie_candidates(db, scope, blocklist).await?);
    }
    Ok(out)
}

// ===========================================================================
// Search + grab
// ===========================================================================

struct HuntContext {
    db: Database,
    sources: Arc<crate::services::sources::manager::SourcesManager>,
    torrent: Arc<crate::services::torrent::TorrentService>,
    user_id: Option<uuid::Uuid>,
    blocklist: Blocklist,
}

/// Run every query variant for a candidate and merge the results,
/// de-duplicating on info hash (falling back to guid) and dropping
/// blocklisted releases.
async fn collect_releases(ctx: &HuntContext, candidate: &WantedCandidate) -> Vec<SourceRelease> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut releases = Vec::new();
    for query in &candidate.queries {
        for result in ctx.sources.search_all(query).await {
            for release in result.releases {
                let key = release
                    .info_hash
                    .clone()
                    .unwrap_or_else(|| release.guid.clone())
                    .to_ascii_lowercase();
                if !seen.insert(key) {
                    continue;
                }
                if ctx.blocklist.blocks(&release) {
                    tracing::debug!(
                        candidate = %candidate.label,
                        release = %release.title,
                        "Auto-download: skipping blocklisted release"
                    );
                    continue;
                }
                releases.push(release);
            }
        }
    }
    releases
}

async fn resolve_candidate_profile(
    ctx: &HuntContext,
    candidate: &WantedCandidate,
) -> QualityProfile {
    match profile::resolve_profile(
        &ctx.db,
        candidate.quality_profile_override_id.as_deref(),
        &candidate.library_id,
    )
    .await
    {
        Ok(resolved) => resolved,
        Err(error) => {
            warn!(
                candidate = %candidate.label,
                error = %error,
                "Auto-download: failed to resolve quality profile; falling back to 'Any Quality'"
            );
            profile::builtin_any_profile()
        }
    }
}

/// Search for and grab the best release for one candidate.
/// Returns `Ok(true)` when a release was grabbed.
async fn try_candidate(
    ctx: &HuntContext,
    candidate: &WantedCandidate,
    summary: &mut AutoDownloadRunSummary,
) -> Result<bool> {
    summary.searched += 1;
    let releases = collect_releases(ctx, candidate).await;
    let quality_profile = resolve_candidate_profile(ctx, candidate).await;

    let Some(best) = profile::pick_best_for_profile(
        &candidate.match_title,
        releases,
        &quality_profile,
        candidate.episode_count,
    ) else {
        tracing::debug!(
            candidate = %candidate.label,
            profile = %quality_profile.name,
            "Auto-download: no acceptable release found"
        );
        return Ok(false);
    };

    let release = best.release;
    let Some(uri) = release.magnet_uri.clone().or_else(|| release.link.clone()) else {
        warn!(
            candidate = %candidate.label,
            release = %release.title,
            "Auto-download: best release had neither a magnet URI nor a download link; skipping"
        );
        return Ok(false);
    };

    let metadata = TorrentAddMetadata {
        library_id: Some(candidate.library_id.clone()),
        source_url: release
            .details
            .clone()
            .or_else(|| Some(release.guid.clone())),
        source_indexer_id: release.source_id.clone(),
        source_feed_id: None,
        episode_id: candidate.linkage.episode_id.clone(),
        movie_id: candidate.linkage.movie_id.clone(),
        track_id: candidate.linkage.track_id.clone(),
        chapter_id: candidate.linkage.chapter_id.clone(),
        show_id: candidate.linkage.show_id.clone(),
        album_id: candidate.linkage.album_id.clone(),
        audiobook_id: candidate.linkage.audiobook_id.clone(),
    };

    match ctx
        .torrent
        .add_magnet_with_metadata(&uri, ctx.user_id, metadata)
        .await
    {
        Ok(info) => {
            summary.grabbed += 1;
            // Season (for packs) and the tracker's seeding floors are not
            // carried by `TorrentAddMetadata`; stamp them on the row the
            // torrent service just created.
            let details = TorrentGrabDetails::from_release_limits(
                candidate.linkage.season,
                release.minimum_ratio,
                release.minimum_seed_time,
            );
            if let Err(error) = stamp_torrent_grab_details(&ctx.db, &info.info_hash, details).await
            {
                warn!(
                    candidate = %candidate.label,
                    info_hash = %info.info_hash,
                    error = %error,
                    "Auto-download: grabbed a release but could not stamp its grab details"
                );
            }
            info!(
                candidate = %candidate.label,
                torrent = %info.name,
                release = %release.title,
                seeders = release.seeders.unwrap_or(0),
                profile = %quality_profile.name,
                "Auto-download: grabbed release"
            );
            Ok(true)
        }
        Err(error) => {
            warn!(
                error = %error,
                candidate = %candidate.label,
                release = %release.title,
                "Auto-download: grab failed"
            );
            summary
                .errors
                .push(format!("{}: {}", candidate.label, error));
            Ok(false)
        }
    }
}

/// Run one hunt pass over `scope`.
pub async fn run_scoped(
    manager: &Arc<ServicesManager>,
    scope: &HuntScope,
) -> Result<AutoDownloadRunSummary> {
    let db_service = manager
        .get_database()
        .await
        .context("database service not available")?;
    let db = db_service.pool().clone();

    let sources_service = manager
        .get_sources()
        .await
        .context("sources service not available")?;
    let sources_manager = sources_service
        .get_manager()
        .await
        .context("sources manager not initialized")?;

    let torrent_service = manager
        .get_torrent()
        .await
        .context("torrent service not available")?;

    let user_id = get_default_user_id(&db)
        .await
        .context("failed to look up default user")?;

    let blocklist = Blocklist::load(&db).await.unwrap_or_else(|error| {
        warn!(error = %error, "Auto-download: failed to load the release blocklist; continuing without it");
        Blocklist::default()
    });

    let candidates = discover_candidates(&db, scope, &blocklist).await?;
    let mut summary = AutoDownloadRunSummary {
        candidates_considered: candidates.len(),
        ..Default::default()
    };
    if candidates.is_empty() {
        return Ok(summary);
    }

    let mut state = load_search_state(&db).await;
    let now = Utc::now();
    let now_iso = now.to_rfc3339();

    info!(
        count = summary.candidates_considered,
        blocklist_active = !blocklist.is_empty(),
        scope_library = scope.library_id.as_deref().unwrap_or("all"),
        "Auto-download: found wanted candidates to search"
    );

    let ctx = HuntContext {
        db: db.clone(),
        sources: sources_manager,
        torrent: torrent_service,
        user_id,
        blocklist,
    };

    for candidate in candidates {
        if !scope.ignore_backoff && !backoff_elapsed(&state, &candidate.search_key, now) {
            tracing::debug!(
                candidate = %candidate.label,
                "Auto-download: skipping, searched within the last {SEARCH_BACKOFF_HOURS}h"
            );
            continue;
        }
        state.insert(candidate.search_key.clone(), now_iso.clone());

        let grabbed = try_candidate(&ctx, &candidate, &mut summary).await?;
        if grabbed {
            continue;
        }

        // Season-pack fallback: one level only, and each fallback carries its
        // own backoff key so a failed pack search does not burn the
        // per-episode budget.
        for fallback in &candidate.fallbacks {
            if !scope.ignore_backoff && !backoff_elapsed(&state, &fallback.search_key, now) {
                continue;
            }
            state.insert(fallback.search_key.clone(), now_iso.clone());
            let _ = try_candidate(&ctx, fallback, &mut summary).await?;
        }
    }

    if let Err(error) = save_search_state(&db, &state).await {
        warn!(error = %error, "Auto-download: failed to persist search backoff state");
    }

    Ok(summary)
}

/// Run one full auto-download pass, optionally scoped to a single library.
///
/// Called both by the background timer loop (when enabled) and by the
/// `triggerAutoDownload` GraphQL mutation (always, regardless of the enabled
/// gate — that gate only controls the automatic loop).
pub async fn run_once(
    manager: &Arc<ServicesManager>,
    library_id: Option<&str>,
) -> Result<AutoDownloadRunSummary> {
    run_scoped(
        manager,
        &HuntScope::library(library_id.map(ToString::to_string)),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_auto_download_treats_mode_as_authoritative() {
        assert_eq!(
            normalize_auto_download(true, AutoDownloadMode::None),
            (false, AutoDownloadMode::None)
        );
        assert_eq!(
            normalize_auto_download(false, AutoDownloadMode::Wanted),
            (true, AutoDownloadMode::Wanted)
        );
        assert_eq!(
            normalize_auto_download(false, AutoDownloadMode::All),
            (true, AutoDownloadMode::All)
        );
    }

    #[test]
    fn season_pack_threshold_uses_count_or_ratio() {
        // Three missing episodes is always enough.
        assert!(should_hunt_season_pack(3, 20, true));
        // Half of an aired season is enough even below three.
        assert!(should_hunt_season_pack(1, 2, true));
        // One missing episode out of ten is not.
        assert!(!should_hunt_season_pack(1, 10, true));
        // Never when the profile forbids packs.
        assert!(!should_hunt_season_pack(10, 10, false));
        // Never with nothing missing.
        assert!(!should_hunt_season_pack(0, 10, true));
    }

    #[test]
    fn backoff_blocks_recent_searches_only() {
        let now = Utc::now();
        let mut state = SearchState::new();
        assert!(backoff_elapsed(&state, "movie:1", now));

        state.insert(
            "movie:1".to_string(),
            (now - chrono::Duration::hours(1)).to_rfc3339(),
        );
        assert!(!backoff_elapsed(&state, "movie:1", now));

        state.insert(
            "movie:1".to_string(),
            (now - chrono::Duration::hours(SEARCH_BACKOFF_HOURS + 1)).to_rfc3339(),
        );
        assert!(backoff_elapsed(&state, "movie:1", now));

        state.insert("movie:1".to_string(), "not-a-timestamp".to_string());
        assert!(backoff_elapsed(&state, "movie:1", now));
    }

    #[test]
    fn is_released_handles_missing_and_future_dates() {
        let now = Utc::now();
        assert!(is_released(None, None, now));
        assert!(is_released(Some("2020-01-01"), None, now));
        assert!(!is_released(Some("2999-01-01"), None, now));
        assert!(!is_released(
            None,
            Some(&(now + chrono::Duration::days(2)).to_rfc3339()),
            now
        ));
        assert!(is_released(
            None,
            Some(&(now - chrono::Duration::days(2)).to_rfc3339()),
            now
        ));
    }

    #[test]
    fn blocklist_matches_info_hash_and_guid_case_insensitively() {
        let mut blocklist = Blocklist::default();
        blocklist.info_hashes.insert("abc123".to_string());
        blocklist.guids.insert("https://x/1".to_string());

        let mut release = SourceRelease::new(
            "Show.S01E01".to_string(),
            "HTTPS://X/1".to_string(),
            Utc::now(),
        );
        assert!(blocklist.blocks(&release));

        release.guid = "https://x/2".to_string();
        assert!(!blocklist.blocks(&release));

        release.info_hash = Some("ABC123".to_string());
        assert!(blocklist.blocks(&release));
    }

    #[test]
    fn hunt_scope_targets_only_the_named_kind() {
        let show_scope = HuntScope {
            show_id: Some("show-1".to_string()),
            ..Default::default()
        };
        assert!(show_scope.wants_shows());
        assert!(!show_scope.wants_movies());
        assert!(!show_scope.wants_albums());
        assert!(!show_scope.wants_audiobooks());

        let library_scope = HuntScope::library(Some("lib-1".to_string()));
        assert!(library_scope.wants_shows());
        assert!(library_scope.wants_movies());
        assert!(library_scope.wants_albums());
        assert!(library_scope.wants_audiobooks());
    }

    #[test]
    fn year_from_date_reads_the_leading_year() {
        assert_eq!(year_from_date(Some("2019-04-01")), Some(2019));
        assert_eq!(year_from_date(Some("2019")), Some(2019));
        assert_eq!(year_from_date(Some("nope")), None);
        assert_eq!(year_from_date(None), None);
    }
}
