//! Recommended releases: the best release for anything missing, worked out ahead of time.
//!
//! For a missing movie, season, episode, album, track or book, this finds the release automatic
//! downloading would take, without taking it, and keeps it as a [`ReleaseRecommendation`] so the
//! item's page can show "Recommended" with a one-press Download.
//!
//! **One brain.** Candidates come from the hunt's own discovery (`discover_candidates`, the
//! season-pack rule, the per-episode candidates, `Expectation`), releases are searched with
//! `collect_releases_from`, filtered against the blocklist and ranked by the quality profile through
//! `rank_for_candidate`: the same functions the hunt grabs with. Two things are added, because
//! nothing grabs them: a series pack and a whole album. Within the profile's best quality tier the
//! smallest release is preferred, so a recommendation can differ from the hunt's pick between two
//! releases of equal quality.
//!
//! **Cheapest input first.** The feed catalogue (releases already listed, no request) is tried
//! before any indexer is asked, and an indexer is asked only if the catalogue did not settle it.
//! One search covers a whole season or album: a season's search returns packs and episodes alike.
//!
//! **Never a burst.** Indexer requests are counted against an hourly budget
//! (`auto_download.hourly_search_budget`, default 30; 0 turns lookups off). Background lookups cover
//! only items flagged wanted; anything else is looked up on demand, when its page is opened
//! (`lookupRecommendations`), from a queue the worker drains within the budget. A result is kept for
//! about a day, a "nothing found" too, so it is not searched again meanwhile. A recommendation
//! disappears when its release is blocklisted or gone from its feed, or when what it was for is no
//! longer missing.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use graphql_orm::graphql::filters::{DateFilter, StringFilter};
use once_cell::sync::Lazy;
use tracing::{debug, info, warn};

use crate::db::Database;
use crate::graphql::entities::{
    CreateReleaseRecommendationInput, Library, ReleaseRecommendation,
    ReleaseRecommendationWhereInput, RssFeed, RssFeedItem, RssFeedItemWhereInput, Torrent,
    TorrentWhereInput,
};
use crate::graphql::entities::{FEED_ITEM_GUID_PREFIX, feed_item_release};
use crate::services::ServicesManager;
use crate::services::quality::profile;
use crate::services::quality::scoring::{ParsedRelease, ScoredRelease};
use crate::services::quality::title_match::WantedTitle;
use crate::services::sources::manager::SourcesManager;
use crate::services::sources::{QueryType, SourceQuery, SourceRelease};

use super::auto_download::{
    Blocklist, CandidateLinkage, Collected, HuntScope, NoGrabFacts, WantedCandidate,
    collect_releases_from, describe_no_grab, discover_candidates, rank_for_candidate,
};
use super::auto_schedule::{Expectation, Timing};
use super::auto_settings::AutoDownloadSettings;

/// How long a recommendation, or a "nothing found", is kept before the lookup is repeated.
pub const TTL: Duration = Duration::hours(24);
/// The version of the rules that choose a recommendation. Raise it whenever a change means stored
/// answers may be wrong: rows from an older version are removed on the next pass and looked up
/// again, instead of standing for up to a day. 2: titles are matched word for word (Q98), so
/// `Babylon Berlin` is no longer recommended for `Babylon 5`.
pub const RULES_VERSION: i32 = 2;

/// Whether a stored answer was chosen by the current rules.
pub fn is_current(row: &ReleaseRecommendation) -> bool {
    row.rules_version.unwrap_or(0) >= RULES_VERSION
}
/// How long a "searching" placeholder lives if its lookup never finishes.
const PLACEHOLDER_TTL: Duration = Duration::minutes(60);
/// Most episode recommendations written for one season.
const MAX_EPISODE_ROWS: usize = 40;
/// Targets one background pass looks up.
const BACKGROUND_TARGETS_PER_PASS: usize = 8;
/// Between background passes.
const BACKGROUND_EVERY: Duration = Duration::minutes(30);

fn string_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

// ===========================================================================
// The hourly search budget
// ===========================================================================

/// Indexer requests made for recommendations in the last hour.
#[derive(Debug, Default)]
pub struct SearchBudget {
    spent: VecDeque<DateTime<Utc>>,
}

impl SearchBudget {
    fn forget_old(&mut self, now: DateTime<Utc>) {
        while self
            .spent
            .front()
            .is_some_and(|at| now - *at >= Duration::hours(1))
        {
            self.spent.pop_front();
        }
    }

    /// Requests that may still be made this hour.
    pub fn remaining(&mut self, per_hour: u32, now: DateTime<Utc>) -> u32 {
        self.forget_old(now);
        per_hour.saturating_sub(self.spent.len() as u32)
    }

    /// Count `requests` indexer requests as made.
    pub fn spend(&mut self, requests: u32, now: DateTime<Utc>) {
        for _ in 0..requests {
            self.spent.push_back(now);
        }
    }
}

static BUDGET: Lazy<StdMutex<SearchBudget>> = Lazy::new(|| StdMutex::new(SearchBudget::default()));

fn budget_remaining(per_hour: u32) -> u32 {
    BUDGET
        .lock()
        .map(|mut budget| budget.remaining(per_hour, Utc::now()))
        .unwrap_or(0)
}

fn budget_spend(requests: u32) {
    if let Ok(mut budget) = BUDGET.lock() {
        budget.spend(requests, Utc::now());
    }
}

// ===========================================================================
// The queue of lookups people asked for
// ===========================================================================

/// What to look up: one season, one movie, one album, one book, or a whole show.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LookupRequest {
    pub show_id: Option<String>,
    pub season: Option<i32>,
    pub episode_id: Option<String>,
    pub movie_id: Option<String>,
    pub album_id: Option<String>,
    pub track_id: Option<String>,
    pub audiobook_id: Option<String>,
}

static QUEUE: Lazy<StdMutex<VecDeque<LookupRequest>>> =
    Lazy::new(|| StdMutex::new(VecDeque::new()));

/// Queue a lookup. `false` when the same one is already waiting.
pub fn enqueue(request: LookupRequest) -> bool {
    let Ok(mut queue) = QUEUE.lock() else {
        return false;
    };
    if queue.contains(&request) {
        return false;
    }
    queue.push_back(request);
    true
}

fn next_request() -> Option<LookupRequest> {
    QUEUE.lock().ok()?.pop_front()
}

// ===========================================================================
// Choosing and describing a release (pure)
// ===========================================================================

/// `1080p WEB-DL h264` from what a title says.
pub fn quality_label(parsed: &ParsedRelease) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(resolution) = &parsed.resolution {
        parts.push(resolution.clone());
    }
    if let Some(source) = &parsed.source_type {
        parts.push(source.to_ascii_uppercase());
    }
    if let Some(codec) = &parsed.codec {
        parts.push(codec.clone());
    }
    if let Some(hdr) = &parsed.hdr_type {
        parts.push(hdr.to_ascii_uppercase());
    }
    if let Some(audio) = &parsed.audio {
        parts.push(audio.to_ascii_uppercase());
    }
    if parts.is_empty() {
        "quality not stated".to_string()
    } else {
        parts.join(" ")
    }
}

/// From a profile-ranked list (best first), the smallest release in the best quality tier: the same
/// resolution as the first, at least half its seeders when both are known (a tiny release nobody
/// seeds is no bargain). Returns the choice and whether size, not seeders, decided it.
pub fn smallest_at_best_quality(ranked: &[ScoredRelease]) -> Option<(&ScoredRelease, bool)> {
    let top = ranked.first()?;
    let top_seeders = top.release.seeders;
    let tier = ranked.iter().filter(|candidate| {
        candidate.parsed.resolution_rank == top.parsed.resolution_rank
            && candidate.parsed.is_hdr == top.parsed.is_hdr
            && match (candidate.release.seeders, top_seeders) {
                (Some(seeders), Some(best)) => seeders * 2 >= best && seeders > 0,
                _ => true,
            }
    });
    let smallest = tier
        .filter(|candidate| candidate.release.size.is_some())
        .min_by_key(|candidate| candidate.release.size.unwrap_or(i64::MAX));
    match smallest {
        Some(smallest) if smallest.release.guid != top.release.guid => Some((smallest, true)),
        _ => Some((top, false)),
    }
}

