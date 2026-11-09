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
    collect_releases_from, describe_no_grab, discover_candidates, explain_profile_rejections,
    rank_for_candidate,
};
use super::auto_schedule::{Expectation, Timing};
use super::auto_settings::AutoDownloadSettings;
use super::recommend_music::{
    MissingTrack, MusicClaim, albums_of, claimed_tracks, describe_tracks, download_note,
    music_claim, release_kind,
};
use super::recommend_plan::{
    Claim, CoverPlan, MissingEpisode, PlanOption, claim_of, claimed_cover, describe_items,
    plan_cover, seasons_of,
};
use crate::services::quality::scoring::parse_release;
use crate::services::torrent::inspect::{self, Inspection};
use crate::services::torrent::judge::{self, ContentsVerdict, WantedItems};

/// How long a recommendation, or a "nothing found", is kept before the lookup is repeated.
pub const TTL: Duration = Duration::hours(24);
/// The version of the rules that choose a recommendation. Raise it whenever a change means stored
/// answers may be wrong: rows from an older version are removed on the next pass and looked up
/// again, instead of standing for up to a day. 2: titles are matched word for word (Q98), so
/// `Babylon Berlin` is no longer recommended for `Babylon 5`. 3: a show's answer is a coverage
/// plan (Q101), and releases are judged by their files (Q100). 4: music is an artist's coverage
/// plan (Q110), and bundled subtitles are a reason and a tiebreak (Q114).
pub const RULES_VERSION: i32 = 4;

/// Whether a stored answer was chosen by the current rules.
pub fn is_current(row: &ReleaseRecommendation) -> bool {
    row.rules_version.unwrap_or(0) >= RULES_VERSION
}
/// How long a "searching" placeholder lives if its lookup never finishes.
const PLACEHOLDER_TTL: Duration = Duration::minutes(60);
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

/// Spend `requests` from the hourly search budget if they fit, for searches made outside a lookup:
/// the air-time pass (Q113). One budget covers recommendations and air-time searches, so neither
/// can burst past `auto_download.hourly_search_budget`.
pub(crate) fn take_search_budget(requests: u32, per_hour: u32) -> bool {
    let Ok(mut budget) = BUDGET.lock() else {
        return false;
    };
    let now = Utc::now();
    if budget.remaining(per_hour, now) < requests {
        return false;
    }
    budget.spend(requests, now);
    true
}

/// What the owner's feeds already list for a hunt target (a show's episode or pack, a movie), for
/// the air-time pass to try before any indexer is asked (Q113).
pub(crate) async fn catalogue_for_candidate(
    db: &Database,
    candidate: &WantedCandidate,
    blocklist: &Blocklist,
) -> Collected {
    let target = CatalogueTarget {
        show_id: candidate.linkage.show_id.clone(),
        movie_id: candidate.linkage.movie_id.clone(),
        ..Default::default()
    };
    if target.show_id.is_none() && target.movie_id.is_none() {
        return Collected::default();
    }
    let Some(owner) = Library::get(db.pool(), &candidate.library_id)
        .await
        .ok()
        .flatten()
        .map(|library| library.user_id)
    else {
        return Collected::default();
    };
    catalogue_releases(db, &owner, &target, blocklist).await.0
}

/// Add `more` to `collected`, without repeating a release.
pub(crate) fn merge_collected(collected: &mut Collected, more: Collected) {
    merge(collected, more)
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
    /// An artist: answered by the artist's coverage plan (Q110). Album and track lookups are
    /// asked as their artist's.
    pub artist_id: Option<String>,
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
#[derive(Debug, Clone, Default)]
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
    /// Seasons the covered episodes are in (shows only).
    covers_seasons: Vec<i32>,
    /// Where a release comes in its plan.
    plan_position: Option<i32>,
    /// A plan row's own reasons (a release row's come with its release).
    plan_reasons: Vec<String>,
    note: Option<String>,
    /// An artist's plan and its releases (Q110).
    artist_id: Option<String>,
    /// Albums of the missing tracks covered (music plans).
    covers_albums: Vec<String>,
    /// Bytes of exactly the files to download, when only some are wanted.
    selected_size: Option<i64>,
    /// "Downloads 1 track (8 MB) from a 600 MB album".
    download_note: Option<String>,
}

#[derive(Debug, Clone)]
struct Chosen {
    release: SourceRelease,
    parsed: ParsedRelease,
    reasons: Vec<String>,
    feed_item_id: Option<String>,
    source_name: Option<String>,
    /// Exactly the files to download, from an inspection.
    file_indices: Vec<usize>,
}

/// Where a lookup is made and what it knows.
struct Env<'a> {
    db: &'a Database,
    /// For looking into torrents (Q99); `None` in pure tests.
    manager: Option<&'a Arc<ServicesManager>>,
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
        file_indices: Vec::new(),
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

/// Most packs looked into (Q99) for one show's plan.
const INSPECT_PER_PLAN: usize = 4;
/// Most releases one plan lists.
const MAX_PLAN_RELEASES: usize = 60;

/// A release that could go into a show's plan.
#[derive(Debug, Clone)]
struct ShowOption {
    scored: ScoredRelease,
    claim: Claim,
    /// Indexes of the missing episodes it covers: by name, or by its files once inspected.
    covers: Vec<usize>,
    /// What its files said, when it was looked into.
    verdict: Option<ContentsVerdict>,
}

/// Where a show's plan stands after one round of gathering.
#[derive(Debug, Default)]
struct ShowEvaluation {
    options: Vec<ShowOption>,
    plan: CoverPlan,
    /// Named after the show but saying no season or episode.
    unnumbered: usize,
    unnumbered_example: Option<String>,
    /// Turned down by their files.
    rejected_by_files: Vec<String>,
}

fn missing_pairs(missing: &[MissingEpisode], items: &[usize]) -> Vec<(i32, i32)> {
    items
        .iter()
        .filter_map(|index| missing.get(*index))
        .map(|item| (item.season, item.episode))
        .collect()
}

