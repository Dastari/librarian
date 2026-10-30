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
    MovieSearchParam, QueryType, Source, SourceCapabilities, SourceContinuationSupport, SourcePage,
    SourceQuery, SourceRelease, SourceSearchPageState, SourceType, TrackerType,
};

#[derive(Debug, Deserialize)]
struct YtsResponse {
    status: Option<String>,
    data: YtsData,
}

#[derive(Debug, Deserialize)]
struct YtsData {
    movie_count: Option<i32>,
    limit: Option<i32>,
    page_number: Option<i32>,
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
            client: crate::services::http_client::outbound_client_builder(
                crate::services::http_client::OutboundHttpProfile::Indexer,
            )
            .gzip(true)
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()?,
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

impl YtsSource {
    fn page_position(query: &SourceQuery) -> Result<(i32, i32)> {
        let limit = query.limit.unwrap_or(50).clamp(1, 50);
        let offset = query.offset.unwrap_or(0);
        if offset < 0 || offset % limit != 0 {
            anyhow::bail!("Invalid YTS movie page offset");
        }
        let page = (offset / limit)
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("YTS page offset overflow"))?;
        Ok((limit, page))
    }

    fn decode_page(&self, response: YtsResponse, query: &SourceQuery) -> SourcePage {
        let movies = response.data.movies.unwrap_or_default();
        let movie_rows = movies.len() as i32;
        let (limit, page_number) = Self::page_position(query).expect("validated YTS position");
        let offset = query.offset.unwrap_or(0);
        let verified = response.data.limit == Some(limit)
            && response.data.page_number == Some(page_number)
            && movie_rows <= limit;
        let total = response.data.movie_count.filter(|total| *total >= 0);
        let (support, state, next_offset) = if !verified {
            (
                SourceContinuationSupport::Unknown,
                SourceSearchPageState::Unknown,
                None,
            )
        } else if let Some(total) = total {
            if offset <= total && i64::from(offset) + i64::from(movie_rows) == i64::from(total) {
                (
                    SourceContinuationSupport::Supported,
                    SourceSearchPageState::Exhausted,
                    None,
                )
            } else if movie_rows == limit
                && i64::from(offset) + i64::from(movie_rows) < i64::from(total)
            {
                (
                    SourceContinuationSupport::Supported,
                    SourceSearchPageState::More,
                    offset.checked_add(limit),
                )
            } else {
                (
                    SourceContinuationSupport::Supported,
                    SourceSearchPageState::Partial,
                    None,
                )
            }
        } else {
            (
                SourceContinuationSupport::Supported,
                SourceSearchPageState::Unknown,
                if movie_rows == limit {
                    offset.checked_add(limit)
                } else {
                    None
                },
            )
        };
        let mut releases = Vec::new();
        for movie in movies {
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

        // A movie can expose many quality variants. Do not truncate the
        // flattened releases or advance past undisplayed variants.
        releases.sort_by_key(|release| std::cmp::Reverse(release.seeders.unwrap_or_default()));
        SourcePage {
            releases,
            support,
            state,
            next_offset,
        }
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
        let mut releases = self.search_page(query).await?.releases;
        if let Some(limit) = query.limit.filter(|limit| *limit > 0) {
            releases.truncate(limit as usize);
        }
        Ok(releases)
    }

    async fn search_page(&self, query: &SourceQuery) -> Result<SourcePage> {
        if query.query_type != QueryType::Search && query.query_type != QueryType::MovieSearch {
            return Ok(SourcePage::unsupported(vec![]));
        }
        let term = query
            .imdb_id
            .as_deref()
            .filter(|term| !term.trim().is_empty())
            .or(query.search_term.as_deref())
            .unwrap_or("")
            .trim();
        if term.is_empty() {
            return Ok(SourcePage::unsupported(vec![]));
        }
        let (limit, page) = Self::page_position(query)?;
        self.rate_limiter.wait_for_permit().await;
        let response = self
            .client
            .get(format!(
                "{}/api/v2/list_movies.json",
                self.site_link.trim_end_matches('/')
            ))
            .query(&[
                ("query_term", term.to_string()),
                ("limit", limit.to_string()),
                ("page", page.to_string()),
                ("sort_by", "seeds".into()),
                ("order_by", "desc".into()),
            ])
            .send()
            .await?
            .error_for_status()?;
        let response: YtsResponse = crate::services::http_client::response_json_limited(
            response,
            crate::services::http_client::INDEXER_RESPONSE_LIMIT,
        )
        .await?;
        if response
            .status
            .as_deref()
            .is_some_and(|status| status != "ok")
        {
            anyhow::bail!("YTS rejected this search");
        }
        Ok(self.decode_page(response, query))
    }

    async fn download(&self, link: &str) -> Result<Vec<u8>> {
        if link.starts_with("magnet:") {
            anyhow::bail!("Magnet links are not directly downloadable");
        }
        self.rate_limiter.wait_for_permit().await;
        let response = self.client.get(link).send().await?;
        let bytes = crate::services::http_client::response_bytes_limited(
            response,
            crate::services::http_client::DOWNLOAD_RESPONSE_LIMIT,
        )
        .await?;
        Ok(bytes.to_vec())
    }
}

#[cfg(test)]
mod paging_tests {
    use super::*;
    use serde_json::json;
    fn source() -> YtsSource {
        YtsSource::new(
            "yts-fixture".into(),
            "Fixture".into(),
            Some("https://fixture.invalid/".into()),
            HashMap::new(),
        )
        .unwrap()
    }
    fn movie() -> serde_json::Value {
        json!({"title":"Fixture","year":2026,"imdb_code":"tt1","torrents":[
            {"hash":"a","quality":"720p","seeds":1},{"hash":"b","quality":"1080p","seeds":2}
        ]})
    }
    #[test]
    fn movie_paging_caps_remote_limit_but_preserves_all_release_variants() {
        let source = source();
        let query = SourceQuery {
            limit: Some(1),
            offset: Some(0),
            ..Default::default()
        };
        let data: YtsResponse = serde_json::from_value(json!({"status":"ok","data":{"movie_count":2,"limit":1,"page_number":1,"movies":[movie()]}})).unwrap();
        let page = source.decode_page(data, &query);
        assert_eq!(
            page.releases.len(),
            2,
            "one movie's encodes must not be truncated"
        );
        assert_eq!(page.state, SourceSearchPageState::More);
        assert_eq!(page.next_offset, Some(1));
        assert_ne!(
            page.releases[0].stable_id("yts"),
            page.releases[1].stable_id("yts")
        );
        let second_query = SourceQuery {
            offset: Some(1),
            ..query.clone()
        };
        assert_eq!(YtsSource::page_position(&second_query).unwrap(), (1, 2));
        let data: YtsResponse = serde_json::from_value(json!({"status":"ok","data":{"movie_count":2,"limit":1,"page_number":2,"movies":[movie()]}})).unwrap();
        assert_eq!(
            source.decode_page(data, &second_query).state,
            SourceSearchPageState::Exhausted
        );
        assert_eq!(
            YtsSource::page_position(&SourceQuery {
                limit: Some(100),
                offset: Some(50),
                ..Default::default()
            })
            .unwrap(),
            (50, 2)
        );
        assert!(
            YtsSource::page_position(&SourceQuery {
                limit: Some(50),
                offset: Some(1),
                ..Default::default()
            })
            .is_err()
        );
        assert!(
            YtsSource::page_position(&SourceQuery {
                limit: Some(1),
                offset: Some(i32::MAX),
                ..Default::default()
            })
            .is_err()
        );
    }
    #[test]
    fn missing_metadata_wrong_echo_and_inconsistent_short_pages_do_not_prove_exhaustion() {
        let source = source();
        let query = SourceQuery {
            limit: Some(2),
            offset: Some(0),
            ..Default::default()
        };
        for data in [
            json!({"movies":[movie()]}),
            json!({"movie_count":1,"limit":2,"page_number":2,"movies":[movie()]}),
            json!({"movie_count":10,"limit":2,"page_number":1,"movies":[movie()]}),
        ] {
            let response: YtsResponse = serde_json::from_value(json!({"data":data})).unwrap();
            let page = source.decode_page(response, &query);
            assert_ne!(page.state, SourceSearchPageState::Exhausted);
            assert!(page.next_offset.is_none());
            assert_eq!(page.releases.len(), 2);
        }
        let response =
            serde_json::from_value(json!({"data":{"movie_count":0,"limit":2,"page_number":1}}))
                .unwrap();
        assert_eq!(
            source.decode_page(response, &query).state,
            SourceSearchPageState::Exhausted
        );
    }
}