fn human_size(bytes: i64) -> String {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GB {
        format!("{:.1} GB", bytes / GB)
    } else {
        format!("{:.0} MB", bytes / MB)
    }
}

/// Short reasons a release won.
pub fn reasons_for(
    chosen: &ScoredRelease,
    smaller_than_top: bool,
    profile_name: &str,
    feed_name: Option<&str>,
    covers: usize,
    missing: usize,
    alternatives: usize,
) -> Vec<String> {
    let mut reasons = vec![format!(
        "{} fits the '{profile_name}' profile",
        quality_label(&chosen.parsed)
    )];
    if smaller_than_top {
        reasons.push(match chosen.release.size {
            Some(size) => format!("Smallest at that quality ({})", human_size(size)),
            None => "Smallest at that quality".to_string(),
        });
    } else if alternatives > 1 {
        reasons.push("Best of the releases the profile accepts".to_string());
    }
    if let Some(seeders) = chosen.release.seeders {
        reasons.push(format!("{seeders} seeders"));
    }
    if let Some(feed) = feed_name {
        reasons.push(format!("Already listed by feed '{feed}': no search needed"));
    }
    if missing > 1 {
        reasons.push(format!("Covers {covers} of {missing} missing"));
    }
    reasons
}

/// The seasons a pack release covers, as an inclusive range, from its title. `None` for a
/// complete-series pack that names no seasons.
pub fn pack_season_range(parsed: &ParsedRelease) -> Option<(i32, i32)> {
    let first = parsed.season?;
    Some((first, parsed.season_end.unwrap_or(first).max(first)))
}

// ===========================================================================
// The feed catalogue as a first input
// ===========================================================================

/// What a catalogue lookup is for.
#[derive(Debug, Clone, Default)]
struct CatalogueTarget {
    show_id: Option<String>,
    movie_id: Option<String>,
    album_id: Option<String>,
    audiobook_id: Option<String>,
}

/// Releases the owner's enabled feeds already list for this target.
async fn catalogue_releases(
    db: &Database,
    owner_id: &str,
    target: &CatalogueTarget,
    blocklist: &Blocklist,
) -> (Collected, HashMap<String, String>) {
    let mut collected = Collected::default();
    let mut feed_names = HashMap::new();
    let feeds = match RssFeed::query(db.pool())
        .filter(crate::graphql::entities::RssFeedWhereInput {
            user_id: Some(string_eq(owner_id)),
            enabled: Some(graphql_orm::graphql::filters::BoolFilter {
                eq: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await
    {
        Ok(feeds) => feeds,
        Err(error) => {
            warn!(error = %error, "Recommendations: could not list feeds for the catalogue");
            return (collected, feed_names);
        }
    };
    if feeds.is_empty() {
        return (collected, feed_names);
    }
    let mut filter = RssFeedItemWhereInput {
        feed_id: Some(StringFilter {
            in_list: Some(feeds.iter().map(|feed| feed.id.clone()).collect()),
            ..Default::default()
        }),
        ..Default::default()
    };
    if let Some(id) = &target.show_id {
        filter.match_show_id = Some(string_eq(id));
    } else if let Some(id) = &target.movie_id {
        filter.match_movie_id = Some(string_eq(id));
    } else if let Some(id) = &target.album_id {
        filter.match_album_id = Some(string_eq(id));
    } else if let Some(id) = &target.audiobook_id {
        filter.match_audiobook_id = Some(string_eq(id));
    } else {
        return (collected, feed_names);
    }
    let items = match RssFeedItem::query(db.pool())
        .filter(filter)
        .limit(400)
        .fetch_all()
        .await
    {
        Ok(items) => items,
        Err(error) => {
            warn!(error = %error, "Recommendations: could not read the feed catalogue");
            return (collected, feed_names);
        }
    };
    collected.sources = feeds.len();
    for item in items {
        let Some(feed) = feeds.iter().find(|feed| feed.id == item.feed_id) else {
            continue;
        };
        let release = feed_item_release(&item, feed);
        collected.seen += 1;
        if blocklist.blocks(&release) {
            collected.blocklisted += 1;
            continue;
        }
        feed_names.insert(release.guid.clone(), feed.name.clone());
        collected.releases.push(release);
    }
    (collected, feed_names)
}

/// Add `more` to `collected`, skipping releases already there (same info hash, else same guid).
fn merge(collected: &mut Collected, more: Collected) {
    let key = |release: &SourceRelease| {
        release
            .info_hash
            .clone()
            .unwrap_or_else(|| release.guid.clone())
            .to_ascii_lowercase()
    };
    let mut seen: HashSet<String> = collected.releases.iter().map(key).collect();
    for release in more.releases {
        if seen.insert(key(&release)) {
            collected.releases.push(release);
        }
    }
    collected.seen += more.seen;
    collected.blocklisted += more.blocklisted;
    collected.sources = collected.sources.max(more.sources);
    collected.errors.extend(more.errors);
}

// ===========================================================================
// What a lookup writes
// ===========================================================================

/// One row to store.
#[derive(Debug, Clone)]
struct Row {
    scope: &'static str,
    status: &'static str,
    target_label: String,
    show_id: Option<String>,
    season: Option<i32>,
    episode_id: Option<String>,
    movie_id: Option<String>,
    album_id: Option<String>,
    track_id: Option<String>,
    audiobook_id: Option<String>,
    chosen: Option<Chosen>,
    covers_ids: Vec<String>,
    covers_count: i32,
    missing_count: i32,
    note: Option<String>,
}

#[derive(Debug, Clone)]
struct Chosen {
    release: SourceRelease,
    parsed: ParsedRelease,
    reasons: Vec<String>,
    feed_item_id: Option<String>,
    source_name: Option<String>,
}

/// Where a lookup is made and what it knows.
struct Env<'a> {
    db: &'a Database,
    sources: Option<&'a SourcesManager>,
    blocklist: &'a Blocklist,
    settings: &'a AutoDownloadSettings,
    owner_id: String,
}

/// The catalogue and the indexers, in that order: the catalogue first, an indexer search only if
/// `settled` says the catalogue's releases did not answer, and only within the budget.
async fn gather<'a>(
    env: &Env<'a>,
    target: &CatalogueTarget,
    search_with: &WantedCandidate,
    settled: impl Fn(&Collected) -> bool,
) -> (Collected, HashMap<String, String>, bool) {
    let (mut collected, feeds) =
        catalogue_releases(env.db, &env.owner_id, target, env.blocklist).await;
    let mut ran_out = false;
    if !settled(&collected) {
        let wanted_requests = search_with.queries.len() as u32;
        match env.sources {
            Some(sources)
                if budget_remaining(env.settings.hourly_search_budget) >= wanted_requests =>
            {
                budget_spend(wanted_requests);
                let searched = collect_releases_from(sources, env.blocklist, search_with).await;
                merge(&mut collected, searched);
            }
            Some(_) => ran_out = true,
            None => {}
        }
    }
    (collected, feeds, ran_out)
}

fn chosen_from(
    ranked: &[ScoredRelease],
    profile_name: &str,
    feeds: &HashMap<String, String>,
    covers: usize,
    missing: usize,
) -> Option<Chosen> {
    let (picked, smaller) = smallest_at_best_quality(ranked)?;
    let feed_name = feeds.get(&picked.release.guid).map(String::as_str);
    let reasons = reasons_for(
        picked,
        smaller,
        profile_name,
        feed_name,
        covers,
        missing,
        ranked.len(),
    );
    Some(Chosen {
        release: picked.release.clone(),
        parsed: picked.parsed.clone(),
        reasons,
        feed_item_id: picked
            .release
            .guid
            .strip_prefix(FEED_ITEM_GUID_PREFIX)
            .map(str::to_string),
        source_name: picked.release.source_name.clone(),
    })
}

fn none_note(facts: &NoGrabFacts, ran_out: bool) -> String {
    let base = describe_no_grab(facts);
    if ran_out {
        format!("{base}; the hourly search budget is used up, so indexers were not asked")
    } else {
        base
    }
}

async fn resolve_profile(
    env: &Env<'_>,
    candidate: &WantedCandidate,
) -> crate::graphql::entities::QualityProfile {
    profile::resolve_profile(
        env.db,
        candidate.quality_profile_override_id.as_deref(),
        &candidate.library_id,
    )
    .await
    .unwrap_or_else(|_| profile::builtin_any_profile())
}

fn ids_of(
    candidates: &[&WantedCandidate],
    pick: impl Fn(&CandidateLinkage) -> Option<&String>,
) -> Vec<String> {
    candidates
        .iter()
        .filter_map(|candidate| pick(&candidate.linkage).cloned())
        .collect()
}

// ---- TV ------------------------------------------------------------------

/// One season of one show: the episodes missing, and the pack candidate if the hunt would look for
/// a pack.
struct SeasonGroup<'a> {
    show_id: String,
    show_name: String,
    season: i32,
    pack: Option<&'a WantedCandidate>,
    episodes: Vec<&'a WantedCandidate>,
}

impl SeasonGroup<'_> {
    fn label(&self) -> String {
        format!("{} S{:02}", self.show_name, self.season)
    }

    /// The candidate to search with: the pack's, or one built from an episode's for the whole season
    /// (a season search returns packs and single episodes alike).
    fn search_candidate(&self) -> Option<WantedCandidate> {
        if let Some(pack) = self.pack {
            return Some(pack.clone());
        }
        let first = self.episodes.first()?;
        let base = first.queries.first()?.clone();
        let padded = format!("{} S{:02}", self.show_name, self.season);
        let worded = format!("{} Season {}", self.show_name, self.season);
        let mut candidate = (*first).clone();
        candidate.label = format!("season '{padded}'");
        candidate.queries = [padded, worded]
            .into_iter()
            .map(|term| SourceQuery {
                search_term: Some(term),
                season: Some(self.season),
                episode: None,
                ..base.clone()
            })
            .collect();
        candidate.expectation = Expectation::SeasonPack {
            season: self.season,
        };
        Some(candidate)
    }
}

