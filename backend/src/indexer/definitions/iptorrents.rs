//! IPTorrents indexer implementation
//!
//! IPTorrents is a private torrent tracker. This implementation is based on
//! Jackett's IPTorrents indexer, ported to Rust.
//!
//! # Authentication
//!
//! IPTorrents uses cookie-based authentication. Users need to:
//! 1. Log into IPTorrents in their browser
//! 2. Copy the cookie value from their browser's developer tools
//! 3. Configure the cookie and user agent in Librarian
//!
//! # Configuration
//!
//! Required credentials:
//! - `cookie`: Session cookie from the browser
//! - `user_agent`: Browser user agent string
//!
//! Optional settings:
//! - `freeleech`: Only search for freeleech torrents
//! - `sort`: Sort order (time, size, seeders, name)

use std::collections::HashMap;

use anyhow::{Result, anyhow};
use async_graphql::async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::{Client, header};
use scraper::{Html, Selector};

use crate::indexer::categories::CategoryMapping;
use crate::indexer::{
    Indexer, IndexerType, MovieSearchParam, ReleaseInfo, TorznabCapabilities, TorznabQuery,
    TrackerType, TvSearchParam, categories::cats,
};

/// IPTorrents alternative site links (for future use)
#[allow(dead_code)]
const ALTERNATIVE_LINKS: &[&str] = &[
    "https://iptorrents.com/",
    "https://www.iptorrents.com/",
    "https://iptorrents.me/",
    "https://nemo.iptorrents.com/",
    "https://ip.findnemo.net/",
    "https://ip.venom.global/",
    "https://ip.getcrazy.me/",
    "https://ip.workisboring.net/",
    "https://ipt.cool/",
    "https://ipt.lol/",
    "https://ipt.world/",
];

/// IPTorrents indexer
pub struct IPTorrentsIndexer {
    /// Unique instance ID
    id: String,
    /// Display name (kept for future use in logging/UI)
    #[allow(dead_code)]
    name: String,
    /// Site URL
    site_link: String,
    /// HTTP client
    client: Client,
    /// Session cookie (stored for potential re-authentication)
    #[allow(dead_code)]
    cookie: String,
    /// Settings
    settings: HashMap<String, String>,
    /// Cached capabilities
    capabilities: TorznabCapabilities,
}

impl IPTorrentsIndexer {
    /// Create a new IPTorrents indexer instance
    pub fn new(
        id: String,
        name: String,
        site_url: Option<String>,
        cookie: &str,
        user_agent: &str,
        settings: HashMap<String, String>,
    ) -> Result<Self> {
        let site_link = site_url.unwrap_or_else(|| "https://iptorrents.com/".to_string());

        if cookie.is_empty() {
            tracing::warn!(indexer_id = %id, "Cookie is empty - authentication will fail");
        }

        // Build HTTP client with cookie and user agent
        let mut headers = header::HeaderMap::new();
        match cookie.parse() {
            Ok(v) => {
                headers.insert(header::COOKIE, v);
            }
            Err(e) => {
                tracing::error!(indexer_id = %id, error = %e, "Failed to parse cookie header");
                return Err(anyhow!("Invalid cookie format: {}", e));
            }
        }
        if !user_agent.is_empty() {
            match user_agent.parse() {
                Ok(v) => {
                    headers.insert(header::USER_AGENT, v);
                }
                Err(e) => {
                    tracing::warn!(indexer_id = %id, error = %e, "Failed to parse user agent header, using default");
                }
            }
        }

        let client = Client::builder()
            .default_headers(headers)
            .cookie_store(true)
            .gzip(true)
            .build()?;

        let capabilities = Self::build_capabilities();

        Ok(Self {
            id,
            name,
            site_link,
            client,
            cookie: cookie.to_string(),
            settings,
            capabilities,
        })
    }