/// Say what each gathered release covers, look into the most promising packs, and plan.
async fn evaluate_show(
    env: &Env<'_>,
    show_title: &WantedTitle,
    quality: &crate::graphql::entities::QualityProfile,
    collected: &Collected,
    missing: &[MissingEpisode],
    inspected: &mut HashMap<String, Inspection>,
) -> ShowEvaluation {
    let mut evaluation = ShowEvaluation::default();
    let mut options: Vec<ShowOption> = Vec::new();
    for release in &collected.releases {
        if show_title.mismatch(&release.title).is_some() {
            continue;
        }
        let parsed = parse_release(&release.title);
        let Some(claim) = claim_of(&parsed, &release.title) else {
            evaluation.unnumbered += 1;
            evaluation.unnumbered_example.get_or_insert_with(|| {
                format!(
                    "'{}' does not say which season or episode it is",
                    release.title
                )
            });
            continue;
        };
        let covers = claimed_cover(&claim, missing);
        if covers.is_empty() && claim != Claim::Vague {
            continue;
        }
        let count = if claim == Claim::Vague {
            missing.len()
        } else {
            covers.len()
        };
        let Some(scored) = profile::filter_and_rank(
            show_title,
            vec![release.clone()],
            quality,
            count.max(1) as i32,
        )
        .into_iter()
        .next() else {
            continue;
        };
        options.push(ShowOption {
            scored,
            claim,
            covers,
            verdict: None,
        });
    }
    // The profile's order, so `rank` means the same as everywhere else.
    let mut ranked: Vec<ScoredRelease> =
        options.iter().map(|option| option.scored.clone()).collect();
    profile::sort_by_profile(&mut ranked, quality);
    let rank_of = |guid: &str| {
        ranked
            .iter()
            .position(|scored| scored.release.guid == guid)
            .unwrap_or(usize::MAX)
    };

    // Look into the packs that would cover most, best ranked first, within the budget.
    let mut packs: Vec<&ShowOption> = options
        .iter()
        .filter(|option| option.claim.is_pack())
        .collect();
    packs.sort_by_key(|option| {
        (
            std::cmp::Reverse(if option.claim == Claim::Vague {
                missing.len()
            } else {
                option.covers.len()
            }),
            rank_of(&option.scored.release.guid),
        )
    });
    for option in packs {
        if inspected.len() >= INSPECT_PER_PLAN {
            break;
        }
        let guid = option.scored.release.guid.clone();
        if inspected.contains_key(&guid) {
            continue;
        }
        let reading = match env.manager {
            Some(manager) => {
                inspect::inspect_release(
                    manager,
                    env.db,
                    &option.scored.release,
                    env.settings.hourly_search_budget,
                )
                .await
            }
            None => Inspection::Unavailable("the torrent client is not available".to_string()),
        };
        inspected.insert(guid, reading);
    }

    // Files beat names.
    let everything: Vec<usize> = (0..missing.len()).collect();
    let all_pairs = missing_pairs(missing, &everything);
    let mut kept: Vec<ShowOption> = Vec::new();
    for mut option in options {
        let reading = inspected
            .get(&option.scored.release.guid)
            .and_then(Inspection::contents);
        match reading {
            Some(contents) => {
                let default_season = match option.claim {
                    Claim::Seasons(first, last) if first == last => Some(first),
                    Claim::Episodes(season, _) => Some(season),
                    _ => None,
                };
                let verdict = judge::judge(
                    contents,
                    &WantedItems::Episodes {
                        show: show_title.clone(),
                        episodes: all_pairs.clone(),
                        default_season,
                    },
                );
                if let Some(why) = &verdict.reject {
                    debug!(release = %option.scored.release.title, reason = %why, "Recommendations: '{}' turned down by its files: {why}", option.scored.release.title);
                    evaluation
                        .rejected_by_files
                        .push(format!("'{}' {why}", option.scored.release.title));
                    continue;
                }
                option.covers = verdict.covers.clone();
                option.verdict = Some(verdict);
            }
            // A pack that names no season is used only once its files say what it holds.
            None if option.claim == Claim::Vague => continue,
            None => {}
        }
        if !option.covers.is_empty() {
            kept.push(option);
        }
    }
    let plan_options: Vec<PlanOption> = kept
        .iter()
        .map(|option| PlanOption {
            covers: option.covers.clone(),
            tier: profile::resolution_tier(&option.scored.parsed, quality),
            seeders: option.scored.release.seeders,
            size: option.scored.release.size,
            rank: rank_of(&option.scored.release.guid),
            subtitles: option
                .verdict
                .as_ref()
                .is_some_and(|verdict| !verdict.subtitle_languages.is_empty()),
        })
        .collect();
    evaluation.plan = plan_cover(missing.len(), &plan_options);
    evaluation.options = kept;
    evaluation
}

/// The candidate a complete-series search is made with, from a season's.
fn series_search(base: &WantedCandidate, show_name: &str, missing: usize) -> WantedCandidate {
    let template = base.queries.first().cloned().unwrap_or_default();
    let mut candidate = base.clone();
    let complete = format!("{show_name} Complete Series");
    let short = format!("{show_name} Complete");
    candidate.label = format!("complete series '{show_name}'");
    candidate.queries = [complete, short]
        .into_iter()
        .map(|term| SourceQuery {
            search_term: Some(term),
            season: None,
            episode: None,
            ..template.clone()
        })
        .collect();
    candidate.expectation = Expectation::SeriesPack;
    candidate.episode_count = missing as i32;
    candidate
}

