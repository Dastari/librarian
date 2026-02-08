//! 1337x source implementation

use std::collections::HashMap;
use std::time::Duration;

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

pub struct Source1337x {
    id: String,
    name: String,
    site_link: String,
    client: Client,
    rate_limiter: RateLimitedClient,
    capabilities: SourceCapabilities,
    _settings: HashMap<String, String>,
}

impl Source1337x {
    pub fn new(
        id: String,
        name: String,
        site_url: Option<String>,
        settings: HashMap<String, String>,
    ) -> Result<Self> {
        Ok(Self {
            id,
            name,
            site_link: site_url.unwrap_or_else(|| "https://1337x.to/".to_string()),
            client: Client::builder().gzip(true).build()?,
            // Jackett definition sets requestDelay: 2 seconds.
            rate_limiter: RateLimitedClient::for_indexer_with_request_delay(Duration::from_secs(2)),
            capabilities: SourceCapabilities {
                search_available: true,
                tv_search_params: vec![TvSearchParam::Q],
                movie_search_params: vec![MovieSearchParam::Q],
                music_search_params: vec![MusicSearchParam::Q],
                book_search_params: vec![BookSearchParam::Q],
                categories: vec![
                    CategoryMapping::new("movies", cats::MOVIES, "Movies"),
                    CategoryMapping::new("tv", cats::TV, "TV"),
                    CategoryMapping::new("games", cats::PC_GAMES, "Games"),
                    CategoryMapping::new("music", cats::AUDIO, "Music"),
                    CategoryMapping::new("apps", cats::PC, "Applications"),
                    CategoryMapping::new("anime", cats::TV_ANIME, "Anime"),
                    CategoryMapping::new("documentaries", cats::TV_DOCUMENTARY, "Documentaries"),
                    CategoryMapping::new("other", cats::OTHER, "Other"),
                ],
            },
            _settings: settings,
        })
    }

    fn parse_size(input: &str) -> Option<i64> {
        let upper = input.to_uppercase();
        let (num, unit) = upper
            .split_whitespace()
            .find_map(|part| {
                if part.parse::<f64>().is_ok() {
                    Some((part.to_string(), String::new()))
                } else if part.ends_with("B") {
                    Some(("".to_string(), part.to_string()))
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let n = num.parse::<f64>().ok()?;
        let mult = if unit.starts_with("KB") {
            1024.0
        } else if unit.starts_with("MB") {
            1024.0 * 1024.0
        } else if unit.starts_with("GB") {
            1024.0 * 1024.0 * 1024.0
        } else if unit.starts_with("TB") {
            1024.0 * 1024.0 * 1024.0 * 1024.0
        } else {
            1.0
        };
        Some((n * mult) as i64)
    }

    fn category_from_text(text: &str) -> Vec<i32> {
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
        } else if t.contains("documentary") {
            vec![cats::TV_DOCUMENTARY]
        } else if t.contains("app") || t.contains("software") {
            vec![cats::PC]
        } else {
            vec![cats::OTHER]
        }
    }
}

#[async_trait]
impl Source for Source1337x {
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
        "1337x"
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

        let search_url = format!("{}search/{}/1/", self.site_link, urlencoding::encode(term));

        self.rate_limiter.wait_for_permit().await;
        let html = self.client.get(search_url).send().await?.text().await?;
        let doc = Html::parse_document(&html);

        let row_selector = Selector::parse("table.table-list tbody tr, tbody tr").unwrap();
        let cat_selector = Selector::parse("td.coll-1 a, td.category a").unwrap();
        let title_selector =
            Selector::parse("td.name a:last-child, td.coll-1 a:last-child").unwrap();
        let seed_selector = Selector::parse("td.seeds, td.coll-2").unwrap();
        let leech_selector = Selector::parse("td.leeches, td.coll-3").unwrap();
        let size_selector = Selector::parse("td.size, td.coll-4").unwrap();

        let mut releases = Vec::new();
        for row in doc.select(&row_selector) {
            let title_link = row.select(&title_selector).next();
            let title_link = match title_link {
                Some(v) => v,
                None => continue,
            };

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
            let categories = Self::category_from_text(&cat_text);

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
            release.categories = categories;
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
