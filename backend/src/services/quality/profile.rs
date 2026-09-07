//! Quality profile evaluation (design.md Q23/Q26) and profile resolution.
//!
//! `evaluate` implements the Phase 4 "Quality Evaluation" decision guide:
//! compare a parsed release/media file against a [`QualityProfile`]'s allowed
//! lists and return `Optimal` or `Suboptimal(reasons)`. An profile with every
//! restriction empty (and `require_hdr = false`) `allows_any()` and is always
//! `Optimal` (Q26).
//!
//! Profile *resolution* (which profile applies to a given media file) follows
//! the precedence documented in `docs/tier1-features-plan.md` §2: per-entity
//! `quality_profile_id` override (Show/Movie/Album/Audiobook) wins, else the
//! owning `Library.quality_profile_id`, else the seeded default profile
//! (`is_default = true`).

use anyhow::{Context, Result};
use graphql_orm::graphql::filters::StringFilter;

use crate::db::Database;
use crate::graphql::entities::{
    Album, Audiobook, Chapter, Episode, Library, MediaFile, MediaKind, Movie, QualityProfile, Show,
    Track,
};

use super::scoring::{self, ParsedRelease};
use crate::services::sources::SourceRelease;

/// Result of evaluating a parsed release/media file against a [`QualityProfile`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualityEvaluation {
    Optimal,
    Suboptimal(Vec<String>),
}

impl QualityEvaluation {
    pub fn is_optimal(&self) -> bool {
        matches!(self, Self::Optimal)
    }

    /// The value stored in `MediaFile.quality_status`.
    pub fn status_str(&self) -> &'static str {
        if self.is_optimal() {
            "optimal"
        } else {
            "suboptimal"
        }
    }

    pub fn reasons(&self) -> &[String] {
        match self {
            Self::Optimal => &[],
            Self::Suboptimal(reasons) => reasons,
        }
    }
}

fn string_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

/// Q26: a profile with every restriction empty (and HDR not required) allows
/// anything — always `Optimal`.
pub fn allows_any(profile: &QualityProfile) -> bool {
    profile.allowed_resolutions.is_empty()
        && profile.allowed_video_codecs.is_empty()
        && profile.allowed_audio_formats.is_empty()
        && profile.allowed_hdr_types.is_empty()
        && profile.allowed_sources.is_empty()
        && profile.release_group_blacklist.is_empty()
        && profile.release_group_whitelist.is_empty()
        && !profile.require_hdr
        && (!profile.require_language_match || profile.preferred_languages.is_empty())
        && profile.allow_season_packs
}

fn contains_ci(list: &[String], value: &str) -> bool {
    list.iter().any(|item| item.eq_ignore_ascii_case(value))
}

/// Evaluate a parsed release/media file against a [`QualityProfile`] (Q23).
pub fn evaluate(parsed: &ParsedRelease, profile: &QualityProfile) -> QualityEvaluation {
    if allows_any(profile) {
        return QualityEvaluation::Optimal;
    }

    let mut reasons = Vec::new();

    if !profile.allowed_resolutions.is_empty() {
        match &parsed.resolution {
            Some(res) if contains_ci(&profile.allowed_resolutions, res) => {}
            other => reasons.push(format!(
                "resolution {} is not in the allowed list ({})",
                other.as_deref().unwrap_or("unknown"),
                profile.allowed_resolutions.join(", ")
            )),
        }
    }

    if profile.require_hdr && !parsed.is_hdr {
        reasons.push("HDR is required but the source is not HDR".to_string());
    }

    if !profile.allowed_hdr_types.is_empty() && parsed.is_hdr {
        match &parsed.hdr_type {
            Some(t) if contains_ci(&profile.allowed_hdr_types, t) => {}
            other => reasons.push(format!(
                "HDR type {} is not in the allowed list ({})",
                other.as_deref().unwrap_or("unknown"),
                profile.allowed_hdr_types.join(", ")
            )),
        }
    }

    if !profile.allowed_sources.is_empty() {
        match &parsed.source_type {
            Some(s) if contains_ci(&profile.allowed_sources, s) => {}
            other => reasons.push(format!(
                "source {} is not in the allowed list ({})",
                other.as_deref().unwrap_or("unknown"),
                profile.allowed_sources.join(", ")
            )),
        }
    }

    if !profile.allowed_video_codecs.is_empty() {
        match &parsed.codec {
            Some(c) if contains_ci(&profile.allowed_video_codecs, c) => {}
            other => reasons.push(format!(
                "video codec {} is not in the allowed list ({})",
                other.as_deref().unwrap_or("unknown"),
                profile.allowed_video_codecs.join(", ")
            )),
        }
    }

    if !profile.allowed_audio_formats.is_empty() {
        match &parsed.audio {
            Some(a) if contains_ci(&profile.allowed_audio_formats, a) => {}
            other => reasons.push(format!(
                "audio format {} is not in the allowed list ({})",
                other.as_deref().unwrap_or("unknown"),
                profile.allowed_audio_formats.join(", ")
            )),
        }
    }

    if !profile.release_group_blacklist.is_empty()
        && let Some(group) = &parsed.release_group
        && contains_ci(&profile.release_group_blacklist, group)
    {
        reasons.push(format!("release group {group} is blacklisted"));
    }

    if !profile.release_group_whitelist.is_empty() {
        let ok = parsed
            .release_group
            .as_ref()
            .is_some_and(|group| contains_ci(&profile.release_group_whitelist, group));
        if !ok {
            reasons.push(format!(
                "release group {} is not in the whitelist ({})",
                parsed.release_group.as_deref().unwrap_or("unknown"),
                profile.release_group_whitelist.join(", ")
            ));
        }
    }

    if profile.require_language_match
        && !profile.preferred_languages.is_empty()
        && !parsed.matches_language(&profile.preferred_languages)
    {
        reasons.push(format!(
            "language {} is not in the preferred list ({})",
            parsed.effective_languages().join(", "),
            profile.preferred_languages.join(", ")
        ));
    }

    if !profile.allow_season_packs && parsed.is_season_pack {
        reasons.push("season packs are not allowed by this profile".to_string());
    }

    if reasons.is_empty() {
        QualityEvaluation::Optimal
    } else {
        QualityEvaluation::Suboptimal(reasons)
    }
}

