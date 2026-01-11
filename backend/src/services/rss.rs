//! RSS Feed fetching and parsing service
//!
//! This service handles:
//! - Fetching RSS feeds from URLs
//! - Parsing RSS XML into structured items
//! - Using the filename parser to extract show/episode info

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use super::filename_parser::{parse_episode, parse_quality};

/// Parsed RSS item with extracted metadata
#[derive(Debug, Clone)]
pub struct ParsedRssItem {
    pub guid: Option<String>,
    pub title: String,
    pub link: String,
    pub pub_date: Option<DateTime<Utc>>,
    pub description: Option<String>,
    // Parsed metadata from title
    pub parsed_show_name: Option<String>,
    pub parsed_season: Option<i32>,
    pub parsed_episode: Option<i32>,
    pub parsed_resolution: Option<String>,
    pub parsed_codec: Option<String>,
    pub parsed_source: Option<String>,
    // Hashes for deduplication
    pub link_hash: String,
    pub title_hash: String,
}

/// RSS feed service for fetching and parsing feeds
pub struct RssService {
    client: Client,
}

impl RssService {
    /// Create a new RSS service
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Librarian/1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Fetch and parse an RSS feed from a URL
    pub async fn fetch_feed(&self, url: &str) -> Result<Vec<ParsedRssItem>> {
        info!("Fetching RSS feed: {}", url);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch RSS feed")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "RSS feed returned error status: {}",
                response.status()
            );
        }

        let content = response
            .text()
            .await
            .context("Failed to read RSS feed content")?;

        self.parse_feed(&content)
    }

    /// Parse RSS XML content into items
    pub fn parse_feed(&self, content: &str) -> Result<Vec<ParsedRssItem>> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut reader = Reader::from_str(content);
        reader.config_mut().trim_text(true);

        let mut items = Vec::new();
        let mut current_item: Option<RssItemBuilder> = None;
        let mut current_tag = String::new();
        let mut in_item = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    current_tag = tag_name.clone();

                    if tag_name == "item" {
                        in_item = true;
                        current_item = Some(RssItemBuilder::default());
                    }
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "item" {
                        in_item = false;
                        if let Some(builder) = current_item.take()
                            && let Some(item) = self.build_item(builder) {
                                items.push(item);
                            }
                    }
                    current_tag.clear();
                }
                Ok(Event::Text(ref e)) => {
                    if in_item
                        && let Some(ref mut builder) = current_item {
                            let text = e.unescape().unwrap_or_default().to_string();
                            match current_tag.as_str() {
                                "title" => builder.title = Some(text),
                                "link" => builder.link = Some(text),
                                "guid" => builder.guid = Some(text),
                                "pubDate" => builder.pub_date = Some(text),
                                "description" => builder.description = Some(text),
                                _ => {}
                            }
                        }
                }
                Ok(Event::CData(ref e)) => {
                    if in_item
                        && let Some(ref mut builder) = current_item {
                            let text = String::from_utf8_lossy(e.as_ref()).to_string();
                            match current_tag.as_str() {
                                "title" => builder.title = Some(text),
                                "description" => builder.description = Some(text),
                                _ => {}
                            }
                        }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    warn!("Error parsing RSS XML: {:?}", e);
                    break;
                }
                _ => {}
            }
        }

        info!("Parsed {} items from RSS feed", items.len());
        Ok(items)
    }

    /// Build a parsed RSS item from the builder
    fn build_item(&self, builder: RssItemBuilder) -> Option<ParsedRssItem> {
        let title = builder.title?;
        let link = builder.link?;

        // Generate hashes for deduplication
        let link_hash = Self::hash_string(&link);
        let title_hash = Self::hash_string(&title);

        // Parse the title to extract show/episode info
        let parsed_ep = parse_episode(&title);
        let parsed_quality = parse_quality(&title);

        // Parse pub_date
        let pub_date = builder.pub_date.and_then(|s| Self::parse_rss_date(&s));

        Some(ParsedRssItem {
            guid: builder.guid,
            title,
            link,
            pub_date,
            description: builder.description,
            parsed_show_name: parsed_ep.show_name,
            parsed_season: parsed_ep.season.map(|s| s as i32),
            parsed_episode: parsed_ep.episode.map(|e| e as i32),
            parsed_resolution: parsed_quality.resolution,
            parsed_codec: parsed_quality.codec,
            parsed_source: parsed_quality.source,
            link_hash,
            title_hash,
        })
    }

    /// Generate SHA256 hash of a string
    fn hash_string(s: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(s.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Parse RSS date format (RFC 2822)
    fn parse_rss_date(s: &str) -> Option<DateTime<Utc>> {
        // Try RFC 2822 format first
        if let Ok(dt) = DateTime::parse_from_rfc2822(s) {
            return Some(dt.with_timezone(&Utc));
        }

        // Try some common variations
        let formats = [
            "%a, %d %b %Y %H:%M:%S %z",
            "%a, %d %b %Y %H:%M:%S GMT",
            "%Y-%m-%dT%H:%M:%S%z",
            "%Y-%m-%d %H:%M:%S",
        ];

        for fmt in formats {
            if let Ok(dt) = chrono::DateTime::parse_from_str(s, fmt) {
                return Some(dt.with_timezone(&Utc));
            }
            // Try with NaiveDateTime for formats without timezone
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, fmt) {
                return Some(dt.and_utc());
            }
        }

        debug!("Failed to parse RSS date: {}", s);
        None
    }
}

impl Default for RssService {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for RSS items during parsing
#[derive(Default)]
struct RssItemBuilder {
    guid: Option<String>,
    title: Option<String>,
    link: Option<String>,
    pub_date: Option<String>,
    description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_feed() {
        let rss = RssService::new();
        let content = r#"
        <rss version="2.0">
        <channel>
            <title>Test Feed</title>
            <item>
                <title>Chicago Fire S14E08 1080p WEB h264-ETHEL</title>
                <link>https://example.com/download.php/12345/file.torrent</link>
                <pubDate>Thu, 08 Jan 2026 10:01:59 +0000</pubDate>
                <description>1.48 GB; TV/Web-DL</description>
            </item>
            <item>
                <title>Corner Gas S06E12 Super Sensitive 1080p AMZN WEB-DL DDP2 0 H 264-QOQ</title>
                <link>https://example.com/download.php/67890/file.torrent</link>
                <pubDate>Thu, 08 Jan 2026 10:14:25 +0000</pubDate>
            </item>
        </channel>
        </rss>
        "#;

        let items = rss.parse_feed(content).unwrap();
        assert_eq!(items.len(), 2);

        // Check first item
        assert_eq!(items[0].parsed_show_name.as_deref(), Some("Chicago Fire"));
        assert_eq!(items[0].parsed_season, Some(14));
        assert_eq!(items[0].parsed_episode, Some(8));
        assert_eq!(items[0].parsed_resolution.as_deref(), Some("1080p"));

        // Check second item
        assert_eq!(items[1].parsed_show_name.as_deref(), Some("Corner Gas"));
        assert_eq!(items[1].parsed_season, Some(6));
        assert_eq!(items[1].parsed_episode, Some(12));
    }
}
