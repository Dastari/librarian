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
use graphql_orm::graphql::orm::OrderDirection;
use once_cell::sync::Lazy;
use regex::Regex;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use crate::db::Database;
use crate::graphql::entities::{
    Album, AlbumWhereInput, Artist, ArtistWhereInput, Audiobook, AudiobookWhereInput,
    CreateRssFeedItemInput, Library, LibraryWhereInput, Movie, MovieWhereInput, QualityProfile,
    RssFeed, RssFeedItem, RssFeedItemOrderByInput, RssFeedItemWhereInput, RssFeedWhereInput, Show,
    ShowWhereInput, UpdateRssFeedInput, UpdateRssFeedItemInput,
};
use crate::services::ServicesManager;
use crate::services::feed_catalogue::{self, ItemFacts, ItemMatch, TitleIndex};
use crate::services::quality::{profile, scoring};
use crate::services::redact::redact_text;
use crate::services::sources::SourceRelease;
use crate::services::sources::health::{self, FailureKind};

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

/// Why a feed address cannot be used, if it cannot: only `http` and `https` are fetched.
pub fn feed_url_problem(url: &str) -> Option<String> {
    match reqwest::Url::parse(url.trim()) {
        Err(_) => Some("the feed address is not a valid URL".to_string()),
        Ok(parsed) if !matches!(parsed.scheme(), "http" | "https") => Some(format!(
            "feeds must be http or https, not {}",
            parsed.scheme()
        )),
        Ok(parsed) if parsed.host_str().is_none() => {
            Some("the feed address has no host".to_string())
        }
        Ok(_) => None,
    }
}

fn validate_feed_url(url: &str) -> Result<reqwest::Url> {
    if let Some(problem) = feed_url_problem(url) {
        anyhow::bail!(problem);
    }
    reqwest::Url::parse(url.trim()).context("the feed address is not a valid URL")
}

/// The text of a feed's address: it is stored as an encrypted envelope (or, on rows written before
/// that, as plain text, which is encrypted the next time the feed is saved).
pub async fn reveal_feed_url(manager: &Arc<ServicesManager>, feed: &RssFeed) -> Result<String> {
    let sources = manager
        .get_sources()
        .await
        .context("the sources service is not available")?;
    let sources = sources
        .get_manager()
        .await
        .context("source credentials are not available: no credential key is configured")?;
    sources
        .encryption()
        .reveal(&feed.url)
        .context("the feed address could not be decrypted with the current credential key")
}

/// Reveal a stored item link the same way.
pub async fn reveal_item_link(manager: &Arc<ServicesManager>, stored: &str) -> Result<String> {
    let sources = manager
        .get_sources()
        .await
        .context("the sources service is not available")?;
    let sources = sources
        .get_manager()
        .await
        .context("source credentials are not available: no credential key is configured")?;
    sources
        .encryption()
        .reveal(stored)
        .context("the item link could not be decrypted with the current credential key")
}

/// Fetch one feed from `address` (its revealed URL). A host that asks to be left alone (429/503
/// with `Retry-After`) is believed. Nothing in an error may quote the address: it holds the
/// account's passkey.
async fn fetch_feed(
    client: &reqwest::Client,
    address: &str,
) -> Result<Vec<SourceRelease>, FeedFailure> {
    let url = validate_feed_url(address)
        .map_err(|error| FeedFailure::new(error.to_string(), None, FailureKind::Other))?;
    let response = client
        .get(url)
        .header(
            reqwest::header::ACCEPT,
            "application/rss+xml, application/xml, text/xml, */*",
        )
        .send()
        .await
        .map_err(|error| {
            FeedFailure::new(
                redact_text(&format!(
                    "the feed could not be reached: {}",
                    error.without_url()
                )),
                None,
                FailureKind::Unreachable,
            )
        })?;
    let status = response.status();
    if !status.is_success() {
        let retry_after = retry_after_seconds(
            response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok()),
        );
        let kind = health::classify_http_status(status.as_u16()).unwrap_or(FailureKind::Other);
        return Err(FeedFailure::new(
            match kind {
                FailureKind::RateLimited => "the feed host is rate limiting requests".to_string(),
                FailureKind::Auth => format!(
                    "the feed refused the request (HTTP {}): its link may have expired",
                    status.as_u16()
                ),
                _ => format!("the feed answered HTTP {status}"),
            },
            retry_after,
            kind,
        ));
    }
    let body = crate::services::http_client::response_text_limited(response, MAX_FEED_BYTES)
        .await
        .map_err(|error| {
            FeedFailure::new(
                redact_text(&format!("the feed could not be read: {error}")),
                None,
                FailureKind::Unreachable,
            )
        })?;
    // Some trackers answer an expired link with a login page and a 200.
    let head: String = body
        .chars()
        .take(2000)
        .collect::<String>()
        .to_ascii_lowercase();
    if !head.contains("<rss")
        && !head.contains("<feed")
        && (head.contains("<html") || head.contains("login"))
    {
        return Err(FeedFailure::new(
            "the feed returned a web page instead of RSS: its link may have expired".to_string(),
            None,
            FailureKind::Auth,
        ));
    }
    let mut items = crate::services::sources::definitions::torznab::parse_torznab_rss(&body)
        .map_err(|error| {
            FeedFailure::new(
                redact_text(&format!("the feed is not RSS 2.0: {error}")),
                None,
                FailureKind::Other,
            )
        })?;
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
    kind: FailureKind,
}

