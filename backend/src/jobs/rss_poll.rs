//! RSS polling: the cheap way to find what is wanted.
//!
//! A feed lists what was just released. Reading it costs one request per feed per interval, where a
//! search costs one per wanted title, so a feed is how a new episode is usually found; the
//! air-time search is the safety net behind it.
//!
//! Each enabled [`RssFeed`] is polled on its own interval (never faster than
//! [`MIN_POLL_MINUTES`], and slower while it keeps failing or its host says to wait). Items are
//! de-duplicated through [`RssFeedItem`] rows, then matched against what is wanted with the same
//! discovery the hunt uses, so a feed honours the same modes (`NONE`/`WANTED`/`ALL`), quality
//! profiles and blocklist. A match must name the show or movie exactly *and* be the right episode,
//! season or year: a feed carries every release of every show, so resemblance is not enough.
//!
//! Only shows and movies are matched. Music and audiobook targets are per-track and whole-book
//! searches; an album or book release in a feed cannot be tied to one of them safely, so those are
//! left to the search.
//!
//! A feed only ever grabs for titles in libraries owned by the feed's owner (and only in the feed's
//! library when it names one), which is also what the torrent service requires of `sourceFeedId`.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use graphql_orm::graphql::filters::{DateFilter, StringFilter};
use once_cell::sync::Lazy;
use regex::Regex;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use crate::db::Database;
use crate::graphql::entities::{
    CreateRssFeedItemInput, Library, LibraryWhereInput, QualityProfile, RssFeed, RssFeedItem,
    RssFeedItemWhereInput, RssFeedWhereInput, UpdateRssFeedInput,
};
use crate::services::ServicesManager;
use crate::services::quality::{profile, scoring};
use crate::services::sources::SourceRelease;

use super::auto_download::{
    self, Blocklist, GrabEnv, HuntScope, NoGrabFacts, WantedCandidate, describe_no_grab,
    explain_profile_rejections,
};
use super::auto_schedule::grab_budget;
use super::auto_settings::AutoDownloadSettings;
use super::auto_trail::{GrabRecord, Trail, Trigger};

/// A feed is never read more often than this, whatever its own interval says.
pub const MIN_POLL_MINUTES: i64 = 15;
/// The longest a failing feed is left alone.
const MAX_BACKOFF_MINUTES: i64 = 24 * 60;
/// Items read from one feed per poll; a feed longer than this is cut off at the newest.
const MAX_ITEMS_PER_POLL: usize = 200;
/// The most a feed response may weigh.
const MAX_FEED_BYTES: usize = 4 * 1024 * 1024;
/// How long seen items are remembered, so a feed that repeats old entries does not re-grab them.
const ITEM_RETENTION_DAYS: i64 = 30;
/// A pause between feeds in one pass, so feeds on one host are not requested in a burst.
const GAP_BETWEEN_FEEDS: Duration = Duration::from_secs(2);

// ===========================================================================
// When to poll
// ===========================================================================

/// How long a feed waits between polls: its own interval (at least [`MIN_POLL_MINUTES`]), doubled for
/// every consecutive failure up to a day, so a broken or rate-limiting feed backs off by itself.
pub fn effective_interval_minutes(poll_interval_minutes: i32, consecutive_failures: i32) -> i64 {
    let base = i64::from(poll_interval_minutes).max(MIN_POLL_MINUTES);
    let doublings = consecutive_failures.clamp(0, 10) as u32;
    base.saturating_mul(1_i64 << doublings)
        .min(MAX_BACKOFF_MINUTES.max(base))
}

/// Whether a feed is due: never polled, or its interval has elapsed since the last poll. A
/// `last_polled_at` in the future (set when a host asked to be left alone) is simply not due yet.
pub fn feed_due(
    last_polled_at: Option<&str>,
    poll_interval_minutes: i32,
    consecutive_failures: i32,
    now: DateTime<Utc>,
) -> bool {
    let Some(last) = last_polled_at.and_then(|value| DateTime::parse_from_rfc3339(value).ok())
    else {
        return true;
    };
    now.signed_duration_since(last.with_timezone(&Utc))
        >= chrono::Duration::minutes(effective_interval_minutes(
            poll_interval_minutes,
            consecutive_failures,
        ))
}

/// Seconds a host asked to be left alone for, from a `Retry-After` header holding seconds.
pub fn retry_after_seconds(header: Option<&str>) -> Option<i64> {
    header?
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|seconds| *seconds > 0)
}