async fn look_up_season(env: &Env<'_>, group: &SeasonGroup<'_>) -> Vec<Row> {
    let missing = group.episodes.len();
    let Some(search) = group.search_candidate() else {
        return Vec::new();
    };
    let quality = resolve_profile(env, &search).await;
    let target = CatalogueTarget {
        show_id: Some(group.show_id.clone()),
        ..Default::default()
    };
    let pack_candidate = group.pack;
    let episodes = &group.episodes;
    let rank_all = |collected: &Collected| {
        let pack = pack_candidate.map(|pack| rank_for_candidate(pack, collected, &quality, true));
        let singles: Vec<_> = episodes
            .iter()
            .map(|episode| {
                (
                    *episode,
                    rank_for_candidate(episode, collected, &quality, true),
                )
            })
            .collect();
        (pack, singles)
    };
    // The catalogue settles it if it has a pack, or an acceptable release for every episode.
    let settled = |collected: &Collected| {
        let (pack, singles) = rank_all(collected);
        pack.as_ref().is_some_and(|pack| pack.is_ok())
            || singles.iter().all(|(_, ranked)| ranked.is_ok())
    };
    let (collected, feeds, ran_out) = gather(env, &target, &search, settled).await;
    let (pack, singles) = rank_all(&collected);

    let covered_ids = ids_of(&group.episodes, |linkage| linkage.episode_id.as_ref());
    if let Some(Ok(ranked)) = &pack
        && let Some(chosen) = chosen_from(ranked, &quality.name, &feeds, missing, missing)
    {
        return vec![Row {
            scope: "season_pack",
            status: "ready",
            target_label: group.label(),
            show_id: Some(group.show_id.clone()),
            season: Some(group.season),
            episode_id: None,
            movie_id: None,
            album_id: None,
            track_id: None,
            audiobook_id: None,
            chosen: Some(chosen),
            covers_ids: covered_ids,
            covers_count: missing as i32,
            missing_count: missing as i32,
            note: None,
        }];
    }
    let mut rows = Vec::new();
    for (episode, ranked) in singles.into_iter().take(MAX_EPISODE_ROWS) {
        let Ok(ranked) = ranked else { continue };
        let Some(chosen) = chosen_from(&ranked, &quality.name, &feeds, 1, missing) else {
            continue;
        };
        rows.push(Row {
            scope: "episode",
            status: "ready",
            target_label: episode
                .label
                .trim_start_matches("episode '")
                .trim_end_matches('\'')
                .to_string(),
            show_id: Some(group.show_id.clone()),
            season: Some(group.season),
            episode_id: episode.linkage.episode_id.clone(),
            movie_id: None,
            album_id: None,
            track_id: None,
            audiobook_id: None,
            chosen: Some(chosen),
            covers_ids: episode.linkage.episode_id.clone().into_iter().collect(),
            covers_count: 1,
            missing_count: missing as i32,
            note: None,
        });
    }
    if rows.is_empty() {
        let facts = match pack {
            Some(Err(facts)) => facts,
            _ => rank_for_candidate(&search, &collected, &quality, true)
                .err()
                .unwrap_or_default(),
        };
        rows.push(Row {
            scope: "season_pack",
            status: "none",
            target_label: group.label(),
            show_id: Some(group.show_id.clone()),
            season: Some(group.season),
            episode_id: None,
            movie_id: None,
            album_id: None,
            track_id: None,
            audiobook_id: None,
            chosen: None,
            covers_ids: Vec::new(),
            covers_count: 0,
            missing_count: missing as i32,
            note: Some(none_note(&facts, ran_out)),
        });
    }
    rows
}