/// The coverage plan for a show's missing episodes (Q101): the feed catalogue first, then a
/// complete-series search (when two or more seasons are missing) and the season searches for
/// seasons still uncovered, within the hourly budget, looking into the most promising packs on
/// the way. Returns the plan's row and one row per release in it.
async fn look_up_show(
    env: &Env<'_>,
    show_id: &str,
    show_name: &str,
    groups: &[&SeasonGroup<'_>],
) -> Option<(Row, Vec<Row>)> {
    let mut missing: Vec<MissingEpisode> = Vec::new();
    for group in groups {
        for episode in &group.episodes {
            if let (Some(id), Expectation::Episode { season, episode }) =
                (&episode.linkage.episode_id, &episode.expectation)
                && !missing.iter().any(|item| &item.id == id)
            {
                missing.push(MissingEpisode {
                    id: id.clone(),
                    season: *season,
                    episode: *episode,
                });
            }
        }
    }
    missing.sort_by_key(|item| (item.season, item.episode));
    if missing.is_empty() {
        return None;
    }
    let mut seasons: Vec<&SeasonGroup<'_>> = groups.to_vec();
    seasons.sort_by_key(|group| (std::cmp::Reverse(group.episodes.len()), group.season));
    let base = seasons.iter().find_map(|group| group.search_candidate())?;
    let quality = resolve_profile(env, &base).await;
    let show_title = base.match_title.clone();
    let target = CatalogueTarget {
        show_id: Some(show_id.to_string()),
        ..Default::default()
    };
    let (mut collected, feeds) =
        catalogue_releases(env.db, &env.owner_id, &target, env.blocklist).await;

    let mut searches: Vec<WantedCandidate> = Vec::new();
    let season_count = seasons_of(&missing, &(0..missing.len()).collect::<Vec<_>>()).len();
    if season_count >= 2 {
        searches.push(series_search(&base, show_name, missing.len()));
    }
    searches.extend(seasons.iter().filter_map(|group| group.search_candidate()));

    let mut inspected: HashMap<String, Inspection> = HashMap::new();
    let mut ran_out = false;
    let mut evaluation = evaluate_show(
        env,
        &show_title,
        &quality,
        &collected,
        &missing,
        &mut inspected,
    )
    .await;
    for search in &searches {
        if evaluation.plan.uncovered.is_empty() {
            break;
        }
        if let Expectation::SeasonPack { season } = search.expectation
            && !seasons_of(&missing, &evaluation.plan.uncovered).contains(&season)
        {
            continue;
        }
        let requests = search.queries.len() as u32;
        match env.sources {
            Some(sources) if budget_remaining(env.settings.hourly_search_budget) >= requests => {
                budget_spend(requests);
                let searched = collect_releases_from(sources, env.blocklist, search).await;
                merge(&mut collected, searched);
            }
            Some(_) => {
                ran_out = true;
                break;
            }
            None => break,
        }
        evaluation = evaluate_show(
            env,
            &show_title,
            &quality,
            &collected,
            &missing,
            &mut inspected,
        )
        .await;
    }

    let plan = &evaluation.plan;
    let covered_ids: Vec<String> = plan
        .releases
        .iter()
        .flat_map(|planned| planned.adds.iter().map(|index| missing[*index].id.clone()))
        .collect();
    let mut header = Row {
        scope: "plan",
        status: if plan.releases.is_empty() {
            "none"
        } else {
            "ready"
        },
        target_label: show_name.to_string(),
        show_id: Some(show_id.to_string()),
        season: None,
        episode_id: None,
        movie_id: None,
        album_id: None,
        track_id: None,
        audiobook_id: None,
        chosen: None,
        covers_ids: covered_ids,
        covers_count: plan.covered() as i32,
        missing_count: missing.len() as i32,
        covers_seasons: seasons_of(&missing, &(0..missing.len()).collect::<Vec<_>>()),
        plan_position: None,
        plan_reasons: Vec::new(),
        note: None,
        ..Default::default()
    };
    let budget_note = "the hourly search budget is used up, so some seasons were not searched";
    if plan.releases.is_empty() {
        let rejections = explain_profile_rejections(&show_title, &collected.releases, &quality, 1);
        let facts = NoGrabFacts {
            sources: collected.sources,
            errors: collected.errors.clone(),
            seen: collected.seen,
            blocklisted: collected.blocklisted,
            wrong_target: evaluation.unnumbered,
            wrong_example: evaluation.unnumbered_example.clone(),
            too_different: rejections.too_different,
            too_different_example: rejections.too_different_example,
            below_profile: rejections.below,
            profile: quality.name.clone(),
            profile_reasons: rejections.reasons,
        };
        let mut note = describe_no_grab(&facts);
        if !evaluation.rejected_by_files.is_empty() {
            note = format!(
                "{note}; {} turned down by their files (e.g. {})",
                evaluation.rejected_by_files.len(),
                evaluation.rejected_by_files[0]
            );
        }
        if ran_out {
            note = format!("{note}; {budget_note}");
        }
        header.note = Some(note);
        return Some((header, Vec::new()));
    }

    let mut packs = 0usize;
    let mut episodes = 0usize;
    let mut members: Vec<Row> = Vec::new();
    for (position, planned) in plan.releases.iter().take(MAX_PLAN_RELEASES).enumerate() {
        let option = &evaluation.options[planned.option];
        let adds = &planned.adds;
        let adds_seasons = seasons_of(&missing, adds);
        if option.claim.is_pack() {
            packs += 1;
        } else {
            episodes += 1;
        }
        // Exactly the files for what this release adds, when its files were read.
        let file_indices = match inspected
            .get(&option.scored.release.guid)
            .and_then(Inspection::contents)
        {
            Some(contents) if option.claim.is_pack() => {
                judge::judge(
                    contents,
                    &WantedItems::Episodes {
                        show: show_title.clone(),
                        episodes: missing_pairs(&missing, adds),
                        default_season: match option.claim {
                            Claim::Seasons(first, last) if first == last => Some(first),
                            _ => None,
                        },
                    },
                )
                .files
            }
            _ => Vec::new(),
        };
        let mut reasons = vec![format!(
            "{} fits the '{}' profile",
            quality_label(&option.scored.parsed),
            quality.name
        )];
        if let Some(seeders) = option.scored.release.seeders {
            reasons.push(format!("{seeders} seeders"));
        }
        match &option.verdict {
            Some(verdict) => {
                reasons.push(match &verdict.note {
                    Some(note) => format!("Files checked: {note}"),
                    None => "Files checked".to_string(),
                });
                reasons.extend(crate::services::torrent::subtitles::describe_languages(
                    &verdict.subtitle_languages,
                ));
            }
            None if option.claim.is_pack() => {
                reasons.push("Files not checked: judged by its name".to_string())
            }
            None => {}
        }
        if let Some(feed) = feeds.get(&option.scored.release.guid) {
            reasons.push(format!("Already listed by feed '{feed}': no search needed"));
        }
        reasons.push(format!("Adds {} of {} missing", adds.len(), missing.len()));
        let label = match &option.claim {
            Claim::Episodes(season, numbers) => numbers
                .iter()
                .map(|number| format!("S{season:02}E{number:02}"))
                .collect::<Vec<_>>()
                .join(" "),
            Claim::Seasons(first, last) if first == last => format!("Season {first}"),
            Claim::Seasons(first, last) => format!("Seasons {first}–{last}"),
            Claim::Everything | Claim::Vague => "Complete series".to_string(),
        };
        members.push(Row {
            scope: option.claim.scope(),
            status: "ready",
            target_label: format!("{show_name} {label}"),
            show_id: Some(show_id.to_string()),
            season: (adds_seasons.len() == 1).then(|| adds_seasons[0]),
            episode_id: (!option.claim.is_pack() && adds.len() == 1)
                .then(|| missing[adds[0]].id.clone()),
            movie_id: None,
            album_id: None,
            track_id: None,
            audiobook_id: None,
            chosen: Some(Chosen {
                release: option.scored.release.clone(),
                parsed: option.scored.parsed.clone(),
                reasons,
                feed_item_id: option
                    .scored
                    .release
                    .guid
                    .strip_prefix(FEED_ITEM_GUID_PREFIX)
                    .map(str::to_string),
                source_name: option.scored.release.source_name.clone(),
                file_indices,
            }),
            covers_ids: adds
                .iter()
                .map(|index| missing[*index].id.clone())
                .collect(),
            covers_count: adds.len() as i32,
            missing_count: missing.len() as i32,
            covers_seasons: adds_seasons,
            plan_position: Some(position as i32),
            plan_reasons: Vec::new(),
            note: None,
            ..Default::default()
        });
    }
    let mut summary = Vec::new();
    if packs > 0 {
        summary.push(format!("{packs} pack{}", if packs == 1 { "" } else { "s" }));
    }
    if episodes > 0 {
        summary.push(format!(
            "{episodes} single episode{}",
            if episodes == 1 { "" } else { "s" }
        ));
    }
    let mut reasons = vec![format!(
        "{} cover {} of {} missing at the best quality found",
        summary.join(" and "),
        plan.covered(),
        missing.len()
    )];
    if !evaluation.rejected_by_files.is_empty() {
        reasons.push(format!(
            "{} turned down by their files (e.g. {})",
            evaluation.rejected_by_files.len(),
            evaluation.rejected_by_files[0]
        ));
    }
    header.plan_reasons = reasons;
    if !plan.uncovered.is_empty() {
        let mut note = format!(
            "Still missing: {} (no usable release found)",
            describe_items(&missing, &plan.uncovered)
        );
        if ran_out {
            note = format!("{note}; {budget_note}");
        }
        header.note = Some(note);
    }
    Some((header, members))
}

// ---- Movie, book, album ----------------------------------------------------

#[allow(clippy::too_many_arguments)]
async fn look_up_single(
    env: &Env<'_>,
    candidate: &WantedCandidate,
    scope: &'static str,
    target: CatalogueTarget,
    label: String,
    linkage: &CandidateLinkage,
    covers: Vec<String>,
    items: Option<WantedItems>,
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
        covers_seasons: Vec::new(),
        plan_position: None,
        plan_reasons: Vec::new(),
        note: None,
        ..Default::default()
    };
    match ranked {
        Ok(mut ranked) => {
            let mut turned_down: Vec<String> = Vec::new();
            // Its files must be what is wanted (an album's tracks, a book's audio): the pick is
            // looked into, and the next one if it is turned down.
            let mut looked = 0;
            let chosen = loop {
                let Some(mut chosen) =
                    chosen_from(&ranked, &quality.name, &feeds, missing, missing)
                else {
                    break None;
                };
                let (Some(items), Some(manager)) = (&items, env.manager) else {
                    break Some(chosen);
                };
                if looked >= SINGLE_INSPECTIONS {
                    break Some(chosen);
                }
                looked += 1;
                let reading = inspect::inspect_release(
                    manager,
                    env.db,
                    &chosen.release,
                    env.settings.hourly_search_budget,
                )
                .await;
                let Some(contents) = reading.contents() else {
                    break Some(chosen);
                };
                let verdict = judge::judge(contents, items);
                if let Some(why) = verdict.reject {
                    turned_down.push(format!("'{}' {why}", chosen.release.title));
                    let guid = chosen.release.guid.clone();
                    ranked.retain(|scored| scored.release.guid != guid);
                    continue;
                }
                chosen.reasons.push(match &verdict.note {
                    Some(note) => format!("Files checked: {note}"),
                    None => "Files checked".to_string(),
                });
                chosen
                    .reasons
                    .extend(crate::services::torrent::subtitles::describe_languages(
                        &verdict.subtitle_languages,
                    ));
                chosen.file_indices = verdict.files;
                break Some(chosen);
            };
            match chosen {
                Some(chosen) => {
                    row.status = "ready";
                    row.chosen = Some(chosen);
                }
                None if !turned_down.is_empty() => {
                    row.note = Some(format!(
                        "{} release(s) turned down by their files: {}",
                        turned_down.len(),
                        turned_down.join("; ")
                    ))
                }
                None => row.note = Some("nothing usable".to_string()),
            }
        }
        Err(facts) => row.note = Some(none_note(&facts, ran_out)),
    }
    row
}

