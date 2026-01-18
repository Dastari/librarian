//! RSS Feed fetching and parsing service
//!
//! This service handles:
//! - Fetching RSS feeds from URLs
//! - Parsing RSS XML into structured items
//! - Using the filename parser to extract show/episode info
//! - SSRF protection for URL validation

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::net::{IpAddr, ToSocketAddrs};
use tracing::{debug, info, warn};
use url::Url;

use super::filename_parser::{parse_episode, parse_quality};

/// Validates a URL for SSRF protection
///
/// Blocks requests to:
/// - Private/internal IP ranges (10.x.x.x, 172.16-31.x.x, 192.168.x.x)
/// - Loopback addresses (127.x.x.x, ::1)
/// - Link-local addresses (169.254.x.x, fe80::/10)
/// - Multicast addresses
/// - Non-HTTP(S) schemes
///
/// Returns an error if the URL is not allowed.
pub fn validate_url_for_ssrf(url_str: &str) -> Result<()> {
    let url = Url::parse(url_str).context("Invalid URL format")?;

    // Only allow HTTP and HTTPS schemes
    match url.scheme() {
        "http" | "https" => {}
        scheme => anyhow::bail!("URL scheme '{}' is not allowed. Only HTTP(S) is permitted.", scheme),
    }

    // Get the host
    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("URL must have a host"))?;

    // Check if it's a raw IP address
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_internal_ip(&ip) {
            anyhow::bail!("Requests to internal/private IP addresses are not allowed");
        }
        return Ok(());
    }

    // Resolve hostname to check the actual IP addresses
    let port = url.port().unwrap_or(match url.scheme() {
        "https" => 443,
        _ => 80,
    });

    let addr_str = format!("{}:{}", host, port);
    match addr_str.to_socket_addrs() {
        Ok(addrs) => {
            for addr in addrs {
                if is_internal_ip(&addr.ip()) {
                    anyhow::bail!(
                        "Hostname '{}' resolves to internal IP address '{}', which is not allowed",
                        host,
                        addr.ip()
                    );
                }
            }
        }
        Err(e) => {
            // DNS resolution failed - this could be a temporary issue or invalid hostname
            // Log a warning but allow the request to proceed (reqwest will fail anyway)
            warn!("Failed to resolve hostname '{}': {}. Allowing request to proceed.", host, e);
        }
    }

    Ok(())
}