// ============================================================================
// Release-level evaluation + ranking (the auto-download hunt)
// ============================================================================

/// How a candidate release relates to a [`QualityProfile`].
///
/// - `Rejected`: fails a hard filter and must never be grabbed.
/// - `Suboptimal`: grabbable, but does not satisfy every soft preference.
/// - `Optimal`: satisfies the hard filters and every soft preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileMatch {
    Optimal,
    Suboptimal,
    Rejected,
}

impl ProfileMatch {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Optimal => "optimal",
            Self::Suboptimal => "suboptimal",
            Self::Rejected => "rejected",
        }
    }
}

/// The parts of a release a profile checks that cannot be read off the title:
/// size, seeders and publish date, plus how many episodes the release is
/// expected to satisfy (used to interpret size limits per episode for season
/// packs).
#[derive(Debug, Clone)]
pub struct ReleaseFacts {
    pub size_bytes: Option<i64>,
    pub seeders: Option<i32>,
    pub publish_date: Option<chrono::DateTime<chrono::Utc>>,
    /// Episodes covered by this release; `1` for movies, books, albums and
    /// single episodes. Season packs pass the season's episode count so
    /// `min_size_mb`/`max_size_mb` are compared per episode.
    pub episode_count: i32,
    pub now: chrono::DateTime<chrono::Utc>,
}

impl Default for ReleaseFacts {
    fn default() -> Self {
        Self {
            size_bytes: None,
            seeders: None,
            publish_date: None,
            episode_count: 1,
            now: chrono::Utc::now(),
        }
    }
}

impl ReleaseFacts {
    pub fn from_release(release: &SourceRelease, episode_count: i32) -> Self {
        Self {
            size_bytes: release.size,
            seeders: release.seeders,
            publish_date: Some(release.publish_date),
            episode_count: episode_count.max(1),
            ..Default::default()
        }
    }
}

const BYTES_PER_MB: i64 = 1024 * 1024;

/// Hard filters that need the release's own metadata rather than its title.
/// Returned reasons are worded for the `searchSources` `rejectReasons` field
/// and for debug logging in the hunt job.
fn release_metadata_reasons(profile: &QualityProfile, facts: &ReleaseFacts) -> Vec<String> {
    let mut reasons = Vec::new();

    // Sources that do not report seeders (usenet, plain RSS) are not filtered
    // on seeders at all — a missing count is "unknown", not "zero".
    if let Some(seeders) = facts.seeders
        && seeders < profile.min_seeders
    {
        reasons.push(format!(
            "{seeders} seeders is below the {} required by this profile",
            profile.min_seeders
        ));
    }

    if let Some(size) = facts.size_bytes {
        let per_item_mb = size / facts.episode_count.max(1) as i64 / BYTES_PER_MB;
        if let Some(min) = profile.min_size_mb
            && per_item_mb < min as i64
        {
            reasons.push(format!("{per_item_mb} MB is below the {min} MB minimum"));
        }
        if let Some(max) = profile.max_size_mb
            && per_item_mb > max as i64
        {
            reasons.push(format!("{per_item_mb} MB is above the {max} MB maximum"));
        }
    }

    if let Some(max_age) = profile.max_release_age_days
        && let Some(published) = facts.publish_date
    {
        let age_days = facts.now.signed_duration_since(published).num_days();
        if age_days > max_age as i64 {
            reasons.push(format!(
                "released {age_days} days ago, older than the {max_age} day limit"
            ));
        }
    }

    reasons
}

/// Full hard-filter evaluation of a candidate release: the title-derived
/// checks from [`evaluate`] plus the metadata checks (seeders/size/age).
/// Anything with a reason must not be auto-grabbed.
pub fn evaluate_release(
    parsed: &ParsedRelease,
    profile: &QualityProfile,
    facts: &ReleaseFacts,
) -> QualityEvaluation {
    let mut reasons: Vec<String> = evaluate(parsed, profile).reasons().to_vec();
    reasons.extend(release_metadata_reasons(profile, facts));
    if reasons.is_empty() {
        QualityEvaluation::Optimal
    } else {
        QualityEvaluation::Suboptimal(reasons)
    }
}