/// Releases of one movie, album or book looked into before it is recommended.
const SINGLE_INSPECTIONS: usize = 2;

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

// ---- Music: an artist's coverage plan (Q110) ---------------------------------

/// One album of an artist with missing tracks.
struct AlbumGroup<'a> {
    album_id: String,
    name: String,
    /// Tracks the album has in all (to tell a whole album from part of one).
    track_count: Option<i32>,
    tracks: Vec<&'a WantedCandidate>,
}

/// Most releases looked into (Q99) for one artist's plan.
const MUSIC_INSPECT_PER_PLAN: usize = 6;
/// Most single-track searches one artist's plan makes, for tracks nothing else covers.
const MAX_TRACK_SEARCHES: usize = 6;

/// A release that could go into an artist's plan.
#[derive(Debug, Clone)]
struct MusicOption {
    scored: ScoredRelease,
    claim: MusicClaim,
    /// Indexes of the missing tracks it covers: by name, or by its files once inspected.
    covers: Vec<usize>,
    verdict: Option<ContentsVerdict>,
}

#[derive(Debug, Default)]
struct MusicEvaluation {
    options: Vec<MusicOption>,
    plan: CoverPlan,
    rejected_by_files: Vec<String>,
}

fn artist_tracks(missing: &[MissingTrack], items: &[usize]) -> WantedItems {
    WantedItems::ArtistTracks {
        tracks: items
            .iter()
            .filter_map(|index| missing.get(*index))
            .map(|track| (track.album.clone(), track.number, track.title.clone()))
            .collect(),
    }
}

/// Say what each gathered release covers, look into the most promising ones, and plan.
#[allow(clippy::too_many_arguments)]
async fn evaluate_artist(
    env: &Env<'_>,
    artist: &str,
    artist_title: &WantedTitle,
    quality: &crate::graphql::entities::QualityProfile,
    collected: &Collected,
    albums: &[String],
    missing: &[MissingTrack],
    inspected: &mut HashMap<String, Inspection>,
) -> MusicEvaluation {
    let mut evaluation = MusicEvaluation::default();
    let mut options: Vec<MusicOption> = Vec::new();
    for release in &collected.releases {
        if artist_title.mismatch(&release.title).is_some() {
            continue;
        }
        let Some(claim) = music_claim(&release.title, artist, albums, missing) else {
            continue;
        };
        let covers = claimed_tracks(&claim, albums, missing);
        if covers.is_empty() && !claim.needs_files() {
            continue;
        }
        let count = if claim.needs_files() {
            missing.len()
        } else {
            covers.len()
        };
        let Some(scored) = profile::filter_and_rank(
            artist_title,
            vec![release.clone()],
            quality,
            count.max(1) as i32,
        )
        .into_iter()
        .next() else {
            continue;
        };
        options.push(MusicOption {
            scored,
            claim,
            covers,
            verdict: None,
        });
    }
    let mut ranked: Vec<ScoredRelease> =
        options.iter().map(|option| option.scored.clone()).collect();
    profile::sort_by_profile(&mut ranked, quality);
    let rank_of = |guid: &str| {
        ranked
            .iter()
            .position(|scored| scored.release.guid == guid)
            .unwrap_or(usize::MAX)
    };

    // Look into what would cover most first: discographies, then albums, then other releases (an
    // EP or a compilation says what it holds only by its files), then singles; best ranked first.
    let mut order: Vec<&MusicOption> = options.iter().collect();
    order.sort_by_key(|option| {
        let kind = match option.claim {
            MusicClaim::Discography => 0,
            MusicClaim::Album(_) => 1,
            MusicClaim::Other => 2,
            MusicClaim::Tracks(_) => 3,
        };
        (
            kind,
            std::cmp::Reverse(option.covers.len()),
            rank_of(&option.scored.release.guid),
        )
    });
    for option in order {
        if inspected.len() >= MUSIC_INSPECT_PER_PLAN {
            break;
        }
        let guid = option.scored.release.guid.clone();
        if inspected.contains_key(&guid) {
            continue;
        }
        let reading = match env.manager {
            Some(manager) => {
                inspect::inspect_release(
                    manager,
                    env.db,
                    &option.scored.release,
                    env.settings.hourly_search_budget,
                )
                .await
            }
            None => Inspection::Unavailable("the torrent client is not available".to_string()),
        };
        inspected.insert(guid, reading);
    }

    // Files beat names.
    let everything: Vec<usize> = (0..missing.len()).collect();
    let wanted_all = artist_tracks(missing, &everything);
    let mut kept: Vec<MusicOption> = Vec::new();
    for mut option in options {
        match inspected
            .get(&option.scored.release.guid)
            .and_then(Inspection::contents)
        {
            Some(contents) => {
                let verdict = judge::judge(contents, &wanted_all);
                if let Some(why) = &verdict.reject {
                    debug!(release = %option.scored.release.title, reason = %why, "Recommendations: '{}' turned down by its files: {why}", option.scored.release.title);
                    evaluation
                        .rejected_by_files
                        .push(format!("'{}' {why}", option.scored.release.title));
                    continue;
                }
                option.covers = verdict.covers.clone();
                option.verdict = Some(verdict);
            }
            // A discography or an EP is used only once its files say what it holds.
            None if option.claim.needs_files() => continue,
            None => {}
        }
        if !option.covers.is_empty() {
            kept.push(option);
        }
    }
    let plan_options: Vec<PlanOption> = kept
        .iter()
        .map(|option| PlanOption {
            covers: option.covers.clone(),
            tier: profile::resolution_tier(&option.scored.parsed, quality),
            seeders: option.scored.release.seeders,
            size: option.scored.release.size,
            rank: rank_of(&option.scored.release.guid),
            subtitles: false,
        })
        .collect();
    evaluation.plan = plan_cover(missing.len(), &plan_options);
    evaluation.options = kept;
    evaluation
}

/// The candidate a discography search is made with.
fn discography_search(base: &WantedCandidate, artist: &str, missing: usize) -> WantedCandidate {
    let mut candidate = base.clone();
    candidate.label = format!("discography '{artist}'");
    candidate.queries = vec![SourceQuery {
        query_type: QueryType::MusicSearch,
        search_term: Some(format!("{artist} discography")),
        artist: Some(artist.to_string()),
        cache: true,
        ..Default::default()
    }];
    candidate.match_title = WantedTitle::Music {
        artist: artist.to_string(),
        album: None,
        track: None,
    };
    candidate.expectation = Expectation::Unchecked;
    candidate.episode_count = missing as i32;
    candidate
}

