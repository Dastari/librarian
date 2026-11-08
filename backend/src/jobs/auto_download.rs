//! Auto-download candidate discovery and grab (the "hunt").
//!
//! For each of the four media types, finds items that should be hunted for,
//! builds one or more `SourceQuery` variants, searches all configured sources,
//! filters candidates against the item's resolved `QualityProfile`, ranks the
//! survivors with `services::quality::profile::filter_and_rank`, and grabs the
//! best release via `TorrentService::add_magnet_for_actor_with_outcome`, stamping the
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
//! Episodes are not searched the moment they air: they wait out a grace period for releases to
//! appear, are retried on a quick schedule for a few days, and then on the ordinary backoff (see
//! `auto_schedule`). Every candidate also carries an [`Expectation`], so a release for the next
//! episode, another season or another year's remake is never taken for the wanted item.
//!
//! Hunts never overlap (one process-wide lock covers every search-and-grab, including RSS grabs),
//! and no more than `auto_download.max_concurrent_grabs` automatic grabs are downloading at once.
//! Every search, grab and skip leaves a trail in Activity (`auto_trail`).
//!
//! Three behaviours keep the hunt from hammering indexers or re-grabbing junk:
//!
//! - **Season packs**: when a season is mostly missing and the profile allows
//!   packs, the pack is searched first ("Show S02" and "Show Season 2"); only
//!   if no pack is grabbed does the job fall back to per-episode searches.
//! - **Search backoff**: a target is not re-searched more often than the configured backoff
//!   (six hours by default). The timestamps live in the
//!   `auto_download.search_state` app setting so a restart does not reset
//!   them. Manual searches (`searchMissing`) ignore the backoff.
//! - **Release blocklist**: releases recorded in `ReleaseBlocklist` (added by
//!   the retry sweep for failed/stalled grabs) are never grabbed again.
//!
//! See `docs/tier1-features-plan.md` §1/§2 for the original design and
//! `docs/design.md`'s Media Pipeline Decision Guide (Q1, Q23, Q25, Q37, Q39,
//! Q50, Q52).

use crate::services::release_match;
use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
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
use crate::services::quality::scoring;
use crate::services::sources::{QueryType, SourceQuery, SourceRelease};
use crate::services::torrent::TorrentAddMetadata;

use super::auto_schedule::{
    Expectation, SearchState, Timing, backoff_elapsed, backoff_for, classify_episode,
    classify_release, grab_budget, next_search_at, release_instant,
};
use super::auto_settings::AutoDownloadSettings;
use super::auto_trail::{GrabRecord, Trail, Trigger};

/// App setting holding the per-target "last searched at" map (JSON object of
/// `target key -> RFC3339 timestamp`).
const SEARCH_STATE_SETTING: &str = "auto_download.search_state";