/// Soft (ranking-only) preferences a release does not satisfy. These never
/// reject a release; they only explain why it is `suboptimal`.
fn soft_preference_reasons(parsed: &ParsedRelease, profile: &QualityProfile) -> Vec<String> {
    let mut reasons = Vec::new();

    if !profile.resolution_preference.is_empty() {
        let matched = parsed
            .resolution
            .as_ref()
            .is_some_and(|res| contains_ci(&profile.resolution_preference, res));
        if !matched {
            reasons.push(format!(
                "resolution {} is not in the preference order ({})",
                parsed.resolution.as_deref().unwrap_or("unknown"),
                profile.resolution_preference.join(", ")
            ));
        }
    }

    if !profile.preferred_release_groups.is_empty() {
        let matched = parsed
            .release_group
            .as_ref()
            .is_some_and(|group| contains_ci(&profile.preferred_release_groups, group));
        if !matched {
            reasons.push(format!(
                "release group {} is not a preferred group ({})",
                parsed.release_group.as_deref().unwrap_or("unknown"),
                profile.preferred_release_groups.join(", ")
            ));
        }
    }

    if !profile.preferred_languages.is_empty()
        && !parsed.matches_language(&profile.preferred_languages)
    {
        reasons.push(format!(
            "language {} is not a preferred language ({})",
            parsed.effective_languages().join(", "),
            profile.preferred_languages.join(", ")
        ));
    }

    reasons
}

/// Classify a release against a profile, returning the match class and every
/// reason that kept it from being `optimal`. Powers the `profileMatch` /
/// `rejectReasons` fields on `searchSources`.
pub fn match_release(
    parsed: &ParsedRelease,
    profile: &QualityProfile,
    facts: &ReleaseFacts,
) -> (ProfileMatch, Vec<String>) {
    let hard = evaluate_release(parsed, profile, facts);
    if let QualityEvaluation::Suboptimal(reasons) = hard {
        return (ProfileMatch::Rejected, reasons);
    }
    let soft = soft_preference_reasons(parsed, profile);
    if soft.is_empty() {
        (ProfileMatch::Optimal, Vec::new())
    } else {
        (ProfileMatch::Suboptimal, soft)
    }
}

/// Sort key for profile-aware ranking. Every field is "lower is better" so the
/// derived `Ord` gives the documented precedence directly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RankKey {
    resolution: u32,
    proper: u8,
    language: u32,
    release_group: u32,
    negated_seeders: i64,
    negated_similarity: i64,
    title: String,
}

fn preference_index(list: &[String], value: Option<&String>) -> u32 {
    let Some(value) = value else {
        return u32::MAX;
    };
    list.iter()
        .position(|item| item.eq_ignore_ascii_case(value))
        .map(|index| index as u32)
        .unwrap_or(u32::MAX)
}

fn rank_key(scored: &scoring::ScoredRelease, profile: &QualityProfile) -> RankKey {
    let parsed = &scored.parsed;

    // 1. Resolution. With an explicit `resolution_preference` the list position
    //    decides (unlisted resolutions sort last); otherwise the highest
    //    parsed resolution wins.
    let resolution = if profile.resolution_preference.is_empty() {
        u32::from(u8::MAX - parsed.resolution_rank.0)
    } else {
        preference_index(&profile.resolution_preference, parsed.resolution.as_ref())
    };

    // 2. PROPER/REPACK, when the profile prefers them.
    let proper = if profile.prefer_proper_repack && parsed.is_proper_or_repack() {
        0
    } else {
        1
    };

    // 3. Language preference order (MULTi counts as the top preference).
    let language = parsed
        .language_preference_index(&profile.preferred_languages)
        .map(|index| index as u32)
        .unwrap_or(u32::MAX);

    // 4. Preferred release group order.
    let release_group = preference_index(
        &profile.preferred_release_groups,
        parsed.release_group.as_ref(),
    );

    RankKey {
        resolution,
        proper,
        language,
        release_group,
        // 5. More seeders wins.
        negated_seeders: -i64::from(scored.release.seeders.unwrap_or(0)),
        // 6. Closer title match wins.
        negated_similarity: -((scored.title_similarity * 10_000.0) as i64),
        // 7. Title, purely so the ordering is total and reproducible.
        title: scored.release.title.clone(),
    }
}

/// Filter `releases` down to the ones this profile accepts and rank them.
///
/// Hard filters (any failure drops the release): title similarity below
/// [`scoring::MIN_TITLE_SIMILARITY`], then [`evaluate_release`] — allowed
/// resolution/codec/audio/HDR/source lists, release-group white/blacklist,
/// `require_hdr`, `require_language_match`, `allow_season_packs`,
/// `min_seeders`, `min_size_mb`/`max_size_mb` (per episode for packs) and
/// `max_release_age_days`.
///
/// Survivors are ordered by, in strict precedence: (1) resolution — the
/// position in `resolution_preference` when it is set, otherwise the highest
/// parsed resolution; (2) PROPER/REPACK first when `prefer_proper_repack`;
/// (3) position in `preferred_languages` (a MULTi release counts as the top
/// preference); (4) position in `preferred_release_groups`; (5) more seeders;
/// (6) higher title similarity; (7) release title, so the order is total and
/// identical between runs.
pub fn filter_and_rank(
    wanted_title: &str,
    releases: Vec<SourceRelease>,
    profile: &QualityProfile,
    episode_count: i32,
) -> Vec<scoring::ScoredRelease> {
    let now = chrono::Utc::now();
    let mut scored: Vec<scoring::ScoredRelease> = releases
        .into_iter()
        .filter_map(|release| {
            let title_similarity = scoring::title_similarity(wanted_title, &release.title);
            if title_similarity < scoring::MIN_TITLE_SIMILARITY {
                return None;
            }
            let parsed = scoring::parse_release(&release.title);
            let facts = ReleaseFacts {
                now,
                ..ReleaseFacts::from_release(&release, episode_count)
            };
            if !evaluate_release(&parsed, profile, &facts).is_optimal() {
                return None;
            }
            Some(scoring::ScoredRelease {
                release,
                parsed,
                title_similarity,
            })
        })
        .collect();

    scored.sort_by_cached_key(|candidate| rank_key(candidate, profile));
    scored
}