/// The coverage plan for an artist's missing tracks (Q110): the feed catalogue first, then a
/// discography search (two or more albums missing something), each album's search for albums still
/// uncovered, and the single-track searches for tracks still uncovered, within the hourly budget,
/// looking into the most promising releases on the way. Returns the plan's row and one row per
/// release in it.
async fn look_up_artist(
    env: &Env<'_>,
    artist_id: &str,
    artist: &str,
    groups: &[AlbumGroup<'_>],
) -> Option<(Row, Vec<Row>)> {
    use crate::graphql::entities::Track;
    let mut missing: Vec<MissingTrack> = Vec::new();
    let mut track_searches: Vec<(usize, &WantedCandidate)> = Vec::new();
    for group in groups {
        for candidate in &group.tracks {
            let Some(id) = &candidate.linkage.track_id else {
                continue;
            };
            if missing.iter().any(|track| &track.id == id) {
                continue;
            }
            let Ok(Some(track)) = Track::get(env.db.pool(), id).await else {
                continue;
            };
            track_searches.push((missing.len(), *candidate));
            missing.push(MissingTrack {
                id: id.clone(),
                album_id: group.album_id.clone(),
                album: group.name.clone(),
                number: track.track_number,
                title: track.title,
            });
        }
    }
    if missing.is_empty() {
        return None;
    }
    let albums: Vec<String> = groups.iter().map(|group| group.name.clone()).collect();
    let mut by_size: Vec<&AlbumGroup<'_>> = groups.iter().collect();
    by_size.sort_by_key(|group| std::cmp::Reverse(group.tracks.len()));
    let (base, _) = album_candidate(&by_size[0].tracks)?;
    let quality = resolve_profile(env, &base).await;
    let artist_title = WantedTitle::Music {
        artist: artist.to_string(),
        album: None,
        track: None,
    };

    // The catalogue: whatever the owner's feeds list for any of the albums.
    let mut collected = Collected::default();
    let mut feeds: HashMap<String, String> = HashMap::new();
    for group in groups {
        let target = CatalogueTarget {
            album_id: Some(group.album_id.clone()),
            ..Default::default()
        };
        let (more, names) = catalogue_releases(env.db, &env.owner_id, &target, env.blocklist).await;
        merge(&mut collected, more);
        feeds.extend(names);
    }

    // Searches, cheapest coverage first.
    enum Search<'a> {
        Discography(WantedCandidate),
        Album(usize, WantedCandidate),
        Track(usize, &'a WantedCandidate),
    }
    let mut searches: Vec<Search<'_>> = Vec::new();
    if groups.len() >= 2 {
        searches.push(Search::Discography(discography_search(
            &base,
            artist,
            missing.len(),
        )));
    }
    for group in &by_size {
        if let Some((candidate, _)) = album_candidate(&group.tracks)
            && let Some(index) = albums.iter().position(|name| name == &group.name)
        {
            searches.push(Search::Album(index, candidate));
        }
    }
    for (index, candidate) in track_searches.iter().take(MAX_TRACK_SEARCHES * 4) {
        searches.push(Search::Track(*index, candidate));
    }

    let mut inspected: HashMap<String, Inspection> = HashMap::new();
    let mut ran_out = false;
    let mut track_searches_made = 0usize;
    let mut evaluation = evaluate_artist(
        env,
        artist,
        &artist_title,
        &quality,
        &collected,
        &albums,
        &missing,
        &mut inspected,
    )
    .await;
    for search in &searches {
        if evaluation.plan.uncovered.is_empty() {
            break;
        }
        let candidate = match search {
            Search::Discography(candidate) => candidate,
            Search::Album(index, candidate) => {
                let album = &albums[*index];
                let open = evaluation.plan.uncovered.iter().any(|item| {
                    missing
                        .get(*item)
                        .is_some_and(|track| &track.album == album)
                });
                if !open {
                    continue;
                }
                candidate
            }
            Search::Track(index, candidate) => {
                if !evaluation.plan.uncovered.contains(index)
                    || track_searches_made >= MAX_TRACK_SEARCHES
                {
                    continue;
                }
                track_searches_made += 1;
                *candidate
            }
        };
        let requests = candidate.queries.len().max(1) as u32;
        match env.sources {
            Some(sources) if budget_remaining(env.settings.hourly_search_budget) >= requests => {
                budget_spend(requests);
                let searched = collect_releases_from(sources, env.blocklist, candidate).await;
                merge(&mut collected, searched);
            }
            Some(_) => {
                ran_out = true;
                break;
            }
            None => break,
        }
        evaluation = evaluate_artist(
            env,
            artist,
            &artist_title,
            &quality,
            &collected,
            &albums,
            &missing,
            &mut inspected,
        )
        .await;
    }

    let plan = &evaluation.plan;
    let all: Vec<usize> = (0..missing.len()).collect();
    let mut header = Row {
        scope: "plan",
        status: if plan.releases.is_empty() {
            "none"
        } else {
            "ready"
        },
        target_label: artist.to_string(),
        artist_id: Some(artist_id.to_string()),
        covers_ids: plan
            .releases
            .iter()
            .flat_map(|planned| planned.adds.iter().map(|index| missing[*index].id.clone()))
            .collect(),
        covers_count: plan.covered() as i32,
        missing_count: missing.len() as i32,
        covers_albums: albums_of(&missing, &all),
        ..Default::default()
    };
    let budget_note = "the hourly search budget is used up, so some albums were not searched";
    if plan.releases.is_empty() {
        let mut note = if collected.seen == 0 {
            "no release found".to_string()
        } else {
            format!(
                "{} release(s) found, none usable for the missing tracks",
                collected.seen
            )
        };
        if !evaluation.rejected_by_files.is_empty() {
            note = format!(
                "{note}; {} turned down by their files (e.g. {})",
                evaluation.rejected_by_files.len(),
                evaluation.rejected_by_files[0]
            );
        }
        if ran_out {
            note = format!("{note}; {budget_note}");
        }
        header.note = Some(note);
        return Some((header, Vec::new()));
    }

    let mut members: Vec<Row> = Vec::new();
    let (mut packs, mut whole_albums, mut singles) = (0usize, 0usize, 0usize);
    for (position, planned) in plan.releases.iter().take(MAX_PLAN_RELEASES).enumerate() {
        let option = &evaluation.options[planned.option];
        let adds = &planned.adds;
        let release = &option.scored.release;
        match option.claim {
            MusicClaim::Discography => packs += 1,
            MusicClaim::Album(_) => whole_albums += 1,
            _ => singles += 1,
        }
        let contents = inspected.get(&release.guid).and_then(Inspection::contents);
        // Exactly the files for what this release adds, when its files were read.
        let (file_indices, selected_size) = match contents {
            Some(contents) => {
                let files = judge::judge(contents, &artist_tracks(&missing, adds)).files;
                let size: i64 = contents
                    .files
                    .iter()
                    .filter(|file| files.contains(&file.index))
                    .map(|file| file.size as i64)
                    .sum();
                let some = !files.is_empty();
                (files, some.then_some(size))
            }
            None => (Vec::new(), None),
        };
        let kind = release_kind(&option.claim, &release.title);
        let total = release
            .size
            .or(contents.map(|contents| contents.total_size));
        // Whether the grab takes only part of the release.
        let partial = match (contents, &option.claim) {
            (Some(_), _) => selected_size.is_some(),
            (None, MusicClaim::Album(index)) => groups
                .iter()
                .find(|group| group.name == albums[*index])
                .and_then(|group| group.track_count)
                .is_some_and(|count| (adds.len() as i32) < count),
            (None, _) => false,
        };
        let download_note = partial.then(|| {
            download_note(
                adds.len(),
                selected_size,
                total,
                kind,
                env.settings.only_missing_files,
            )
        });
        let mut reasons = vec![format!(
            "{} fits the '{}' profile",
            quality_label(&option.scored.parsed),
            quality.name
        )];
        if let Some(seeders) = release.seeders {
            reasons.push(format!("{seeders} seeders"));
        }
        match &option.verdict {
            Some(verdict) => reasons.push(match &verdict.note {
                Some(note) => format!("Files checked: {note}"),
                None => "Files checked".to_string(),
            }),
            None => reasons.push("Files not checked: judged by its name".to_string()),
        }
        if let Some(note) = &download_note {
            reasons.push(note.clone());
        }
        if let Some(feed) = feeds.get(&release.guid) {
            reasons.push(format!("Already listed by feed '{feed}': no search needed"));
        }
        reasons.push(format!(
            "Adds {} of {} missing tracks",
            adds.len(),
            missing.len()
        ));
        let adds_albums = albums_of(&missing, adds);
        let label = match &option.claim {
            MusicClaim::Discography => format!("{artist} discography"),
            MusicClaim::Album(index) => format!("{artist} – {}", albums[*index]),
            _ if adds.len() == 1 => format!("{artist} – {}", missing[adds[0]].title),
            _ => format!("{artist} – {} tracks", adds.len()),
        };
        members.push(Row {
            scope: option.claim.scope(),
            status: "ready",
            target_label: label,
            artist_id: Some(artist_id.to_string()),
            album_id: (adds_albums.len() == 1).then(|| adds_albums[0].clone()),
            track_id: (adds.len() == 1).then(|| missing[adds[0]].id.clone()),
            chosen: Some(Chosen {
                release: release.clone(),
                parsed: option.scored.parsed.clone(),
                reasons,
                feed_item_id: release
                    .guid
                    .strip_prefix(FEED_ITEM_GUID_PREFIX)
                    .map(str::to_string),
                source_name: release.source_name.clone(),
                file_indices,
            }),
            covers_ids: adds
                .iter()
                .map(|index| missing[*index].id.clone())
                .collect(),
            covers_count: adds.len() as i32,
            missing_count: missing.len() as i32,
            covers_albums: adds_albums,
            plan_position: Some(position as i32),
            selected_size,
            download_note,
            ..Default::default()
        });
    }
    let mut summary = Vec::new();
    for (count, one, many) in [
        (packs, "discography pack", "discography packs"),
        (whole_albums, "album", "albums"),
        (singles, "single release", "single releases"),
    ] {
        if count > 0 {
            summary.push(format!("{count} {}", if count == 1 { one } else { many }));
        }
    }
    let mut reasons = vec![format!(
        "{} cover {} of {} missing tracks at the best quality found",
        summary.join(" and "),
        plan.covered(),
        missing.len()
    )];
    if !evaluation.rejected_by_files.is_empty() {
        reasons.push(format!(
            "{} turned down by their files (e.g. {})",
            evaluation.rejected_by_files.len(),
            evaluation.rejected_by_files[0]
        ));
    }
    header.plan_reasons = reasons;
    if !plan.uncovered.is_empty() {
        let mut note = format!(
            "Still missing: {} (no usable release found)",
            describe_tracks(&missing, &plan.uncovered)
        );
        if ran_out {
            note = format!("{note}; {budget_note}");
        }
        header.note = Some(note);
    }
    Some((header, members))
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
    plan_id: Option<&str>,
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
        file_indices,
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
                (!chosen.file_indices.is_empty())
                    .then(|| serde_json::to_string(&chosen.file_indices).ok())
                    .flatten(),
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
            row.plan_reasons.clone(),
            None,
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
        plan_id: plan_id.map(str::to_string),
        plan_position: row.plan_position,
        covers_seasons: (!row.covers_seasons.is_empty())
            .then(|| serde_json::to_string(&row.covers_seasons).ok())
            .flatten(),
        file_indices,
        artist_id: row.artist_id.clone(),
        covers_albums: (!row.covers_albums.is_empty())
            .then(|| serde_json::to_string(&row.covers_albums).ok())
            .flatten(),
        selected_size: row.selected_size,
        download_note: row.download_note.clone(),
        rules_version: Some(RULES_VERSION),
        found_at: now.to_rfc3339(),
        expires_at: expires.to_rfc3339(),
    }
}