/// Checks if an IP address is internal/private
fn is_internal_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            // Loopback: 127.0.0.0/8
            if ipv4.is_loopback() {
                return true;
            }
            // Private: 10.0.0.0/8
            if ipv4.octets()[0] == 10 {
                return true;
            }
            // Private: 172.16.0.0/12
            if ipv4.octets()[0] == 172 && (16..=31).contains(&ipv4.octets()[1]) {
                return true;
            }
            // Private: 192.168.0.0/16
            if ipv4.octets()[0] == 192 && ipv4.octets()[1] == 168 {
                return true;
            }
            // Link-local: 169.254.0.0/16
            if ipv4.is_link_local() {
                return true;
            }
            // Broadcast
            if ipv4.is_broadcast() {
                return true;
            }
            // Unspecified (0.0.0.0)
            if ipv4.is_unspecified() {
                return true;
            }
            // Documentation: 192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24
            let octets = ipv4.octets();
            if (octets[0] == 192 && octets[1] == 0 && octets[2] == 2)
                || (octets[0] == 198 && octets[1] == 51 && octets[2] == 100)
                || (octets[0] == 203 && octets[1] == 0 && octets[2] == 113)
            {
                return true;
            }
            // Carrier-grade NAT: 100.64.0.0/10
            if octets[0] == 100 && (64..=127).contains(&octets[1]) {
                return true;
            }
            false
        }
        IpAddr::V6(ipv6) => {
            // Loopback: ::1
            if ipv6.is_loopback() {
                return true;
            }
            // Unspecified: ::
            if ipv6.is_unspecified() {
                return true;
            }
            // Multicast
            if ipv6.is_multicast() {
                return true;
            }
            // Unique local addresses (ULA): fc00::/7
            let segments = ipv6.segments();
            if (segments[0] & 0xfe00) == 0xfc00 {
                return true;
            }
            // Link-local: fe80::/10
            if (segments[0] & 0xffc0) == 0xfe80 {
                return true;
            }
            false
        }
    }
}

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
    pub parsed_audio: Option<String>,
    pub parsed_hdr: Option<String>,
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
    ///
    /// This method includes SSRF protection to prevent requests to internal networks.
    pub async fn fetch_feed(&self, url: &str) -> Result<Vec<ParsedRssItem>> {
        // Validate URL for SSRF protection
        validate_url_for_ssrf(url).context("URL validation failed")?;

        info!("Fetching RSS feed: {}", url);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch RSS feed")?;

        if !response.status().is_success() {
            anyhow::bail!("RSS feed returned error status: {}", response.status());
        }

        let content = response
            .text()
            .await
            .context("Failed to read RSS feed content")?;

        self.parse_feed(&content)
    }

    /// Parse RSS XML content into items
    pub fn parse_feed(&self, content: &str) -> Result<Vec<ParsedRssItem>> {
        use quick_xml::Reader;
        use quick_xml::events::Event;

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
                            && let Some(item) = self.build_item(builder)
                        {
                            items.push(item);
                        }
                    }
                    current_tag.clear();
                }
                Ok(Event::Text(ref e)) => {
                    if in_item && let Some(ref mut builder) = current_item {
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
                    if in_item && let Some(ref mut builder) = current_item {
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
            parsed_audio: parsed_quality.audio,
            parsed_hdr: parsed_quality.hdr,
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

    #[test]
    fn test_ssrf_blocks_loopback() {
        assert!(validate_url_for_ssrf("http://127.0.0.1/feed.xml").is_err());
        assert!(validate_url_for_ssrf("http://127.1.2.3:8080/feed").is_err());
        assert!(validate_url_for_ssrf("https://localhost/feed.xml").is_err());
    }

    #[test]
    fn test_ssrf_blocks_private_ranges() {
        // 10.0.0.0/8
        assert!(validate_url_for_ssrf("http://10.0.0.1/feed.xml").is_err());
        assert!(validate_url_for_ssrf("http://10.255.255.255/feed.xml").is_err());

        // 172.16.0.0/12
        assert!(validate_url_for_ssrf("http://172.16.0.1/feed.xml").is_err());
        assert!(validate_url_for_ssrf("http://172.31.255.255/feed.xml").is_err());
        // 172.32.x.x should be allowed (not in private range)
        assert!(validate_url_for_ssrf("http://172.32.0.1/feed.xml").is_ok());

        // 192.168.0.0/16
        assert!(validate_url_for_ssrf("http://192.168.0.1/feed.xml").is_err());
        assert!(validate_url_for_ssrf("http://192.168.255.255/feed.xml").is_err());
    }

    #[test]
    fn test_ssrf_blocks_link_local() {
        assert!(validate_url_for_ssrf("http://169.254.0.1/feed.xml").is_err());
        assert!(validate_url_for_ssrf("http://169.254.255.255/feed.xml").is_err());
    }

    #[test]
    fn test_ssrf_allows_public_ips() {
        assert!(validate_url_for_ssrf("https://1.2.3.4/feed.xml").is_ok());
        assert!(validate_url_for_ssrf("https://8.8.8.8/dns-query").is_ok());
        assert!(validate_url_for_ssrf("http://93.184.216.34/feed.xml").is_ok());
    }

    #[test]
    fn test_ssrf_blocks_invalid_schemes() {
        assert!(validate_url_for_ssrf("file:///etc/passwd").is_err());
        assert!(validate_url_for_ssrf("ftp://example.com/feed.xml").is_err());
        assert!(validate_url_for_ssrf("gopher://example.com/").is_err());
    }

    #[test]
    fn test_ssrf_allows_valid_external_urls() {
        assert!(validate_url_for_ssrf("https://example.com/feed.xml").is_ok());
        assert!(validate_url_for_ssrf("http://feeds.example.org/rss").is_ok());
    }
}
