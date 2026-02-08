//! YTS source implementation
//!
//! Uses the public YTS API for movie torrents.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use async_graphql::async_trait::async_trait;
use chrono::{TimeZone, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::services::rate_limiter::RateLimitedClient;
use crate::services::sources::categories::CategoryMapping;
use crate::services::sources::categories::cats;
use crate::services::sources::{
    MovieSearchParam, QueryType, Source, SourceCapabilities, SourceQuery, SourceRelease,
    SourceType, TrackerType,
};

#[derive(Debug, Deserialize)]
struct YtsResponse {
    data: YtsData,
}

#[derive(Debug, Deserialize)]
struct YtsData {
    movies: Option<Vec<YtsMovie>>,
}

#[derive(Debug, Deserialize)]
struct YtsMovie {
    title: String,
    year: Option<i32>,
    imdb_code: Option<String>,
    torrents: Option<Vec<YtsTorrent>>,
}

#[derive(Debug, Deserialize)]
struct YtsTorrent {
    hash: String,
    quality: String,
    #[serde(rename = "type")]
    torrent_type: Option<String>,
    seeds: Option<i32>,
    peers: Option<i32>,
    size_bytes: Option<i64>,
    url: Option<String>,
    date_uploaded_unix: Option<i64>,
}

pub struct YtsSource {
    id: String,
    name: String,
    site_link: String,
    client: Client,
    rate_limiter: RateLimitedClient,
    capabilities: SourceCapabilities,
    _settings: HashMap<String, String>,
}

impl YtsSource {
    pub fn new(
        id: String,
        name: String,
        site_url: Option<String>,
        settings: HashMap<String, String>,
    ) -> Result<Self> {
        Ok(Self {
            id,
            name,
            site_link: site_url.unwrap_or_else(|| "https://yts.mx/".to_string()),
            client: Client::builder().gzip(true).build()?,
            // Jackett definition sets requestDelay: 2.5 seconds.
            rate_limiter: RateLimitedClient::for_indexer_with_request_delay(Duration::from_millis(
                2500,
            )),
            capabilities: SourceCapabilities {
                search_available: true,
                movie_search_params: vec![MovieSearchParam::Q, MovieSearchParam::ImdbId],
                categories: vec![
                    CategoryMapping::new("movies", cats::MOVIES, "Movies"),
                    CategoryMapping::new("movies-hd", cats::MOVIES_HD, "Movies HD"),
                ],
                ..Default::default()
            },
            _settings: settings,
        })
    }
}

#[async_trait]
impl Source for YtsSource {
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
        "yts"
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
            .get("https://yts.mx/api/v2/list_movies.json?limit=1")
            .send()
            .await?;
        Ok(response.status().is_success())
    }

    async fn search(&self, query: &SourceQuery) -> Result<Vec<SourceRelease>> {
        if query.query_type != QueryType::Search && query.query_type != QueryType::MovieSearch {
            return Ok(vec![]);
        }

        let term = query.search_term.as_deref().unwrap_or("").trim();
        if term.is_empty() {
            return Ok(vec![]);
        }

        self.rate_limiter.wait_for_permit().await;
        let response: YtsResponse = self
            .client
            .get("https://yts.mx/api/v2/list_movies.json")
            .query(&[
                ("query_term", term),
                ("limit", "50"),
                ("sort_by", "seeds"),
                ("order_by", "desc"),
            ])
            .send()
            .await?
            .json()
            .await?;

        let mut releases = Vec::new();
        for movie in response.data.movies.unwrap_or_default() {
            let imdb_numeric = movie
                .imdb_code
                .as_deref()
                .map(|s| s.trim_start_matches("tt"))
                .and_then(|s| s.parse::<i64>().ok());

            for torrent in movie.torrents.unwrap_or_default() {
                let title = if let Some(ref t) = torrent.torrent_type {
                    format!("{} {} {}", movie.title, torrent.quality, t)
                } else {
                    format!("{} {}", movie.title, torrent.quality)
                };

                let publish_date = torrent
                    .date_uploaded_unix
                    .and_then(|ts| Utc.timestamp_opt(ts, 0).single())
                    .unwrap_or_else(Utc::now);

                let mut release = SourceRelease::new(
                    title,
                    format!(
                        "{}movie-imdb/{}",
                        self.site_link,
                        movie.imdb_code.clone().unwrap_or_default()
                    ),
                    publish_date,
                );

                release.details = Some(format!(
                    "{}movie-imdb/{}",
                    self.site_link,
                    movie.imdb_code.clone().unwrap_or_default()
                ));
                release.link = torrent.url.clone();
                release.info_hash = Some(torrent.hash.clone());
                release.magnet_uri = Some(format!(
                    "magnet:?xt=urn:btih:{}&dn={}",
                    torrent.hash,
                    urlencoding::encode(&release.title)
                ));
                release.categories = vec![cats::MOVIES_HD];
                release.size = torrent.size_bytes;
                release.seeders = torrent.seeds;
                release.peers = match (torrent.seeds, torrent.peers) {
                    (Some(s), Some(p)) => Some(s + p),
                    _ => None,
                };
                release.imdb = imdb_numeric;
                release.year = movie.year;

                releases.push(release);
            }
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