// ===========================================================================
// Item identity
// ===========================================================================

fn hash_text(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

/// `(link hash, title hash)` for an item: what makes it the same item next poll.
pub fn item_hashes(release: &SourceRelease) -> (String, String) {
    let link = release
        .magnet_uri
        .as_deref()
        .or(release.link.as_deref())
        .unwrap_or(&release.guid);
    (
        hash_text(&format!("{}\n{}", release.guid, link)),
        hash_text(&scoring::normalize_title(&release.title)),
    )
}

/// The info hash inside a magnet link, lower-cased.
pub fn magnet_info_hash(magnet: &str) -> Option<String> {
    let lower = magnet.to_ascii_lowercase();
    let start = lower.find("urn:btih:")? + "urn:btih:".len();
    let hash: String = lower[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    (hash.len() == 40 || hash.len() == 32).then_some(hash)
}

/// Case, punctuation and apostrophes flattened ("Don't" and "Dont" are the same word in a release).
fn flatten(text: &str) -> String {
    let without: String = text
        .chars()
        .filter(|c| !matches!(c, '\'' | '\u{2019}' | '`'))
        .collect();
    scoring::normalize_title(&without)
}

static TITLE_STOP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
        \b s\d{1,2} \s? (e\d{1,3} | \b) |   # S02E05, S02
        \b \d{1,2} x \d{2,3} \b |           # 2x05
        \b season \s \d |                   # Season 2
        \b (19|20)\d{2} \b |                # a year
        \b \d{3,4} p \b |                   # 1080p
        \b (web|hdtv|bluray|bdrip|dvdrip|remux|proper|repack|complete) \b",
    )
    .expect("title stop pattern is valid")
});

/// What a release title calls the show or movie: everything before the season/episode, year or
/// quality tags. `Some.Show.S02E05.1080p-GRP` -> `some show`.
pub fn title_prefix(release_title: &str) -> String {
    let spaced: String = release_title
        .chars()
        .map(|c| if c == '.' || c == '_' { ' ' } else { c })
        .collect();
    let cut = TITLE_STOP
        .find(&spaced)
        .map_or(spaced.len(), |found| found.start());
    flatten(&spaced[..cut])
}

/// The names a wanted title may be released under: its own, without a trailing `(2019)` or `(US)`,
/// and with a year or country left on.
pub fn acceptable_names(name: &str) -> Vec<String> {
    static TRAILING_TAG: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"\s*\(([^)]*)\)\s*$").expect("trailing tag pattern is valid"));
    let mut names = vec![flatten(name)];
    if let Some(tag) = TRAILING_TAG.find(name) {
        names.push(flatten(&name[..tag.start()]));
    }
    names.sort();
    names.dedup();
    names.retain(|name| !name.is_empty());
    names
}

/// Whether a release title names the wanted show or movie. Exact, after normalising case and
/// punctuation: "The Office" and "The Office US" are different shows, and a near miss is a miss.
pub fn names_match(release_title: &str, wanted_name: &str) -> bool {
    let prefix = title_prefix(release_title);
    !prefix.is_empty() && acceptable_names(wanted_name).contains(&prefix)
}

// ===========================================================================
// Matching items to wanted targets
// ===========================================================================

/// What became of one feed item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemFate {
    /// Names no wanted title (the usual case: a feed lists everything).
    NotWanted,
    /// On the release blocklist.
    Blocklisted,
    /// Names a wanted title but is the wrong episode, season or year.
    WrongTarget(String),
    /// A release of this wanted target (index into the candidate list).
    Candidate(usize),
}

/// Decide an item against the wanted targets. `candidates` are the flat list of targets.
pub fn classify_item(
    release: &SourceRelease,
    candidates: &[WantedCandidate],
    blocklist: &Blocklist,
) -> Vec<ItemFate> {
    if blocklist.blocks(release) {
        return vec![ItemFate::Blocklisted];
    }
    let parsed = scoring::parse_release(&release.title);
    let mut fates = Vec::new();
    let mut named_but_wrong: Option<String> = None;
    for (index, candidate) in candidates.iter().enumerate() {
        let Some(name) = candidate.feed_name.as_deref() else {
            continue;
        };
        if !names_match(&release.title, name) || !ids_agree(release, candidate) {
            continue;
        }
        match candidate.expectation.mismatch(&parsed, true) {
            None => fates.push(ItemFate::Candidate(index)),
            Some(reason) => {
                named_but_wrong.get_or_insert(reason);
            }
        }
    }
    if fates.is_empty() {
        return vec![match named_but_wrong {
            Some(reason) => ItemFate::WrongTarget(reason),
            None => ItemFate::NotWanted,
        }];
    }
    fates
}

