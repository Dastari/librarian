//! Torznab/Newznab client source implementation
//!
//! A generic client for any Torznab-compatible **torrent** indexer API — the
//! shape exposed by Jackett/Prowlarr proxies and many native indexer APIs.
//! Deliberately scoped to `SourceType::TorrentIndexer` per
//! `docs/tier1-features-plan.md` §3: real Newznab/usenet support (NZB
//! download + NNTP posting) is a Tier-2 follow-on since no NNTP client exists
//! in this codebase — this source only ever produces torrent/magnet links.
//!
//! # Configuration
//! - `site_url`: the indexer's Torznab base URL (required). If it doesn't
//!   already end in `/api`, `/api` is appended (see [`TorznabSource::resolve_api_base`]).
//! - Required credential: `ApiKey`.
//! - Optional setting: `DefaultCategories` — comma-separated Torznab category
//!   IDs used when a query specifies none (mirrors the `iptorrents`
//!   `optional_settings` shape at `definitions/mod.rs`).
//!
//! # Capability discovery
//! [`TorznabSource::test_connection`] calls `t=caps` and parses `<caps>` into
//! a [`SourceCapabilities`], cached once on the struct via [`OnceLock`] (a
//! fresh instance is created whenever `SourcesService` reloads, so "set once
//! per instance" is not a staleness problem in practice). If `t=caps` errors,
//! is unimplemented, or the response can't be parsed, a conservative default
//! capability set (computed at construction from `DefaultCategories`, or a
//! generic bucket set otherwise) is used instead — fail-soft, per the plan's
//! explicit "cache aggressively, fail soft if t=caps is unimplemented" note.
//!
//! # Category handling
//! Outbound category IDs are already Torznab-shaped in `SourceQuery`
//! (`sources/types.rs` doc: "modeled after the Torznab specification"), so no
//! translation is strictly required. We still route them through
//! [`SourceCapabilities::map_torznab_to_tracker`] (reusing the existing
//! parent-bucket-fallback logic in `types.rs`, not reimplementing it) against
//! an *identity* [`CategoryMapping`] table built from `<caps><categories>` —
//! this gets us "query for the 5000 TV bucket even though the indexer only
//! advertises subcat 5030" for free.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{Result, anyhow};
use async_graphql::async_trait::async_trait;
use chrono::{DateTime, Utc};
use quick_xml::Reader;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use reqwest::Client;

use crate::services::rate_limiter::RateLimitedClient;
use crate::services::sources::categories::CategoryMapping;
use crate::services::sources::categories::cats;
use crate::services::sources::{
    BookSearchParam, MovieSearchParam, MusicSearchParam, QueryType, Source, SourceCapabilities,
    SourceQuery, SourceRelease, SourceType, TrackerType, TvSearchParam,
};

/// Generic Torznab/Newznab client source (torrent indexers only).
pub struct TorznabSource {
    id: String,
    name: String,
    site_link: String,
    /// Fully-resolved `.../api` endpoint URL.
    api_base: String,
    api_key: String,
    /// Torznab category IDs used when a query specifies none.
    default_categories: Vec<i32>,
    client: Client,
    rate_limiter: RateLimitedClient,
    /// Populated once by `test_connection`'s `t=caps` call; falls back to
    /// `fallback_capabilities` until then (or forever, if `t=caps` fails).
    capabilities: OnceLock<SourceCapabilities>,
    fallback_capabilities: SourceCapabilities,
}