/// [`filter_and_rank`] returning only the winner.
pub fn pick_best_for_profile(
    wanted_title: &str,
    releases: Vec<SourceRelease>,
    profile: &QualityProfile,
    episode_count: i32,
) -> Option<scoring::ScoredRelease> {
    filter_and_rank(wanted_title, releases, profile, episode_count)
        .into_iter()
        .next()
}

/// Build a [`ParsedRelease`] for an already-imported `MediaFile`.
///
/// Per design.md's "Quality is verified, not assumed" rule, ffprobe-verified
/// columns (`resolution`, `video_codec`, `is_hdr`, `hdr_type`, `audio_codec`)
/// take priority. `MediaFile` has no `source_type`/`release_group` columns
/// (ffprobe cannot determine those), so those two fields are always
/// supplemented by parsing the original filename.
pub fn parsed_release_for_media_file(media_file: &MediaFile) -> ParsedRelease {
    let filename = media_file
        .original_name
        .as_deref()
        .unwrap_or(&media_file.path);
    let from_name = scoring::parse_release(filename);

    let resolution = media_file.resolution.clone().or(from_name.resolution);
    let resolution_rank = if media_file.resolution.is_some() {
        // Re-derive the rank from the verified resolution string rather than
        // trusting the filename-derived rank, which may disagree.
        scoring::parse_resolution_rank(media_file.resolution.as_deref().unwrap_or_default())
    } else {
        from_name.resolution_rank
    };

    ParsedRelease {
        resolution,
        resolution_rank,
        codec: media_file.video_codec.clone().or(from_name.codec),
        is_hdr: media_file.is_hdr || from_name.is_hdr,
        hdr_type: media_file.hdr_type.clone().or(from_name.hdr_type),
        source_type: from_name.source_type,
        audio: media_file.audio_codec.clone().or(from_name.audio),
        release_group: from_name.release_group,
        // Language/edition/numbering can only come from the original name:
        // ffprobe reports audio stream languages per stream, not the release's
        // advertised language set, and it knows nothing about PROPER/REPACK.
        languages: from_name.languages,
        is_multi_language: from_name.is_multi_language,
        is_proper: from_name.is_proper,
        is_repack: from_name.is_repack,
        // An imported, organized file is never a "pack": it is one episode.
        is_season_pack: false,
        season: from_name.season,
        season_end: from_name.season_end,
        episodes: from_name.episodes,
        year: from_name.year,
    }
}

/// An in-memory "Any Quality" profile, used as a last-resort fallback when
/// the seeded default profile can't be found (should not happen in practice —
/// `bootstrap_defaults::seed_defaults` seeds it at startup — but resolution
/// must never hard-fail the pipeline over a missing settings row).
pub fn builtin_any_profile() -> QualityProfile {
    let now = chrono::Utc::now().to_rfc3339();
    QualityProfile {
        id: "builtin-any-quality".to_string(),
        name: "Any Quality".to_string(),
        media_kind: MediaKind::Video,
        allowed_resolutions: Vec::new(),
        allowed_video_codecs: Vec::new(),
        allowed_audio_formats: Vec::new(),
        allowed_hdr_types: Vec::new(),
        allowed_sources: Vec::new(),
        release_group_blacklist: Vec::new(),
        release_group_whitelist: Vec::new(),
        require_hdr: false,
        preferred_languages: Vec::new(),
        require_language_match: false,
        min_size_mb: None,
        max_size_mb: None,
        min_seeders: 1,
        max_release_age_days: None,
        preferred_release_groups: Vec::new(),
        allow_season_packs: true,
        prefer_proper_repack: true,
        resolution_preference: Vec::new(),
        cutoff_resolution: None,
        upgrade_until_cutoff: false,
        is_default: true,
        created_at: now.clone(),
        updated_at: now,
    }
}

/// The seeded default profile (`is_default = true`), falling back to an
/// in-memory "Any Quality" profile if none is found.
pub async fn default_profile(db: &Database) -> Result<QualityProfile> {
    let profiles = QualityProfile::query(db.pool())
        .fetch_all()
        .await
        .context("failed to query quality profiles")?;
    Ok(profiles
        .into_iter()
        .find(|p| p.is_default)
        .unwrap_or_else(builtin_any_profile))
}

/// Resolve a profile given an optional per-entity override id and a fallback
/// library id: the override wins if it resolves to a real profile, else fall
/// back to `resolve_profile_for_library`. Used by `jobs::auto_download` to
/// resolve a wanted candidate's profile without needing a `MediaFile` row
/// (nothing has been downloaded yet).
pub async fn resolve_profile(
    db: &Database,
    override_profile_id: Option<&str>,
    library_id: &str,
) -> Result<QualityProfile> {
    if let Some(id) = override_profile_id
        && let Some(profile) = get_profile_by_id(db, id).await?
    {
        return Ok(profile);
    }
    resolve_profile_for_library(db, library_id).await
}