impl FeedFailure {
    fn new(message: String, retry_after: Option<i64>, kind: FailureKind) -> Self {
        Self {
            message,
            retry_after,
            kind,
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
            health_status: Some(Some(health::STATUS_OK.to_string())),
            health_kind: Some(None),
            ..Default::default()
        },
    )
    .await
    {
        warn!(feed = %feed.name, error = %error, "RSS: could not record a successful poll");
    }
    super::source_health::feed_outcome(db, feed, None).await;
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
            health_status: Some(Some(failure.kind.status().to_string())),
            health_kind: Some(Some(failure.kind.as_str().to_string())),
            ..Default::default()
        },
    )
    .await
    {
        warn!(feed = %feed.name, error = %error, "RSS: could not record a failed poll");
    }
    super::source_health::feed_outcome(
        db,
        feed,
        Some((
            failure.kind,
            &failure.message,
            feed.consecutive_failures.unwrap_or(0) + 1,
        )),
    )
    .await;
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

/// What the library holds that a feed's items may be matched to: every show, movie, album and
/// audiobook in the libraries the feed grabs for.
pub async fn load_title_index(db: &Database, libraries: &HashSet<String>) -> Result<TitleIndex> {
    use crate::services::feed_catalogue::{AlbumRef, BookRef, MovieRef, ShowRef};
    if libraries.is_empty() {
        return Ok(TitleIndex::default());
    }
    let in_libraries = StringFilter {
        in_list: Some(libraries.iter().cloned().collect()),
        ..Default::default()
    };
    let shows = Show::query(db.pool())
        .filter(ShowWhereInput {
            library_id: Some(in_libraries.clone()),
            ..Default::default()
        })
        .fetch_all()
        .await
        .context("failed to list shows for the feed catalogue")?
        .into_iter()
        .map(|show| ShowRef {
            id: show.id,
            library_id: show.library_id,
            name: show.name,
            year: show.year,
        })
        .collect();
    let movies = Movie::query(db.pool())
        .filter(MovieWhereInput {
            library_id: Some(in_libraries.clone()),
            ..Default::default()
        })
        .fetch_all()
        .await
        .context("failed to list movies for the feed catalogue")?
        .into_iter()
        .map(|movie| MovieRef {
            id: movie.id,
            library_id: movie.library_id,
            title: movie.title,
            year: movie.year,
        })
        .collect();
    let artists: HashMap<String, String> = Artist::query(db.pool())
        .filter(ArtistWhereInput {
            library_id: Some(in_libraries.clone()),
            ..Default::default()
        })
        .fetch_all()
        .await
        .context("failed to list artists for the feed catalogue")?
        .into_iter()
        .map(|artist| (artist.id, artist.name))
        .collect();
    let albums = Album::query(db.pool())
        .filter(AlbumWhereInput {
            library_id: Some(in_libraries.clone()),
            ..Default::default()
        })
        .fetch_all()
        .await
        .context("failed to list albums for the feed catalogue")?
        .into_iter()
        .map(|album| AlbumRef {
            artist: artists.get(&album.artist_id).cloned().unwrap_or_default(),
            id: album.id,
            library_id: album.library_id,
            name: album.name,
            year: album.year,
        })
        .collect();
    let books = Audiobook::query(db.pool())
        .filter(AudiobookWhereInput {
            library_id: Some(in_libraries),
            ..Default::default()
        })
        .fetch_all()
        .await
        .context("failed to list audiobooks for the feed catalogue")?
        .into_iter()
        .map(|book| BookRef {
            id: book.id,
            library_id: book.library_id,
            title: book.title,
            author: book.author_name,
        })
        .collect();
    Ok(TitleIndex::new(shows, movies, albums, books))
}