/// Delete the rows a new answer replaces: everything stored for the same target and scope family.
async fn clear_for(db: &Database, row: &Row) {
    let mut filters = Vec::new();
    match row.scope {
        "season_pack" | "episode" if row.show_id.is_some() => {
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
        // An artist's plan replaces everything stored for the artist, and the per-album answers of
        // older rules for its albums.
        "plan" if row.artist_id.is_some() => {
            filters.push(ReleaseRecommendationWhereInput {
                artist_id: row.artist_id.as_deref().map(string_eq),
                ..Default::default()
            });
            for album in &row.covers_albums {
                filters.push(ReleaseRecommendationWhereInput {
                    album_id: Some(string_eq(album)),
                    plan_id: Some(StringFilter {
                        is_null: Some(true),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
            }
        }
        // A show's plan replaces everything stored for the show, older per-season answers too.
        "plan" | "series_pack" if row.show_id.is_some() => {
            filters.push(ReleaseRecommendationWhereInput {
                show_id: row.show_id.as_deref().map(string_eq),
                ..Default::default()
            })
        }
        "movie" if row.movie_id.is_some() => filters.push(ReleaseRecommendationWhereInput {
            movie_id: row.movie_id.as_deref().map(string_eq),
            ..Default::default()
        }),
        "album" | "track" if row.album_id.is_some() => {
            for scope in ["album", "track"] {
                filters.push(ReleaseRecommendationWhereInput {
                    album_id: row.album_id.as_deref().map(string_eq),
                    scope: Some(string_eq(scope)),
                    ..Default::default()
                });
            }
        }
        "book" if row.audiobook_id.is_some() => filters.push(ReleaseRecommendationWhereInput {
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
            ReleaseRecommendation::insert(db, to_input(row, owner_id, library_id, None, now)).await
        {
            warn!(target = %row.target_label, error = %crate::services::redact::redact_text(&error.to_string()), "Recommendations: could not store an answer");
        }
    }
    summary
}

/// Store a show's plan: its own row first, then each release's row pointing at it. Replaces
/// everything stored for the show.
async fn store_plan(
    db: &Database,
    header: Row,
    members: &[Row],
    owner_id: &str,
    library_id: &str,
) -> LookupSummary {
    let now = Utc::now();
    let mut summary = LookupSummary::default();
    clear_for(db, &header).await;
    match header.status {
        "ready" => summary.ready += 1,
        "none" => summary.none += 1,
        _ => {}
    }
    let plan = match ReleaseRecommendation::insert(
        db,
        to_input(&header, owner_id, library_id, None, now),
    )
    .await
    {
        Ok(plan) => plan,
        Err(error) => {
            warn!(target = %header.target_label, error = %crate::services::redact::redact_text(&error.to_string()), "Recommendations: could not store the plan for '{}'", header.target_label);
            return summary;
        }
    };
    for member in members {
        if let Err(error) = ReleaseRecommendation::insert(
            db,
            to_input(member, owner_id, library_id, Some(&plan.id), now),
        )
        .await
        {
            warn!(target = %member.target_label, error = %crate::services::redact::redact_text(&error.to_string()), "Recommendations: could not store a release of the plan for '{}'", header.target_label);
        }
    }
    info!(
        show = %header.target_label,
        releases = members.len(),
        covers = header.covers_count,
        missing = header.missing_count,
        "Recommendations: plan for '{}': {} release(s) cover {} of {} missing",
        header.target_label,
        members.len(),
        header.covers_count,
        header.missing_count
    );
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

    // Shows: one coverage plan each, over every missing season the scope names.
    let mut by_show: HashMap<String, Vec<&SeasonGroup<'_>>> = HashMap::new();
    let mut ordered: Vec<&SeasonGroup<'_>> = seasons.values().collect();
    ordered.sort_by(|a, b| (&a.show_id, a.season).cmp(&(&b.show_id, b.season)));
    for group in ordered {
        by_show
            .entry(group.show_id.clone())
            .or_default()
            .push(group);
    }
    let mut shows: Vec<(String, Vec<&SeasonGroup<'_>>)> = by_show.into_iter().collect();
    shows.sort_by(|a, b| a.0.cmp(&b.0));
    for (show_id, groups) in shows {
        if left == 0 || existing.contains(&format!("plan:{show_id}")) {
            continue;
        }
        let Some(library_id) = groups
            .iter()
            .find_map(|group| group.episodes.first().or(group.pack.as_ref()))
            .map(|candidate| candidate.library_id.clone())
        else {
            continue;
        };
        let Some(owner) = library_owner(&db, &mut owners, &library_id).await else {
            continue;
        };
        let env = Env {
            db: &db,
            manager: Some(manager),
            sources: sources_manager.as_deref(),
            blocklist: &blocklist,
            settings: &settings,
            owner_id: owner.clone(),
        };
        left -= 1;
        summary.targets += 1;
        let show_name = groups[0].show_name.clone();
        let Some((header, members)) = look_up_show(&env, &show_id, &show_name, &groups).await
        else {
            continue;
        };
        let part = store_plan(&db, header, &members, &owner, &library_id).await;
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
            manager: Some(manager),
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
        let items = if scope == "book" {
            WantedItems::Book
        } else {
            WantedItems::Movie
        };
        let row = look_up_single(
            &env,
            candidate,
            scope,
            target,
            label,
            &candidate.linkage,
            Vec::new(),
            Some(items),
        )
        .await;
        let part = store(&db, &[row], &owner, &candidate.library_id).await;
        summary.ready += part.ready;
        summary.none += part.none;
    }

    // Music: one coverage plan per artist, over every album of theirs missing tracks (Q110).
    let mut artists: HashMap<String, Vec<AlbumGroup<'_>>> = HashMap::new();
    let mut album_ids: Vec<&String> = albums.keys().collect();
    album_ids.sort();
    for album_id in album_ids {
        let Ok(Some(album)) = crate::graphql::entities::Album::get(db.pool(), album_id).await
        else {
            continue;
        };
        artists
            .entry(album.artist_id.clone())
            .or_default()
            .push(AlbumGroup {
                album_id: album.id.clone(),
                name: album.name.clone(),
                track_count: album.track_count,
                tracks: albums[album_id].clone(),
            });
    }
    let mut artist_ids: Vec<String> = artists.keys().cloned().collect();
    artist_ids.sort();
    for artist_id in artist_ids {
        if left == 0 || existing.contains(&format!("artist:{artist_id}")) {
            continue;
        }
        let groups = &artists[&artist_id];
        let Some(first) = groups.first().and_then(|group| group.tracks.first()) else {
            continue;
        };
        let library_id = first.library_id.clone();
        let Some(owner) = library_owner(&db, &mut owners, &library_id).await else {
            continue;
        };
        let artist_name = match crate::graphql::entities::Artist::get(db.pool(), &artist_id).await {
            Ok(Some(artist)) => artist.name,
            _ => match &first.match_title {
                WantedTitle::Music { artist, .. } => artist.clone(),
                _ => continue,
            },
        };
        let env = Env {
            db: &db,
            manager: Some(manager),
            sources: sources_manager.as_deref(),
            blocklist: &blocklist,
            settings: &settings,
            owner_id: owner.clone(),
        };
        left -= 1;
        summary.targets += 1;
        let Some((header, members)) = look_up_artist(&env, &artist_id, &artist_name, groups).await
        else {
            continue;
        };
        let part = store_plan(&db, header, &members, &owner, &library_id).await;
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

/// Keys of targets that already have an answer that has not expired: `plan:<show id>`,
/// `movie:<id>`, `book:<id>`, `album:<id>`.
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
        if row.scope == "plan"
            && let Some(show) = &row.show_id
        {
            keys.insert(format!("plan:{show}"));
        }
        if row.scope == "plan"
            && let Some(artist) = &row.artist_id
        {
            keys.insert(format!("artist:{artist}"));
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
        // A show's plan is not dropped because one thing in it, or near it, was grabbed: only the
        // releases the grab makes pointless go (the same episode, or another pack of the same
        // season). The plan's own row goes when it has no releases left (`prune`).
        let rows = ReleaseRecommendation::query(db.pool())
            .filter(ReleaseRecommendationWhereInput {
                show_id: Some(string_eq(show)),
                ..Default::default()
            })
            .fetch_all()
            .await
            .unwrap_or_default();
        for row in rows {
            let in_plan = row.plan_id.is_some() || row.scope == "plan";
            let pointless = if in_plan {
                row.scope != "plan"
                    && match (&linkage.episode_id, linkage.season) {
                        (Some(episode), _) => row.episode_id.as_ref() == Some(episode),
                        // Another pack of the same season; the plan's single episodes of that
                        // season fill what its pack lacked, so they stay.
                        (None, Some(season)) => {
                            row.season == Some(season) && row.scope == "season_pack"
                        }
                        (None, None) => false,
                    }
            } else {
                linkage
                    .season
                    .is_none_or(|season| row.season == Some(season))
                    && linkage
                        .episode_id
                        .as_ref()
                        .is_none_or(|episode| row.episode_id.as_ref() == Some(episode))
            };
            if pointless && let Err(error) = ReleaseRecommendation::delete_by_id(db, &row.id).await
            {
                warn!(error = %error, "Recommendations: could not forget a grabbed target's answer");
            }
        }
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
    let plan_ids: HashSet<String> = rows
        .iter()
        .filter(|row| row.scope == "plan")
        .map(|row| row.id.clone())
        .collect();
    let mut kept_members: HashSet<String> = HashSet::new();
    let mut plans: Vec<ReleaseRecommendation> = Vec::new();
    for row in rows {
        if row.scope == "plan" && row.status == "ready" && is_current(&row) {
            // Judged once its releases have been.
            plans.push(row);
            continue;
        }
        let expired = DateTime::parse_from_rfc3339(&row.expires_at)
            .map(|at| at.with_timezone(&Utc) <= now)
            .unwrap_or(true);
        let mut drop = expired;
        let mut why = "expired";
        if !drop && !is_current(&row) {
            drop = true;
            why = "it was chosen by older rules";
        }
        if !drop
            && let Some(plan) = &row.plan_id
            && !plan_ids.contains(plan)
        {
            drop = true;
            why = "its plan is gone";
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
            } else if if row.plan_id.is_some() {
                plan_release_is_settled(db, &row).await
            } else {
                target_is_settled(db, &row).await
            } {
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
        } else if let Some(plan) = &row.plan_id {
            kept_members.insert(plan.clone());
        }
    }
    for plan in plans {
        let expired = DateTime::parse_from_rfc3339(&plan.expires_at)
            .map(|at| at.with_timezone(&Utc) <= now)
            .unwrap_or(true);
        if expired || !kept_members.contains(&plan.id) {
            debug!(target = %plan.target_label, "Recommendations: removing the plan for '{}': {}", plan.target_label, if expired { "expired" } else { "every release in it was grabbed or is gone" });
            if ReleaseRecommendation::delete_by_id(db, &plan.id)
                .await
                .is_ok()
            {
                removed += 1;
            }
        }
    }
    removed
}

/// Whether a release in a show's plan is no longer needed: every episode it adds has a file, or a
/// torrent that has not failed was grabbed for exactly what it is (that episode, that season's
/// pack, or a pack of the whole show).
async fn plan_release_is_settled(db: &Database, row: &ReleaseRecommendation) -> bool {
    use crate::graphql::entities::{Episode, Track};
    let music = row.artist_id.is_some();
    if let Some(json) = &row.covers_ids
        && let Ok(ids) = serde_json::from_str::<Vec<String>>(json)
        && !ids.is_empty()
    {
        let mut all_filled = true;
        for id in &ids {
            let filled = if music {
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
                all_filled = false;
                break;
            }
        }
        if all_filled {
            return true;
        }
    }
    if music {
        // A torrent that has not failed was grabbed for exactly its track or album.
        let filter = match (&row.track_id, &row.album_id) {
            (Some(track), _) => TorrentWhereInput {
                track_id: Some(string_eq(track)),
                ..Default::default()
            },
            (None, Some(album)) if row.scope == "album" => TorrentWhereInput {
                album_id: Some(string_eq(album)),
                ..Default::default()
            },
            _ => return false,
        };
        return Torrent::query(db.pool())
            .filter(filter)
            .fetch_all()
            .await
            .map(|rows| {
                rows.iter()
                    .any(|torrent| torrent.post_process_status.as_deref() != Some("failed"))
            })
            .unwrap_or(false);
    }
    let Some(show) = &row.show_id else {
        return false;
    };
    let torrents = Torrent::query(db.pool())
        .filter(TorrentWhereInput {
            show_id: Some(string_eq(show)),
            ..Default::default()
        })
        .fetch_all()
        .await
        .unwrap_or_default();
    torrents
        .iter()
        .filter(|torrent| torrent.post_process_status.as_deref() != Some("failed"))
        .any(|torrent| match row.scope.as_str() {
            "episode" => row.episode_id.is_some() && torrent.episode_id == row.episode_id,
            "season_pack" => {
                torrent.episode_id.is_none() && row.season.is_some() && torrent.season == row.season
            }
            _ => {
                torrent.episode_id.is_none()
                    && torrent.season.is_none()
                    && row
                        .info_hash
                        .as_deref()
                        .is_some_and(|hash| hash.eq_ignore_ascii_case(&torrent.info_hash))
            }
        })
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
    } else if request.artist_id.is_some() {
        // An artist, an album or a track: the answer is the artist's coverage plan.
        Some("plan")
    } else if request.album_id.is_some() {
        Some("album")
    } else if request.show_id.is_some() {
        // A show, a season or an episode: the answer is the show's coverage plan.
        Some("plan")
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
    // A season or an episode of a show is answered by the show's plan, an album or a track by its
    // artist's.
    let request = match (&request.show_id, &request.artist_id) {
        (Some(show_id), _) => LookupRequest {
            show_id: Some(show_id.clone()),
            ..Default::default()
        },
        (None, Some(artist_id)) => LookupRequest {
            artist_id: Some(artist_id.clone()),
            ..Default::default()
        },
        _ => request,
    };
    let existing = ReleaseRecommendation::query(db.pool())
        .filter(ReleaseRecommendationWhereInput {
            show_id: request.show_id.as_deref().map(string_eq),
            movie_id: request.movie_id.as_deref().map(string_eq),
            album_id: request.album_id.as_deref().map(string_eq),
            artist_id: request.artist_id.as_deref().map(string_eq),
            audiobook_id: request.audiobook_id.as_deref().map(string_eq),
            scope: (scope == "plan").then(|| string_eq("plan")),
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
        artist_id: request.artist_id.clone(),
        chosen: None,
        covers_ids: Vec::new(),
        covers_count: 0,
        missing_count: 0,
        covers_seasons: Vec::new(),
        plan_position: None,
        plan_reasons: Vec::new(),
        note: None,
        ..Default::default()
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
        artist_id: request.artist_id.as_deref().map(string_eq),
        scope: (request.show_id.is_some() || request.artist_id.is_some())
            .then(|| string_eq("plan")),
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
        || !matches!(
            rec.scope.as_str(),
            "season_pack" | "series_pack" | "album" | "track" | "discography"
        )
    {
        return None;
    }
    // Looked into before it was recommended: exactly those files.
    if let Some(indices) = rec
        .file_indices
        .as_deref()
        .and_then(|json| serde_json::from_str::<Vec<usize>>(json).ok())
        .filter(|indices| !indices.is_empty())
    {
        return Some(WantedFiles::exactly(indices));
    }
    let ids: Vec<String> = serde_json::from_str(rec.covers_ids.as_deref()?).ok()?;
    let mut wanted = WantedFiles::default();
    for id in ids {
        if matches!(rec.scope.as_str(), "album" | "track" | "discography") {
            if let Ok(Some(track)) = Track::get(db.pool(), &id).await {
                // Track numbers repeat across a discography's albums: match those by title only.
                let number = if rec.scope == "album" {
                    track.track_number
                } else {
                    -1
                };
                wanted.tracks.push((number, track.title));
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
    anyhow::ensure!(
        rec.scope != "plan",
        "That is a plan: download its releases, or the whole plan"
    );
    grab_row(manager, &db, rec).await
}

/// What downloading a whole plan did.
#[derive(Debug, Clone, Default)]
pub struct PlanGrabOutcome {
    pub started: Vec<GrabOutcome>,
    /// `'release title': why` for each release that could not be started.
    pub failed: Vec<String>,
}

/// Download every release of a show's plan, in its order, through the same path as one release.
/// The releases are read up front, so grabbing one (which may settle another's row) does not
/// change what is downloaded. `actor_user_id` is checked against the plan's owner unless
/// `is_admin`.
pub async fn grab_plan(
    manager: &Arc<ServicesManager>,
    plan_id: &str,
    actor_user_id: &str,
    is_admin: bool,
) -> Result<PlanGrabOutcome> {
    let deny = || anyhow::anyhow!("Plan not found or access denied");
    let db = manager
        .get_database()
        .await
        .context("database service not available")?
        .pool()
        .clone();
    let plan = ReleaseRecommendation::get(db.pool(), &plan_id.to_string())
        .await?
        .ok_or_else(deny)?;
    if (!is_admin && plan.user_id != actor_user_id) || plan.scope != "plan" {
        return Err(deny());
    }
    anyhow::ensure!(plan.status == "ready", "That plan has no releases");
    anyhow::ensure!(
        is_current(&plan),
        "That plan was made by older rules and is being looked up again"
    );
    let mut members = ReleaseRecommendation::query(db.pool())
        .filter(ReleaseRecommendationWhereInput {
            plan_id: Some(string_eq(&plan.id)),
            ..Default::default()
        })
        .fetch_all()
        .await?;
    members.sort_by_key(|member| member.plan_position.unwrap_or(i32::MAX));
    anyhow::ensure!(!members.is_empty(), "That plan has no releases left");
    let mut outcome = PlanGrabOutcome::default();
    for member in members {
        let title = member.title.clone().unwrap_or_default();
        match grab_row(manager, &db, member).await {
            Ok(started) => outcome.started.push(started),
            Err(error) => outcome.failed.push(format!(
                "'{title}': {}",
                crate::services::redact::redact_text(&format!("{error:#}"))
            )),
        }
    }
    info!(
        show = %plan.target_label,
        started = outcome.started.len(),
        failed = outcome.failed.len(),
        "Recommendations: downloading the plan for '{}': {} started, {} could not start",
        plan.target_label,
        outcome.started.len(),
        outcome.failed.len()
    );
    Ok(outcome)
}

/// Grab one stored recommendation (a movie, album or book, or one release of a plan).
async fn grab_row(
    manager: &Arc<ServicesManager>,
    db: &Database,
    rec: ReleaseRecommendation,
) -> Result<GrabOutcome> {
    use super::auto_download::{GrabEnv, grab_release_with, hunt_lock};
    use super::auto_trail::{GrabRecord, Trail, Trigger};
    let db = db.clone();
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
    // A release in a plan stands for what it adds, not for its whole season: the grab forgets only
    // what it makes pointless (`forget_for_linkage`), and this row.
    let in_plan = rec.plan_id.is_some();
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
        released_at: None,
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
    if !in_plan {
        forget_for_linkage(&db, &linkage).await;
    }
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
            artist_id: request.artist_id.clone(),
            audiobook_id: request.audiobook_id.clone(),
            ignore_backoff: true,
            ..Default::default()
        };
        // A show is one target: its plan.
        let targets = 4;
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