    /// Build the capabilities for IPTorrents
    fn build_capabilities() -> TorznabCapabilities {
        let mut caps = TorznabCapabilities {
            search_available: true,
            limits_default: Some(100),
            limits_max: Some(100),
            tv_search_params: vec![
                TvSearchParam::Q,
                TvSearchParam::Season,
                TvSearchParam::Ep,
                TvSearchParam::ImdbId,
                TvSearchParam::Genre,
            ],
            tv_search_imdb_available: true,
            movie_search_params: vec![
                MovieSearchParam::Q,
                MovieSearchParam::ImdbId,
                MovieSearchParam::Genre,
            ],
            ..Default::default()
        };

        // Category mappings from Jackett
        let mappings = Self::category_mappings();
        caps.categories = mappings;

        caps
    }

    /// Get category mappings (tracker category -> Torznab category)
    fn category_mappings() -> Vec<CategoryMapping> {
        vec![
            // Movies
            CategoryMapping::new("72", cats::MOVIES, "Movies"),
            CategoryMapping::new("87", cats::MOVIES_3D, "Movie/3D"),
            CategoryMapping::new("77", cats::MOVIES_SD, "Movie/480p"),
            CategoryMapping::new("101", cats::MOVIES_UHD, "Movie/4K"),
            CategoryMapping::new("89", cats::MOVIES_BLURAY, "Movie/BD-R"),
            CategoryMapping::new("90", cats::MOVIES_HD, "Movie/BD-Rip"),
            CategoryMapping::new("96", cats::MOVIES_SD, "Movie/Cam"),
            CategoryMapping::new("6", cats::MOVIES_DVD, "Movie/DVD-R"),
            CategoryMapping::new("48", cats::MOVIES_HD, "Movie/HD/Bluray"),
            CategoryMapping::new("54", cats::MOVIES, "Movie/Kids"),
            CategoryMapping::new("62", cats::MOVIES_SD, "Movie/MP4"),
            CategoryMapping::new("38", cats::MOVIES_FOREIGN, "Movie/Non-English"),
            CategoryMapping::new("68", cats::MOVIES, "Movie/Packs"),
            CategoryMapping::new("20", cats::MOVIES_WEBDL, "Movie/Web-DL"),
            CategoryMapping::new("100", cats::MOVIES_HD, "Movie/x265"),
            CategoryMapping::new("7", cats::MOVIES_SD, "Movie/Xvid"),
            // TV
            CategoryMapping::new("73", cats::TV, "TV"),
            CategoryMapping::new("26", cats::TV_DOCUMENTARY, "TV/Documentaries"),
            CategoryMapping::new("55", cats::TV_SPORT, "Sports"),
            CategoryMapping::new("78", cats::TV_SD, "TV/480p"),
            CategoryMapping::new("23", cats::TV_HD, "TV/BD"),
            CategoryMapping::new("24", cats::TV_SD, "TV/DVD-R"),
            CategoryMapping::new("25", cats::TV_SD, "TV/DVD-Rip"),
            CategoryMapping::new("66", cats::TV_SD, "TV/Mobile"),
            CategoryMapping::new("82", cats::TV_FOREIGN, "TV/Non-English"),
            CategoryMapping::new("65", cats::TV, "TV/Packs"),
            CategoryMapping::new("83", cats::TV_FOREIGN, "TV/Packs/Non-English"),
            CategoryMapping::new("79", cats::TV_SD, "TV/SD/x264"),
            CategoryMapping::new("22", cats::TV_WEBDL, "TV/Web-DL"),
            CategoryMapping::new("5", cats::TV_HD, "TV/x264"),
            CategoryMapping::new("99", cats::TV_HD, "TV/x265"),
            CategoryMapping::new("4", cats::TV_SD, "TV/Xvid"),
            // Games
            CategoryMapping::new("74", cats::CONSOLE, "Games"),
            CategoryMapping::new("2", cats::CONSOLE_OTHER, "Games/Mixed"),
            CategoryMapping::new("47", cats::CONSOLE_OTHER, "Games/Nintendo"),
            CategoryMapping::new("43", cats::PC_GAMES, "Games/PC-ISO"),
            CategoryMapping::new("45", cats::PC_GAMES, "Games/PC-Rip"),
            CategoryMapping::new("71", cats::CONSOLE_PS4, "Games/Playstation"),
            CategoryMapping::new("50", cats::CONSOLE_WII, "Games/Wii"),
            CategoryMapping::new("44", cats::CONSOLE_XBOX, "Games/Xbox"),
            // Music
            CategoryMapping::new("75", cats::AUDIO, "Music"),
            CategoryMapping::new("3", cats::AUDIO_MP3, "Music/Audio"),
            CategoryMapping::new("80", cats::AUDIO_LOSSLESS, "Music/Flac"),
            CategoryMapping::new("93", cats::AUDIO, "Music/Packs"),
            CategoryMapping::new("37", cats::AUDIO_VIDEO, "Music/Video"),
            CategoryMapping::new("21", cats::AUDIO_OTHER, "Podcast"),
            // Other
            CategoryMapping::new("76", cats::OTHER, "Miscellaneous"),
            CategoryMapping::new("60", cats::TV_ANIME, "Anime"),
            CategoryMapping::new("1", cats::PC_0DAY, "Appz"),
            CategoryMapping::new("86", cats::PC_0DAY, "Appz/Non-English"),
            CategoryMapping::new("64", cats::AUDIO_AUDIOBOOK, "AudioBook"),
            CategoryMapping::new("35", cats::BOOKS, "Books"),
            CategoryMapping::new("102", cats::BOOKS, "Books/Non-English"),
            CategoryMapping::new("94", cats::BOOKS_COMICS, "Comics"),
            CategoryMapping::new("95", cats::BOOKS_OTHER, "Educational"),
            CategoryMapping::new("98", cats::OTHER, "Fonts"),
            CategoryMapping::new("69", cats::PC_MAC, "Mac"),
            CategoryMapping::new("92", cats::BOOKS_MAGS, "Magazines / Newspapers"),
            CategoryMapping::new("58", cats::PC_MOBILE_OTHER, "Mobile"),
            CategoryMapping::new("36", cats::OTHER, "Pics/Wallpapers"),
            // XXX
            CategoryMapping::new("88", 6000, "XXX"),
            CategoryMapping::new("85", 6070, "XXX/Magazines"),
            CategoryMapping::new("8", 6000, "XXX/Movie"),
            CategoryMapping::new("81", 6000, "XXX/Movie/0Day"),
            CategoryMapping::new("91", 6050, "XXX/Packs"),
            CategoryMapping::new("84", 6060, "XXX/Pics/Wallpapers"),
        ]
    }