/// When the feed names an id the title also has, they must be the same one. An id the feed does not
/// carry decides nothing (most plain RSS has none).
fn ids_agree(release: &SourceRelease, candidate: &WantedCandidate) -> bool {
    let Some(query) = candidate.queries.first() else {
        return true;
    };
    let differs = |item: Option<i64>, wanted: Option<i64>| matches!((item, wanted), (Some(a), Some(b)) if a != b);
    !(differs(release.tvdb_id, query.tvdb_id.map(i64::from))
        || differs(release.tvmaze_id, query.tvmaze_id.map(i64::from))
        || differs(release.tmdb, query.tmdb_id.map(i64::from)))
}

/// A release picked for a target.
#[derive(Debug, Clone)]
pub struct PlannedGrab {
    pub candidate_index: usize,
    pub release: SourceRelease,
}

/// Every feed target flattened: the top-level targets, then the per-episode fallbacks of season
/// packs. Season packs come first so a grabbed pack can supersede the episodes inside it.
pub fn flatten_targets(candidates: Vec<WantedCandidate>) -> Vec<WantedCandidate> {
    let mut packs = Vec::new();
    let mut singles = Vec::new();
    for mut candidate in candidates {
        let fallbacks = std::mem::take(&mut candidate.fallbacks);
        if matches!(
            candidate.expectation,
            super::auto_schedule::Expectation::SeasonPack { .. }
        ) {
            packs.push(candidate);
        } else {
            singles.push(candidate);
        }
        singles.extend(fallbacks);
    }
    packs.extend(singles);
    packs
}

/// Whether grabbing `pack` makes `other` pointless: the pack is the season `other` is an episode of.
pub fn pack_covers(pack: &WantedCandidate, other: &WantedCandidate) -> bool {
    use super::auto_schedule::Expectation;
    match (&pack.expectation, &other.expectation) {
        (
            Expectation::SeasonPack { season },
            Expectation::Episode {
                season: other_season,
                ..
            },
        ) => {
            season == other_season
                && pack.linkage.show_id.is_some()
                && pack.linkage.show_id == other.linkage.show_id
        }
        _ => false,
    }
}

// ===========================================================================
// Feed rows
// ===========================================================================

fn string_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

/// What came out of a poll pass, for the log and Settings.
#[derive(Debug, Clone, Default)]
pub struct RssPassSummary {
    pub feeds_polled: usize,
    pub feeds_failed: usize,
    pub new_items: usize,
    pub matched: usize,
    pub grabbed: usize,
    pub capped: usize,
}

/// A feed fetched and parsed, ready to be matched.
struct FetchedFeed {
    feed: RssFeed,
    items: Vec<SourceRelease>,
}

fn validate_feed_url(url: &str) -> Result<reqwest::Url> {
    let parsed = reqwest::Url::parse(url.trim()).context("the feed address is not a valid URL")?;
    match parsed.scheme() {
        "http" | "https" => Ok(parsed),
        other => anyhow::bail!("feeds must be http or https, not {other}"),
    }
}

/// Fetch one feed. A host that asks to be left alone (429/503 with `Retry-After`) is believed.
async fn fetch_feed(
    client: &reqwest::Client,
    feed: &RssFeed,
) -> Result<Vec<SourceRelease>, FeedFailure> {
    let url =
        validate_feed_url(&feed.url).map_err(|error| FeedFailure::new(error.to_string(), None))?;
    let response = client
        .get(url)
        .header(
            reqwest::header::ACCEPT,
            "application/rss+xml, application/xml, text/xml, */*",
        )
        .send()
        .await
        .map_err(|error| {
            FeedFailure::new(format!("the feed could not be reached: {error}"), None)
        })?;
    let status = response.status();
    if !status.is_success() {
        let retry_after = retry_after_seconds(
            response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok()),
        );
        return Err(FeedFailure::new(
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                "the feed host is rate limiting requests".to_string()
            } else {
                format!("the feed answered HTTP {status}")
            },
            retry_after,
        ));
    }
    let body = crate::services::http_client::response_text_limited(response, MAX_FEED_BYTES)
        .await
        .map_err(|error| FeedFailure::new(format!("the feed could not be read: {error}"), None))?;
    let mut items = crate::services::sources::definitions::torznab::parse_torznab_rss(&body)
        .map_err(|error| FeedFailure::new(format!("the feed is not RSS 2.0: {error}"), None))?;
    items.truncate(MAX_ITEMS_PER_POLL);
    for item in &mut items {
        if item.info_hash.is_none() {
            item.info_hash = item.magnet_uri.as_deref().and_then(magnet_info_hash);
        }
    }
    Ok(items)
}