/// The columns an item's reading and match are stored in.
#[derive(Debug, Clone, Default)]
struct CatalogueColumns {
    size: Option<i64>,
    category: Option<String>,
    parsed_scope: Option<String>,
    parsed_year: Option<i32>,
    parsed_season_end: Option<i32>,
    parsed_group: Option<String>,
    parsed_artist: Option<String>,
    parsed_album: Option<String>,
    parsed_format: Option<String>,
    parsed_bitrate: Option<String>,
    match_kind: Option<String>,
    match_show_id: Option<String>,
    match_movie_id: Option<String>,
    match_album_id: Option<String>,
    match_audiobook_id: Option<String>,
    match_season: Option<i32>,
    match_episode: Option<i32>,
    match_label: Option<String>,
    match_confidence: Option<i32>,
    match_reason: Option<String>,
}

impl CatalogueColumns {
    fn new(facts: &ItemFacts, matched: &Result<ItemMatch, String>) -> Self {
        let mut columns = Self {
            size: facts.size,
            category: facts.category.clone(),
            parsed_scope: Some(facts.scope.as_str().to_string()),
            parsed_year: facts.year(),
            parsed_season_end: facts.parsed.season_end,
            parsed_group: facts.parsed.release_group.clone(),
            parsed_artist: facts.music.artist.clone(),
            parsed_album: facts.music.album.clone(),
            parsed_format: facts.music.format.clone(),
            parsed_bitrate: facts.music.bitrate.clone(),
            ..Default::default()
        };
        match matched {
            Ok(hit) => {
                columns.match_kind = Some(hit.kind.as_str().to_string());
                columns.match_show_id = hit.show_id.clone();
                columns.match_movie_id = hit.movie_id.clone();
                columns.match_album_id = hit.album_id.clone();
                columns.match_audiobook_id = hit.audiobook_id.clone();
                columns.match_season = hit.season;
                columns.match_episode = hit.episode;
                columns.match_label = Some(hit.label.clone());
                columns.match_confidence = Some(hit.confidence);
                columns.match_reason = Some(hit.reason.clone());
            }
            Err(reason) => columns.match_reason = Some(reason.clone()),
        }
        columns
    }
}

/// Read one release the way the catalogue does and say what it is.
fn describe_release(
    release: &SourceRelease,
    index: &TitleIndex,
) -> (ItemFacts, Result<ItemMatch, String>) {
    let facts = feed_catalogue::read_item(
        &release.title,
        release.description.as_deref(),
        &release.categories,
        release.size,
    );
    let matched = feed_catalogue::match_item(&facts, index);
    (facts, matched)
}

#[allow(clippy::too_many_arguments)]
async fn remember_item(
    db: &Database,
    feed: &RssFeed,
    release: &SourceRelease,
    index: &TitleIndex,
    torrent_id: Option<&str>,
    skipped_reason: Option<&str>,
    now: DateTime<Utc>,
    processed: bool,
) {
    let (link_hash, title_hash) = item_hashes(release);
    let parsed = scoring::parse_release(&release.title);
    let (facts, matched) = describe_release(release, index);
    let columns = CatalogueColumns::new(&facts, &matched);
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
            info_hash: release.info_hash.clone(),
            size: columns.size,
            seeders: release.seeders,
            category: columns.category,
            parsed_scope: columns.parsed_scope,
            parsed_year: columns.parsed_year,
            parsed_season_end: columns.parsed_season_end,
            parsed_group: columns.parsed_group,
            parsed_artist: columns.parsed_artist,
            parsed_album: columns.parsed_album,
            parsed_format: columns.parsed_format,
            parsed_bitrate: columns.parsed_bitrate,
            match_kind: columns.match_kind,
            match_show_id: columns.match_show_id,
            match_movie_id: columns.match_movie_id,
            match_album_id: columns.match_album_id,
            match_audiobook_id: columns.match_audiobook_id,
            match_season: columns.match_season,
            match_episode: columns.match_episode,
            match_label: columns.match_label,
            match_confidence: columns.match_confidence,
            match_reason: columns.match_reason,
            matched_at: Some(now.to_rfc3339()),
            pub_date: Some(release.publish_date.to_rfc3339()),
            description: release.description.clone(),
            parsed_show_name: Some(title_prefix(&release.title)).filter(|name| !name.is_empty()),
            parsed_season: parsed.season,
            parsed_episode: parsed.episodes.first().copied(),
            parsed_resolution: parsed.resolution,
            parsed_codec: parsed.codec,
            parsed_source: parsed.source_type,
            parsed_audio: parsed.audio,
            parsed_hdr: parsed.hdr_type,
            processed,
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
            error = %redact_text(&error.to_string()),
            "RSS: could not remember a feed item"
        );
    }
}