    /// Build the search URL
    fn build_search_url(&self, query: &TorznabQuery) -> String {
        let mut url = format!("{}t?", self.site_link);

        // Add categories
        let tracker_cats = self.capabilities.map_torznab_to_tracker(&query.categories);
        for cat in tracker_cats {
            url.push_str(&format!("{}=&", cat));
        }

        // Freeleech filter
        if self
            .settings
            .get("freeleech")
            .map(|s| s == "true")
            .unwrap_or(false)
        {
            url.push_str("free=on&");
        }

        // Build search query
        let mut search_parts = Vec::new();

        // IMDB search
        if let Some(ref imdb_id) = query.imdb_id {
            search_parts.push(format!("+({})", imdb_id));
            url.push_str("qf=all&"); // Search in description for IMDB
        } else if let Some(ref genre) = query.genre {
            search_parts.push(format!("+({})", genre));
        }

        // Text search
        if let Some(ref term) = query.search_term {
            let mut search_term = term.clone();

            // Add wildcard for season-only searches
            if query.season.is_some() && query.episode.is_none() {
                search_term.push('*');
            }

            search_parts.push(format!("+({})", search_term));
        }

        // Episode string
        if let Some(ep_str) = query.get_episode_string() {
            search_parts.push(format!("+({})", ep_str));
        }

        if !search_parts.is_empty() {
            url.push_str(&format!("q={}&", search_parts.join(" ")));
        }

        // Sort order
        let sort = self
            .settings
            .get("sort")
            .map(|s| s.as_str())
            .unwrap_or("time");
        url.push_str(&format!("o={}&", sort));

        // Pagination
        if let (Some(limit), Some(offset)) = (query.limit, query.offset) {
            if limit > 0 && offset > 0 {
                let page = offset / limit + 1;
                url.push_str(&format!("p={}", page));
            }
        }

        url.trim_end_matches('&').to_string()
    }