#[derive(Debug)]
struct FeedFailure {
    message: String,
    retry_after: Option<i64>,
}

impl FeedFailure {
    fn new(message: String, retry_after: Option<i64>) -> Self {
        Self {
            message,
            retry_after,
        }
    }
}

async fn record_poll_ok(db: &Database, feed: &RssFeed, now: DateTime<Utc>) {
    if let Err(error) = RssFeed::update_by_id(
        db,
        &feed.id,
        UpdateRssFeedInput {
            last_polled_at: Some(Some(now.to_rfc3339())),
            last_successful_at: Some(Some(now.to_rfc3339())),
            last_error: Some(None),
            consecutive_failures: Some(Some(0)),
            ..Default::default()
        },
    )
    .await
    {
        warn!(feed = %feed.name, error = %error, "RSS: could not record a successful poll");
    }
}

async fn record_poll_failed(
    db: &Database,
    feed: &RssFeed,
    now: DateTime<Utc>,
    failure: &FeedFailure,
) {
    // A host that said "wait N seconds" gets a last-polled time pushed into the future so the
    // ordinary interval check leaves it alone until then.
    let interval = chrono::Duration::minutes(effective_interval_minutes(
        feed.poll_interval_minutes,
        feed.consecutive_failures.unwrap_or(0) + 1,
    ));
    let polled_at = match failure.retry_after {
        Some(seconds) => {
            let wait = chrono::Duration::seconds(seconds).min(chrono::Duration::hours(24));
            now + (wait - interval).max(chrono::Duration::zero())
        }
        None => now,
    };
    if let Err(error) = RssFeed::update_by_id(
        db,
        &feed.id,
        UpdateRssFeedInput {
            last_polled_at: Some(Some(polled_at.to_rfc3339())),
            last_error: Some(Some(failure.message.clone())),
            consecutive_failures: Some(Some(feed.consecutive_failures.unwrap_or(0) + 1)),
            ..Default::default()
        },
    )
    .await
    {
        warn!(feed = %feed.name, error = %error, "RSS: could not record a failed poll");
    }
}

/// Library ids a feed may grab for: the one it names, or every library its owner has.
async fn feed_libraries(db: &Database, feed: &RssFeed) -> Result<HashSet<String>> {
    if let Some(library_id) = &feed.library_id {
        return Ok(HashSet::from([library_id.clone()]));
    }
    Ok(Library::query(db.pool())
        .filter(LibraryWhereInput {
            user_id: Some(string_eq(&feed.user_id)),
            ..Default::default()
        })
        .fetch_all()
        .await
        .context("failed to list the feed owner's libraries")?
        .into_iter()
        .map(|library| library.id)
        .collect())
}

