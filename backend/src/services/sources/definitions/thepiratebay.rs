//! The Pirate Bay source implementation
//!
//! Uses TPB's public JSON API (`apibay.org`) for search results and maps
//! categories to our Torznab-compatible source model.

use std::collections::HashMap;

use anyhow::Result;
use async_graphql::async_trait::async_trait;
use chrono::{TimeZone, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::services::rate_limiter::RateLimitedClient;
use crate::services::sources::categories::CategoryMapping;
use crate::services::sources::categories::cats;
use crate::services::sources::{
    BookSearchParam, MovieSearchParam, MusicSearchParam, QueryType, Source, SourceCapabilities,
    SourceQuery, SourceRelease, SourceType, TrackerType, TvSearchParam,
};

#[derive(Debug, Deserialize)]
struct ApiBayResult {
    id: String,
    name: String,
    info_hash: String,
    leechers: String,
    seeders: String,
    num_files: String,
    size: String,
    username: String,
    imdb: String,
    added: String,
    category: String,
}

/// The Pirate Bay source
pub struct ThePirateBaySource {
    id: String,
    name: String,
    site_link: String,
    client: Client,
    rate_limiter: RateLimitedClient,
    settings: HashMap<String, String>,
    capabilities: SourceCapabilities,
}

impl ThePirateBaySource {
    pub fn new(
        id: String,
        name: String,
        site_url: Option<String>,
        settings: HashMap<String, String>,
    ) -> Result<Self> {
        let site_link = site_url.unwrap_or_else(|| "https://thepiratebay.org/".to_string());
        let client = Client::builder().gzip(true).build()?;

        Ok(Self {
            id,
            name,
            site_link,
            client,
            rate_limiter: RateLimitedClient::for_indexer(),
            settings,
            capabilities: Self::build_capabilities(),
        })
    }

    fn build_capabilities() -> SourceCapabilities {
        SourceCapabilities {
            search_available: true,
            tv_search_params: vec![TvSearchParam::Q, TvSearchParam::Season, TvSearchParam::Ep],
            movie_search_params: vec![MovieSearchParam::Q],
            music_search_params: vec![MusicSearchParam::Q],
            book_search_params: vec![BookSearchParam::Q],
            categories: Self::category_mappings(),
        }
    }

    fn category_mappings() -> Vec<CategoryMapping> {
        vec![
            // Audio
            CategoryMapping::new("100", cats::AUDIO, "Audio"),
            CategoryMapping::new("101", cats::AUDIO, "Music"),
            CategoryMapping::new("102", cats::AUDIO_AUDIOBOOK, "Audio Books"),
            CategoryMapping::new("103", cats::AUDIO, "Sound Clips"),
            CategoryMapping::new("104", cats::AUDIO_LOSSLESS, "FLAC"),
            CategoryMapping::new("199", cats::AUDIO_OTHER, "Audio Other"),
            // Video
            CategoryMapping::new("200", cats::MOVIES, "Video"),
            CategoryMapping::new("201", cats::MOVIES, "Movies"),
            CategoryMapping::new("202", cats::MOVIES_DVD, "Movies DVDR"),
            CategoryMapping::new("203", cats::AUDIO_VIDEO, "Music Videos"),
            CategoryMapping::new("204", cats::MOVIES, "Movie Clips"),
            CategoryMapping::new("205", cats::TV, "TV Shows"),
            CategoryMapping::new("206", cats::TV, "Handheld"),
            CategoryMapping::new("207", cats::MOVIES_HD, "HD Movies"),
            CategoryMapping::new("208", cats::TV_HD, "HD TV"),
            CategoryMapping::new("209", cats::MOVIES_3D, "3D"),
            CategoryMapping::new("210", cats::MOVIES_SD, "CAM/TS"),
            CategoryMapping::new("211", cats::MOVIES_UHD, "UHD/4k Movies"),
            CategoryMapping::new("212", cats::TV_HD, "UHD/4k TV"),
            CategoryMapping::new("299", cats::MOVIES, "Video Other"),
            // Applications
            CategoryMapping::new("300", cats::PC, "Applications"),
            CategoryMapping::new("301", cats::PC, "Windows"),
            CategoryMapping::new("302", cats::PC_MAC, "Mac"),
            CategoryMapping::new("303", cats::PC, "UNIX"),
            CategoryMapping::new("304", cats::PC_MOBILE_OTHER, "Handheld"),
            CategoryMapping::new("305", cats::PC_MOBILE_OTHER, "iOS"),
            CategoryMapping::new("306", cats::PC_MOBILE_OTHER, "Android"),
            CategoryMapping::new("399", cats::PC, "Other OS"),
            // Games
            CategoryMapping::new("400", cats::CONSOLE, "Games"),
            CategoryMapping::new("401", cats::PC_GAMES, "PC"),
            CategoryMapping::new("402", cats::PC_MAC, "Mac"),
            CategoryMapping::new("403", cats::CONSOLE_PS4, "PSx"),
            CategoryMapping::new("404", cats::CONSOLE_XBOX, "XBOX360"),
            CategoryMapping::new("405", cats::CONSOLE_WII, "Wii"),
            CategoryMapping::new("406", cats::CONSOLE_OTHER, "Handheld"),
            CategoryMapping::new("407", cats::CONSOLE_OTHER, "iOS"),
            CategoryMapping::new("408", cats::CONSOLE_OTHER, "Android"),
            CategoryMapping::new("499", cats::CONSOLE_OTHER, "Games Other"),
            // Other
            CategoryMapping::new("600", cats::OTHER, "Other"),
            CategoryMapping::new("601", cats::BOOKS, "E-books"),
            CategoryMapping::new("602", cats::BOOKS_COMICS, "Comics"),
            CategoryMapping::new("603", cats::BOOKS, "Pictures"),
            CategoryMapping::new("604", cats::BOOKS, "Covers"),
            CategoryMapping::new("605", cats::BOOKS, "Physibles"),
            CategoryMapping::new("699", cats::BOOKS_OTHER, "Other Other"),
        ]
    }

    fn build_keywords(query: &SourceQuery) -> Option<String> {
        let mut parts = Vec::new();

        if let Some(ref term) = query.search_term {
            let t = term.trim();
            if !t.is_empty() {
                parts.push(t.to_string());
            }
        }

        if let Some(ep) = query.get_episode_string() {
            parts.push(ep);
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" "))
        }
    }
}