/// A complete-series pack covering several of a show's missing seasons.
async fn look_up_series(
    env: &Env<'_>,
    show_name: &str,
    groups: &[&SeasonGroup<'_>],
) -> Option<Row> {
    let first = groups.iter().find_map(|group| group.pack)?;
    let missing: usize = groups.iter().map(|group| group.episodes.len()).sum();
    let base = first.queries.first()?.clone();
    let mut candidate = first.clone();
    let complete = format!("{show_name} Complete Series");
    let short = format!("{show_name} Complete");
    candidate.label = format!("complete series '{show_name}'");
    candidate.queries = [complete, short]
        .into_iter()
        .map(|term| SourceQuery {
            search_term: Some(term),
            season: None,
            episode: None,
            ..base.clone()
        })
        .collect();
    candidate.expectation = Expectation::SeriesPack;
    candidate.episode_count = missing as i32;
    candidate.linkage = CandidateLinkage {
        show_id: first.linkage.show_id.clone(),
        ..Default::default()
    };
    let quality = resolve_profile(env, &candidate).await;
    let target = CatalogueTarget {
        show_id: first.linkage.show_id.clone(),
        ..Default::default()
    };
    let (collected, feeds, _) = gather(env, &target, &candidate, |collected| {
        rank_for_candidate(&candidate, collected, &quality, true).is_ok()
    })
    .await;
    let ranked = rank_for_candidate(&candidate, &collected, &quality, true).ok()?;
    let chosen = chosen_from(&ranked, &quality.name, &feeds, missing, missing)?;
    // Only the missing seasons the release covers count.
    let (covered_ids, covered) = match pack_season_range(&chosen.parsed) {
        Some((from, to)) => {
            let inside: Vec<&&SeasonGroup<'_>> = groups
                .iter()
                .filter(|group| (from..=to).contains(&group.season))
                .collect();
            (
                inside
                    .iter()
                    .flat_map(|group| {
                        ids_of(&group.episodes, |linkage| linkage.episode_id.as_ref())
                    })
                    .collect::<Vec<_>>(),
                inside
                    .iter()
                    .map(|group| group.episodes.len())
                    .sum::<usize>(),
            )
        }
        None => (
            groups
                .iter()
                .flat_map(|group| ids_of(&group.episodes, |linkage| linkage.episode_id.as_ref()))
                .collect(),
            missing,
        ),
    };
    if covered < 2 {
        return None;
    }
    Some(Row {
        scope: "series_pack",
        status: "ready",
        target_label: format!("{show_name} (complete series)"),
        show_id: first.linkage.show_id.clone(),
        season: None,
        episode_id: None,
        movie_id: None,
        album_id: None,
        track_id: None,
        audiobook_id: None,
        chosen: Some(chosen),
        covers_ids: covered_ids,
        covers_count: covered as i32,
        missing_count: missing as i32,
        note: None,
    })
}

// ---- Movie, book, album ----------------------------------------------------

async fn look_up_single(
    env: &Env<'_>,
    candidate: &WantedCandidate,
    scope: &'static str,
    target: CatalogueTarget,
    label: String,
    linkage: &CandidateLinkage,
    covers: Vec<String>,
) -> Row {
    let quality = resolve_profile(env, candidate).await;
    let (collected, feeds, ran_out) = gather(env, &target, candidate, |collected| {
        rank_for_candidate(candidate, collected, &quality, true).is_ok()
    })
    .await;
    // A search for a book or a track was narrowed already; a title that does not say its year or
    // episode is judged strictly only when it came from a feed.
    let ranked = rank_for_candidate(candidate, &collected, &quality, true).or_else(|facts| {
        rank_for_candidate(candidate, &collected, &quality, false).map_err(|_| facts)
    });
    let missing = covers.len().max(1);
    let mut row = Row {
        scope,
        status: "none",
        target_label: label,
        show_id: linkage.show_id.clone(),
        season: linkage.season,
        episode_id: linkage.episode_id.clone(),
        movie_id: linkage.movie_id.clone(),
        album_id: linkage.album_id.clone(),
        track_id: linkage.track_id.clone(),
        audiobook_id: linkage.audiobook_id.clone(),
        chosen: None,
        covers_ids: covers,
        covers_count: missing as i32,
        missing_count: missing as i32,
        note: None,
    };
    match ranked {
        Ok(ranked) => match chosen_from(&ranked, &quality.name, &feeds, missing, missing) {
            Some(chosen) => {
                row.status = "ready";
                row.chosen = Some(chosen);
            }
            None => row.note = Some("nothing usable".to_string()),
        },
        Err(facts) => row.note = Some(none_note(&facts, ran_out)),
    }
    row
}

/// A whole album, built from the per-track candidates the hunt would make for it.
fn album_candidate(tracks: &[&WantedCandidate]) -> Option<(WantedCandidate, String)> {
    let first = tracks.first()?;
    let query = first.queries.first()?;
    let artist = query.artist.clone()?;
    let album = query.album.clone()?;
    let term = format!("{artist} {album}");
    let mut candidate = (*first).clone();
    candidate.label = format!("album '{term}'");
    candidate.match_title = WantedTitle::Music {
        artist: artist.clone(),
        album: Some(album.clone()),
        track: None,
    };
    candidate.queries = vec![SourceQuery {
        query_type: QueryType::MusicSearch,
        search_term: Some(term.clone()),
        artist: Some(artist.clone()),
        album: Some(album.clone()),
        year: query.year,
        cache: true,
        ..Default::default()
    }];
    candidate.expectation = Expectation::Album {
        artist,
        name: album,
    };
    candidate.episode_count = 1;
    candidate.timing = Timing::Back;
    candidate.linkage = CandidateLinkage {
        album_id: first.linkage.album_id.clone(),
        ..Default::default()
    };
    candidate.search_key = format!(
        "album:{}",
        first.linkage.album_id.clone().unwrap_or_default()
    );
    candidate.fallbacks = Vec::new();
    Some((candidate, term))
}

// ===========================================================================
// Running a lookup
// ===========================================================================

/// Whether a lookup is for items the hunt itself would take (flagged wanted), or any missing item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// Background: only items flagged wanted.
    Wanted,
    /// On demand: any missing item.
    AnyMissing,
}

/// What a lookup did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LookupSummary {
    pub targets: usize,
    pub ready: usize,
    pub none: usize,
}

async fn library_owner(
    db: &Database,
    cache: &mut HashMap<String, Option<String>>,
    library_id: &str,
) -> Option<String> {
    if let Some(owner) = cache.get(library_id) {
        return owner.clone();
    }
    let owner = Library::get(db.pool(), &library_id.to_string())
        .await
        .ok()
        .flatten()
        .map(|library| library.user_id);
    cache.insert(library_id.to_string(), owner.clone());
    owner
}

fn to_input(
    row: &Row,
    owner_id: &str,
    library_id: &str,
    now: DateTime<Utc>,
) -> CreateReleaseRecommendationInput {
    let (
        title,
        source_kind,
        source_id,
        source_name,
        feed_item_id,
        link,
        info_hash,
        guid,
        size,
        seeders,
        quality,
        reasons,
    ) = match &row.chosen {
        Some(chosen) => {
            let release = &chosen.release;
            let from_feed = chosen.feed_item_id.is_some();
            (
                Some(release.title.clone()),
                Some(if from_feed { "feed" } else { "indexer" }.to_string()),
                release.source_id.clone(),
                chosen.source_name.clone(),
                chosen.feed_item_id.clone(),
                if from_feed {
                    String::new()
                } else {
                    release
                        .magnet_uri
                        .clone()
                        .or_else(|| release.link.clone())
                        .unwrap_or_default()
                },
                release.info_hash.clone(),
                Some(release.guid.clone()),
                release.size,
                release.seeders,
                Some(quality_label(&chosen.parsed)),
                chosen.reasons.clone(),
            )
        }
        None => (
            None,
            None,
            None,
            None,
            None,
            String::new(),
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
        ),
    };
    let expires = match row.status {
        "searching" => now + PLACEHOLDER_TTL,
        _ => now + TTL,
    };
    CreateReleaseRecommendationInput {
        user_id: owner_id.to_string(),
        library_id: library_id.to_string(),
        scope: row.scope.to_string(),
        status: row.status.to_string(),
        target_label: row.target_label.clone(),
        show_id: row.show_id.clone(),
        season: row.season,
        episode_id: row.episode_id.clone(),
        movie_id: row.movie_id.clone(),
        album_id: row.album_id.clone(),
        track_id: row.track_id.clone(),
        audiobook_id: row.audiobook_id.clone(),
        title,
        source_kind,
        source_id,
        source_name,
        feed_item_id,
        release_link: link,
        info_hash,
        guid,
        size,
        seeders,
        quality,
        covers_count: Some(row.covers_count),
        missing_count: Some(row.missing_count),
        covers_ids: Some(
            serde_json::to_string(&row.covers_ids).unwrap_or_else(|_| "[]".to_string()),
        ),
        reasons,
        note: row.note.clone(),
        rules_version: Some(RULES_VERSION),
        found_at: now.to_rfc3339(),
        expires_at: expires.to_rfc3339(),
    }
}