/// Items of this feed already seen (by link hash), so only new ones are considered.
async fn seen_link_hashes(
    db: &Database,
    feed: &RssFeed,
    hashes: &[String],
) -> Result<HashSet<String>> {
    if hashes.is_empty() {
        return Ok(HashSet::new());
    }
    Ok(RssFeedItem::query(db.pool())
        .filter(RssFeedItemWhereInput {
            feed_id: Some(string_eq(&feed.id)),
            link_hash: Some(StringFilter {
                in_list: Some(hashes.to_vec()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await
        .context("failed to read the feed items already seen")?
        .into_iter()
        .map(|item| item.link_hash)
        .collect())
}

async fn remember_item(
    db: &Database,
    feed: &RssFeed,
    release: &SourceRelease,
    torrent_id: Option<&str>,
    skipped_reason: Option<&str>,
    now: DateTime<Utc>,
) {
    let (link_hash, title_hash) = item_hashes(release);
    let parsed = scoring::parse_release(&release.title);
    if let Err(error) = RssFeedItem::insert(
        db,
        CreateRssFeedItemInput {
            feed_id: feed.id.clone(),
            guid: Some(release.guid.clone()),
            link_hash,
            title_hash,
            title: release.title.clone(),
            link: release
                .magnet_uri
                .clone()
                .or_else(|| release.link.clone())
                .unwrap_or_else(|| release.guid.clone()),
            pub_date: Some(release.publish_date.to_rfc3339()),
            description: None,
            parsed_show_name: Some(title_prefix(&release.title)).filter(|name| !name.is_empty()),
            parsed_season: parsed.season,
            parsed_episode: parsed.episodes.first().copied(),
            parsed_resolution: parsed.resolution,
            parsed_codec: parsed.codec,
            parsed_source: parsed.source_type,
            parsed_audio: parsed.audio,
            parsed_hdr: parsed.hdr_type,
            processed: true,
            torrent_id: torrent_id.map(ToString::to_string),
            skipped_reason: skipped_reason.map(ToString::to_string),
            seen_at: now.to_rfc3339(),
        },
    )
    .await
    {
        warn!(
            feed = %feed.name,
            item = %release.title,
            error = %error,
            "RSS: could not remember a feed item"
        );
    }
}

async fn prune_old_items(db: &Database, now: DateTime<Utc>) {
    let cutoff = (now - chrono::Duration::days(ITEM_RETENTION_DAYS)).to_rfc3339();
    if let Err(error) = RssFeedItem::delete_where(
        db,
        RssFeedItemWhereInput {
            seen_at: Some(DateFilter {
                lt: Some(cutoff),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    {
        warn!(error = %error, "RSS: could not prune old feed items");
    }
}

// ===========================================================================
// The pass
// ===========================================================================

/// Poll every feed that is due, match what is new, and grab what is wanted.
///
/// Whoever calls this has decided automatic downloading is on; the pass does not check.
pub async fn poll_due_feeds(manager: &Arc<ServicesManager>) -> Result<RssPassSummary> {
    let db_service = manager
        .get_database()
        .await
        .context("database service not available")?;
    let db = db_service.pool().clone();
    let now = Utc::now();

    let feeds: Vec<RssFeed> = RssFeed::query(db.pool())
        .filter(RssFeedWhereInput {
            enabled: Some(graphql_orm::graphql::filters::BoolFilter {
                eq: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await
        .context("failed to list RSS feeds")?;
    let due: Vec<RssFeed> = feeds
        .into_iter()
        .filter(|feed| {
            feed_due(
                feed.last_polled_at.as_deref(),
                feed.poll_interval_minutes,
                feed.consecutive_failures.unwrap_or(0),
                now,
            )
        })
        .collect();

    let mut summary = RssPassSummary::default();
    if due.is_empty() {
        return Ok(summary);
    }

    let client = crate::services::http_client::outbound_client(
        crate::services::http_client::OutboundHttpProfile::Indexer,
    )?;

    // Fetch first, without holding the hunt lock: this is the slow part, and it grabs nothing.
    let mut fetched = Vec::new();
    for (position, feed) in due.into_iter().enumerate() {
        if position > 0 {
            tokio::time::sleep(GAP_BETWEEN_FEEDS).await;
        }
        let polled_at = Utc::now();
        match fetch_feed(&client, &feed).await {
            Ok(items) => {
                summary.feeds_polled += 1;
                info!(
                    feed = %feed.name,
                    items = items.len(),
                    "RSS: read {} item(s) from feed '{}'",
                    items.len(),
                    feed.name
                );
                record_poll_ok(&db, &feed, polled_at).await;
                fetched.push(FetchedFeed { feed, items });
            }
            Err(failure) => {
                summary.feeds_polled += 1;
                summary.feeds_failed += 1;
                warn!(
                    feed = %feed.name,
                    failures = feed.consecutive_failures.unwrap_or(0) + 1,
                    retry_after = ?failure.retry_after,
                    "RSS: feed '{}' failed: {}",
                    feed.name,
                    failure.message
                );
                record_poll_failed(&db, &feed, polled_at, &failure).await;
            }
        }
    }
    if fetched.is_empty() {
        return Ok(summary);
    }

    let torrent = manager
        .get_torrent()
        .await
        .context("torrent service not available")?;
    let env = GrabEnv {
        db: db.clone(),
        torrent,
        manager: manager.clone(),
    };

    let _hunting = auto_download::hunt_lock().await;
    let settings = AutoDownloadSettings::load(&db).await;
    let blocklist = Blocklist::load(&db).await.unwrap_or_else(|error| {
        warn!(error = %error, "RSS: failed to load the release blocklist; continuing without it");
        Blocklist::default()
    });
    let mut trail = Trail::new(&db);
    let mut budget = grab_budget(
        auto_download::count_in_flight(&db).await.unwrap_or(0),
        settings.max_concurrent_grabs,
    );

    for FetchedFeed { feed, items } in fetched {
        if let Err(error) = process_feed(
            &env,
            &feed,
            items,
            &settings,
            &blocklist,
            &mut trail,
            &mut budget,
            &mut summary,
        )
        .await
        {
            warn!(feed = %feed.name, error = %error, "RSS: could not process feed '{}'", feed.name);
        }
    }
    prune_old_items(&db, now).await;
    Ok(summary)
}

#[allow(clippy::too_many_arguments)]
async fn process_feed(
    env: &GrabEnv,
    feed: &RssFeed,
    items: Vec<SourceRelease>,
    settings: &AutoDownloadSettings,
    blocklist: &Blocklist,
    trail: &mut Trail,
    budget: &mut usize,
    summary: &mut RssPassSummary,
) -> Result<()> {
    let db = &env.db;
    let now = Utc::now();

    // New items only.
    let hashes: Vec<String> = items.iter().map(|item| item_hashes(item).0).collect();
    let seen = seen_link_hashes(db, feed, &hashes).await?;
    let new_items: Vec<SourceRelease> = items
        .into_iter()
        .zip(hashes)
        .filter(|(_, hash)| !seen.contains(hash))
        .map(|(item, _)| item)
        .collect();
    summary.new_items += new_items.len();
    if new_items.is_empty() {
        return Ok(());
    }

    // What is wanted, from the feed owner's libraries. A feed may find an episode before its
    // listed air time, so the grace period does not apply: a release that exists is a release.
    let libraries = feed_libraries(db, feed).await?;
    let scope = HuntScope::default();
    let feed_settings = AutoDownloadSettings {
        air_grace_minutes: 0,
        ..settings.clone()
    };
    let candidates = auto_download::discover_candidates(db, &scope, blocklist, &feed_settings, now)
        .await?
        .into_iter()
        .filter(|candidate| libraries.contains(&candidate.library_id))
        .collect::<Vec<_>>();
    let targets = flatten_targets(candidates);

    // Group what matches by target.
    let mut by_target: HashMap<usize, Vec<SourceRelease>> = HashMap::new();
    let mut fates: Vec<(SourceRelease, Vec<ItemFate>)> = Vec::new();
    for item in new_items {
        let item_fates = classify_item(&item, &targets, blocklist);
        for fate in &item_fates {
            if let ItemFate::Candidate(index) = fate {
                by_target.entry(*index).or_default().push(item.clone());
            }
        }
        fates.push((item, item_fates));
    }

    let trigger = Trigger::Feed(feed.name.clone());
    let mut grabbed_items: HashMap<String, String> = HashMap::new(); // item guid -> torrent id
    let mut unresolved: HashSet<String> = HashSet::new(); // item guids to leave for the next poll
    let mut item_notes: HashMap<String, String> = HashMap::new();
    let mut superseded: HashSet<usize> = HashSet::new();

    let mut order: Vec<usize> = by_target.keys().copied().collect();
    order.sort_unstable();
    for index in order {
        let releases = by_target.remove(&index).unwrap_or_default();
        summary.matched += 1;
        if superseded.contains(&index) {
            for release in &releases {
                item_notes.insert(
                    release.guid.clone(),
                    "a season pack already grabbed covers this episode".to_string(),
                );
            }
            continue;
        }
        let candidate = targets[index].clone();
        if *budget == 0 {
            // Left for the next poll rather than remembered as skipped.
            summary.capped += 1;
            for release in &releases {
                unresolved.insert(release.guid.clone());
            }
            continue;
        }

        let quality_profile = resolve_profile(db, &candidate).await;
        let picked = profile::pick_best_for_profile(
            &candidate.match_title,
            releases.clone(),
            &quality_profile,
            candidate.episode_count,
        );
        let Some(best) = picked else {
            let (too_different, below_profile, profile_reasons) = explain_profile_rejections(
                &candidate.match_title,
                &releases,
                &quality_profile,
                candidate.episode_count,
            );
            let why = describe_no_grab(&NoGrabFacts {
                sources: 1,
                seen: releases.len(),
                too_different,
                below_profile,
                profile: quality_profile.name.clone(),
                profile_reasons,
                ..Default::default()
            });
            info!(
                feed = %feed.name,
                candidate = %candidate.label,
                "RSS: feed '{}' has {} for {}, nothing taken: {why}",
                feed.name,
                if releases.len() == 1 { "a release" } else { "releases" },
                candidate.label
            );
            for release in &releases {
                item_notes.insert(release.guid.clone(), why.clone());
            }
            trail
                .waiting(
                    &candidate.library_id,
                    &candidate.label,
                    &why,
                    None,
                    &trigger,
                    true,
                )
                .await;
            continue;
        };

        let release = best.release;
        match auto_download::grab_release(env, &candidate, &release, Some(&feed.id)).await {
            Ok(Some(grabbed)) => {
                summary.grabbed += 1;
                *budget = budget.saturating_sub(1);
                info!(
                    feed = %feed.name,
                    candidate = %candidate.label,
                    torrent = %grabbed.torrent_name,
                    profile = %quality_profile.name,
                    "RSS: grabbed '{}' for {} from feed '{}'",
                    release.title,
                    candidate.label,
                    feed.name
                );
                trail
                    .grabbed(&GrabRecord {
                        library_id: &candidate.library_id,
                        label: &candidate.label,
                        release: &release.title,
                        source: Some(&feed.name),
                        profile: &quality_profile.name,
                        trigger: &trigger,
                        torrent_id: grabbed.torrent_id.as_deref(),
                    })
                    .await;
                grabbed_items.insert(release.guid.clone(), grabbed.torrent_id.unwrap_or_default());
                for other in &releases {
                    if other.guid != release.guid {
                        item_notes.insert(
                            other.guid.clone(),
                            format!("a better release of {} was grabbed", candidate.label),
                        );
                    }
                }
                for (other_index, other) in targets.iter().enumerate() {
                    if other_index != index && pack_covers(&candidate, other) {
                        superseded.insert(other_index);
                    }
                }
            }
            Ok(None) => {
                for release in &releases {
                    item_notes.insert(
                        release.guid.clone(),
                        "already in the download client".to_string(),
                    );
                }
            }
            Err(error) => {
                warn!(
                    feed = %feed.name,
                    candidate = %candidate.label,
                    error = %error,
                    "RSS: grab of '{}' for {} from feed '{}' failed",
                    release.title,
                    candidate.label,
                    feed.name
                );
                trail
                    .grab_failed(
                        &candidate.library_id,
                        &candidate.label,
                        &release.title,
                        &error.to_string(),
                    )
                    .await;
                // Not remembered, so the next poll tries again.
                for release in &releases {
                    unresolved.insert(release.guid.clone());
                }
            }
        }
    }

    // Remember everything seen, so it is not looked at again.
    for (item, item_fates) in &fates {
        if unresolved.contains(&item.guid) {
            continue;
        }
        let reason = match grabbed_items.get(&item.guid) {
            Some(_) => None,
            None => Some(match item_fates.first() {
                Some(ItemFate::Blocklisted) => "blocklisted".to_string(),
                Some(ItemFate::WrongTarget(reason)) => format!("wrong target: {reason}"),
                Some(ItemFate::Candidate(_)) => item_notes
                    .get(&item.guid)
                    .cloned()
                    .unwrap_or_else(|| "not taken".to_string()),
                _ => "not wanted".to_string(),
            }),
        };
        let torrent_id = grabbed_items
            .get(&item.guid)
            .map(String::as_str)
            .filter(|id| !id.is_empty());
        remember_item(db, feed, item, torrent_id, reason.as_deref(), now).await;
    }
    debug!(
        feed = %feed.name,
        remembered = fates.len().saturating_sub(unresolved.len()),
        "RSS: finished feed '{}'",
        feed.name
    );
    Ok(())
}

async fn resolve_profile(db: &Database, candidate: &WantedCandidate) -> QualityProfile {
    profile::resolve_profile(
        db,
        candidate.quality_profile_override_id.as_deref(),
        &candidate.library_id,
    )
    .await
    .unwrap_or_else(|error| {
        warn!(
            candidate = %candidate.label,
            error = %error,
            "RSS: failed to resolve quality profile; falling back to 'Any Quality'"
        );
        profile::builtin_any_profile()
    })
}

#[cfg(test)]
#[path = "rss_poll_tests.rs"]
mod tests;