/// One search-and-grab at a time, process-wide: scheduled sweeps, the air-time pass, manual
/// searches, retries and RSS grabs all take this before deciding what to grab, so two of them can
/// never both pick a release for the same episode.
static HUNT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Hold the lock that keeps automatic grabs from overlapping.
pub async fn hunt_lock() -> tokio::sync::MutexGuard<'static, ()> {
    HUNT_LOCK.lock().await
}

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
    /// Where the target stands relative to its release: decides which backoff applies.
    pub timing: Timing,
    /// What a release must be to count as this target (right episode, season, year).
    pub expectation: Expectation,
    /// The show or movie title an RSS item has to be named after. `None` for music and
    /// audiobooks, which a feed cannot be matched to (releases are albums and whole books, not
    /// the per-track targets that are hunted).
    pub feed_name: Option<String>,
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
    /// Discovery for recommendations (`recommend`): every missing, un-ignored item counts, whether
    /// or not it is flagged wanted or its show is on auto-download, and nothing is marked wanted.
    /// Nothing is ever grabbed from such a pass.
    pub include_unwanted: bool,
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
    /// Left alone because they were searched within their backoff.
    pub backoff_skipped: usize,
    /// Left for later because the limit on automatic downloads was reached.
    pub capped: usize,
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

    #[cfg(test)]
    pub(crate) fn block_info_hash(&mut self, info_hash: &str) {
        self.info_hashes.insert(info_hash.to_ascii_lowercase());
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

pub(crate) async fn load_search_state(db: &Database) -> SearchState {
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

pub(crate) async fn save_search_state(db: &Database, state: &SearchState) -> Result<()> {
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

/// Timing of an item with a single release date (movies, albums, audiobooks): searchable from
/// that day on, always on the ordinary backoff. Movies carry only TMDB's primary release date, not
/// a digital one, so they get no grace period and no quick-retry window.
fn dated_timing(date: Option<&str>, now: DateTime<Utc>) -> Timing {
    classify_release(
        release_instant(date, None),
        now,
        Duration::zero(),
        Duration::zero(),
    )
}

fn not_yet(timing: Timing) -> bool {
    matches!(timing, Timing::NotYet { .. })
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
    timing: Timing,
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
        timing,
        expectation: Expectation::Episode {
            season: episode.season,
            episode: episode.episode,
        },
        feed_name: Some(show.name.clone()),
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
        timing: Timing::Back,
        expectation: Expectation::SeasonPack { season },
        feed_name: Some(show.name.clone()),
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
    settings: &AutoDownloadSettings,
    now: DateTime<Utc>,
) -> Result<Vec<WantedCandidate>> {
    let mut filter = ShowWhereInput::default();
    if let Some(library_id) = &scope.library_id {
        filter.library_id = Some(string_eq(library_id));
    }
    if let Some(show_id) = &scope.show_id {
        filter.id = Some(string_eq(show_id));
    }
    let shows = Show::query(db.pool()).filter(filter).fetch_all().await?;

    let mut out = Vec::new();

    for show in shows {
        let (enabled, mode) = normalize_auto_download(show.auto_download, show.auto_download_mode);
        if !enabled && !scope.include_unwanted {
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

        let timing_of = |episode: &Episode| {
            classify_episode(
                episode.air_date.as_deref(),
                episode.air_stamp.as_deref(),
                now,
                settings,
            )
        };

        // Missing = no file, not ignored, aired and past its grace period for releases to appear.
        let missing: Vec<Episode> = episodes
            .iter()
            .filter(|episode| {
                episode.media_file_id.is_none()
                    && !is_ignored(episode.ignored)
                    && !not_yet(timing_of(episode))
            })
            .cloned()
            .collect();

        // ALL mode: everything missing and aired counts as wanted, and the row
        // is updated so status reflects that.
        if mode == AutoDownloadMode::All && !scope.include_unwanted {
            for episode in missing.iter().filter(|episode| !episode.wanted) {
                mark_episode_wanted(db, episode, &show.name).await;
            }
        }

        let mut targets: Vec<Episode> = match mode {
            _ if scope.include_unwanted => missing.clone(),
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
                .filter(|episode| episode.season == season && !not_yet(timing_of(episode)))
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
                    timing_of(episode),
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
    now: DateTime<Utc>,
) -> Result<Vec<WantedCandidate>> {
    let mut filter = AlbumWhereInput::default();
    if let Some(library_id) = &scope.library_id {
        filter.library_id = Some(string_eq(library_id));
    }
    if let Some(album_id) = &scope.album_id {
        filter.id = Some(string_eq(album_id));
    }
    let albums = Album::query(db.pool()).filter(filter).fetch_all().await?;

    let mut out = Vec::new();

    for album in albums {
        let (enabled, mode) =
            normalize_auto_download(album.auto_download, album.auto_download_mode);
        if !enabled && !scope.include_unwanted {
            continue;
        }
        let timing = dated_timing(album.release_date.as_deref(), now);
        if not_yet(timing) {
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

        if mode == AutoDownloadMode::All && !scope.include_unwanted {
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
            _ if scope.include_unwanted => missing,
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
                timing,
                expectation: Expectation::Unchecked,
                feed_name: None,
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
    now: DateTime<Utc>,
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

    let mut out = Vec::new();

    for audiobook in audiobooks {
        let (enabled, mode) =
            normalize_auto_download(audiobook.auto_download, audiobook.auto_download_mode);
        if !enabled && !scope.include_unwanted {
            continue;
        }
        let timing = dated_timing(audiobook.published_date.as_deref(), now);
        if not_yet(timing) {
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

        if mode == AutoDownloadMode::All && !scope.include_unwanted {
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
            _ if scope.include_unwanted => !missing.is_empty(),
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

        let short = release_match::short_title(&audiobook.title);
        let term = match &audiobook.author_name {
            Some(author) => format!("{author} {short}"),
            None => short.clone(),
        };
        // Every spelling at each width: results are merged, so a wider term only adds choices, and
        // the ranking below still has to find a title close to what's wanted.
        let queries = release_match::book_search_ladder(
            &term,
            audiobook.author_name.as_deref(),
            &audiobook.title,
            audiobook.series_name.as_deref(),
        )
        .into_iter()
        .flat_map(|term| {
            let plain = release_match::without_apostrophes(&term);
            std::iter::once(term).chain(plain)
        })
        .map(|term| SourceQuery {
            query_type: QueryType::BookSearch,
            search_term: Some(term),
            title: Some(audiobook.title.clone()),
            author: audiobook.author_name.clone(),
            year: year_from_date(audiobook.published_date.as_deref()),
            cache: true,
            ..Default::default()
        })
        .collect();
        out.push(WantedCandidate {
            label: format!("audiobook '{term}'"),
            library_id: audiobook.library_id.clone(),
            match_title: short,
            queries,
            linkage: CandidateLinkage {
                audiobook_id: Some(audiobook.id.clone()),
                ..Default::default()
            },
            quality_profile_override_id: audiobook.quality_profile_id.clone(),
            search_key: format!("audiobook:{}", audiobook.id),
            episode_count: 1,
            timing,
            expectation: Expectation::Unchecked,
            feed_name: None,
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
    now: DateTime<Utc>,
) -> Result<Vec<WantedCandidate>> {
    let mut filter = MovieWhereInput {
        monitored: (!scope.include_unwanted).then(|| bool_eq(true)),
        wanted: (!scope.include_unwanted).then(|| bool_eq(true)),
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

    let mut out = Vec::new();

    for movie in movies {
        if is_ignored(movie.ignored) || movie.media_file_id.is_some() {
            continue;
        }
        let timing = dated_timing(movie.release_date.as_deref(), now);
        if not_yet(timing) {
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
            timing,
            expectation: Expectation::Movie {
                year: movie
                    .year
                    .or_else(|| year_from_date(movie.release_date.as_deref())),
            },
            feed_name: Some(movie.title.clone()),
            fallbacks: Vec::new(),
        });
    }
    Ok(out)
}

/// Everything the scope names that is wanted right now, with each target's timing and identity.
/// `now` and the settings are parameters so a pass is reproducible.
pub async fn discover_candidates(
    db: &Database,
    scope: &HuntScope,
    blocklist: &Blocklist,
    settings: &AutoDownloadSettings,
    now: DateTime<Utc>,
) -> Result<Vec<WantedCandidate>> {
    let mut out = Vec::new();
    if scope.wants_shows() {
        out.extend(discover_episode_candidates(db, scope, blocklist, settings, now).await?);
    }
    if scope.wants_albums() {
        out.extend(discover_track_candidates(db, scope, blocklist, now).await?);
    }
    if scope.wants_audiobooks() {
        out.extend(discover_audiobook_candidates(db, scope, blocklist, now).await?);
    }
    if scope.wants_movies() {
        out.extend(discover_movie_candidates(db, scope, blocklist, now).await?);
    }
    Ok(out)
}

// ===========================================================================
// Grabbing
// ===========================================================================

/// What a grab needs: the database, the torrent client, and the services manager (to fetch a
/// `.torrent` file through a source's login).
#[derive(Clone)]
pub struct GrabEnv {
    pub db: Database,
    pub torrent: Arc<crate::services::torrent::TorrentService>,
    pub manager: Arc<ServicesManager>,
}

/// A release that was added to the download client as a new record.
#[derive(Debug, Clone)]
pub struct GrabbedRelease {
    pub info_hash: String,
    pub torrent_name: String,
    pub torrent_id: Option<String>,
}

/// Add `release` to the download client for `candidate`, stamping the wanted-target linkage and the
/// tracker's seeding floors. `Ok(None)` means the client already had that torrent.
///
/// A release with no magnet is fetched through its source first, as the manual add does: some
/// trackers only hand a `.torrent` file to a logged-in client and the torrent client has no login.
pub async fn grab_release(
    env: &GrabEnv,
    candidate: &WantedCandidate,
    release: &SourceRelease,
    feed_id: Option<&str>,
) -> Result<Option<GrabbedRelease>> {
    grab_release_with(env, candidate, release, feed_id, None).await
}

/// [`grab_release`] for a pack that should download only the files filling what is missing.
pub async fn grab_release_with(
    env: &GrabEnv,
    candidate: &WantedCandidate,
    release: &SourceRelease,
    feed_id: Option<&str>,
    wanted_files: Option<crate::services::torrent::partial::WantedFiles>,
) -> Result<Option<GrabbedRelease>> {
    let uri = release
        .magnet_uri
        .clone()
        .or_else(|| release.link.clone())
        .context("the release has neither a magnet URI nor a download link")?;

    let metadata = TorrentAddMetadata {
        library_id: Some(candidate.library_id.clone()),
        // A feed's guid is sometimes its download link, which carries the account's passkey.
        source_url: release
            .details
            .clone()
            .or_else(|| Some(release.guid.clone()))
            .map(|url| crate::services::redact::redact_url(&url, false)),
        source_indexer_id: release.source_id.clone(),
        source_feed_id: feed_id.map(ToString::to_string),
        episode_id: candidate.linkage.episode_id.clone(),
        movie_id: candidate.linkage.movie_id.clone(),
        track_id: candidate.linkage.track_id.clone(),
        chapter_id: candidate.linkage.chapter_id.clone(),
        show_id: candidate.linkage.show_id.clone(),
        album_id: candidate.linkage.album_id.clone(),
        audiobook_id: candidate.linkage.audiobook_id.clone(),
        wanted_files,
    };

    let file = match (release.magnet_uri.as_deref(), release.source_id.as_deref()) {
        (None, Some(source_id)) => crate::services::graphql::entities::fetch_torrent_from_source(
            &env.manager,
            source_id,
            &uri,
        )
        .await
        .map_err(redacted_error)?,
        _ => None,
    };

    let actor = crate::services::torrent::system_auth_user();
    let outcome = match file {
        Some(bytes) => {
            env.torrent
                .add_torrent_file_for_actor_with_outcome(bytes, &actor, metadata)
                .await
        }
        None => {
            env.torrent
                .add_magnet_for_actor_with_outcome(&uri, &actor, metadata)
                .await
        }
    }
    .map_err(redacted_error)?;
    if !outcome.created {
        tracing::debug!(
            candidate = %candidate.label,
            info_hash = %outcome.info.info_hash,
            "Auto-download: the download client already has this torrent; grab details unchanged"
        );
        return Ok(None);
    }
    let info = outcome.info;
    // Whatever was recommended for this target is settled now.
    super::recommend::forget_for_linkage(&env.db, &candidate.linkage).await;

    // Season (for packs) and the tracker's seeding floors are not carried by
    // `TorrentAddMetadata`; stamp them on the row the torrent service just created.
    let details = TorrentGrabDetails::from_release_limits(
        candidate.linkage.season,
        release.minimum_ratio,
        release.minimum_seed_time,
    );
    if let Err(error) = stamp_torrent_grab_details(&env.db, &info.info_hash, details).await {
        warn!(
            candidate = %candidate.label,
            info_hash = %info.info_hash,
            error = %error,
            "Auto-download: grabbed a release but could not stamp its grab details"
        );
    }
    let torrent_id = Torrent::query(env.db.pool())
        .filter(TorrentWhereInput {
            info_hash: Some(string_eq(&info.info_hash)),
            ..Default::default()
        })
        .fetch_all()
        .await
        .ok()
        .and_then(|rows| rows.into_iter().next())
        .map(|row| row.id);
    Ok(Some(GrabbedRelease {
        info_hash: info.info_hash,
        torrent_name: info.name,
        torrent_id,
    }))
}

/// An error with any address in its text redacted: a failed grab of a feed item quotes the link,
/// which holds the account's passkey, and the message reaches logs and notifications.
fn redacted_error(error: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!(crate::services::redact::redact_text(&format!("{error:#}")))
}

/// Whether a torrent still counts against the cap on automatic grabs: it is linked to a wanted
/// target, has not finished and has not failed.
pub fn counts_as_in_flight(torrent: &Torrent) -> bool {
    super::download_monitor::has_wanted_linkage(torrent)
        && torrent.completed_at.is_none()
        && torrent.post_process_status.as_deref() != Some("failed")
}

/// How many automatic grabs are downloading right now.
pub async fn count_in_flight(db: &Database) -> Result<usize> {
    let rows = Torrent::query(db.pool())
        .filter(TorrentWhereInput {
            completed_at: Some(graphql_orm::graphql::filters::DateFilter {
                is_null: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await
        .context("failed to count automatic downloads in progress")?;
    Ok(rows
        .iter()
        .filter(|torrent| counts_as_in_flight(torrent))
        .count())
}

// ===========================================================================
// Search + decide
// ===========================================================================

struct HuntContext {
    env: GrabEnv,
    sources: Arc<crate::services::sources::manager::SourcesManager>,
    blocklist: Blocklist,
    trail: Trail,
    trigger: Trigger,
    /// Automatic grabs that may still start in this pass.
    budget: usize,
}

/// What searching the sources for one candidate turned up.
#[derive(Default, Clone)]
pub struct Collected {
    pub releases: Vec<SourceRelease>,
    /// Distinct releases returned, blocklisted ones included.
    pub seen: usize,
    pub blocklisted: usize,
    /// How many sources could take the search.
    pub sources: usize,
    pub errors: Vec<String>,
}

/// Run every query variant for a candidate and merge the results,
/// de-duplicating on info hash (falling back to guid) and dropping
/// blocklisted releases.
async fn collect_releases(ctx: &HuntContext, candidate: &WantedCandidate) -> Collected {
    collect_releases_from(&ctx.sources, &ctx.blocklist, candidate).await
}

/// [`collect_releases`] without a hunt around it: the recommender searches the same way.
pub async fn collect_releases_from(
    sources: &crate::services::sources::manager::SourcesManager,
    blocklist: &Blocklist,
    candidate: &WantedCandidate,
) -> Collected {
    let mut seen: HashSet<String> = HashSet::new();
    let mut collected = Collected::default();
    for query in &candidate.queries {
        let results = sources.search_all(query).await;
        collected.sources = collected.sources.max(results.len());
        for result in results {
            if let Some(error) = &result.error {
                let error = format!("{}: {error}", result.source_name);
                if !collected.errors.contains(&error) {
                    collected.errors.push(error);
                }
            }
            for release in result.releases {
                // Only a person can sort out a pack, an ebook is never what was wanted, and neither
                // is another book of the series.
                if candidate.linkage.audiobook_id.is_some()
                    && (release_match::book_pack_hint(&release.title).is_some()
                        || query.title.as_deref().is_some_and(|title| {
                            release_match::release_relevance(
                                &release.title,
                                &release_match::BookFacts {
                                    title: title.to_string(),
                                    author: query.author.clone(),
                                    ..Default::default()
                                },
                                None,
                            )
                            .1
                            .is_some()
                        })
                        || release_match::classify_book_release(
                            &release.title,
                            &release.categories,
                            release.size,
                            None,
                        )
                        .0 == release_match::ReleaseFormat::Ebook)
                {
                    continue;
                }
                let key = release
                    .info_hash
                    .clone()
                    .unwrap_or_else(|| release.guid.clone())
                    .to_ascii_lowercase();
                if !seen.insert(key) {
                    continue;
                }
                collected.seen += 1;
                if blocklist.blocks(&release) {
                    collected.blocklisted += 1;
                    tracing::debug!(
                        candidate = %candidate.label,
                        release = %release.title,
                        "Auto-download: skipping blocklisted release"
                    );
                    continue;
                }
                collected.releases.push(release);
            }
        }
    }
    collected
}

/// Releases that are what the candidate asked for, plus how many were not and one example why.
/// A search can return the neighbours of what was asked for (the next episode, last year's remake).
pub fn verify_releases(
    candidate: &WantedCandidate,
    releases: Vec<SourceRelease>,
    strict: bool,
) -> (Vec<SourceRelease>, usize, Option<String>) {
    let mut kept = Vec::new();
    let mut wrong = 0;
    let mut example = None;
    for release in releases {
        let parsed = scoring::parse_release(&release.title);
        let mismatch = candidate
            .expectation
            .mismatch(&parsed, strict)
            .or_else(|| candidate.expectation.title_mismatch(&release.title, strict));
        match mismatch {
            None => kept.push(release),
            Some(reason) => {
                wrong += 1;
                example.get_or_insert_with(|| format!("'{}' is {reason}", release.title));
            }
        }
    }
    (kept, wrong, example)
}

/// Everything known about why a search took nothing.
#[derive(Debug, Default, Clone)]
pub struct NoGrabFacts {
    pub sources: usize,
    pub errors: Vec<String>,
    pub seen: usize,
    pub blocklisted: usize,
    pub wrong_target: usize,
    pub wrong_example: Option<String>,
    pub too_different: usize,
    pub below_profile: usize,
    pub profile: String,
    pub profile_reasons: Vec<String>,
}

/// One line saying why nothing was grabbed: no release, below the profile, blocklisted, or the
/// wrong episode/season/year.
pub fn describe_no_grab(facts: &NoGrabFacts) -> String {
    if facts.sources == 0 {
        let mut text = "no source can take this search; add or enable one in Settings".to_string();
        if !facts.errors.is_empty() {
            text = format!("{text} ({})", facts.errors.join("; "));
        }
        return text;
    }
    if facts.seen == 0 {
        return if facts.errors.is_empty() {
            format!("no release found on {} source(s)", facts.sources)
        } else {
            format!(
                "no release found; {} source error(s): {}",
                facts.errors.len(),
                facts.errors.join("; ")
            )
        };
    }
    let mut parts = Vec::new();
    if facts.blocklisted > 0 {
        parts.push(format!("{} blocklisted", facts.blocklisted));
    }
    if facts.wrong_target > 0 {
        parts.push(format!(
            "{} for something else{}",
            facts.wrong_target,
            facts
                .wrong_example
                .as_deref()
                .map(|example| format!(" (e.g. {example})"))
                .unwrap_or_default()
        ));
    }
    if facts.too_different > 0 {
        parts.push(format!(
            "{} with a title too different",
            facts.too_different
        ));
    }
    if facts.below_profile > 0 {
        parts.push(format!(
            "{} below the '{}' profile{}",
            facts.below_profile,
            facts.profile,
            if facts.profile_reasons.is_empty() {
                String::new()
            } else {
                format!(" ({})", facts.profile_reasons.join("; "))
            }
        ));
    }
    format!(
        "{} release(s) found, none usable: {}",
        facts.seen,
        parts.join(", ")
    )
}

/// Why each release the profile turned down was turned down: how many were too different in title,
/// how many were below the profile, and the most common reasons for the latter.
pub fn explain_profile_rejections(
    match_title: &str,
    releases: &[SourceRelease],
    quality_profile: &QualityProfile,
    episode_count: i32,
) -> (usize, usize, Vec<String>) {
    let mut too_different = 0;
    let mut below = 0;
    let mut tally: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for release in releases {
        if scoring::title_similarity(match_title, &release.title) < scoring::MIN_TITLE_SIMILARITY {
            too_different += 1;
            continue;
        }
        let parsed = scoring::parse_release(&release.title);
        let facts = profile::ReleaseFacts::from_release(release, episode_count);
        let (verdict, reasons) = profile::match_release(&parsed, quality_profile, &facts);
        if verdict == profile::ProfileMatch::Rejected {
            below += 1;
            for reason in reasons {
                *tally.entry(reason).or_default() += 1;
            }
        }
    }
    let mut ranked: Vec<(String, usize)> = tally.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let reasons = ranked
        .into_iter()
        .take(2)
        .map(|(reason, _)| reason)
        .collect();
    (too_different, below, reasons)
}

async fn resolve_candidate_profile(
    ctx: &HuntContext,
    candidate: &WantedCandidate,
) -> QualityProfile {
    match profile::resolve_profile(
        &ctx.env.db,
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

/// Whether a search with no result opens a "Waiting for …" notification. Everything does except the
/// long tail, back-catalogue episodes and individual tracks (a manual search always does): a first
/// sweep of a show with two hundred missing old episodes must not open two hundred notifications.
fn opens_notification(candidate: &WantedCandidate, trigger: &Trigger) -> bool {
    let long_tail = candidate.linkage.track_id.is_some()
        || (candidate.timing == Timing::Back
            && matches!(
                candidate.expectation,
                Expectation::Episode { .. } | Expectation::SeasonPack { .. }
            ));
    *trigger == Trigger::Manual || !long_tail
}

/// The release a candidate would be taken as, from what was collected: verified to be the right
/// episode, season or year, then filtered and ranked by the quality profile. `strict` refuses a title
/// that does not say which episode or year it is (feeds and season-wide results); a search narrowed to
/// one episode may keep one that does not. On `Err`, everything known about why nothing was taken.
///
/// The hunt grabs what this returns and the recommender only shows it, so a recommendation is what
/// automatic downloading would take.
pub fn pick_for_candidate(
    candidate: &WantedCandidate,
    collected: &Collected,
    quality_profile: &QualityProfile,
    strict: bool,
) -> Result<scoring::ScoredRelease, NoGrabFacts> {
    rank_for_candidate(candidate, collected, quality_profile, strict).map(|ranked| {
        ranked
            .into_iter()
            .next()
            .expect("a successful ranking is never empty")
    })
}

/// Every release the profile accepts for a candidate, best first. [`pick_for_candidate`] is its
/// first. The recommender looks further down the list (the smallest release at the best quality).
pub fn rank_for_candidate(
    candidate: &WantedCandidate,
    collected: &Collected,
    quality_profile: &QualityProfile,
    strict: bool,
) -> Result<Vec<scoring::ScoredRelease>, NoGrabFacts> {
    let (verified, wrong_target, wrong_example) =
        verify_releases(candidate, collected.releases.clone(), strict);
    let ranked = profile::filter_and_rank(
        &candidate.match_title,
        verified.clone(),
        quality_profile,
        candidate.episode_count,
    );
    if !ranked.is_empty() {
        return Ok(ranked);
    }
    let (too_different, below_profile, profile_reasons) = explain_profile_rejections(
        &candidate.match_title,
        &verified,
        quality_profile,
        candidate.episode_count,
    );
    Err(NoGrabFacts {
        sources: collected.sources,
        errors: collected.errors.clone(),
        seen: collected.seen,
        blocklisted: collected.blocklisted,
        wrong_target,
        wrong_example,
        too_different,
        below_profile,
        profile: quality_profile.name.clone(),
        profile_reasons,
    })
}

/// Search for and grab the best release for one candidate.
/// Returns `Ok(true)` when a release was grabbed.
async fn try_candidate(
    ctx: &mut HuntContext,
    candidate: &WantedCandidate,
    summary: &mut AutoDownloadRunSummary,
    next_search: Option<DateTime<Utc>>,
) -> Result<bool> {
    summary.searched += 1;
    let collected = collect_releases(ctx, candidate).await;
    let quality_profile = resolve_candidate_profile(ctx, candidate).await;

    // A search narrowed by season and episode may return a title that does not say which episode
    // it is; one that names another episode is never accepted.
    let best = match pick_for_candidate(candidate, &collected, &quality_profile, false) {
        Ok(best) => best,
        Err(facts) => {
            let why = describe_no_grab(&facts);
            info!(
                candidate = %candidate.label,
                trigger = ctx.trigger.tag(),
                profile = %quality_profile.name,
                "Auto-download: searched for {}, nothing taken: {why}",
                candidate.label
            );
            ctx.trail
                .waiting(
                    &candidate.library_id,
                    &candidate.label,
                    &why,
                    next_search,
                    &ctx.trigger,
                    opens_notification(candidate, &ctx.trigger),
                )
                .await;
            return Ok(false);
        }
    };

    let release = best.release;
    if release.magnet_uri.is_none() && release.link.is_none() {
        warn!(
            candidate = %candidate.label,
            release = %release.title,
            "Auto-download: best release had neither a magnet URI nor a download link; skipping"
        );
        ctx.trail
            .waiting(
                &candidate.library_id,
                &candidate.label,
                &format!("'{}' has no magnet or download link", release.title),
                next_search,
                &ctx.trigger,
                true,
            )
            .await;
        return Ok(false);
    }

    match grab_release(&ctx.env, candidate, &release, None).await {
        Ok(Some(grabbed)) => {
            summary.grabbed += 1;
            ctx.budget = ctx.budget.saturating_sub(1);
            info!(
                candidate = %candidate.label,
                trigger = ctx.trigger.tag(),
                torrent = %grabbed.torrent_name,
                release = %release.title,
                source = release.source_name.as_deref().unwrap_or("unknown"),
                seeders = release.seeders.unwrap_or(0),
                profile = %quality_profile.name,
                "Auto-download: grabbed release '{}' for {}",
                release.title,
                candidate.label
            );
            ctx.trail
                .grabbed(&GrabRecord {
                    library_id: &candidate.library_id,
                    label: &candidate.label,
                    release: &release.title,
                    source: release.source_name.as_deref(),
                    profile: &quality_profile.name,
                    trigger: &ctx.trigger,
                    torrent_id: grabbed.torrent_id.as_deref(),
                })
                .await;
            Ok(true)
        }
        Ok(None) => {
            ctx.trail
                .waiting(
                    &candidate.library_id,
                    &candidate.label,
                    &format!("'{}' is already in the download client", release.title),
                    next_search,
                    &ctx.trigger,
                    true,
                )
                .await;
            Ok(false)
        }
        Err(error) => {
            warn!(
                error = %error,
                candidate = %candidate.label,
                release = %release.title,
                "Auto-download: grab of '{}' for {} failed",
                release.title,
                candidate.label
            );
            summary
                .errors
                .push(format!("{}: {}", candidate.label, error));
            ctx.trail
                .grab_failed(
                    &candidate.library_id,
                    &candidate.label,
                    &release.title,
                    &error.to_string(),
                )
                .await;
            Ok(false)
        }
    }
}

/// The candidates the air-time pass looks at: targets still on the quick-retry schedule. A season
/// pack is left to the full sweep, but its per-episode fallbacks that are fresh are searched.
pub fn fresh_targets(candidates: Vec<WantedCandidate>) -> Vec<WantedCandidate> {
    let mut out = Vec::new();
    for candidate in candidates {
        if candidate.timing == Timing::Fresh {
            out.push(candidate);
        } else {
            out.extend(
                candidate
                    .fallbacks
                    .into_iter()
                    .filter(|fallback| fallback.timing == Timing::Fresh),
            );
        }
    }
    out
}

/// Run one hunt pass over `scope`, on behalf of a manual search.
pub async fn run_scoped(
    manager: &Arc<ServicesManager>,
    scope: &HuntScope,
) -> Result<AutoDownloadRunSummary> {
    run_pass(manager, scope, Trigger::Manual, false).await
}

/// Run one hunt pass.
///
/// `fresh_only` is the air-time pass: it looks only at episodes inside their quick-retry window.
/// Whoever calls this decides whether automatic downloading is on; the pass itself does not.
pub async fn run_pass(
    manager: &Arc<ServicesManager>,
    scope: &HuntScope,
    trigger: Trigger,
    fresh_only: bool,
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

    // One search-and-grab at a time, so two passes cannot both pick a release for one episode.
    let _hunting = hunt_lock().await;

    let settings = AutoDownloadSettings::load(&db).await;
    let blocklist = Blocklist::load(&db).await.unwrap_or_else(|error| {
        warn!(error = %error, "Auto-download: failed to load the release blocklist; continuing without it");
        Blocklist::default()
    });

    let now = Utc::now();
    let mut candidates = discover_candidates(&db, scope, &blocklist, &settings, now).await?;
    if fresh_only {
        candidates = fresh_targets(candidates);
    }
    let mut summary = AutoDownloadRunSummary {
        candidates_considered: candidates.len(),
        ..Default::default()
    };
    if candidates.is_empty() {
        return Ok(summary);
    }

    // A person pressing Search on one title is not held to the cap; everything automatic is.
    let exempt_from_cap = scope.ignore_backoff && trigger == Trigger::Manual;
    let in_flight = count_in_flight(&db).await.unwrap_or(0);
    let budget = if exempt_from_cap {
        usize::MAX
    } else {
        grab_budget(in_flight, settings.max_concurrent_grabs)
    };

    let mut state = load_search_state(&db).await;
    let now_iso = now.to_rfc3339();

    info!(
        count = summary.candidates_considered,
        trigger = trigger.tag(),
        in_flight,
        cap = settings.max_concurrent_grabs,
        blocklist_active = !blocklist.is_empty(),
        scope_library = scope.library_id.as_deref().unwrap_or("all"),
        "Auto-download: found wanted candidates to search"
    );

    let mut ctx = HuntContext {
        env: GrabEnv {
            db: db.clone(),
            torrent: torrent_service,
            manager: manager.clone(),
        },
        sources: sources_manager,
        blocklist,
        trail: Trail::new(&db),
        trigger,
        budget,
    };

    let mut libraries: HashSet<String> = HashSet::new();
    let mut capped: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for candidate in candidates {
        libraries.insert(candidate.library_id.clone());

        if ctx.budget == 0 {
            // Not searched, so no backoff is spent: it goes first once a slot is free.
            summary.capped += 1;
            *capped.entry(candidate.library_id.clone()).or_default() += 1;
            tracing::debug!(
                candidate = %candidate.label,
                "Auto-download: not searching {}, the limit on automatic downloads is reached",
                candidate.label
            );
            continue;
        }

        let mut grabbed = false;
        if due(&state, &candidate, scope, &settings, now) {
            let backoff = backoff_for(candidate.timing, &settings);
            state.insert(candidate.search_key.clone(), now_iso.clone());
            grabbed = try_candidate(
                &mut ctx,
                &candidate,
                &mut summary,
                next_search_at(&state, &candidate.search_key, backoff),
            )
            .await?;
        } else {
            summary.backoff_skipped += 1;
            tracing::debug!(
                candidate = %candidate.label,
                "Auto-download: not searching {} yet, it was searched recently (next after {:?})",
                candidate.label,
                next_search_at(
                    &state,
                    &candidate.search_key,
                    backoff_for(candidate.timing, &settings)
                )
            );
        }
        if grabbed {
            continue;
        }

        // Season-pack fallback: one level only, and each fallback carries its own backoff key so a
        // failed pack search does not burn the per-episode budget, and a pack still inside its
        // backoff does not hold the per-episode searches back.
        for fallback in &candidate.fallbacks {
            if ctx.budget == 0 {
                summary.capped += 1;
                *capped.entry(fallback.library_id.clone()).or_default() += 1;
                continue;
            }
            if !due(&state, fallback, scope, &settings, now) {
                summary.backoff_skipped += 1;
                continue;
            }
            let backoff = backoff_for(fallback.timing, &settings);
            state.insert(fallback.search_key.clone(), now_iso.clone());
            let _ = try_candidate(
                &mut ctx,
                fallback,
                &mut summary,
                next_search_at(&state, &fallback.search_key, backoff),
            )
            .await?;
        }
    }

    if summary.capped > 0 {
        warn!(
            capped = summary.capped,
            in_flight,
            cap = settings.max_concurrent_grabs,
            "Auto-download: the limit on automatic downloads was reached; {} wanted item(s) wait for a free slot",
            summary.capped
        );
        for (library_id, waiting) in &capped {
            ctx.trail
                .cap_reached(
                    library_id,
                    in_flight,
                    settings.max_concurrent_grabs,
                    *waiting,
                )
                .await;
        }
    } else {
        for library_id in &libraries {
            ctx.trail.cap_cleared(library_id).await;
        }
    }

    if let Err(error) = save_search_state(&db, &state).await {
        warn!(error = %error, "Auto-download: failed to persist search backoff state");
    }

    Ok(summary)
}

/// Whether a candidate may be searched in this pass.
fn due(
    state: &SearchState,
    candidate: &WantedCandidate,
    scope: &HuntScope,
    settings: &AutoDownloadSettings,
    now: DateTime<Utc>,
) -> bool {
    scope.ignore_backoff
        || backoff_elapsed(
            state,
            &candidate.search_key,
            now,
            backoff_for(candidate.timing, settings),
        )
}

/// Run one full pass over the library (or one library) for a person who asked for it. Respects the
/// backoff and the cap like the scheduled sweep, but runs whether or not automatic downloading is on.
pub async fn run_once(
    manager: &Arc<ServicesManager>,
    library_id: Option<&str>,
) -> Result<AutoDownloadRunSummary> {
    run_pass(
        manager,
        &HuntScope::library(library_id.map(ToString::to_string)),
        Trigger::Manual,
        false,
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

#[cfg(test)]
#[path = "auto_download_tests.rs"]
mod discovery_tests;