/// Delete the rows a new answer replaces: everything stored for the same target and scope family.
async fn clear_for(db: &Database, row: &Row) {
    let mut filters = Vec::new();
    match row.scope {
        "season_pack" | "episode" => {
            for scope in ["season_pack", "episode"] {
                filters.push(ReleaseRecommendationWhereInput {
                    show_id: row.show_id.as_deref().map(string_eq),
                    season: row
                        .season
                        .map(|season| graphql_orm::graphql::filters::IntFilter {
                            eq: Some(season),
                            ..Default::default()
                        }),
                    scope: Some(string_eq(scope)),
                    ..Default::default()
                });
            }
        }
        "series_pack" => filters.push(ReleaseRecommendationWhereInput {
            show_id: row.show_id.as_deref().map(string_eq),
            scope: Some(string_eq("series_pack")),
            ..Default::default()
        }),
        "movie" => filters.push(ReleaseRecommendationWhereInput {
            movie_id: row.movie_id.as_deref().map(string_eq),
            ..Default::default()
        }),
        "album" | "track" => {
            for scope in ["album", "track"] {
                filters.push(ReleaseRecommendationWhereInput {
                    album_id: row.album_id.as_deref().map(string_eq),
                    scope: Some(string_eq(scope)),
                    ..Default::default()
                });
            }
        }
        "book" => filters.push(ReleaseRecommendationWhereInput {
            audiobook_id: row.audiobook_id.as_deref().map(string_eq),
            ..Default::default()
        }),
        _ => {}
    }
    for filter in filters {
        if let Err(error) = ReleaseRecommendation::delete_where(db, filter).await {
            warn!(target = %row.target_label, error = %error, "Recommendations: could not clear the old answer");
        }
    }
}

async fn store(db: &Database, rows: &[Row], owner_id: &str, library_id: &str) -> LookupSummary {
    let now = Utc::now();
    let mut summary = LookupSummary::default();
    let mut cleared: HashSet<String> = HashSet::new();
    for row in rows {
        let key = format!(
            "{}:{:?}:{:?}:{:?}:{:?}",
            row.scope, row.show_id, row.season, row.movie_id, row.album_id
        );
        if cleared.insert(format!("{key}:{:?}", row.audiobook_id)) {
            clear_for(db, row).await;
        }
        match row.status {
            "ready" => summary.ready += 1,
            "none" => summary.none += 1,
            _ => {}
        }
        if let Err(error) =
            ReleaseRecommendation::insert(db, to_input(row, owner_id, library_id, now)).await
        {
            warn!(target = %row.target_label, error = %crate::services::redact::redact_text(&error.to_string()), "Recommendations: could not store an answer");
        }
    }
    summary
}