    /// Parse a relative time string (e.g., "2 hours ago")
    fn parse_time_ago(time_str: &str) -> DateTime<Utc> {
        let time_str = time_str.to_lowercase();
        let now = Utc::now();

        // Try to extract number and unit
        let parts: Vec<&str> = time_str.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(num) = parts[0].parse::<i64>() {
                let unit = parts[1];
                let duration = if unit.starts_with("second") {
                    chrono::Duration::seconds(num)
                } else if unit.starts_with("minute") {
                    chrono::Duration::minutes(num)
                } else if unit.starts_with("hour") {
                    chrono::Duration::hours(num)
                } else if unit.starts_with("day") {
                    chrono::Duration::days(num)
                } else if unit.starts_with("week") {
                    chrono::Duration::weeks(num)
                } else if unit.starts_with("month") {
                    chrono::Duration::days(num * 30)
                } else if unit.starts_with("year") {
                    chrono::Duration::days(num * 365)
                } else {
                    chrono::Duration::zero()
                };
                return now - duration;
            }
        }

        now
    }

    /// Parse size string (e.g., "1.5 GB")
    fn parse_size(size_str: &str) -> Option<i64> {
        let size_str = size_str.trim().to_uppercase();
        let parts: Vec<&str> = size_str.split_whitespace().collect();

        if parts.len() >= 2 {
            if let Ok(num) = parts[0].replace(',', "").parse::<f64>() {
                let multiplier = match parts[1] {
                    "B" | "BYTES" => 1.0,
                    "KB" | "KIB" => 1024.0,
                    "MB" | "MIB" => 1024.0 * 1024.0,
                    "GB" | "GIB" => 1024.0 * 1024.0 * 1024.0,
                    "TB" | "TIB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
                    _ => return None,
                };
                return Some((num * multiplier) as i64);
            }
        }

        None
    }

    /// Clean title string
    fn clean_title(title: &str) -> String {
        // Remove [REQ] or [REQUEST] tags at the start
        let title = if let Some(stripped) = title.strip_prefix("[REQ]") {
            stripped.trim_start()
        } else if let Some(stripped) = title.strip_prefix("[REQUEST]") {
            stripped.trim_start()
        } else if let Some(stripped) = title.strip_prefix("[REQUESTED]") {
            stripped.trim_start()
        } else {
            title
        };

        // Just trim whitespace and common separators
        title
            .trim()
            .trim_matches(|c| c == '-' || c == ':')
            .trim()
            .to_string()
    }
}