async fn get_profile_by_id(db: &Database, id: &str) -> Result<Option<QualityProfile>> {
    let rows = QualityProfile::query(db.pool())
        .filter(crate::graphql::entities::QualityProfileWhereInput {
            id: Some(string_eq(id)),
            ..Default::default()
        })
        .fetch_all()
        .await
        .context("failed to query quality profile by id")?;
    Ok(rows.into_iter().next())
}

/// Resolve the effective profile for a `Library`: `Library.quality_profile_id`
/// if set, else the seeded default.
pub async fn resolve_profile_for_library(
    db: &Database,
    library_id: &str,
) -> Result<QualityProfile> {
    let libraries = Library::query(db.pool())
        .filter(crate::graphql::entities::LibraryWhereInput {
            id: Some(string_eq(library_id)),
            ..Default::default()
        })
        .fetch_all()
        .await
        .context("failed to query library")?;

    if let Some(library) = libraries.into_iter().next()
        && let Some(profile_id) = library.quality_profile_id
        && let Some(profile) = get_profile_by_id(db, &profile_id).await?
    {
        return Ok(profile);
    }

    default_profile(db).await
}

/// Resolve the effective profile for a `MediaFile`, following the precedence
/// in `docs/tier1-features-plan.md` §2: per-entity override (Show/Movie/
/// Album/Audiobook) > `Library.quality_profile_id` > seeded default.
pub async fn resolve_profile_for_media_file(
    db: &Database,
    media_file: &MediaFile,
) -> Result<QualityProfile> {
    if let Some(movie_id) = &media_file.movie_id {
        let movies = Movie::query(db.pool())
            .filter(crate::graphql::entities::MovieWhereInput {
                id: Some(string_eq(movie_id)),
                ..Default::default()
            })
            .fetch_all()
            .await
            .context("failed to query movie")?;
        if let Some(movie) = movies.into_iter().next() {
            if let Some(profile_id) = &movie.quality_profile_id
                && let Some(profile) = get_profile_by_id(db, profile_id).await?
            {
                return Ok(profile);
            }
            return resolve_profile_for_library(db, &movie.library_id).await;
        }
    }

    if let Some(episode_id) = &media_file.episode_id {
        let episodes = Episode::query(db.pool())
            .filter(crate::graphql::entities::EpisodeWhereInput {
                id: Some(string_eq(episode_id)),
                ..Default::default()
            })
            .fetch_all()
            .await
            .context("failed to query episode")?;
        if let Some(episode) = episodes.into_iter().next() {
            let shows = Show::query(db.pool())
                .filter(crate::graphql::entities::ShowWhereInput {
                    id: Some(string_eq(&episode.show_id)),
                    ..Default::default()
                })
                .fetch_all()
                .await
                .context("failed to query show")?;
            if let Some(show) = shows.into_iter().next() {
                if let Some(profile_id) = &show.quality_profile_id
                    && let Some(profile) = get_profile_by_id(db, profile_id).await?
                {
                    return Ok(profile);
                }
                return resolve_profile_for_library(db, &show.library_id).await;
            }
        }
    }

    if let Some(track_id) = &media_file.track_id {
        let tracks = Track::query(db.pool())
            .filter(crate::graphql::entities::TrackWhereInput {
                id: Some(string_eq(track_id)),
                ..Default::default()
            })
            .fetch_all()
            .await
            .context("failed to query track")?;
        if let Some(track) = tracks.into_iter().next() {
            let albums = Album::query(db.pool())
                .filter(crate::graphql::entities::AlbumWhereInput {
                    id: Some(string_eq(&track.album_id)),
                    ..Default::default()
                })
                .fetch_all()
                .await
                .context("failed to query album")?;
            if let Some(album) = albums.into_iter().next() {
                if let Some(profile_id) = &album.quality_profile_id
                    && let Some(profile) = get_profile_by_id(db, profile_id).await?
                {
                    return Ok(profile);
                }
                return resolve_profile_for_library(db, &album.library_id).await;
            }
        }
    }

    if let Some(chapter_id) = &media_file.chapter_id {
        let chapters = Chapter::query(db.pool())
            .filter(crate::graphql::entities::ChapterWhereInput {
                id: Some(string_eq(chapter_id)),
                ..Default::default()
            })
            .fetch_all()
            .await
            .context("failed to query chapter")?;
        if let Some(chapter) = chapters.into_iter().next() {
            let audiobooks = Audiobook::query(db.pool())
                .filter(crate::graphql::entities::AudiobookWhereInput {
                    id: Some(string_eq(&chapter.audiobook_id)),
                    ..Default::default()
                })
                .fetch_all()
                .await
                .context("failed to query audiobook")?;
            if let Some(audiobook) = audiobooks.into_iter().next() {
                if let Some(profile_id) = &audiobook.quality_profile_id
                    && let Some(profile) = get_profile_by_id(db, profile_id).await?
                {
                    return Ok(profile);
                }
                return resolve_profile_for_library(db, &audiobook.library_id).await;
            }
        }
    }

    if let Some(library_id) = &media_file.library_id {
        return resolve_profile_for_library(db, library_id).await;
    }

    default_profile(db).await
}