#[async_trait]
impl Source for ThePirateBaySource {
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
        "thepiratebay"
    }

    fn site_link(&self) -> &str {
        &self.site_link
    }

    fn tracker_type(&self) -> TrackerType {
        TrackerType::Public
    }

    fn capabilities(&self) -> &SourceCapabilities {
        &self.capabilities
    }

    fn is_configured(&self) -> bool {
        true
    }

    async fn test_connection(&self) -> Result<bool> {
        self.rate_limiter.wait_for_permit().await;
        let response = self
            .client
            .get("https://apibay.org/precompiled/data_top100_recent.json")
            .send()
            .await?;
        Ok(response.status().is_success())
    }

    async fn search(&self, query: &SourceQuery) -> Result<Vec<SourceRelease>> {
        let tracker_cats = self.capabilities.map_torznab_to_tracker(&query.categories);
        let cat_csv = if tracker_cats.is_empty() {
            "0".to_string()
        } else {
            tracker_cats.join(",")
        };

        let endpoint = match Self::build_keywords(query) {
            Some(keywords) => format!(
                "https://apibay.org/q.php?q={}&cat={}",
                urlencoding::encode(&keywords),
                cat_csv
            ),
            None => "https://apibay.org/precompiled/data_top100_recent.json".to_string(),
        };

        self.rate_limiter.wait_for_permit().await;
        let rows: Vec<ApiBayResult> = self.client.get(endpoint).send().await?.json().await?;
        let uploader_filter = self
            .settings
            .get("Uploader")
            .map(|s| s.trim())
            .unwrap_or("");

        let mut releases = Vec::new();
        for row in rows {
            if row.id == "0" || row.name.trim().is_empty() || row.info_hash.trim().is_empty() {
                continue;
            }

            if !uploader_filter.is_empty() && row.username != uploader_filter {
                continue;
            }

            let publish_date = row
                .added
                .parse::<i64>()
                .ok()
                .and_then(|ts| Utc.timestamp_opt(ts, 0).single())
                .unwrap_or_else(Utc::now);

            let imdb = row
                .imdb
                .trim_start_matches("tt")
                .parse::<i64>()
                .ok()
                .filter(|v| *v > 0);

            let mut release = SourceRelease::new(
                row.name.clone(),
                format!("{}description.php?id={}", self.site_link, row.id),
                publish_date,
            );

            release.details = Some(format!("{}description.php?id={}", self.site_link, row.id));
            release.info_hash = Some(row.info_hash.clone());
            release.magnet_uri = Some(format!(
                "magnet:?xt=urn:btih:{}&dn={}",
                row.info_hash,
                urlencoding::encode(&row.name)
            ));
            release.categories = self.capabilities.map_tracker_to_torznab(&row.category);
            release.size = row.size.parse::<i64>().ok();
            release.files = row.num_files.parse::<i32>().ok();
            release.seeders = row.seeders.parse::<i32>().ok();
            release.peers = match (
                row.seeders.parse::<i32>().ok(),
                row.leechers.parse::<i32>().ok(),
            ) {
                (Some(seeders), Some(leechers)) => Some(seeders + leechers),
                _ => None,
            };
            release.imdb = imdb;
            release.description = Some(format!("Uploader: {}", row.username));
            release.download_volume_factor = 1.0;
            release.upload_volume_factor = 1.0;

            // TPB search endpoint does broad matching, so do lightweight client-side title filtering.
            if let Some(ref term) = query.search_term {
                let t = term.trim().to_lowercase();
                if !t.is_empty() {
                    let title = release.title.to_lowercase();
                    if !t.split_whitespace().all(|w| title.contains(w)) {
                        continue;
                    }
                }
            }

            releases.push(release);
        }

        // Keep deterministic ordering by seeders (desc) then newest.
        releases.sort_by(|a, b| {
            b.seeders
                .unwrap_or_default()
                .cmp(&a.seeders.unwrap_or_default())
                .then_with(|| b.publish_date.cmp(&a.publish_date))
        });

        if let Some(limit) = query.limit {
            if limit > 0 && releases.len() > limit as usize {
                releases.truncate(limit as usize);
            }
        }

        Ok(releases)
    }

    async fn download(&self, link: &str) -> Result<Vec<u8>> {
        if link.starts_with("magnet:") {
            anyhow::bail!("Magnet links are not directly downloadable");
        }
        self.rate_limiter.wait_for_permit().await;
        let bytes = self.client.get(link).send().await?.bytes().await?;
        Ok(bytes.to_vec())
    }
}