#[async_trait]
impl Indexer for IPTorrentsIndexer {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "IPTorrents is a Private site. Always a step ahead."
    }

    fn indexer_type(&self) -> IndexerType {
        IndexerType::Native
    }

    fn site_link(&self) -> &str {
        &self.site_link
    }

    fn tracker_type(&self) -> TrackerType {
        TrackerType::Private
    }

    fn language(&self) -> &str {
        "en-US"
    }

    fn capabilities(&self) -> &TorznabCapabilities {
        &self.capabilities
    }

    fn is_configured(&self) -> bool {
        !self.cookie.is_empty()
    }

    fn supports_pagination(&self) -> bool {
        true
    }

    async fn test_connection(&self) -> Result<bool> {
        let url = format!("{}t", self.site_link);

        let response = self.client.get(&url).send().await?;
        let text = response.text().await?;

        // Check if we're logged in by looking for the logout link
        let is_logged_in = text.contains("/lout.php");

        if !is_logged_in {
            tracing::warn!(
                indexer_id = %self.id,
                "Connection test failed - cookie may be invalid"
            );
        }

        Ok(is_logged_in)
    }

    async fn search(&self, query: &TorznabQuery) -> Result<Vec<ReleaseInfo>> {
        let search_url = self.build_search_url(query);

        let response = self
            .client
            .get(&search_url)
            .header(header::REFERER, format!("{}t", self.site_link))
            .send()
            .await?;

        let text = response.text().await?;

        // Check if logged in
        if !text.contains("/lout.php") {
            return Err(anyhow!(
                "The user is not logged in. The cookie may have expired or is incorrect."
            ));
        }

        // Check for no results
        if text.contains("No Torrents Found!") {
            return Ok(vec![]);
        }

        // Parse HTML
        let document = Html::parse_document(&text);

        // Try to find the torrent table - IPTorrents uses different table structures
        // Common selectors in order of preference
        let table_selectors = [
            "table#torrents",          // Main torrent table by ID
            "table.t1",                // Alternative class-based
            "table[class*='torrent']", // Any table with torrent in class
            "#content table",          // Table inside content div
            "table",                   // Fallback to any table
        ];

        let mut table_element = None;
        for sel in &table_selectors {
            if let Ok(selector) = Selector::parse(sel) {
                if let Some(table) = document.select(&selector).next() {
                    table_element = Some(table);
                    break;
                }
            }
        }

        let table = match table_element {
            Some(t) => t,
            None => return Ok(vec![]),
        };

        // Find all rows - try different approaches
        let row_selectors = ["tbody > tr", "tr"];

        let mut rows = Vec::new();
        for sel in &row_selectors {
            if let Ok(selector) = Selector::parse(sel) {
                rows = table.select(&selector).collect();
                if !rows.is_empty() {
                    break;
                }
            }
        }

        if rows.is_empty() {
            return Ok(vec![]);
        }

        // Selectors for various elements
        let link_selector = Selector::parse("a").unwrap();
        let free_selector = Selector::parse("span.free").unwrap();

        let mut releases = Vec::new();

        for row in &rows {
            // Find the title link - it's the link with href starting with /t/
            // and contains the torrent title text
            let mut title_link = None;

            for link in row.select(&link_selector) {
                let href = link.value().attr("href").unwrap_or("");
                // Title links have href like /t/1234567
                if href.starts_with("/t/") && !href.contains("?") {
                    // Check it has substantial text (not just an icon)
                    let text: String = link.text().collect();
                    if text.len() > 5 {
                        title_link = Some(link);
                        break;
                    }
                }
            }

            let title_el = match title_link {
                Some(el) => el,
                None => continue,
            };

            let raw_title = title_el.text().collect::<String>();
            let title = Self::clean_title(&raw_title);

            // Skip empty titles
            if title.trim().is_empty() {
                continue;
            }

            // Skip if title doesn't match query (for non-ID searches)
            if query.imdb_id.is_none() && query.genre.is_none() {
                if let Some(ref term) = query.search_term {
                    if !term.is_empty() {
                        let term_lower = term.to_lowercase();
                        let title_lower = title.to_lowercase();
                        if !term_lower
                            .split_whitespace()
                            .all(|word| title_lower.contains(word))
                        {
                            continue;
                        }
                    }
                }
            }

            // Get details URL
            let href = title_el.value().attr("href").unwrap_or("");
            let details = format!("{}{}", self.site_link, href.trim_start_matches('/'));

            // Get download URL - find link with /download in href
            let mut download_url = None;
            for link in row.select(&link_selector) {
                let href = link.value().attr("href").unwrap_or("");
                if href.contains("/download") {
                    download_url = Some(format!(
                        "{}{}",
                        self.site_link,
                        href.trim_start_matches('/')
                    ));
                    break;
                }
            }
            let link = download_url;

            // Get all cells from the row
            let cells: Vec<_> = row.select(&Selector::parse("td").unwrap()).collect();

            // Get category from first cell's link
            let cat_selector = Selector::parse("td:first-child a[href^='?']").ok();
            let categories = if let Some(sel) = &cat_selector {
                if let Some(icon) = row.select(sel).next() {
                    let cat_href = icon.value().attr("href").unwrap_or("");
                    let cat_id = cat_href.trim_start_matches('?');
                    self.capabilities.map_tracker_to_torznab(cat_id)
                } else {
                    vec![]
                }
            } else {
                vec![]
            };

            // Get description and date from sub div
            let descr_selector = Selector::parse("div.sub, .sub, span.sub").ok();
            let (description, publish_date) = if let Some(sel) = &descr_selector {
                if let Some(el) = row.select(sel).next() {
                    let text = el.text().collect::<String>();
                    let parts: Vec<&str> = text.split('|').collect();

                    let tags = if parts.len() > 1 {
                        Some(format!("Tags: {}", parts[0].trim()))
                    } else {
                        None
                    };

                    let date_part = parts.last().unwrap_or(&"");
                    let date_parts: Vec<&str> = date_part.split(" by ").collect();
                    let date = Self::parse_time_ago(date_parts[0].trim());

                    (tags, date)
                } else {
                    (None, Utc::now())
                }
            } else {
                (None, Utc::now())
            };

            // Try to find size - look for cells with size-like content (e.g., "32.7 GB")
            let mut size = None;
            for cell in &cells {
                let text = cell.text().collect::<String>();
                if let Some(s) = Self::parse_size(&text) {
                    size = Some(s);
                    break;
                }
            }

            // If no size found from parsing, try by column index (usually column 4-5)
            if size.is_none() && cells.len() > 4 {
                for idx in [4, 5, 3] {
                    if let Some(cell) = cells.get(idx) {
                        let text = cell.text().collect::<String>();
                        if let Some(s) = Self::parse_size(&text) {
                            size = Some(s);
                            break;
                        }
                    }
                }
            }

            // Files count not critical, skip for now
            let files: Option<i32> = None;

            // Get seeders/leechers/grabs
            let col_count = cells.len();
            let (grabs, seeders, leechers) = if col_count >= 3 {
                let grabs_idx = col_count.saturating_sub(3);
                let seeders_idx = col_count.saturating_sub(2);
                let leechers_idx = col_count.saturating_sub(1);

                let grabs: Option<i32> = cells
                    .get(grabs_idx)
                    .map(|el| el.text().collect::<String>())
                    .and_then(|s| s.trim().replace(',', "").parse().ok());
                let seeders: Option<i32> = cells
                    .get(seeders_idx)
                    .map(|el| el.text().collect::<String>())
                    .and_then(|s| s.trim().replace(',', "").parse().ok());
                let leechers: Option<i32> = cells
                    .get(leechers_idx)
                    .map(|el| el.text().collect::<String>())
                    .and_then(|s| s.trim().replace(',', "").parse().ok());

                (grabs, seeders, leechers)
            } else {
                (None, None, None)
            };

            // Check if freeleech
            let is_freeleech = row.select(&free_selector).next().is_some();

            let peers = match (seeders, leechers) {
                (Some(s), Some(l)) => Some(s + l),
                _ => None,
            };

            let release = ReleaseInfo {
                title,
                guid: details.clone(),
                link,
                details: Some(details),
                publish_date,
                categories,
                size,
                files,
                grabs,
                seeders,
                peers,
                description,
                download_volume_factor: if is_freeleech { 0.0 } else { 1.0 },
                upload_volume_factor: 1.0,
                minimum_ratio: Some(1.0),
                minimum_seed_time: Some(1_209_600), // 14 days in seconds
                ..Default::default()
            };

            releases.push(release);
        }

        Ok(releases)
    }

    async fn download(&self, link: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(link)
            .header(header::REFERER, &self.site_link)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_title() {
        assert_eq!(
            IPTorrentsIndexer::clean_title("[REQ] Some Movie 2024"),
            "Some Movie 2024"
        );
        assert_eq!(
            IPTorrentsIndexer::clean_title("[REQUEST] Another Title"),
            "Another Title"
        );
        assert_eq!(
            IPTorrentsIndexer::clean_title("Normal Title"),
            "Normal Title"
        );
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(IPTorrentsIndexer::parse_size("1.5 GB"), Some(1_610_612_736));
        assert_eq!(IPTorrentsIndexer::parse_size("500 MB"), Some(524_288_000));
        assert_eq!(
            IPTorrentsIndexer::parse_size("1 TB"),
            Some(1_099_511_627_776)
        );
    }

    #[test]
    fn test_category_mappings() {
        let mappings = IPTorrentsIndexer::category_mappings();
        assert!(!mappings.is_empty());

        // Check that movie category exists
        assert!(
            mappings
                .iter()
                .any(|m| m.tracker_id == "72" && m.torznab_cat == cats::MOVIES)
        );
    }
}