impl TorznabSource {
    pub fn new(
        id: String,
        name: String,
        site_url: Option<String>,
        api_key: &str,
        settings: HashMap<String, String>,
    ) -> Result<Self> {
        let site_link = site_url
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                anyhow!("Torznab source requires a site_url (the indexer's Torznab base URL)")
            })?;
        let site_link = if site_link.ends_with('/') {
            site_link
        } else {
            format!("{}/", site_link)
        };
        let api_base = Self::resolve_api_base(&site_link);

        if api_key.trim().is_empty() {
            tracing::warn!(
                source_id = %id,
                "Torznab source has no ApiKey configured - requests will likely fail"
            );
        }

        let default_categories = settings
            .get("DefaultCategories")
            .map(|s| Self::parse_category_list(s))
            .unwrap_or_default();

        let fallback_capabilities = Self::default_capabilities(&default_categories);

        Ok(Self {
            id,
            name,
            site_link,
            api_base,
            api_key: api_key.to_string(),
            default_categories,
            client: crate::services::http_client::outbound_client_builder(
                crate::services::http_client::OutboundHttpProfile::Indexer,
            )
            .gzip(true)
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()?,
            rate_limiter: RateLimitedClient::for_indexer(),
            capabilities: OnceLock::new(),
            fallback_capabilities,
        })
    }

    /// Torznab indexers conventionally expose the endpoint at `<base>/api`.
    /// If `site_url` already points directly at it, use it as-is.
    fn resolve_api_base(site_link: &str) -> String {
        let trimmed = site_link.trim_end_matches('/');
        if trimmed.ends_with("/api") {
            trimmed.to_string()
        } else {
            format!("{}/api", trimmed)
        }
    }

    fn parse_category_list(raw: &str) -> Vec<i32> {
        raw.split(',')
            .filter_map(|s| s.trim().parse::<i32>().ok())
            .collect()
    }

    /// Conservative default capabilities, used until (or unless) `t=caps`
    /// succeeds. Generic-but-permissive: most Torznab indexers support basic
    /// search plus TV/movie search with the common ID params.
    fn default_capabilities(default_categories: &[i32]) -> SourceCapabilities {
        let categories = if default_categories.is_empty() {
            vec![
                CategoryMapping::new(cats::MOVIES.to_string(), cats::MOVIES, "Movies"),
                CategoryMapping::new(cats::TV.to_string(), cats::TV, "TV"),
                CategoryMapping::new(cats::AUDIO.to_string(), cats::AUDIO, "Audio"),
                CategoryMapping::new(cats::BOOKS.to_string(), cats::BOOKS, "Books"),
                CategoryMapping::new(cats::PC.to_string(), cats::PC, "PC"),
                CategoryMapping::new(cats::CONSOLE.to_string(), cats::CONSOLE, "Console"),
            ]
        } else {
            default_categories
                .iter()
                .map(|c| CategoryMapping::new(c.to_string(), *c, format!("Category {}", c)))
                .collect()
        };

        SourceCapabilities {
            search_available: true,
            tv_search_params: vec![
                TvSearchParam::Q,
                TvSearchParam::Season,
                TvSearchParam::Ep,
                TvSearchParam::ImdbId,
                TvSearchParam::TvdbId,
                TvSearchParam::Year,
            ],
            movie_search_params: vec![
                MovieSearchParam::Q,
                MovieSearchParam::ImdbId,
                MovieSearchParam::TmdbId,
                MovieSearchParam::Year,
            ],
            music_search_params: vec![
                MusicSearchParam::Q,
                MusicSearchParam::Artist,
                MusicSearchParam::Album,
                MusicSearchParam::Year,
            ],
            book_search_params: vec![
                BookSearchParam::Q,
                BookSearchParam::Title,
                BookSearchParam::Author,
            ],
            categories,
        }
    }

    /// Build the outbound query parameters for a search request.
    fn build_query_params(&self, query: &SourceQuery) -> Vec<(String, String)> {
        let mut params = vec![
            ("t".to_string(), query.query_type.to_string()),
            ("apikey".to_string(), self.api_key.clone()),
            ("extended".to_string(), "1".to_string()),
        ];

        if let Some(term) = query.search_term.as_deref() {
            let term = term.trim();
            if !term.is_empty() {
                params.push(("q".to_string(), term.to_string()));
            }
        }

        let categories = self.resolve_categories(query);
        if !categories.is_empty() {
            params.push(("cat".to_string(), categories.join(",")));
        }

        if let Some(season) = query.season {
            params.push(("season".to_string(), season.to_string()));
        }
        if let Some(ref ep) = query.episode {
            params.push(("ep".to_string(), ep.clone()));
        }
        if let Some(imdb) = query.imdb_id_short() {
            params.push(("imdbid".to_string(), imdb));
        }
        if let Some(tvdb) = query.tvdb_id {
            params.push(("tvdbid".to_string(), tvdb.to_string()));
        }
        if let Some(tmdb) = query.tmdb_id {
            params.push(("tmdbid".to_string(), tmdb.to_string()));
        }
        if let Some(tvmaze) = query.tvmaze_id {
            params.push(("tvmazeid".to_string(), tvmaze.to_string()));
        }
        if let Some(year) = query.year {
            params.push(("year".to_string(), year.to_string()));
        }
        if let Some(ref genre) = query.genre {
            params.push(("genre".to_string(), genre.clone()));
        }
        if let Some(ref album) = query.album {
            params.push(("album".to_string(), album.clone()));
        }
        if let Some(ref artist) = query.artist {
            params.push(("artist".to_string(), artist.clone()));
        }
        if let Some(ref title) = query.title {
            params.push(("title".to_string(), title.clone()));
        }
        if let Some(ref author) = query.author {
            params.push(("author".to_string(), author.clone()));
        }
        if let Some(limit) = query.limit
            && limit > 0
        {
            params.push(("limit".to_string(), limit.to_string()));
        }
        if let Some(offset) = query.offset
            && offset > 0
        {
            params.push(("offset".to_string(), offset.to_string()));
        }

        params
    }

    /// Resolve the Torznab category IDs to request: the query's own
    /// categories if set, else the configured `DefaultCategories`. Routed
    /// through the capabilities' identity mapping so the parent-bucket
    /// fallback in `map_torznab_to_tracker` applies (see module doc).
    fn resolve_categories(&self, query: &SourceQuery) -> Vec<String> {
        let requested: Vec<i32> = if !query.categories.is_empty() {
            query.categories.clone()
        } else {
            self.default_categories.clone()
        };
        if requested.is_empty() {
            return vec![];
        }

        let caps = self.capabilities();
        if caps.categories.is_empty() {
            return requested.iter().map(|c| c.to_string()).collect();
        }

        let mapped = caps.map_torznab_to_tracker(&requested);
        if mapped.is_empty() {
            requested.iter().map(|c| c.to_string()).collect()
        } else {
            mapped
        }
    }
}

