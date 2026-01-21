//! Newznab indexer implementation
//!
//! Newznab is the standard API for Usenet indexers (NZBGeek, DrunkenSlug, etc.).
//! This implementation provides search and NZB download capabilities.
//!
//! # Authentication
//!
//! Newznab uses API key authentication. Users need to:
//! 1. Register at the indexer site
//! 2. Get their API key from the site's settings
//! 3. Configure the API URL and key in Librarian
//!
//! # Configuration
//!
//! Required credentials:
//! - `api_key`: API key from the indexer site
//!
//! Optional settings:
//! - `vip_expiry_check`: Check VIP status expiry (if supported)

use std::collections::HashMap;

use anyhow::{Result, anyhow};
use async_graphql::async_trait::async_trait;
use chrono::{DateTime, Utc};
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::Client;
use tracing::{debug, error, info, warn};

use crate::indexer::categories::CategoryMapping;
use crate::indexer::{
    BookSearchParam, Indexer, IndexerType, MovieSearchParam, MusicSearchParam, ReleaseInfo,
    TorznabCapabilities, TorznabQuery, TrackerType, TvSearchParam, categories::cats,
};

/// Newznab indexer for Usenet sites
pub struct NewznabIndexer {
    /// Unique instance ID (database config ID)
    id: String,
    /// Display name
    name: String,
    /// API base URL (e.g., "https://api.nzbgeek.info")
    api_url: String,
    /// API key
    api_key: String,
    /// HTTP client
    client: Client,
    /// Settings
    settings: HashMap<String, String>,
    /// Cached capabilities (fetched from /api?t=caps)
    capabilities: TorznabCapabilities,
}