/// Resolve the effective profile for a `MediaFile`, evaluate it, and persist
/// the result to `MediaFile.quality_status`. Shared by the `analyzeMediaFile`
/// ffprobe hook (`LibraryScanService::analyze_media_file_inner`) and the
/// `evaluateMediaFileQuality`/`recomputeLibraryQualityStatus` GraphQL
/// mutations so there is exactly one place that computes+writes this column.
pub async fn recompute_and_persist(
    db: &Database,
    media_file_id: &str,
) -> Result<QualityEvaluation> {
    use crate::graphql::entities::UpdateMediaFileInput;

    let files = MediaFile::query(db.pool())
        .filter(crate::graphql::entities::MediaFileWhereInput {
            id: Some(string_eq(media_file_id)),
            ..Default::default()
        })
        .fetch_all()
        .await
        .context("failed to query media file")?;
    let media_file = files
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("media file not found: {media_file_id}"))?;

    let quality_profile = resolve_profile_for_media_file(db, &media_file).await?;
    let parsed = parsed_release_for_media_file(&media_file);
    let evaluation = evaluate(&parsed, &quality_profile);

    MediaFile::update_by_id(
        db,
        &media_file.id,
        UpdateMediaFileInput {
            quality_status: Some(Some(evaluation.status_str().to_string())),
            ..Default::default()
        },
    )
    .await
    .context("failed to persist quality_status")?;

    Ok(evaluation)
}