#[async_trait]
impl Source for TorznabSource {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn source_type(&self) -> SourceType {
        SourceType::TorrentIndexer
    }

    fn definition_id(&self) -> &str {
        "torznab"
    }

    fn site_link(&self) -> &str {
        &self.site_link
    }

    fn tracker_type(&self) -> TrackerType {
        // Most Torznab APIs (native or Jackett/Prowlarr-proxied) require an
        // API key, which is the closer fit to "Private" of the three
        // variants; there's no per-indexer signal available to do better.
        TrackerType::Private
    }

    fn capabilities(&self) -> &SourceCapabilities {
        self.capabilities
            .get()
            .unwrap_or(&self.fallback_capabilities)
    }

    fn is_configured(&self) -> bool {
        !self.api_key.trim().is_empty()
    }

    async fn test_connection(&self) -> Result<bool> {
        self.rate_limiter.wait_for_permit().await;
        let response = self
            .client
            .get(&self.api_base)
            .query(&[("t", "caps"), ("apikey", self.api_key.as_str())])
            .send()
            .await?;

        let status = response.status();
        let body = crate::services::http_client::response_text_limited(
            response,
            crate::services::http_client::INDEXER_RESPONSE_LIMIT,
        )
        .await?;

        if !status.is_success() {
            tracing::warn!(
                source_id = %self.id,
                status = %status,
                "Torznab t=caps request failed"
            );
            return Ok(false);
        }

        if let Some((code, desc)) = extract_newznab_error(&body) {
            // Newznab error code 100/101 = invalid API key/user - treat as a
            // real connection failure. Anything else (e.g. "function not
            // available" for indexers that don't implement t=caps) is a soft
            // failure: keep the fallback capabilities and report "connected".
            if code == "100" || code == "101" {
                tracing::warn!(
                    source_id = %self.id,
                    code = %code,
                    desc = %desc,
                    "Torznab indexer rejected API key"
                );
                return Ok(false);
            }

            tracing::warn!(
                source_id = %self.id,
                code = %code,
                desc = %desc,
                "Torznab indexer returned an error for t=caps; falling back to default capabilities"
            );
            return Ok(true);
        }

        match parse_caps_xml(&body) {
            Ok(caps) if caps.search_available || !caps.categories.is_empty() => {
                let _ = self.capabilities.set(caps);
            }
            Ok(_) => {
                tracing::warn!(
                    source_id = %self.id,
                    "Torznab t=caps response had no usable data; keeping default capabilities"
                );
            }
            Err(e) => {
                tracing::warn!(
                    source_id = %self.id,
                    error = %e,
                    "Failed to parse Torznab t=caps response; keeping default capabilities"
                );
            }
        }

        Ok(true)
    }

    async fn search(&self, query: &SourceQuery) -> Result<Vec<SourceRelease>> {
        let params = self.build_query_params(query);

        self.rate_limiter.wait_for_permit().await;
        let response = self
            .client
            .get(&self.api_base)
            .query(&params)
            .send()
            .await?;
        let status = response.status();
        let body = crate::services::http_client::response_text_limited(
            response,
            crate::services::http_client::INDEXER_RESPONSE_LIMIT,
        )
        .await?;

        if !status.is_success() {
            return Err(anyhow!(
                "Torznab search failed with status {}: {}",
                status,
                truncate(&body, 300)
            ));
        }

        if let Some((code, desc)) = extract_newznab_error(&body) {
            return Err(anyhow!("Torznab indexer returned error {}: {}", code, desc));
        }

        parse_torznab_rss(&body)
    }

    async fn download(&self, link: &str) -> Result<Vec<u8>> {
        if link.starts_with("magnet:") {
            anyhow::bail!("Magnet links are not directly downloadable");
        }
        self.rate_limiter.wait_for_permit().await;
        let response = self.client.get(link).send().await?;
        if !response.status().is_success() {
            return Err(anyhow!(
                "Download failed with status: {}",
                response.status()
            ));
        }
        Ok(crate::services::http_client::response_bytes_limited(
            response,
            crate::services::http_client::DOWNLOAD_RESPONSE_LIMIT,
        )
        .await?
        .to_vec())
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

// ---------------------------------------------------------------------------
// XML parsing
// ---------------------------------------------------------------------------

fn start_local_name(e: &BytesStart) -> String {
    e.name().local_name().as_ref().to_lowercase()
}

fn end_local_name(e: &BytesEnd) -> String {
    e.name().local_name().as_ref().to_lowercase()
}

fn get_attr(e: &BytesStart, key: &str) -> Option<String> {
    for attr in e.attributes().flatten() {
        if attr.key.local_name().as_ref() == key
            && let Ok(v) = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
        {
            return Some(v.to_string());
        }
    }
    None
}

/// Extract a Newznab-style `<error code="..." description="..."/>` response,
/// if present (used for both t=caps and t=search responses).
fn extract_newznab_error(xml: &str) -> Option<(String, String)> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        let event = reader.read_event_into(&mut buf).ok()?;
        match event {
            Event::Start(e) | Event::Empty(e) if start_local_name(&e) == "error" => {
                let code = get_attr(&e, "code").unwrap_or_default();
                let desc = get_attr(&e, "description").unwrap_or_default();
                return Some((code, desc));
            }
            Event::Eof => return None,
            _ => {}
        }
        buf.clear();
    }
}