/// How many unmatched items a poll looks at again.
const REMATCH_BATCH: i64 = 400;

/// Items that matched nothing when they were seen may match now (a title was added to the
/// library), and items remembered before the catalogue existed have no reading at all. Look at
/// the newest unmatched ones again. Returns how many are now matched.
pub async fn rematch_unmatched(db: &Database, feed: &RssFeed, index: &TitleIndex) -> usize {
    if index.is_empty() {
        return 0;
    }
    let rows = match RssFeedItem::query(db.pool())
        .filter(RssFeedItemWhereInput {
            feed_id: Some(string_eq(&feed.id)),
            match_kind: Some(StringFilter {
                is_null: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        })
        .order_by(RssFeedItemOrderByInput {
            seen_at: Some(OrderDirection::Desc),
            ..Default::default()
        })
        .limit(REMATCH_BATCH)
        .fetch_all()
        .await
    {
        Ok(rows) => rows,
        Err(error) => {
            warn!(feed = %feed.name, error = %error, "RSS: could not look for items to match again");
            return 0;
        }
    };
    let now = Utc::now().to_rfc3339();
    let mut newly_matched = 0;
    for row in rows {
        let facts =
            feed_catalogue::read_item(&row.title, row.description.as_deref(), &[], row.size);
        let matched = feed_catalogue::match_item(&facts, index);
        let columns = CatalogueColumns::new(&facts, &matched);
        // Nothing to write when it is still unmatched for the same reason and already read.
        if matched.is_err()
            && row.parsed_scope.is_some()
            && row.match_reason == columns.match_reason
        {
            continue;
        }
        if matched.is_ok() {
            newly_matched += 1;
        }
        if let Err(error) = RssFeedItem::update_by_id(
            db,
            &row.id,
            UpdateRssFeedItemInput {
                size: Some(columns.size),
                category: Some(columns.category),
                parsed_scope: Some(columns.parsed_scope),
                parsed_year: Some(columns.parsed_year),
                parsed_season_end: Some(columns.parsed_season_end),
                parsed_group: Some(columns.parsed_group),
                parsed_artist: Some(columns.parsed_artist),
                parsed_album: Some(columns.parsed_album),
                parsed_format: Some(columns.parsed_format),
                parsed_bitrate: Some(columns.parsed_bitrate),
                match_kind: Some(columns.match_kind),
                match_show_id: Some(columns.match_show_id),
                match_movie_id: Some(columns.match_movie_id),
                match_album_id: Some(columns.match_album_id),
                match_audiobook_id: Some(columns.match_audiobook_id),
                match_season: Some(columns.match_season),
                match_episode: Some(columns.match_episode),
                match_label: Some(columns.match_label),
                match_confidence: Some(columns.match_confidence),
                match_reason: Some(columns.match_reason),
                matched_at: Some(Some(now.clone())),
                ..Default::default()
            },
        )
        .await
        {
            warn!(feed = %feed.name, item = %row.title, error = %error, "RSS: could not store an item's match");
        }
    }
    newly_matched
}

/// Bring feeds and items written before addresses were encrypted up to date: saving a value
/// through the generated update runs the write transform, which encrypts it and, for a feed,
/// stores the redacted display form. Cheap when there is nothing to do.
pub async fn secure_stored_secrets(db: &Database) {
    use crate::services::sources::encryption::CredentialEncryption;
    let feeds = RssFeed::query(db.pool())
        .fetch_all()
        .await
        .unwrap_or_default();
    for feed in feeds {
        let encrypted = CredentialEncryption::looks_like_envelope(&feed.url);
        if encrypted && feed.url_display.is_some() {
            continue;
        }
        if encrypted {
            // Encrypted but without a display form: nothing to redact from without the key here.
            continue;
        }
        if let Err(error) = RssFeed::update_by_id(
            db,
            &feed.id,
            UpdateRssFeedInput {
                // Saving an address clears the feed's failure count (write_rules), which is right
                // here too: the next poll finds out again.
                url: Some(feed.url.clone()),
                ..Default::default()
            },
        )
        .await
        {
            warn!(feed = %feed.name, error = %redact_text(&error.to_string()), "RSS: could not encrypt a stored feed address");
        }
    }
    let plain_items = RssFeedItem::query(db.pool())
        .filter(RssFeedItemWhereInput::default())
        .fetch_all()
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|item| !CredentialEncryption::looks_like_envelope(&item.link))
        .collect::<Vec<_>>();
    for item in plain_items {
        let _ = RssFeedItem::update_by_id(
            db,
            &item.id,
            UpdateRssFeedItemInput {
                link: Some(item.link.clone()),
                ..Default::default()
            },
        )
        .await;
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

/// Poll every feed that is due. Items are always read into the catalogue; with `grab` they are
/// also matched against what is wanted and grabbed. The caller decides `grab` (automatic
/// downloading is on); a feed is a source, so its catalogue fills either way.
pub async fn poll_due_feeds(manager: &Arc<ServicesManager>, grab: bool) -> Result<RssPassSummary> {
    poll_feeds(manager, None, false, grab).await
}

/// Poll `only_feed` (or every enabled feed), whether or not it is due when `force` is set (a
/// person pressed Poll now; a disabled feed is still polled then).
pub async fn poll_feeds(
    manager: &Arc<ServicesManager>,
    only_feed: Option<&str>,
    force: bool,
    grab: bool,
) -> Result<RssPassSummary> {
    let db_service = manager
        .get_database()
        .await
        .context("database service not available")?;
    let db = db_service.pool().clone();
    let now = Utc::now();

    secure_stored_secrets(&db).await;

    let feeds: Vec<RssFeed> = RssFeed::query(db.pool())
        .filter(RssFeedWhereInput {
            id: only_feed.map(string_eq),
            enabled: (only_feed.is_none()).then(|| graphql_orm::graphql::filters::BoolFilter {
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
            force
                || feed_due(
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
        let fetched_items = match reveal_feed_url(manager, &feed).await {
            Ok(address) => fetch_feed(&client, &address).await,
            Err(error) => Err(FeedFailure::new(
                redact_text(&format!("{error:#}")),
                None,
                FailureKind::Auth,
            )),
        };
        match fetched_items {
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

    if !grab {
        for FetchedFeed { feed, items } in fetched {
            if let Err(error) = catalogue_only(&db, &feed, items, &mut summary).await {
                warn!(feed = %feed.name, error = %redact_text(&format!("{error:#}")), "RSS: could not catalogue feed '{}'", feed.name);
            }
        }
        prune_old_items(&db, now).await;
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

/// Read a feed's new items into the catalogue without matching them to wanted targets or grabbing.
async fn catalogue_only(
    db: &Database,
    feed: &RssFeed,
    items: Vec<SourceRelease>,
    summary: &mut RssPassSummary,
) -> Result<()> {
    let now = Utc::now();
    let hashes: Vec<String> = items.iter().map(|item| item_hashes(item).0).collect();
    let seen = seen_link_hashes(db, feed, &hashes).await?;
    let new_items: Vec<SourceRelease> = items
        .into_iter()
        .zip(hashes)
        .filter(|(_, hash)| !seen.contains(hash))
        .map(|(item, _)| item)
        .collect();
    summary.new_items += new_items.len();
    let libraries = feed_libraries(db, feed).await?;
    let index = load_title_index(db, &libraries).await?;
    for item in &new_items {
        remember_item(db, feed, item, &index, None, None, now, false).await;
    }
    rematch_unmatched(db, feed, &index).await;
    Ok(())
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

    // What the feed's items are matched to in the catalogue.
    let catalogue_libraries = feed_libraries(db, feed).await?;
    let index = load_title_index(db, &catalogue_libraries).await?;
    if new_items.is_empty() {
        rematch_unmatched(db, feed, &index).await;
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
            let rejections = explain_profile_rejections(
                &candidate.match_title,
                &releases,
                &quality_profile,
                candidate.episode_count,
            );
            let why = describe_no_grab(&NoGrabFacts {
                sources: 1,
                seen: releases.len(),
                too_different: rejections.too_different,
                too_different_example: rejections.too_different_example,
                below_profile: rejections.below,
                profile: quality_profile.name.clone(),
                profile_reasons: rejections.reasons,
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
        remember_item(
            db,
            feed,
            item,
            &index,
            torrent_id,
            reason.as_deref(),
            now,
            true,
        )
        .await;
    }
    rematch_unmatched(db, feed, &index).await;
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
