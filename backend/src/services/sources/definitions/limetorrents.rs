//! LimeTorrents source implementation

use std::collections::HashMap;

use anyhow::Result;
use async_graphql::async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use scraper::{Html, Selector};

use crate::services::rate_limiter::RateLimitedClient;
use crate::services::sources::categories::CategoryMapping;
use crate::services::sources::categories::cats;
use crate::services::sources::{
    BookSearchParam, MovieSearchParam, MusicSearchParam, Source, SourceCapabilities, SourceQuery,
    SourceRelease, SourceType, TrackerType, TvSearchParam,
};

pub struct LimeTorrentsSource {
    id: String,
    name: String,
    site_link: String,
    client: Client,
    rate_limiter: RateLimitedClient,
    capabilities: SourceCapabilities,
    _settings: HashMap<String, String>,
}

impl LimeTorrentsSource {
    pub fn new(
        id: String,
        name: String,
        site_url: Option<String>,
        settings: HashMap<String, String>,
    ) -> Result<Self> {
        Ok(Self {
            id,
            name,
            site_link: site_url.unwrap_or_else(|| "https://www.limetorrents.lol/".to_string()),
            client: Client::builder().gzip(true).build()?,
            rate_limiter: RateLimitedClient::for_indexer(),
            capabilities: SourceCapabilities {
                search_available: true,
                tv_search_params: vec![TvSearchParam::Q],
                movie_search_params: vec![MovieSearchParam::Q],
                music_search_params: vec![MusicSearchParam::Q],
                book_search_params: vec![BookSearchParam::Q],
                categories: vec![
                    CategoryMapping::new("movies", cats::MOVIES, "Movies"),
                    CategoryMapping::new("tv", cats::TV, "TV"),
                    CategoryMapping::new("music", cats::AUDIO, "Music"),
                    CategoryMapping::new("games", cats::PC_GAMES, "Games"),
                    CategoryMapping::new("applications", cats::PC, "Applications"),
                    CategoryMapping::new("anime", cats::TV_ANIME, "Anime"),
                    CategoryMapping::new("other", cats::OTHER, "Other"),
                ],
            },
            _settings: settings,
        })
    }

    fn parse_size(input: &str) -> Option<i64> {
        let clean = input.replace(',', "");
        let parts: Vec<_> = clean.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }
        let n = parts[0].parse::<f64>().ok()?;
        let mult = match parts[1].to_uppercase().as_str() {
            "KB" | "KIB" => 1024.0,
            "MB" | "MIB" => 1024.0 * 1024.0,
            "GB" | "GIB" => 1024.0 * 1024.0 * 1024.0,
            "TB" | "TIB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
            "B" => 1.0,
            _ => return None,
        };
        Some((n * mult) as i64)
    }

    fn map_category(text: &str) -> Vec<i32> {
        let t = text.to_lowercase();
        if t.contains("movie") {
            vec![cats::MOVIES]
        } else if t.contains("tv") {
            vec![cats::TV]
        } else if t.contains("music") {
            vec![cats::AUDIO]
        } else if t.contains("game") {
            vec![cats::PC_GAMES]
        } else if t.contains("anime") {
            vec![cats::TV_ANIME]
        } else if t.contains("app") || t.contains("software") {
            vec![cats::PC]
        } else if t.contains("book") {
            vec![cats::BOOKS]
        } else {
            vec![cats::OTHER]
        }
    }
}

#[async_trait]
impl Source for LimeTorrentsSource {
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
        "limetorrents"
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
        let response = self.client.get(&self.site_link).send().await?;
        Ok(response.status().is_success())
    }

    async fn search(&self, query: &SourceQuery) -> Result<Vec<SourceRelease>> {
        let term = query.search_term.as_deref().unwrap_or("").trim();
        if term.is_empty() {
            return Ok(vec![]);
        }

        let search_url = format!(
            "{}search/all/{}/",
            self.site_link,
            urlencoding::encode(term)
        );
        self.rate_limiter.wait_for_permit().await;
        let html = self.client.get(search_url).send().await?.text().await?;
        let doc = Html::parse_document(&html);

        let row_selector = Selector::parse("table.table2 tr").unwrap();
        let link_selector = Selector::parse("td.tt-name a").unwrap();
        let seed_selector = Selector::parse("td.tdseed").unwrap();
        let leech_selector = Selector::parse("td.tdleech").unwrap();
        let size_selector = Selector::parse("td.tdnormal").unwrap();
        let cat_selector = Selector::parse("div.tt-name a:first-child").unwrap();

        let mut releases = Vec::new();
        for row in doc.select(&row_selector) {
            let links: Vec<_> = row.select(&link_selector).collect();
            if links.len() < 2 {
                continue;
            }

            let title_link = links[1];
            let title = title_link.text().collect::<String>().trim().to_string();
            if title.is_empty() {
                continue;
            }

            let details = title_link.value().attr("href").map(|h| {
                if h.starts_with("http") {
                    h.to_string()
                } else {
                    format!("{}{}", self.site_link, h.trim_start_matches('/'))
                }
            });

            let cat_text = row
                .select(&cat_selector)
                .next()
                .map(|v| v.text().collect::<String>())
                .unwrap_or_default();

            let seeders = row
                .select(&seed_selector)
                .next()
                .and_then(|v| v.text().collect::<String>().trim().parse::<i32>().ok());
            let leechers = row
                .select(&leech_selector)
                .next()
                .and_then(|v| v.text().collect::<String>().trim().parse::<i32>().ok());

            let size_raw = row
                .select(&size_selector)
                .next()
                .map(|v| v.text().collect::<String>())
                .unwrap_or_default();

            let mut release = SourceRelease::new(
                title.clone(),
                details.clone().unwrap_or_else(|| title.clone()),
                Utc::now(),
            );
            release.details = details;
            release.categories = Self::map_category(&cat_text);
            release.size = Self::parse_size(&size_raw);
            release.seeders = seeders;
            release.peers = match (seeders, leechers) {
                (Some(s), Some(l)) => Some(s + l),
                _ => None,
            };

            releases.push(release);
        }

        releases.sort_by(|a, b| {
            b.seeders
                .unwrap_or_default()
                .cmp(&a.seeders.unwrap_or_default())
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