fn parse_rfc2822_date(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    DateTime::parse_from_rfc2822(s)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        })
}

/// Accumulator for a single `<item>` while streaming through the RSS.
#[derive(Default)]
struct RawItem {
    title: Option<String>,
    guid: Option<String>,
    link: Option<String>,
    comments: Option<String>,
    pub_date: Option<String>,
    description: Option<String>,
    categories: Vec<i32>,
    size: Option<i64>,
    files: Option<i32>,
    grabs: Option<i32>,
    seeders: Option<i32>,
    leechers: Option<i32>,
    peers_total: Option<i32>,
    info_hash: Option<String>,
    magnet_uri: Option<String>,
    imdb: Option<i64>,
    tmdb: Option<i64>,
    tvdb_id: Option<i64>,
    tvmaze_id: Option<i64>,
    year: Option<i32>,
    poster: Option<String>,
    download_volume_factor: Option<f64>,
    upload_volume_factor: Option<f64>,
    minimum_ratio: Option<f64>,
    minimum_seed_time: Option<i64>,
    /// Quality-related torznab:attr values (resolution/codec/audio format)
    /// that don't have a dedicated `SourceRelease` field; folded into the
    /// title (if not already present) so `services::quality::scoring`'s
    /// title parser picks them up downstream.
    quality_hints: Vec<String>,
}

impl RawItem {
    fn into_release(self) -> SourceRelease {
        let publish_date = self
            .pub_date
            .as_deref()
            .and_then(parse_rfc2822_date)
            .unwrap_or_else(Utc::now);

        let mut title = self.title.unwrap_or_default();
        if !self.quality_hints.is_empty() {
            let lower_title = title.to_lowercase();
            for hint in &self.quality_hints {
                if !hint.is_empty() && !lower_title.contains(&hint.to_lowercase()) {
                    title.push(' ');
                    title.push_str(hint);
                }
            }
        }

        let guid = self
            .guid
            .or_else(|| self.link.clone())
            .unwrap_or_else(|| title.clone());

        let peers = match (self.peers_total, self.seeders, self.leechers) {
            (Some(p), _, _) => Some(p),
            (None, Some(s), Some(l)) => Some(s + l),
            _ => None,
        };

        SourceRelease {
            title,
            guid,
            link: self.link,
            magnet_uri: self.magnet_uri,
            info_hash: self.info_hash,
            details: self.comments,
            publish_date,
            categories: self.categories,
            size: self.size,
            files: self.files,
            grabs: self.grabs,
            description: self.description,
            imdb: self.imdb,
            tmdb: self.tmdb,
            tvdb_id: self.tvdb_id,
            tvmaze_id: self.tvmaze_id,
            year: self.year,
            seeders: self.seeders,
            peers,
            poster: self.poster,
            download_volume_factor: self.download_volume_factor.unwrap_or(1.0),
            upload_volume_factor: self.upload_volume_factor.unwrap_or(1.0),
            minimum_ratio: self.minimum_ratio,
            minimum_seed_time: self.minimum_seed_time,
            source_id: None,
            source_name: None,
        }
    }
}

fn apply_enclosure_attrs(e: &BytesStart, item: &mut RawItem) {
    if let Some(url) = get_attr(e, "url") {
        if url.starts_with("magnet:") {
            if item.magnet_uri.is_none() {
                item.magnet_uri = Some(url);
            }
        } else if item.link.is_none() {
            item.link = Some(url);
        }
    }
    if item.size.is_none()
        && let Some(length) = get_attr(e, "length").and_then(|s| s.parse::<i64>().ok())
    {
        item.size = Some(length);
    }
}

/// Handle a `<torznab:attr name="..." value="..."/>` (or `<newznab:attr ...>`)
/// element — `local_name()` strips the namespace prefix so both are treated
/// uniformly.
fn apply_torznab_attr(e: &BytesStart, item: &mut RawItem) {
    let Some(name) = get_attr(e, "name") else {
        return;
    };
    let Some(value) = get_attr(e, "value") else {
        return;
    };

    match name.to_lowercase().as_str() {
        "category" => {
            if let Ok(cat) = value.parse::<i32>() {
                item.categories.push(cat);
            }
        }
        "seeders" => item.seeders = value.parse().ok(),
        "peers" => item.peers_total = value.parse().ok(),
        "leechers" => item.leechers = value.parse().ok(),
        "infohash" | "info_hash" => item.info_hash = Some(value),
        "magneturl" | "magnetlink" | "magnet" if item.magnet_uri.is_none() => {
            item.magnet_uri = Some(value);
        }
        "size" if item.size.is_none() => item.size = value.parse().ok(),
        "files" => item.files = value.parse().ok(),
        "grabs" => item.grabs = value.parse().ok(),
        "downloadvolumefactor" => item.download_volume_factor = value.parse().ok(),
        "uploadvolumefactor" => item.upload_volume_factor = value.parse().ok(),
        "minimumratio" => item.minimum_ratio = value.parse().ok(),
        "minimumseedtime" => item.minimum_seed_time = value.parse().ok(),
        "imdb" | "imdbid" => {
            item.imdb = value.trim_start_matches("tt").parse().ok();
        }
        "tmdbid" | "tmdb" => item.tmdb = value.parse().ok(),
        "tvdbid" | "tvdb" => item.tvdb_id = value.parse().ok(),
        "tvmazeid" | "tvmaze" => item.tvmaze_id = value.parse().ok(),
        "year" => item.year = value.parse().ok(),
        "poster" | "coverurl" | "cover" if item.poster.is_none() => item.poster = Some(value),
        "resolution" | "videoresolution" | "videocodec" | "codec" | "audiocodec"
        | "audioformat" => item.quality_hints.push(value),
        _ => {}
    }
}