/// Whether a profile with `upgrade_until_cutoff` set should still consider
/// `current` a candidate for further upgrades (i.e. the cutoff hasn't been
/// reached yet). Profiles without a cutoff, or with `upgrade_until_cutoff =
/// false`, never require further upgrades beyond simple `is_upgrade` checks.
///
/// Not yet called from a production code path: there is no "actively sweep
/// the library looking for better releases of already-optimal-enough files"
/// background job in this pass (only the passive Q44 duplicate-torrent path
/// and Q38 notification flow are wired up). It's exposed now, with tests,
/// for that future upgrade-sweep job to use — see the "Implemented"/deferred
/// notes in `docs/tier1-features-plan.md` §2.
#[cfg(test)]
pub fn should_seek_upgrade(profile: &QualityProfile, current: &ParsedRelease) -> bool {
    if !profile.upgrade_until_cutoff {
        return false;
    }
    let Some(cutoff) = &profile.cutoff_resolution else {
        return false;
    };
    let cutoff_rank = scoring::parse_resolution_rank(cutoff);
    current.resolution_rank < cutoff_rank
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_profile() -> QualityProfile {
        let now = chrono::Utc::now().to_rfc3339();
        QualityProfile {
            id: "test-profile".to_string(),
            name: "Test".to_string(),
            media_kind: MediaKind::Video,
            allowed_resolutions: Vec::new(),
            allowed_video_codecs: Vec::new(),
            allowed_audio_formats: Vec::new(),
            allowed_hdr_types: Vec::new(),
            allowed_sources: Vec::new(),
            release_group_blacklist: Vec::new(),
            release_group_whitelist: Vec::new(),
            require_hdr: false,
            preferred_languages: Vec::new(),
            require_language_match: false,
            min_size_mb: None,
            max_size_mb: None,
            min_seeders: 1,
            max_release_age_days: None,
            preferred_release_groups: Vec::new(),
            allow_season_packs: true,
            prefer_proper_repack: true,
            resolution_preference: Vec::new(),
            cutoff_resolution: None,
            upgrade_until_cutoff: false,
            is_default: false,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[test]
    fn allows_any_when_all_lists_empty() {
        let profile = base_profile();
        assert!(allows_any(&profile));
        let parsed = scoring::parse_release("Anything.At.All.CAM.mkv");
        assert_eq!(evaluate(&parsed, &profile), QualityEvaluation::Optimal);
    }

    #[test]
    fn rejects_disallowed_resolution() {
        let mut profile = base_profile();
        profile.allowed_resolutions = vec!["1080p".to_string(), "2160p".to_string()];
        let parsed = scoring::parse_release("Show.S01E01.720p.WEB-DL.x264-GROUP");
        let result = evaluate(&parsed, &profile);
        assert!(!result.is_optimal());
        assert_eq!(result.status_str(), "suboptimal");
        assert!(!result.reasons().is_empty());
    }

    #[test]
    fn accepts_allowed_resolution() {
        let mut profile = base_profile();
        profile.allowed_resolutions = vec!["1080p".to_string()];
        let parsed = scoring::parse_release("Show.S01E01.1080p.WEB-DL.x264-GROUP");
        assert_eq!(evaluate(&parsed, &profile), QualityEvaluation::Optimal);
    }

    #[test]
    fn require_hdr_rejects_non_hdr() {
        let mut profile = base_profile();
        profile.require_hdr = true;
        let parsed = scoring::parse_release("Movie.2024.2160p.WEB-DL.x265-GROUP");
        let result = evaluate(&parsed, &profile);
        assert!(!result.is_optimal());
    }

    #[test]
    fn require_hdr_accepts_hdr() {
        let mut profile = base_profile();
        profile.require_hdr = true;
        let parsed = scoring::parse_release("Movie.2024.2160p.HDR.WEB-DL.x265-GROUP");
        assert_eq!(evaluate(&parsed, &profile), QualityEvaluation::Optimal);
    }

    #[test]
    fn release_group_blacklist_rejects() {
        let mut profile = base_profile();
        profile.release_group_blacklist = vec!["BADGROUP".to_string()];
        let parsed = scoring::parse_release("Movie.2024.1080p.WEB-DL.x264-BADGROUP");
        let result = evaluate(&parsed, &profile);
        assert!(!result.is_optimal());
    }

    #[test]
    fn release_group_whitelist_rejects_unlisted() {
        let mut profile = base_profile();
        profile.release_group_whitelist = vec!["GOODGROUP".to_string()];
        let parsed = scoring::parse_release("Movie.2024.1080p.WEB-DL.x264-OTHERGROUP");
        let result = evaluate(&parsed, &profile);
        assert!(!result.is_optimal());
    }

    #[test]
    fn release_group_whitelist_accepts_listed() {
        let mut profile = base_profile();
        profile.release_group_whitelist = vec!["GOODGROUP".to_string()];
        let parsed = scoring::parse_release("Movie.2024.1080p.WEB-DL.x264-GOODGROUP");
        assert_eq!(evaluate(&parsed, &profile), QualityEvaluation::Optimal);
    }

    // -- release-level hard filters ---------------------------------------

    fn release(title: &str, seeders: Option<i32>, size: Option<i64>) -> SourceRelease {
        SourceRelease {
            seeders,
            size,
            ..SourceRelease::new(
                title.to_string(),
                format!("guid-{title}"),
                chrono::Utc::now(),
            )
        }
    }

    const MB: i64 = 1024 * 1024;

    #[test]
    fn rejects_releases_below_min_seeders() {
        let mut profile = base_profile();
        profile.min_seeders = 5;
        let parsed = scoring::parse_release("Show.S01E01.1080p.WEB-DL-GRP");
        let facts = ReleaseFacts {
            seeders: Some(2),
            ..Default::default()
        };
        assert!(!evaluate_release(&parsed, &profile, &facts).is_optimal());

        let enough = ReleaseFacts {
            seeders: Some(9),
            ..Default::default()
        };
        assert!(evaluate_release(&parsed, &profile, &enough).is_optimal());
    }

    #[test]
    fn unknown_seeder_counts_are_not_filtered() {
        let mut profile = base_profile();
        profile.min_seeders = 5;
        let parsed = scoring::parse_release("Book.Title.M4B-GRP");
        assert!(evaluate_release(&parsed, &profile, &ReleaseFacts::default()).is_optimal());
    }

    #[test]
    fn size_limits_are_per_episode_for_season_packs() {
        let mut profile = base_profile();
        profile.max_size_mb = Some(2_000);
        let parsed = scoring::parse_release("Show.S02.1080p.WEB-DL-GRP");

        // 16 GB across 10 episodes is 1.6 GB/episode: acceptable.
        let pack = ReleaseFacts {
            size_bytes: Some(16_000 * MB),
            episode_count: 10,
            ..Default::default()
        };
        assert!(evaluate_release(&parsed, &profile, &pack).is_optimal());

        // The same 16 GB treated as a single item is not.
        let single = ReleaseFacts {
            size_bytes: Some(16_000 * MB),
            episode_count: 1,
            ..Default::default()
        };
        assert!(!evaluate_release(&parsed, &profile, &single).is_optimal());
    }

    #[test]
    fn rejects_releases_older_than_max_release_age_days() {
        let mut profile = base_profile();
        profile.max_release_age_days = Some(30);
        let parsed = scoring::parse_release("Show.S01E01.1080p.WEB-DL-GRP");
        let now = chrono::Utc::now();
        let stale = ReleaseFacts {
            publish_date: Some(now - chrono::Duration::days(60)),
            now,
            ..Default::default()
        };
        assert!(!evaluate_release(&parsed, &profile, &stale).is_optimal());
        let fresh = ReleaseFacts {
            publish_date: Some(now - chrono::Duration::days(2)),
            now,
            ..Default::default()
        };
        assert!(evaluate_release(&parsed, &profile, &fresh).is_optimal());
    }

    #[test]
    fn require_language_match_is_a_hard_filter() {
        let mut profile = base_profile();
        profile.preferred_languages = vec!["fr".to_string()];
        profile.require_language_match = true;

        let english = scoring::parse_release("Movie.2020.1080p.BluRay-GRP");
        assert!(!evaluate(&english, &profile).is_optimal());

        let french = scoring::parse_release("Movie.2020.FRENCH.1080p.BluRay-GRP");
        assert_eq!(evaluate(&french, &profile), QualityEvaluation::Optimal);

        let multi = scoring::parse_release("Movie.2020.MULTi.1080p.BluRay-GRP");
        assert_eq!(evaluate(&multi, &profile), QualityEvaluation::Optimal);
    }

    #[test]
    fn language_preference_without_require_is_only_soft() {
        let mut profile = base_profile();
        profile.preferred_languages = vec!["fr".to_string()];
        let english = scoring::parse_release("Movie.2020.1080p.BluRay-GRP");
        assert_eq!(evaluate(&english, &profile), QualityEvaluation::Optimal);
        let (class, reasons) = match_release(&english, &profile, &ReleaseFacts::default());
        assert_eq!(class, ProfileMatch::Suboptimal);
        assert!(!reasons.is_empty());
    }

    #[test]
    fn season_packs_can_be_disallowed() {
        let mut profile = base_profile();
        profile.allow_season_packs = false;
        let pack = scoring::parse_release("Show.S02.COMPLETE.1080p.WEB-DL-GRP");
        assert!(!evaluate(&pack, &profile).is_optimal());
        let episode = scoring::parse_release("Show.S02E01.1080p.WEB-DL-GRP");
        assert_eq!(evaluate(&episode, &profile), QualityEvaluation::Optimal);
    }

    #[test]
    fn match_release_classifies_optimal_suboptimal_and_rejected() {
        let mut profile = base_profile();
        profile.min_seeders = 5;
        profile.preferred_release_groups = vec!["GOOD".to_string()];

        let optimal = scoring::parse_release("Show.S01E01.1080p.WEB-DL-GOOD");
        let facts = ReleaseFacts {
            seeders: Some(50),
            ..Default::default()
        };
        assert_eq!(
            match_release(&optimal, &profile, &facts).0,
            ProfileMatch::Optimal
        );

        let other_group = scoring::parse_release("Show.S01E01.1080p.WEB-DL-OTHER");
        assert_eq!(
            match_release(&other_group, &profile, &facts).0,
            ProfileMatch::Suboptimal
        );

        let starved = ReleaseFacts {
            seeders: Some(1),
            ..Default::default()
        };
        assert_eq!(
            match_release(&optimal, &profile, &starved).0,
            ProfileMatch::Rejected
        );
    }

    // -- profile-aware ranking --------------------------------------------

    #[test]
    fn ranking_honours_resolution_preference_order() {
        let mut profile = base_profile();
        profile.resolution_preference = vec!["1080p".to_string(), "2160p".to_string()];
        let releases = vec![
            release("Show S01E01 2160p WEB-DL x265-GRP", Some(100), None),
            release("Show S01E01 1080p WEB-DL x264-GRP", Some(10), None),
        ];
        let best = pick_best_for_profile("Show S01E01", releases, &profile, 1)
            .expect("a release should be picked");
        assert!(
            best.release.title.contains("1080p"),
            "expected the preferred 1080p release to win, got '{}'",
            best.release.title
        );
    }

    #[test]
    fn ranking_prefers_proper_over_plain_at_the_same_resolution() {
        let profile = base_profile();
        let releases = vec![
            release("Show S01E01 1080p WEB-DL x264-AAA", Some(500), None),
            release("Show S01E01 PROPER 1080p WEB-DL x264-BBB", Some(20), None),
        ];
        let best = pick_best_for_profile("Show S01E01", releases, &profile, 1)
            .expect("a release should be picked");
        assert!(best.parsed.is_proper, "expected the PROPER release to win");
    }

    #[test]
    fn ranking_prefers_earlier_release_groups() {
        let mut profile = base_profile();
        profile.preferred_release_groups = vec!["BEST".to_string(), "OKAY".to_string()];
        let releases = vec![
            release("Show S01E01 1080p WEB-DL x264-OKAY", Some(900), None),
            release("Show S01E01 1080p WEB-DL x264-BEST", Some(1), None),
        ];
        let best = pick_best_for_profile("Show S01E01", releases, &profile, 1)
            .expect("a release should be picked");
        assert_eq!(best.parsed.release_group.as_deref(), Some("BEST"));
    }

    #[test]
    fn ranking_prefers_earlier_languages() {
        let mut profile = base_profile();
        profile.preferred_languages = vec!["fr".to_string(), "en".to_string()];
        let releases = vec![
            release("Movie 2020 1080p BluRay x264-AAA", Some(900), None),
            release("Movie 2020 FRENCH 1080p BluRay x264-BBB", Some(5), None),
        ];
        let best =
            pick_best_for_profile("Movie 2020", releases, &profile, 1).expect("a release wins");
        assert_eq!(best.parsed.languages, vec!["fr".to_string()]);
    }

    #[test]
    fn ranking_drops_releases_the_profile_rejects() {
        let mut profile = base_profile();
        profile.min_seeders = 10;
        let releases = vec![
            release("Show S01E01 1080p WEB-DL x264-AAA", Some(2), None),
            release("Show S01E01 720p WEB-DL x264-BBB", Some(40), None),
        ];
        let ranked = filter_and_rank("Show S01E01", releases, &profile, 1);
        assert_eq!(ranked.len(), 1);
        assert!(ranked[0].release.title.contains("720p"));
    }

    #[test]
    fn ranking_is_deterministic_for_identical_candidates() {
        let profile = base_profile();
        let build = || {
            vec![
                release("Show S01E01 1080p WEB-DL x264-BBB", Some(10), None),
                release("Show S01E01 1080p WEB-DL x264-AAA", Some(10), None),
            ]
        };
        let first: Vec<String> = filter_and_rank("Show S01E01", build(), &profile, 1)
            .into_iter()
            .map(|scored| scored.release.title)
            .collect();
        let second: Vec<String> = filter_and_rank("Show S01E01", build(), &profile, 1)
            .into_iter()
            .map(|scored| scored.release.title)
            .collect();
        assert_eq!(first, second);
        assert!(first[0].contains("AAA"));
    }

    #[test]
    fn should_seek_upgrade_respects_cutoff() {
        let mut profile = base_profile();
        profile.upgrade_until_cutoff = true;
        profile.cutoff_resolution = Some("1080p".to_string());

        let below_cutoff = scoring::parse_release("Show.S01E01.720p.WEB-DL.x264-GROUP");
        assert!(should_seek_upgrade(&profile, &below_cutoff));

        let at_cutoff = scoring::parse_release("Show.S01E01.1080p.WEB-DL.x264-GROUP");
        assert!(!should_seek_upgrade(&profile, &at_cutoff));
    }

    #[test]
    fn should_seek_upgrade_false_when_disabled() {
        let mut profile = base_profile();
        profile.cutoff_resolution = Some("1080p".to_string());
        profile.upgrade_until_cutoff = false;
        let below_cutoff = scoring::parse_release("Show.S01E01.720p.WEB-DL.x264-GROUP");
        assert!(!should_seek_upgrade(&profile, &below_cutoff));
    }
}