impl NewznabIndexer {
    /// Create a new Newznab indexer instance
    pub fn new(
        id: String,
        name: String,
        api_url: Option<String>,
        api_key: &str,
        settings: HashMap<String, String>,
    ) -> Result<Self> {
        let api_url = api_url.ok_or_else(|| anyhow!("API URL is required for Newznab indexer"))?;

        if api_key.is_empty() {
            return Err(anyhow!("API key is required for Newznab indexer"));
        }

        let client = Client::builder()
            .gzip(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        // Build default capabilities (will be refined by /api?t=caps if available)
        let capabilities = Self::default_capabilities();

        info!(
            indexer_id = %id,
            indexer_name = %name,
            api_url = %api_url,
            "Created Newznab indexer"
        );

        Ok(Self {
            id,
            name,
            api_url,
            api_key: api_key.to_string(),
            client,
            settings,
            capabilities,
        })
    }

    /// Build default capabilities for Newznab
    fn default_capabilities() -> TorznabCapabilities {
        TorznabCapabilities {
            search_available: true,
            limits_default: Some(100),
            limits_max: Some(100),
            tv_search_params: vec![
                TvSearchParam::Q,
                TvSearchParam::Season,
                TvSearchParam::Ep,
                TvSearchParam::TvdbId,
                TvSearchParam::RId, // TVRage ID (legacy)
            ],
            tv_search_imdb_available: true,
            movie_search_params: vec![MovieSearchParam::Q, MovieSearchParam::ImdbId],
            music_search_params: vec![MusicSearchParam::Q],
            book_search_params: vec![BookSearchParam::Q],
            categories: Self::default_categories(),
            ..Default::default()
        }
    }

    /// Default category mappings for Newznab
    fn default_categories() -> Vec<CategoryMapping> {
        vec![
            // Movies
            CategoryMapping::new("2000", cats::MOVIES, "Movies"),
            CategoryMapping::new("2010", cats::MOVIES_FOREIGN, "Movies/Foreign"),
            CategoryMapping::new("2020", cats::MOVIES_OTHER, "Movies/Other"),
            CategoryMapping::new("2030", cats::MOVIES_SD, "Movies/SD"),
            CategoryMapping::new("2040", cats::MOVIES_HD, "Movies/HD"),
            CategoryMapping::new("2045", cats::MOVIES_UHD, "Movies/UHD"),
            CategoryMapping::new("2050", cats::MOVIES_BLURAY, "Movies/BluRay"),
            CategoryMapping::new("2060", cats::MOVIES_3D, "Movies/3D"),
            // TV
            CategoryMapping::new("5000", cats::TV, "TV"),
            CategoryMapping::new("5020", cats::TV_FOREIGN, "TV/Foreign"),
            CategoryMapping::new("5030", cats::TV_SD, "TV/SD"),
            CategoryMapping::new("5040", cats::TV_HD, "TV/HD"),
            CategoryMapping::new("5045", cats::TV_UHD, "TV/UHD"),
            CategoryMapping::new("5060", cats::TV_SPORT, "TV/Sport"),
            CategoryMapping::new("5070", cats::TV_ANIME, "TV/Anime"),
            CategoryMapping::new("5080", cats::TV_DOCUMENTARY, "TV/Documentary"),
            // Audio
            CategoryMapping::new("3000", cats::AUDIO, "Audio"),
            CategoryMapping::new("3010", cats::AUDIO_MP3, "Audio/MP3"),
            CategoryMapping::new("3020", cats::AUDIO_VIDEO, "Audio/Video"),
            CategoryMapping::new("3030", cats::AUDIO_AUDIOBOOK, "Audio/Audiobook"),
            CategoryMapping::new("3040", cats::AUDIO_LOSSLESS, "Audio/Lossless"),
            // Books
            CategoryMapping::new("7000", cats::BOOKS, "Books"),
            CategoryMapping::new("7020", cats::BOOKS_EBOOK, "Books/EBook"),
            CategoryMapping::new("7030", cats::BOOKS_COMICS, "Books/Comics"),
        ]
    }

    /// Build the API URL with query parameters
    fn build_api_url(&self, params: &[(&str, &str)]) -> String {
        let base = self.api_url.trim_end_matches('/');
        let mut url = format!("{}/api?apikey={}", base, self.api_key);

        for (key, value) in params {
            url.push_str(&format!("&{}={}", key, urlencoding::encode(value)));
        }

        url
    }

    /// Parse Newznab XML response into ReleaseInfo list
    fn parse_response(&self, xml: &str) -> Result<Vec<ReleaseInfo>> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut releases = Vec::new();
        let mut current_item: Option<ReleaseInfoBuilder> = None;
        let mut current_tag = String::new();
        let mut in_item = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    current_tag = tag_name.clone();

                    if tag_name == "item" {
                        in_item = true;
                        current_item = Some(ReleaseInfoBuilder::new());
                    } else if in_item && tag_name == "newznab:attr" || tag_name == "torznab:attr" {
                        // Parse attributes like <newznab:attr name="size" value="123456"/>
                        if let Some(ref mut item) = current_item {
                            let mut attr_name = String::new();
                            let mut attr_value = String::new();

                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                let val = String::from_utf8_lossy(&attr.value).to_string();

                                if key == "name" {
                                    attr_name = val;
                                } else if key == "value" {
                                    attr_value = val;
                                }
                            }

                            item.set_newznab_attr(&attr_name, &attr_value);
                        }
                    } else if in_item && tag_name == "enclosure" {
                        // Parse <enclosure url="..." length="..." type="..."/>
                        if let Some(ref mut item) = current_item {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                let val = String::from_utf8_lossy(&attr.value).to_string();

                                match key.as_str() {
                                    "url" => item.link = Some(val),
                                    "length" => {
                                        if let Ok(size) = val.parse::<i64>() {
                                            item.size = Some(size);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    // Handle self-closing tags like <newznab:attr ... />
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if in_item && (tag_name == "newznab:attr" || tag_name == "torznab:attr") {
                        if let Some(ref mut item) = current_item {
                            let mut attr_name = String::new();
                            let mut attr_value = String::new();

                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                let val = String::from_utf8_lossy(&attr.value).to_string();

                                if key == "name" {
                                    attr_name = val;
                                } else if key == "value" {
                                    attr_value = val;
                                }
                            }

                            item.set_newznab_attr(&attr_name, &attr_value);
                        }
                    } else if in_item && tag_name == "enclosure" {
                        if let Some(ref mut item) = current_item {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                let val = String::from_utf8_lossy(&attr.value).to_string();

                                match key.as_str() {
                                    "url" => item.link = Some(val),
                                    "length" => {
                                        if let Ok(size) = val.parse::<i64>() {
                                            item.size = Some(size);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Ok(Event::Text(ref e)) => {
                    if in_item {
                        if let Some(ref mut item) = current_item {
                            let text = e.unescape().unwrap_or_default().to_string();
                            if !text.is_empty() {
                                match current_tag.as_str() {
                                    "title" => item.title = Some(text),
                                    "guid" => {
                                        if item.guid.is_none() {
                                            item.guid = Some(text);
                                        }
                                    }
                                    "link" => {
                                        if item.link.is_none() {
                                            item.link = Some(text);
                                        }
                                    }
                                    "pubDate" => item.pub_date = parse_rfc822_date(&text),
                                    "description" => item.description = Some(text),
                                    "category" => item.add_category(&text),
                                    "comments" => item.details = Some(text),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "item" {
                        if let Some(item) = current_item.take() {
                            if let Some(release) = item.build(&self.id, &self.name) {
                                releases.push(release);
                            }
                        }
                        in_item = false;
                    }
                    current_tag.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    error!(error = %e, "Error parsing Newznab XML");
                    return Err(anyhow!("XML parse error: {}", e));
                }
                _ => {}
            }
        }

        Ok(releases)
    }
}

/// Helper to build ReleaseInfo from parsed XML
struct ReleaseInfoBuilder {
    title: Option<String>,
    guid: Option<String>,
    link: Option<String>,
    pub_date: Option<DateTime<Utc>>,
    description: Option<String>,
    details: Option<String>,
    size: Option<i64>,
    categories: Vec<String>,
    // Newznab attributes
    seeders: Option<i32>,
    peers: Option<i32>,
    grabs: Option<i32>,
    imdb: Option<i64>,
    tvdb_id: Option<i64>,
    rage_id: Option<i64>,
    download_volume_factor: f64,
    upload_volume_factor: f64,
    poster: Option<String>,
}

impl ReleaseInfoBuilder {
    fn new() -> Self {
        Self {
            title: None,
            guid: None,
            link: None,
            pub_date: None,
            description: None,
            details: None,
            size: None,
            categories: Vec::new(),
            seeders: None,
            peers: None,
            grabs: None,
            imdb: None,
            tvdb_id: None,
            rage_id: None,
            download_volume_factor: 1.0,
            upload_volume_factor: 1.0,
            poster: None,
        }
    }

    fn set_newznab_attr(&mut self, name: &str, value: &str) {
        match name {
            "size" => {
                if let Ok(size) = value.parse::<i64>() {
                    self.size = Some(size);
                }
            }
            "seeders" => {
                if let Ok(s) = value.parse::<i32>() {
                    self.seeders = Some(s);
                }
            }
            "peers" => {
                if let Ok(p) = value.parse::<i32>() {
                    self.peers = Some(p);
                }
            }
            "grabs" => {
                if let Ok(g) = value.parse::<i32>() {
                    self.grabs = Some(g);
                }
            }
            "imdb" | "imdbid" => {
                // Parse IMDB ID (may be with or without tt prefix)
                let clean = value.trim_start_matches("tt");
                if let Ok(id) = clean.parse::<i64>() {
                    self.imdb = Some(id);
                }
            }
            "tvdbid" | "tvdb" => {
                if let Ok(id) = value.parse::<i64>() {
                    self.tvdb_id = Some(id);
                }
            }
            "rageid" | "tvrageid" => {
                if let Ok(id) = value.parse::<i64>() {
                    self.rage_id = Some(id);
                }
            }
            "downloadvolumefactor" => {
                if let Ok(f) = value.parse::<f64>() {
                    self.download_volume_factor = f;
                }
            }
            "uploadvolumefactor" => {
                if let Ok(f) = value.parse::<f64>() {
                    self.upload_volume_factor = f;
                }
            }
            "poster" | "coverurl" => {
                self.poster = Some(value.to_string());
            }
            _ => {
                debug!(attr_name = name, attr_value = value, "Unknown newznab attribute");
            }
        }
    }

    fn add_category(&mut self, category: &str) {
        self.categories.push(category.to_string());
    }

    fn build(self, indexer_id: &str, indexer_name: &str) -> Option<ReleaseInfo> {
        let title = self.title?;
        let guid = self.guid.unwrap_or_else(|| title.clone());
        let pub_date = self.pub_date.unwrap_or_else(Utc::now);

        // Parse category IDs
        let category_ids: Vec<i32> = self
            .categories
            .iter()
            .filter_map(|c| c.parse::<i32>().ok())
            .collect();

        Some(ReleaseInfo {
            title,
            guid,
            link: self.link,
            magnet_uri: None, // Usenet doesn't use magnets
            info_hash: None,
            details: self.details,
            publish_date: pub_date,
            categories: category_ids,
            size: self.size,
            seeders: self.seeders,
            peers: self.peers,
            grabs: self.grabs,
            imdb: self.imdb,
            tvdb_id: self.tvdb_id,
            rage_id: self.rage_id,
            download_volume_factor: self.download_volume_factor,
            upload_volume_factor: self.upload_volume_factor,
            poster: self.poster,
            description: self.description,
            indexer_id: Some(indexer_id.to_string()),
            indexer_name: Some(indexer_name.to_string()),
            ..Default::default()
        })
    }
}

/// Parse RFC 822 date format (common in RSS/Atom feeds)
fn parse_rfc822_date(s: &str) -> Option<DateTime<Utc>> {
    // Try multiple formats
    let formats = [
        "%a, %d %b %Y %H:%M:%S %z",     // RFC 822
        "%a, %d %b %Y %H:%M:%S GMT",    // GMT variant
        "%a, %d %b %Y %H:%M:%S %Z",     // Named timezone
        "%Y-%m-%dT%H:%M:%S%z",          // ISO 8601
        "%Y-%m-%dT%H:%M:%SZ",           // ISO 8601 UTC
    ];

    for format in &formats {
        if let Ok(dt) = DateTime::parse_from_str(s, format) {
            return Some(dt.with_timezone(&Utc));
        }
    }

    // Try chrono's flexible parser
    if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(s) {
        return Some(dt.with_timezone(&Utc));
    }

    warn!(date_string = s, "Failed to parse date");
    None
}

#[async_trait]
impl Indexer for NewznabIndexer {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Newznab-compatible Usenet indexer"
    }

    fn indexer_type(&self) -> IndexerType {
        IndexerType::Newznab
    }

    fn site_link(&self) -> &str {
        &self.api_url
    }

    fn tracker_type(&self) -> TrackerType {
        TrackerType::Private // Most Newznab sites require registration
    }

    fn language(&self) -> &str {
        "en-US"
    }

    fn capabilities(&self) -> &TorznabCapabilities {
        &self.capabilities
    }

    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    fn supports_pagination(&self) -> bool {
        true
    }

    async fn test_connection(&self) -> Result<bool> {
        let url = self.build_api_url(&[("t", "caps")]);

        debug!(url = %url, "Testing Newznab connection");

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Connection test failed: HTTP {}",
                response.status()
            ));
        }

        let body = response.text().await?;

        // Check for error response
        if body.contains("<error") {
            if body.contains("Incorrect user credentials") || body.contains("Invalid API key") {
                return Err(anyhow!("Invalid API key"));
            }
            return Err(anyhow!("API error in response"));
        }

        // Check for valid caps response
        if body.contains("<caps>") || body.contains("<categories>") {
            info!(indexer_name = %self.name, "Newznab connection test successful");
            return Ok(true);
        }

        Err(anyhow!("Unexpected response format"))
    }

    async fn search(&self, query: &TorznabQuery) -> Result<Vec<ReleaseInfo>> {
        let mut params: Vec<(&str, String)> = Vec::new();

        // Determine search type
        let search_type = match query.query_type {
            crate::indexer::QueryType::TvSearch => "tvsearch",
            crate::indexer::QueryType::MovieSearch => "movie",
            crate::indexer::QueryType::MusicSearch => "music",
            crate::indexer::QueryType::BookSearch => "book",
            _ => "search",
        };
        params.push(("t", search_type.to_string()));

        // Add search term
        if let Some(ref term) = query.search_term {
            params.push(("q", term.clone()));
        }

        // Add categories
        if !query.categories.is_empty() {
            let cats: String = query
                .categories
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(",");
            params.push(("cat", cats));
        }

        // Add TV-specific params
        if let Some(season) = query.season {
            params.push(("season", season.to_string()));
        }
        if let Some(ref ep) = query.episode {
            params.push(("ep", ep.clone()));
        }
        if let Some(ref imdb_id) = query.imdb_id {
            let clean = imdb_id.trim_start_matches("tt");
            params.push(("imdbid", clean.to_string()));
        }
        if let Some(tvdb_id) = query.tvdb_id {
            params.push(("tvdbid", tvdb_id.to_string()));
        }

        // Add pagination
        if let Some(limit) = query.limit {
            params.push(("limit", limit.to_string()));
        }
        if let Some(offset) = query.offset {
            params.push(("offset", offset.to_string()));
        }

        // Build URL
        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let url = self.build_api_url(&params_ref);

        debug!(
            indexer_name = %self.name,
            url = %url,
            "Searching Newznab indexer"
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Search failed: HTTP {}", response.status()));
        }

        let body = response.text().await?;

        // Check for API error
        if body.contains("<error") {
            // Extract error message
            if let Some(start) = body.find("description=\"") {
                let rest = &body[start + 13..];
                if let Some(end) = rest.find('"') {
                    let error_msg = &rest[..end];
                    return Err(anyhow!("API error: {}", error_msg));
                }
            }
            return Err(anyhow!("Unknown API error"));
        }

        let releases = self.parse_response(&body)?;

        info!(
            indexer_name = %self.name,
            releases_found = releases.len(),
            "Newznab search complete"
        );

        Ok(releases)
    }

    async fn download(&self, link: &str) -> Result<Vec<u8>> {
        debug!(
            indexer_name = %self.name,
            link = %link,
            "Downloading NZB from Newznab"
        );

        // The link should already include the API key if from our search results
        // But if not, we need to add it
        let download_url = if link.contains("apikey=") {
            link.to_string()
        } else if link.contains('?') {
            format!("{}&apikey={}", link, self.api_key)
        } else {
            format!("{}?apikey={}", link, self.api_key)
        };

        let response = self.client.get(&download_url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Download failed: HTTP {}", response.status()));
        }

        let bytes = response.bytes().await?;

        // Verify it looks like an NZB (XML)
        if bytes.len() > 10 {
            let start = String::from_utf8_lossy(&bytes[..std::cmp::min(100, bytes.len())]);
            if !start.contains("<?xml") && !start.contains("<nzb") {
                warn!(
                    indexer_name = %self.name,
                    content_preview = %start,
                    "Downloaded content doesn't look like NZB"
                );
            }
        }

        info!(
            indexer_name = %self.name,
            size = bytes.len(),
            "Downloaded NZB file"
        );

        Ok(bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rfc822_date() {
        let date = parse_rfc822_date("Sat, 18 Jan 2025 14:30:00 +0000");
        assert!(date.is_some());

        let date = parse_rfc822_date("2025-01-18T14:30:00Z");
        assert!(date.is_some());
    }

    #[test]
    fn test_build_api_url() {
        let indexer = NewznabIndexer::new(
            "test".to_string(),
            "Test".to_string(),
            Some("https://api.example.com".to_string()),
            "myapikey",
            HashMap::new(),
        )
        .unwrap();

        let url = indexer.build_api_url(&[("t", "search"), ("q", "test query")]);
        assert!(url.contains("apikey=myapikey"));
        assert!(url.contains("t=search"));
        assert!(url.contains("q=test%20query"));
    }
}