/// Look up everything the scope names that is missing, and store the answers. `reach` says whether
/// only items flagged wanted count. Skips targets that already have an unexpired answer, unless
/// `refresh` is set.
pub async fn look_up(
    manager: &Arc<ServicesManager>,
    scope: &HuntScope,
    reach: Reach,
    refresh: bool,
    max_targets: usize,
) -> Result<LookupSummary> {
    let db = manager
        .get_database()
        .await
        .context("database service not available")?
        .pool()
        .clone();
    let settings = AutoDownloadSettings::load(&db).await;
    if settings.hourly_search_budget == 0 {
        return Ok(LookupSummary::default());
    }
    let sources_manager = match manager.get_sources().await {
        Some(service) => service.get_manager().await,
        None => None,
    };
    let blocklist = Blocklist::load(&db).await.unwrap_or_default();
    let now = Utc::now();
    let mut discovery = scope.clone();
    discovery.include_unwanted = reach == Reach::AnyMissing;
    let candidates = discover_candidates(&db, &discovery, &blocklist, &settings, now).await?;
    if candidates.is_empty() {
        return Ok(LookupSummary::default());
    }

    // Group the candidates into targets.
    let mut seasons: HashMap<(String, i32), SeasonGroup<'_>> = HashMap::new();
    let mut singles: Vec<(&WantedCandidate, &'static str)> = Vec::new();
    let mut albums: HashMap<String, Vec<&WantedCandidate>> = HashMap::new();
    for candidate in &candidates {
        match (&candidate.expectation, &candidate.linkage) {
            (Expectation::SeasonPack { season }, linkage) if linkage.show_id.is_some() => {
                let show_id = linkage.show_id.clone().unwrap_or_default();
                let group = seasons
                    .entry((show_id.clone(), *season))
                    .or_insert_with(|| SeasonGroup {
                        show_id,
                        show_name: candidate.feed_name.clone().unwrap_or_default(),
                        season: *season,
                        pack: None,
                        episodes: Vec::new(),
                    });
                group.pack = Some(candidate);
                group.episodes.extend(candidate.fallbacks.iter());
            }
            (Expectation::Episode { season, .. }, linkage) if linkage.show_id.is_some() => {
                let show_id = linkage.show_id.clone().unwrap_or_default();
                seasons
                    .entry((show_id.clone(), *season))
                    .or_insert_with(|| SeasonGroup {
                        show_id,
                        show_name: candidate.feed_name.clone().unwrap_or_default(),
                        season: *season,
                        pack: None,
                        episodes: Vec::new(),
                    })
                    .episodes
                    .push(candidate);
            }
            (_, linkage) if linkage.movie_id.is_some() => singles.push((candidate, "movie")),
            (_, linkage) if linkage.audiobook_id.is_some() => singles.push((candidate, "book")),
            (_, linkage) if linkage.track_id.is_some() => {
                albums
                    .entry(linkage.album_id.clone().unwrap_or_default())
                    .or_default()
                    .push(candidate);
            }
            _ => {}
        }
    }

    let mut owners: HashMap<String, Option<String>> = HashMap::new();
    let existing = if refresh {
        HashSet::new()
    } else {
        fresh_targets(&db, now).await
    };
    let mut summary = LookupSummary::default();
    let mut left = max_targets;

    // Seasons, then each show's series pack.
    let mut by_show: HashMap<String, Vec<&SeasonGroup<'_>>> = HashMap::new();
    let mut ordered: Vec<&SeasonGroup<'_>> = seasons.values().collect();
    ordered.sort_by(|a, b| (&a.show_id, a.season).cmp(&(&b.show_id, b.season)));
    for group in ordered {
        by_show
            .entry(group.show_id.clone())
            .or_default()
            .push(group);
        if left == 0 || existing.contains(&format!("show:{}:{}", group.show_id, group.season)) {
            continue;
        }
        let Some(library_id) = group
            .episodes
            .first()
            .or(group.pack.as_ref())
            .map(|candidate| candidate.library_id.clone())
        else {
            continue;
        };
        let Some(owner) = library_owner(&db, &mut owners, &library_id).await else {
            continue;
        };
        let env = Env {
            db: &db,
            sources: sources_manager.as_deref(),
            blocklist: &blocklist,
            settings: &settings,
            owner_id: owner.clone(),
        };
        left -= 1;
        summary.targets += 1;
        let rows = look_up_season(&env, group).await;
        let part = store(&db, &rows, &owner, &library_id).await;
        summary.ready += part.ready;
        summary.none += part.none;
    }
    for (show_id, groups) in by_show {
        let qualifying: Vec<&SeasonGroup<'_>> = groups
            .iter()
            .copied()
            .filter(|group| group.pack.is_some())
            .collect();
        if left == 0 || qualifying.len() < 2 || existing.contains(&format!("series:{show_id}")) {
            continue;
        }
        let first = qualifying[0];
        let library_id = first
            .pack
            .map(|pack| pack.library_id.clone())
            .unwrap_or_default();
        let Some(owner) = library_owner(&db, &mut owners, &library_id).await else {
            continue;
        };
        let env = Env {
            db: &db,
            sources: sources_manager.as_deref(),
            blocklist: &blocklist,
            settings: &settings,
            owner_id: owner.clone(),
        };
        left -= 1;
        summary.targets += 1;
        let row = look_up_series(&env, &first.show_name, &qualifying).await;
        let rows = match row {
            Some(row) => vec![row],
            None => vec![Row {
                scope: "series_pack",
                status: "none",
                target_label: format!("{} (complete series)", first.show_name),
                show_id: Some(show_id.clone()),
                season: None,
                episode_id: None,
                movie_id: None,
                album_id: None,
                track_id: None,
                audiobook_id: None,
                chosen: None,
                covers_ids: Vec::new(),
                covers_count: 0,
                missing_count: qualifying
                    .iter()
                    .map(|group| group.episodes.len())
                    .sum::<usize>() as i32,
                note: Some("no complete-series pack found".to_string()),
            }],
        };
        let part = store(&db, &rows, &owner, &library_id).await;
        summary.ready += part.ready;
        summary.none += part.none;
    }

    // Movies and books.
    for (candidate, scope) in singles {
        if left == 0 {
            break;
        }
        let id = candidate
            .linkage
            .movie_id
            .clone()
            .or_else(|| candidate.linkage.audiobook_id.clone())
            .unwrap_or_default();
        if existing.contains(&format!("{scope}:{id}")) {
            continue;
        }
        let Some(owner) = library_owner(&db, &mut owners, &candidate.library_id).await else {
            continue;
        };
        let env = Env {
            db: &db,
            sources: sources_manager.as_deref(),
            blocklist: &blocklist,
            settings: &settings,
            owner_id: owner.clone(),
        };
        left -= 1;
        summary.targets += 1;
        let target = CatalogueTarget {
            movie_id: candidate.linkage.movie_id.clone(),
            audiobook_id: candidate.linkage.audiobook_id.clone(),
            ..Default::default()
        };
        let label = candidate
            .label
            .split_once('\'')
            .map(|(_, rest)| rest.trim_end_matches('\'').to_string())
            .unwrap_or_else(|| candidate.label.clone());
        let row = look_up_single(
            &env,
            candidate,
            scope,
            target,
            label,
            &candidate.linkage,
            Vec::new(),
        )
        .await;
        let part = store(&db, &[row], &owner, &candidate.library_id).await;
        summary.ready += part.ready;
        summary.none += part.none;
    }

    // Albums (whole), then nothing per track unless the scope names a track.
    for (album_id, tracks) in albums {
        if left == 0 || existing.contains(&format!("album:{album_id}")) {
            continue;
        }
        let Some((candidate, term)) = album_candidate(&tracks) else {
            continue;
        };
        let Some(owner) = library_owner(&db, &mut owners, &candidate.library_id).await else {
            continue;
        };
        let env = Env {
            db: &db,
            sources: sources_manager.as_deref(),
            blocklist: &blocklist,
            settings: &settings,
            owner_id: owner.clone(),
        };
        left -= 1;
        summary.targets += 1;
        let covers = ids_of(&tracks, |linkage| linkage.track_id.as_ref());
        let target = CatalogueTarget {
            album_id: Some(album_id.clone()),
            ..Default::default()
        };
        let row = look_up_single(
            &env,
            &candidate,
            "album",
            target,
            term,
            &candidate.linkage,
            covers,
        )
        .await;
        let part = store(&db, &[row], &owner, &candidate.library_id).await;
        summary.ready += part.ready;
        summary.none += part.none;
    }
    info!(
        targets = summary.targets,
        ready = summary.ready,
        none = summary.none,
        "Recommendations: looked up {} target(s): {} with a recommendation, {} with none",
        summary.targets,
        summary.ready,
        summary.none
    );
    Ok(summary)
}

/// Keys of targets that already have an answer that has not expired: `show:<id>:<season>`,
/// `series:<id>`, `movie:<id>`, `book:<id>`, `album:<id>`.
async fn fresh_targets(db: &Database, now: DateTime<Utc>) -> HashSet<String> {
    let rows = ReleaseRecommendation::query(db.pool())
        .filter(ReleaseRecommendationWhereInput {
            expires_at: Some(DateFilter {
                gt: Some(now.to_rfc3339()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await
        .unwrap_or_default();
    let mut keys = HashSet::new();
    // A "searching" placeholder is a request for an answer, not an answer, and an answer from older
    // rules is not one either.
    for row in rows
        .into_iter()
        .filter(|row| row.status != "searching" && is_current(row))
    {
        if let (Some(show), Some(season)) = (&row.show_id, row.season) {
            keys.insert(format!("show:{show}:{season}"));
        }
        if row.scope == "series_pack"
            && let Some(show) = &row.show_id
        {
            keys.insert(format!("series:{show}"));
        }
        if let Some(movie) = &row.movie_id {
            keys.insert(format!("movie:{movie}"));
        }
        if let Some(book) = &row.audiobook_id {
            keys.insert(format!("book:{book}"));
        }
        if let Some(album) = &row.album_id {
            keys.insert(format!("album:{album}"));
        }
    }
    keys
}

// ===========================================================================
// Keeping the rows honest
// ===========================================================================

/// Delete a target's recommendations: it was grabbed (by anyone) or filled.
pub async fn forget_for_linkage(db: &Database, linkage: &CandidateLinkage) {
    let mut filters = Vec::new();
    if let Some(show) = &linkage.show_id {
        filters.push(ReleaseRecommendationWhereInput {
            show_id: Some(string_eq(show)),
            season: linkage
                .season
                .map(|season| graphql_orm::graphql::filters::IntFilter {
                    eq: Some(season),
                    ..Default::default()
                }),
            episode_id: linkage.episode_id.as_deref().map(string_eq),
            ..Default::default()
        });
    }
    let (movie, album, book, track) = (
        linkage.movie_id.as_deref(),
        linkage.album_id.as_deref(),
        linkage.audiobook_id.as_deref(),
        linkage.track_id.as_deref(),
    );
    if movie.is_some() || album.is_some() || book.is_some() || track.is_some() {
        filters.push(ReleaseRecommendationWhereInput {
            movie_id: movie.map(string_eq),
            album_id: album.map(string_eq),
            audiobook_id: book.map(string_eq),
            track_id: track.map(string_eq),
            ..Default::default()
        });
    }
    for filter in filters {
        if let Err(error) = ReleaseRecommendation::delete_where(db, filter).await {
            warn!(error = %error, "Recommendations: could not forget a grabbed target's answer");
        }
    }
}

/// Remove rows that no longer hold: expired, blocklisted, gone from the feed, or for something no
/// longer missing or already on its way.
pub async fn prune(db: &Database) -> usize {
    let now = Utc::now();
    let blocklist = Blocklist::load(db).await.unwrap_or_default();
    let rows = match ReleaseRecommendation::query(db.pool()).fetch_all().await {
        Ok(rows) => rows,
        Err(error) => {
            warn!(error = %error, "Recommendations: could not list rows to prune");
            return 0;
        }
    };
    let mut removed = 0;
    for row in rows {
        let expired = DateTime::parse_from_rfc3339(&row.expires_at)
            .map(|at| at.with_timezone(&Utc) <= now)
            .unwrap_or(true);
        let mut drop = expired;
        let mut why = "expired";
        if !drop && !is_current(&row) {
            drop = true;
            why = "it was chosen by older rules";
        }
        if !drop && row.status == "ready" {
            if row
                .info_hash
                .as_deref()
                .is_some_and(|hash| blocklist.blocks_info_hash(hash))
                || row.guid.as_deref().is_some_and(|guid| {
                    blocklist.blocks(&SourceRelease::new(String::new(), guid.to_string(), now))
                })
            {
                drop = true;
                why = "its release is blocklisted";
            } else if let Some(item_id) = &row.feed_item_id
                && RssFeedItem::get(db.pool(), item_id)
                    .await
                    .ok()
                    .flatten()
                    .is_none()
            {
                drop = true;
                why = "its release is gone from the feed";
            } else if target_is_settled(db, &row).await {
                drop = true;
                why = "what it was for is no longer missing";
            }
        }
        if drop {
            debug!(target = %row.target_label, reason = why, "Recommendations: removing the answer for '{}': {why}", row.target_label);
            if ReleaseRecommendation::delete_by_id(db, &row.id)
                .await
                .is_ok()
            {
                removed += 1;
            }
        }
    }
    removed
}

/// Whether what a recommendation is for has since been filled or taken: a file exists, or a torrent
/// that has not failed is already linked to it.
async fn target_is_settled(db: &Database, row: &ReleaseRecommendation) -> bool {
    use crate::graphql::entities::{Episode, Movie, Track};
    if let Some(id) = &row.movie_id
        && let Ok(Some(movie)) = Movie::get(db.pool(), id).await
        && movie.media_file_id.is_some()
    {
        return true;
    }
    if let Some(id) = &row.episode_id
        && let Ok(Some(episode)) = Episode::get(db.pool(), id).await
        && episode.media_file_id.is_some()
    {
        return true;
    }
    if let Some(id) = &row.track_id
        && let Ok(Some(track)) = Track::get(db.pool(), id).await
        && track.media_file_id.is_some()
    {
        return true;
    }
    // A pack or album is settled when everything it covered has a file.
    if matches!(row.scope.as_str(), "season_pack" | "series_pack" | "album")
        && let Some(json) = &row.covers_ids
        && let Ok(ids) = serde_json::from_str::<Vec<String>>(json)
        && !ids.is_empty()
    {
        let mut any_missing = false;
        for id in &ids {
            let filled = if row.scope == "album" {
                Track::get(db.pool(), id)
                    .await
                    .ok()
                    .flatten()
                    .is_none_or(|track| track.media_file_id.is_some())
            } else {
                Episode::get(db.pool(), id)
                    .await
                    .ok()
                    .flatten()
                    .is_none_or(|episode| episode.media_file_id.is_some())
            };
            if !filled {
                any_missing = true;
                break;
            }
        }
        if !any_missing {
            return true;
        }
    }
    let mut filter = TorrentWhereInput::default();
    if let Some(id) = &row.movie_id {
        filter.movie_id = Some(string_eq(id));
    } else if let Some(id) = &row.audiobook_id {
        filter.audiobook_id = Some(string_eq(id));
    } else if let Some(id) = &row.episode_id {
        filter.episode_id = Some(string_eq(id));
    } else if let Some(id) = &row.track_id {
        filter.track_id = Some(string_eq(id));
    } else if let Some(id) = &row.show_id {
        filter.show_id = Some(string_eq(id));
        if let Some(season) = row.season {
            filter.season = Some(graphql_orm::graphql::filters::IntFilter {
                eq: Some(season),
                ..Default::default()
            });
        }
    } else if let Some(id) = &row.album_id {
        filter.album_id = Some(string_eq(id));
    } else {
        return false;
    }
    Torrent::query(db.pool())
        .filter(filter)
        .fetch_all()
        .await
        .map(|rows| {
            rows.iter()
                .any(|torrent| torrent.post_process_status.as_deref() != Some("failed"))
        })
        .unwrap_or(false)
}

// ===========================================================================
// Asking for a lookup, and grabbing a recommendation
// ===========================================================================

/// What asking for a lookup did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requested {
    /// An answer that has not expired is already stored.
    Cached,
    /// Queued; its page shows "looking" meanwhile.
    Queued,
    /// The same lookup is already waiting.
    AlreadyWaiting,
}

fn request_scope(request: &LookupRequest) -> Option<&'static str> {
    if request.movie_id.is_some() {
        Some("movie")
    } else if request.audiobook_id.is_some() {
        Some("book")
    } else if request.album_id.is_some() {
        Some("album")
    } else if request.show_id.is_some() && request.season.is_some() {
        Some("season_pack")
    } else if request.show_id.is_some() {
        Some("series_pack")
    } else {
        None
    }
}

/// Queue a lookup for what a person opened: one season, movie, album or book. A stored answer is
/// kept (and nothing is searched) until it expires; otherwise a "searching" placeholder is stored
/// so the page can say so, and the worker answers within the hourly budget.
pub async fn request_lookup(
    db: &Database,
    owner_id: &str,
    library_id: &str,
    label: String,
    request: LookupRequest,
) -> Result<Requested> {
    let scope = request_scope(&request).context("a lookup needs a season, movie, album or book")?;
    let existing = ReleaseRecommendation::query(db.pool())
        .filter(ReleaseRecommendationWhereInput {
            show_id: request.show_id.as_deref().map(string_eq),
            season: request
                .season
                .map(|season| graphql_orm::graphql::filters::IntFilter {
                    eq: Some(season),
                    ..Default::default()
                }),
            movie_id: request.movie_id.as_deref().map(string_eq),
            album_id: request.album_id.as_deref().map(string_eq),
            audiobook_id: request.audiobook_id.as_deref().map(string_eq),
            // A whole-show request is about the complete-series pack, not the seasons' own rows.
            scope: (scope == "series_pack").then(|| string_eq("series_pack")),
            expires_at: Some(DateFilter {
                gt: Some(Utc::now().to_rfc3339()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?;
    if existing
        .iter()
        .any(|row| row.status != "searching" && is_current(row))
    {
        return Ok(Requested::Cached);
    }
    if !enqueue(request.clone()) || existing.iter().any(|row| row.status == "searching") {
        return Ok(Requested::AlreadyWaiting);
    }
    let row = Row {
        scope,
        status: "searching",
        target_label: label,
        show_id: request.show_id.clone(),
        season: request.season,
        episode_id: None,
        movie_id: request.movie_id.clone(),
        album_id: request.album_id.clone(),
        track_id: None,
        audiobook_id: request.audiobook_id.clone(),
        chosen: None,
        covers_ids: Vec::new(),
        covers_count: 0,
        missing_count: 0,
        note: None,
    };
    store(db, &[row], owner_id, library_id).await;
    Ok(Requested::Queued)
}

/// Remove the "searching" placeholders of a lookup that found nothing to look up (the item is no
/// longer missing).
async fn drop_placeholders(db: &Database, request: &LookupRequest) {
    let filter = ReleaseRecommendationWhereInput {
        show_id: request.show_id.as_deref().map(string_eq),
        season: request
            .season
            .map(|season| graphql_orm::graphql::filters::IntFilter {
                eq: Some(season),
                ..Default::default()
            }),
        movie_id: request.movie_id.as_deref().map(string_eq),
        album_id: request.album_id.as_deref().map(string_eq),
        audiobook_id: request.audiobook_id.as_deref().map(string_eq),
        scope: (request.show_id.is_some() && request.season.is_none())
            .then(|| string_eq("series_pack")),
        status: Some(string_eq("searching")),
        ..Default::default()
    };
    if let Err(error) = ReleaseRecommendation::delete_where(db, filter).await {
        warn!(error = %error, "Recommendations: could not remove a placeholder");
    }
}

/// The result of a grab.
#[derive(Debug, Clone)]
pub struct GrabOutcome {
    pub torrent_id: Option<String>,
    pub torrent_name: String,
}

/// The files of a pack that fill what the recommendation covered, if "download only the missing
/// files" is on and the release is a pack or an album.
async fn wanted_files_for(
    db: &Database,
    rec: &ReleaseRecommendation,
    settings: &AutoDownloadSettings,
) -> Option<crate::services::torrent::partial::WantedFiles> {
    use crate::graphql::entities::{Episode, Track};
    use crate::services::torrent::partial::WantedFiles;
    if !settings.only_missing_files
        || !matches!(rec.scope.as_str(), "season_pack" | "series_pack" | "album")
    {
        return None;
    }
    let ids: Vec<String> = serde_json::from_str(rec.covers_ids.as_deref()?).ok()?;
    let mut wanted = WantedFiles::default();
    for id in ids {
        if rec.scope == "album" {
            if let Ok(Some(track)) = Track::get(db.pool(), &id).await {
                wanted.tracks.push((track.track_number, track.title));
            }
        } else if let Ok(Some(episode)) = Episode::get(db.pool(), &id).await {
            wanted.episodes.push((episode.season, episode.episode));
        }
    }
    (!wanted.is_empty()).then_some(wanted)
}

/// Grab a recommendation through the same path the hunt uses (`grab_release_with`), carrying the
/// target's linkage so it counts as downloading for it. `actor_user_id` is checked against the
/// recommendation's owner unless `is_admin`.
pub async fn grab_recommendation(
    manager: &Arc<ServicesManager>,
    recommendation_id: &str,
    actor_user_id: &str,
    is_admin: bool,
) -> Result<GrabOutcome> {
    use super::auto_download::{GrabEnv, grab_release_with, hunt_lock};
    use super::auto_trail::{GrabRecord, Trail, Trigger};

    let deny = || anyhow::anyhow!("Recommendation not found or access denied");
    let db = manager
        .get_database()
        .await
        .context("database service not available")?
        .pool()
        .clone();
    let rec = ReleaseRecommendation::get(db.pool(), &recommendation_id.to_string())
        .await?
        .ok_or_else(deny)?;
    if !is_admin && rec.user_id != actor_user_id {
        return Err(deny());
    }
    anyhow::ensure!(rec.status == "ready", "That recommendation is not ready");
    anyhow::ensure!(
        is_current(&rec),
        "That recommendation was chosen by older rules and is being looked up again"
    );
    let title = rec
        .title
        .clone()
        .context("the recommendation has no release")?;

    // The link: a feed release is read from its catalogue item, an indexer release from the link
    // stored with it. Neither reaches a client.
    let mut release = SourceRelease::new(
        title.clone(),
        rec.guid.clone().unwrap_or_else(|| title.clone()),
        Utc::now(),
    );
    release.info_hash = rec.info_hash.clone();
    release.size = rec.size;
    release.seeders = rec.seeders;
    let mut feed_id: Option<String> = None;
    let link = if let Some(item_id) = &rec.feed_item_id {
        let item = RssFeedItem::get(db.pool(), item_id)
            .await?
            .context("the release is no longer in its feed")?;
        feed_id = Some(item.feed_id.clone());
        super::rss_poll::reveal_item_link(manager, &item.link).await?
    } else {
        release.source_id = rec.source_id.clone();
        release.source_name = rec.source_name.clone();
        let sources = manager
            .get_sources()
            .await
            .context("sources service not available")?
            .get_manager()
            .await
            .context("source credentials are not available")?;
        sources
            .encryption()
            .reveal(&rec.release_link)
            .context("the stored link could not be decrypted")?
    };
    anyhow::ensure!(!link.is_empty(), "the recommendation has no download link");
    if link.to_ascii_lowercase().starts_with("magnet:") {
        release.magnet_uri = Some(link);
    } else {
        release.link = Some(link);
    }

    let linkage = CandidateLinkage {
        episode_id: rec.episode_id.clone(),
        movie_id: rec.movie_id.clone(),
        track_id: rec.track_id.clone(),
        chapter_id: None,
        show_id: rec.show_id.clone(),
        album_id: rec.album_id.clone(),
        audiobook_id: rec.audiobook_id.clone(),
        season: if rec.scope == "season_pack" {
            rec.season
        } else {
            None
        },
    };
    let candidate = WantedCandidate {
        label: rec.target_label.clone(),
        library_id: rec.library_id.clone(),
        queries: Vec::new(),
        match_title: WantedTitle::work(rec.target_label.clone()),
        linkage: linkage.clone(),
        quality_profile_override_id: None,
        search_key: format!("recommendation:{}", rec.id),
        episode_count: rec.covers_count.unwrap_or(1).max(1),
        timing: Timing::Back,
        expectation: Expectation::Unchecked,
        feed_name: None,
        fallbacks: Vec::new(),
    };
    let settings = AutoDownloadSettings::load(&db).await;
    let wanted = wanted_files_for(&db, &rec, &settings).await;
    let env = GrabEnv {
        db: db.clone(),
        torrent: manager
            .get_torrent()
            .await
            .context("torrent service not available")?,
        manager: manager.clone(),
    };
    // One search-and-grab at a time, so this cannot race a hunt choosing a release for the same item.
    let _hunting = hunt_lock().await;
    let grabbed = grab_release_with(&env, &candidate, &release, feed_id.as_deref(), wanted)
        .await?
        .context("the download client already has that torrent")?;
    let _ = ReleaseRecommendation::delete_by_id(&db, &rec.id).await;
    forget_for_linkage(&db, &linkage).await;
    Trail::new(&db)
        .grabbed(&GrabRecord {
            library_id: &rec.library_id,
            label: &rec.target_label,
            release: &title,
            source: rec.source_name.as_deref(),
            profile: "a recommendation",
            trigger: &Trigger::Manual,
            torrent_id: grabbed.torrent_id.as_deref(),
        })
        .await;
    Ok(GrabOutcome {
        torrent_id: grabbed.torrent_id,
        torrent_name: grabbed.torrent_name,
    })
}

// ===========================================================================
// The worker
// ===========================================================================

static LAST_BACKGROUND: Lazy<StdMutex<Option<DateTime<Utc>>>> = Lazy::new(|| StdMutex::new(None));

/// One tick of the worker: prune, answer the lookups people queued, and (every half hour) look up
/// what is flagged wanted. All within the hourly search budget.
pub async fn work(manager: &Arc<ServicesManager>) -> Result<()> {
    let db = manager
        .get_database()
        .await
        .context("database service not available")?
        .pool()
        .clone();
    prune(&db).await;
    let settings = AutoDownloadSettings::load(&db).await;
    if settings.hourly_search_budget == 0 {
        // Lookups are off: a queued request would wait forever.
        while next_request().is_some() {}
        return Ok(());
    }
    while budget_remaining(settings.hourly_search_budget) > 0
        && let Some(request) = next_request()
    {
        let scope = HuntScope {
            show_id: request.show_id.clone(),
            season: request.season,
            movie_id: request.movie_id.clone(),
            album_id: request.album_id.clone(),
            audiobook_id: request.audiobook_id.clone(),
            ignore_backoff: true,
            ..Default::default()
        };
        // A whole show looks at each of its seasons and then the series pack.
        let targets = if request.show_id.is_some() && request.season.is_none() {
            12
        } else {
            4
        };
        if let Err(error) = look_up(manager, &scope, Reach::AnyMissing, false, targets).await {
            warn!(error = %crate::services::redact::redact_text(&format!("{error:#}")), "Recommendations: a queued lookup failed");
        }
        // Whatever the lookup replaced is no longer a placeholder; one it never reached (the item
        // is no longer missing, or the show has no season worth a pack) must not say "looking".
        drop_placeholders(&db, &request).await;
    }
    let due = LAST_BACKGROUND
        .lock()
        .map(|last| last.is_none_or(|at| Utc::now() - at >= BACKGROUND_EVERY))
        .unwrap_or(false);
    if due && budget_remaining(settings.hourly_search_budget) > 0 {
        if let Ok(mut last) = LAST_BACKGROUND.lock() {
            *last = Some(Utc::now());
        }
        look_up(
            manager,
            &HuntScope::default(),
            Reach::Wanted,
            false,
            BACKGROUND_TARGETS_PER_PASS,
        )
        .await?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "recommend_tests.rs"]
mod tests;