fn apply_text_field(tag: &str, text: &str, item: &mut RawItem) {
    match tag {
        "title" => item.title = Some(text.to_string()),
        "guid" => item.guid = Some(text.to_string()),
        "link" if item.link.is_none() && item.magnet_uri.is_none() && !text.is_empty() => {
            if text.starts_with("magnet:") {
                item.magnet_uri = Some(text.to_string());
            } else {
                item.link = Some(text.to_string());
            }
        }
        "comments" => item.comments = Some(text.to_string()),
        "pubdate" => item.pub_date = Some(text.to_string()),
        "description" => item.description = Some(text.to_string()),
        "category" => {
            if let Ok(cat) = text.parse::<i32>() {
                item.categories.push(cat);
            }
        }
        "size" if item.size.is_none() => item.size = text.parse().ok(),
        _ => {}
    }
}

/// Parse a Torznab/Newznab RSS 2.0 search response into `SourceRelease`s.
fn parse_torznab_rss(xml: &str) -> Result<Vec<SourceRelease>> {
    let mut reader = Reader::from_str(xml);
    // Not `trim_text(true)`: an entity reference splits element text into
    // several events, and trimming each one separately eats the spaces around
    // it ("Show &amp; Tell" -> "Show&Tell"). The whole accumulated buffer is
    // trimmed once, at the closing tag.
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    let mut releases = Vec::new();
    let mut current: Option<RawItem> = None;
    let mut text_buf = String::new();

    loop {
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| anyhow!("Failed to parse Torznab RSS XML: {}", e))?;
        match event {
            Event::Start(e) => {
                let local = start_local_name(&e);
                if local == "item" {
                    current = Some(RawItem::default());
                } else if let Some(item) = current.as_mut() {
                    if local == "enclosure" {
                        apply_enclosure_attrs(&e, item);
                    } else if local == "attr" {
                        apply_torznab_attr(&e, item);
                    }
                }
                text_buf.clear();
            }
            Event::Empty(e) => {
                let local = start_local_name(&e);
                if local == "item" {
                    // A self-closing <item/> is degenerate but shouldn't crash parsing.
                    releases.push(RawItem::default().into_release());
                } else if let Some(item) = current.as_mut() {
                    if local == "enclosure" {
                        apply_enclosure_attrs(&e, item);
                    } else if local == "attr" {
                        apply_torznab_attr(&e, item);
                    }
                }
            }
            Event::Text(t) => {
                if let Ok(unescaped) = quick_xml::escape::unescape(&t) {
                    text_buf.push_str(&unescaped);
                }
            }
            Event::CData(t) => {
                text_buf.push_str(&t);
            }
            // quick-xml reports an entity reference inside element text as its
            // own event rather than as part of the surrounding `Text`. Ignoring
            // it silently *deletes* the character: a `<link>` holding
            // `magnet:?xt=urn:btih:HASH&amp;dn=...&amp;tr=...` came back with
            // every `&` missing, so the display name and every tracker were
            // swallowed into the info hash and the magnet resolved against DHT
            // alone. Indexers overwhelmingly put the magnet in `<link>`.
            Event::GeneralRef(reference) => {
                if let Ok(Some(character)) = reference.resolve_char_ref() {
                    text_buf.push(character);
                } else if let Some(resolved) =
                    quick_xml::escape::resolve_predefined_entity(&reference)
                {
                    text_buf.push_str(resolved);
                }
            }
            Event::End(e) => {
                let local = end_local_name(&e);
                if local == "item" {
                    if let Some(item) = current.take() {
                        releases.push(item.into_release());
                    }
                } else if let Some(item) = current.as_mut() {
                    apply_text_field(&local, text_buf.trim(), item);
                }
                text_buf.clear();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(releases)
}

fn parse_param_tokens<T>(raw: &str, map: impl Fn(&str) -> Option<T>) -> Vec<T> {
    raw.split(',').filter_map(|tok| map(tok.trim())).collect()
}

fn parse_tv_params(raw: &str) -> Vec<TvSearchParam> {
    parse_param_tokens(raw, |tok| match tok.to_lowercase().as_str() {
        "q" => Some(TvSearchParam::Q),
        "season" => Some(TvSearchParam::Season),
        "ep" => Some(TvSearchParam::Ep),
        "imdbid" => Some(TvSearchParam::ImdbId),
        "tvdbid" => Some(TvSearchParam::TvdbId),
        "rid" => Some(TvSearchParam::RId),
        "tmdbid" => Some(TvSearchParam::TmdbId),
        "tvmazeid" => Some(TvSearchParam::TvmazeId),
        "traktid" => Some(TvSearchParam::TraktId),
        "doubanid" => Some(TvSearchParam::DoubanId),
        "year" => Some(TvSearchParam::Year),
        "genre" => Some(TvSearchParam::Genre),
        _ => None,
    })
}

fn parse_movie_params(raw: &str) -> Vec<MovieSearchParam> {
    parse_param_tokens(raw, |tok| match tok.to_lowercase().as_str() {
        "q" => Some(MovieSearchParam::Q),
        "imdbid" => Some(MovieSearchParam::ImdbId),
        "tmdbid" => Some(MovieSearchParam::TmdbId),
        "traktid" => Some(MovieSearchParam::TraktId),
        "doubanid" => Some(MovieSearchParam::DoubanId),
        "year" => Some(MovieSearchParam::Year),
        "genre" => Some(MovieSearchParam::Genre),
        _ => None,
    })
}

fn parse_music_params(raw: &str) -> Vec<MusicSearchParam> {
    parse_param_tokens(raw, |tok| match tok.to_lowercase().as_str() {
        "q" => Some(MusicSearchParam::Q),
        "album" => Some(MusicSearchParam::Album),
        "artist" => Some(MusicSearchParam::Artist),
        "label" => Some(MusicSearchParam::Label),
        "track" => Some(MusicSearchParam::Track),
        "year" => Some(MusicSearchParam::Year),
        "genre" => Some(MusicSearchParam::Genre),
        _ => None,
    })
}

fn parse_book_params(raw: &str) -> Vec<BookSearchParam> {
    parse_param_tokens(raw, |tok| match tok.to_lowercase().as_str() {
        "q" => Some(BookSearchParam::Q),
        "title" => Some(BookSearchParam::Title),
        "author" => Some(BookSearchParam::Author),
        "publisher" => Some(BookSearchParam::Publisher),
        "year" => Some(BookSearchParam::Year),
        "genre" => Some(BookSearchParam::Genre),
        _ => None,
    })
}

/// Parse a Torznab/Newznab `<caps>` response into a [`SourceCapabilities`].
fn parse_caps_xml(xml: &str) -> Result<SourceCapabilities> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut caps = SourceCapabilities {
        search_available: true,
        ..Default::default()
    };
    let mut categories = Vec::new();

    loop {
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| anyhow!("Failed to parse Torznab caps XML: {}", e))?;
        match event {
            Event::Start(e) | Event::Empty(e) => {
                let local = start_local_name(&e);
                let available = || {
                    get_attr(&e, "available")
                        .map(|v| v.eq_ignore_ascii_case("yes"))
                        .unwrap_or(false)
                };
                match local.as_str() {
                    "search" => {
                        caps.search_available = caps.search_available || available();
                    }
                    "tv-search" if available() => {
                        caps.tv_search_params =
                            parse_tv_params(&get_attr(&e, "supportedParams").unwrap_or_default());
                    }
                    "movie-search" if available() => {
                        caps.movie_search_params = parse_movie_params(
                            &get_attr(&e, "supportedParams").unwrap_or_default(),
                        );
                    }
                    "music-search" if available() => {
                        caps.music_search_params = parse_music_params(
                            &get_attr(&e, "supportedParams").unwrap_or_default(),
                        );
                    }
                    "book-search" if available() => {
                        caps.book_search_params =
                            parse_book_params(&get_attr(&e, "supportedParams").unwrap_or_default());
                    }
                    "category" => {
                        if let Some(id) = get_attr(&e, "id").and_then(|v| v.parse::<i32>().ok()) {
                            let name = get_attr(&e, "name").unwrap_or_else(|| id.to_string());
                            categories.push(CategoryMapping::new(id.to_string(), id, name));
                        }
                    }
                    "subcat" => {
                        if let Some(id) = get_attr(&e, "id").and_then(|v| v.parse::<i32>().ok()) {
                            let name = get_attr(&e, "name").unwrap_or_else(|| id.to_string());
                            categories.push(CategoryMapping::new(id.to_string(), id, name));
                        }
                    }
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    caps.categories = categories;
    Ok(caps)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:torznab="http://torznab.com/schemas/2015/feed" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>Example Indexer</title>
    <item>
      <title>Some Show S01E02 1080p WEB-DL DDP5.1 H264-GROUP</title>
      <guid isPermaLink="true">https://example.com/details/abc123</guid>
      <comments>https://example.com/details/abc123</comments>
      <link>https://example.com/download/abc123.torrent</link>
      <pubDate>Tue, 02 Jan 2024 15:04:05 +0000</pubDate>
      <size>1610612736</size>
      <description>Some Show S01E02</description>
      <category>5030</category>
      <enclosure url="https://example.com/download/abc123.torrent" length="1610612736" type="application/x-bittorrent" />
      <torznab:attr name="category" value="5030" />
      <torznab:attr name="seeders" value="42" />
      <torznab:attr name="peers" value="55" />
      <torznab:attr name="infohash" value="AABBCCDDEEFF00112233445566778899AABBCCDD" />
      <torznab:attr name="size" value="1610612736" />
      <torznab:attr name="imdbid" value="tt1234567" />
      <torznab:attr name="downloadvolumefactor" value="0" />
      <torznab:attr name="uploadvolumefactor" value="1" />
      <torznab:attr name="minimumratio" value="1.0" />
      <torznab:attr name="minimumseedtime" value="172800" />
    </item>
    <item>
      <title>Another Release 2023</title>
      <guid>https://example.com/details/def456</guid>
      <pubDate>Tue, 03 Jan 2023 00:00:00 +0000</pubDate>
      <enclosure url="magnet:?xt=urn:btih:DEADBEEF&amp;dn=Another+Release" length="500000000" type="application/x-bittorrent" />
      <torznab:attr name="category" value="2040" />
      <torznab:attr name="seeders" value="5" />
      <torznab:attr name="leechers" value="2" />
      <torznab:attr name="tmdbid" value="98765" />
      <torznab:attr name="resolution" value="2160p" />
    </item>
  </channel>
</rss>"#;

    /// A magnet in `<link>` — the shape most indexers actually emit — with the
    /// `&` separators XML-escaped, as they must be.
    const MAGNET_IN_LINK_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:torznab="http://torznab.com/schemas/2015/feed">
  <channel>
    <item>
      <title>Show &amp; Tell S01E01 1080p WEB-DL</title>
      <guid isPermaLink="false">https://example.com/d?id=1&amp;x=2</guid>
      <link>magnet:?xt=urn:btih:AABBCCDDEEFF00112233445566778899AABBCCDD&amp;dn=Show+Tell&amp;tr=http%3A%2F%2Ftracker.example%2Fannounce</link>
      <torznab:attr name="seeders" value="9" />
    </item>
  </channel>
</rss>"#;

    #[test]
    fn entity_references_in_element_text_survive_parsing() {
        // Regression guard: quick-xml emits `&amp;` inside element text as a
        // separate `Event::GeneralRef`. Dropping it deleted every `&` from the
        // magnet, so `dn` and `tr` were absorbed into the info hash and the
        // torrent had no trackers at all.
        let releases = parse_torznab_rss(MAGNET_IN_LINK_RSS).expect("parse rss");
        let release = releases.first().expect("one release");

        assert_eq!(
            release.magnet_uri.as_deref(),
            Some(
                "magnet:?xt=urn:btih:AABBCCDDEEFF00112233445566778899AABBCCDD\
                 &dn=Show+Tell&tr=http%3A%2F%2Ftracker.example%2Fannounce"
                    .replace(char::is_whitespace, "")
                    .as_str()
            )
        );
        assert_eq!(release.title, "Show & Tell S01E01 1080p WEB-DL");
        assert_eq!(release.guid, "https://example.com/d?id=1&x=2");
    }

    const SAMPLE_CAPS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<caps>
  <server version="1.1" title="Example Indexer" />
  <limits max="100" default="50" />
  <searching>
    <search available="yes" supportedParams="q" />
    <tv-search available="yes" supportedParams="q,season,ep,imdbid,tvdbid" />
    <movie-search available="yes" supportedParams="q,imdbid,tmdbid,year" />
    <music-search available="no" supportedParams="q" />
    <book-search available="yes" supportedParams="q,title,author" />
  </searching>
  <categories>
    <category id="2000" name="Movies">
      <subcat id="2040" name="Movies/HD" />
    </category>
    <category id="5000" name="TV">
      <subcat id="5030" name="TV/SD" />
      <subcat id="5040" name="TV/HD" />
    </category>
  </categories>
</caps>"#;

    const SAMPLE_ERROR: &str = r#"<?xml version="1.0" encoding="UTF-8"?><error code="100" description="Invalid API Key" />"#;

    #[test]
    fn parses_rss_items_with_torznab_attrs() {
        let releases = parse_torznab_rss(SAMPLE_RSS).expect("parse should succeed");
        assert_eq!(releases.len(), 2);

        let first = &releases[0];
        assert_eq!(
            first.title,
            "Some Show S01E02 1080p WEB-DL DDP5.1 H264-GROUP"
        );
        assert_eq!(first.guid, "https://example.com/details/abc123");
        assert_eq!(
            first.link.as_deref(),
            Some("https://example.com/download/abc123.torrent")
        );
        assert_eq!(
            first.details.as_deref(),
            Some("https://example.com/details/abc123")
        );
        assert_eq!(first.size, Some(1_610_612_736));
        assert_eq!(first.seeders, Some(42));
        assert_eq!(first.peers, Some(55));
        assert_eq!(first.leechers(), Some(13));
        assert_eq!(
            first.info_hash.as_deref(),
            Some("AABBCCDDEEFF00112233445566778899AABBCCDD")
        );
        assert_eq!(first.imdb, Some(1_234_567));
        assert_eq!(first.categories, vec![5030, 5030]);
        assert!(first.is_freeleech());
        assert_eq!(first.upload_volume_factor, 1.0);
        assert_eq!(first.minimum_ratio, Some(1.0));
        assert_eq!(first.minimum_seed_time, Some(172_800));
        assert_eq!(first.publish_date.timestamp(), 1_704_207_845);

        let second = &releases[1];
        assert_eq!(
            second.magnet_uri.as_deref(),
            Some("magnet:?xt=urn:btih:DEADBEEF&dn=Another+Release")
        );
        assert_eq!(second.link, None);
        assert_eq!(second.seeders, Some(5));
        assert_eq!(second.peers, Some(7)); // seeders(5) + leechers(2), no explicit "peers" attr
        assert_eq!(second.tmdb, Some(98765));
        assert!(second.title.to_lowercase().contains("2160p"));
    }

    #[test]
    fn parses_caps_into_capabilities() {
        let caps = parse_caps_xml(SAMPLE_CAPS).expect("parse should succeed");
        assert!(caps.search_available);
        assert!(caps.tv_search_available());
        assert!(caps.movie_search_available());
        assert!(!caps.music_search_available());
        assert!(caps.book_search_available());

        assert!(caps.tv_search_params.contains(&TvSearchParam::Season));
        assert!(caps.tv_search_params.contains(&TvSearchParam::TvdbId));
        assert!(caps.movie_search_params.contains(&MovieSearchParam::TmdbId));
        assert!(caps.book_search_params.contains(&BookSearchParam::Author));

        // 4 subcats/cats total: 2000, 2040, 5000, 5030, 5040 = 5 entries.
        assert_eq!(caps.categories.len(), 5);

        // Parent-bucket fallback (types.rs's existing logic, reused not
        // reimplemented): querying the round-thousands TV bucket (5000)
        // should still match subcat 5030/5040 via the same-thousands rule.
        let tracker_ids = caps.map_torznab_to_tracker(&[5000]);
        assert!(tracker_ids.contains(&"5030".to_string()));
        assert!(tracker_ids.contains(&"5040".to_string()));
    }

    #[test]
    fn detects_newznab_error_response() {
        let err = extract_newznab_error(SAMPLE_ERROR).expect("should detect error");
        assert_eq!(err.0, "100");
        assert_eq!(err.1, "Invalid API Key");

        assert!(extract_newznab_error(SAMPLE_RSS).is_none());
    }

    #[test]
    fn construction_requires_site_url() {
        let result = TorznabSource::new(
            "src-1".to_string(),
            "Test".to_string(),
            None,
            "somekey",
            HashMap::new(),
        );
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("missing site_url should error"),
        };
        assert!(err.to_string().contains("site_url"));
    }

    #[test]
    fn resolve_api_base_appends_api_suffix() {
        assert_eq!(
            TorznabSource::resolve_api_base("https://example.com/"),
            "https://example.com/api"
        );
        assert_eq!(
            TorznabSource::resolve_api_base("https://example.com/api/"),
            "https://example.com/api"
        );
    }

    fn build_source(site_url: &str, settings: HashMap<String, String>) -> TorznabSource {
        TorznabSource::new(
            "src-1".to_string(),
            "Test Indexer".to_string(),
            Some(site_url.to_string()),
            "test-api-key",
            settings,
        )
        .expect("construction should succeed")
    }

    #[test]
    fn build_query_params_includes_core_fields() {
        let source = build_source("https://example.com", HashMap::new());
        let mut query = SourceQuery::tv_search("Some Show");
        query.season = Some(1);
        query.episode = Some("02".to_string());
        query.imdb_id = Some("tt1234567".to_string());
        query.categories = vec![5030];

        let params = source.build_query_params(&query);
        let get = |key: &str| {
            params
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
        };

        assert_eq!(get("t").as_deref(), Some("tvsearch"));
        assert_eq!(get("apikey").as_deref(), Some("test-api-key"));
        assert_eq!(get("q").as_deref(), Some("Some Show"));
        assert_eq!(get("season").as_deref(), Some("1"));
        assert_eq!(get("ep").as_deref(), Some("02"));
        assert_eq!(get("imdbid").as_deref(), Some("1234567"));
        // No `t=caps` has been fetched yet, so only the generic round-thousands
        // bucket capabilities (built at construction) are available; 5030
        // maps to its parent bucket 5000 via the reused parent-bucket fallback
        // in `SourceCapabilities::map_torznab_to_tracker`.
        assert_eq!(get("cat").as_deref(), Some("5000"));
    }

    #[test]
    fn default_categories_setting_used_when_query_has_none() {
        let mut settings = HashMap::new();
        settings.insert("DefaultCategories".to_string(), "5000,2000".to_string());
        let source = build_source("https://example.com", settings);

        let query = SourceQuery::search("term");
        let params = source.build_query_params(&query);
        let cat = params
            .iter()
            .find(|(k, _)| k == "cat")
            .map(|(_, v)| v.clone());
        assert_eq!(cat.as_deref(), Some("5000,2000"));
    }

    #[test]
    fn is_configured_reflects_api_key() {
        let configured = build_source("https://example.com", HashMap::new());
        assert!(configured.is_configured());

        let unconfigured = TorznabSource::new(
            "src-2".to_string(),
            "Test".to_string(),
            Some("https://example.com".to_string()),
            "",
            HashMap::new(),
        )
        .unwrap();
        assert!(!unconfigured.is_configured());
    }
}
