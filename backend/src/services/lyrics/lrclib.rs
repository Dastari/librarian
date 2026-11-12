//! LRCLIB (<https://lrclib.net>): free synced and plain lyrics, no key.
//!
//! `GET /api/get` takes the artist, title, album and duration and answers with one record or
//! 404; LRCLIB matches the duration within two seconds itself. When it has nothing,
//! `GET /api/search` by artist and title is tried, and a result is only taken when its duration
//! is within two seconds of the track's (or the track's is unknown and the names match). LRCLIB
//! asks clients to name themselves in the User-Agent; the shared client sends `Librarian/<version>`.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::services::http_client::{
    METADATA_RESPONSE_LIMIT, OutboundHttpProfile, outbound_client, response_json_limited,
    send_with_policy,
};
use crate::services::rate_limiter::{RateLimitConfig, RateLimitedClient};

pub const DEFAULT_BASE_URL: &str = "https://lrclib.net";

/// What LRCLIB holds for a track.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LrclibRecord {
    pub id: i64,
    #[serde(default)]
    pub track_name: String,
    #[serde(default)]
    pub artist_name: String,
    #[serde(default)]
    pub album_name: Option<String>,
    #[serde(default)]
    pub duration: Option<f64>,
    #[serde(default)]
    pub instrumental: bool,
    #[serde(default)]
    pub plain_lyrics: Option<String>,
    #[serde(default)]
    pub synced_lyrics: Option<String>,
}

/// The track to look up.
#[derive(Clone, Debug, Default)]
pub struct LyricsQuery {
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
    pub duration_secs: Option<i64>,
}

pub struct LrclibClient {
    base_url: String,
    http: reqwest::Client,
    limiter: Arc<RateLimitedClient>,
}

fn same_name(a: &str, b: &str) -> bool {
    let fold = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    fold(a) == fold(b)
}

/// The search result to take: the closest duration within two seconds, records with lyrics first.
pub fn pick_search_result(query: &LyricsQuery, results: Vec<LrclibRecord>) -> Option<LrclibRecord> {
    results
        .into_iter()
        .filter(|record| {
            record.instrumental || record.plain_lyrics.is_some() || record.synced_lyrics.is_some()
        })
        .filter_map(|record| {
            let distance = match (query.duration_secs, record.duration) {
                (Some(want), Some(have)) => {
                    let distance = (want as f64 - have).abs();
                    if distance > 2.0 {
                        return None;
                    }
                    distance
                }
                // Without a duration only an exact name match is safe.
                _ if same_name(&record.track_name, &query.title)
                    && same_name(&record.artist_name, &query.artist) =>
                {
                    1.5
                }
                _ => return None,
            };
            Some((record.synced_lyrics.is_none(), distance, record))
        })
        .min_by(|a, b| {
            (a.0, a.1)
                .partial_cmp(&(b.0, b.1))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, _, record)| record)
}

impl LrclibClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            http: outbound_client(OutboundHttpProfile::Metadata)?,
            // LRCLIB publishes no limit; stay polite.
            limiter: Arc::new(RateLimitedClient::new(
                "lrclib",
                RateLimitConfig {
                    requests_per_second: 2,
                    burst_size: 4,
                },
            )),
        })
    }

    /// The best record for a track, or `None` when LRCLIB has nothing for it.
    pub async fn find(&self, query: &LyricsQuery) -> Result<Option<LrclibRecord>> {
        if let Some(record) = self.get(query).await? {
            return Ok(Some(record));
        }
        let results = self.search(query).await?;
        Ok(pick_search_result(query, results))
    }

    async fn get(&self, query: &LyricsQuery) -> Result<Option<LrclibRecord>> {
        let mut params = vec![
            ("artist_name", query.artist.clone()),
            ("track_name", query.title.clone()),
        ];
        if let Some(album) = query.album.as_ref().filter(|album| !album.is_empty()) {
            params.push(("album_name", album.clone()));
        }
        match query.duration_secs {
            Some(duration) if duration > 0 => params.push(("duration", duration.to_string())),
            // `/api/get` needs every field; without a duration only search can answer.
            _ => return Ok(None),
        }
        if query.album.as_deref().is_none_or(str::is_empty) {
            return Ok(None);
        }
        self.limiter.wait_for_permit().await;
        let request = self
            .http
            .get(format!("{}/api/get", self.base_url))
            .query(&params)
            .timeout(Duration::from_secs(15));
        let response = send_with_policy(request, "lrclib", "get").await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            anyhow::bail!("LRCLIB answered {}", response.status());
        }
        let record: LrclibRecord = response_json_limited(response, METADATA_RESPONSE_LIMIT)
            .await
            .context("LRCLIB sent an answer that could not be read")?;
        Ok(Some(record))
    }

    async fn search(&self, query: &LyricsQuery) -> Result<Vec<LrclibRecord>> {
        self.limiter.wait_for_permit().await;
        let request = self
            .http
            .get(format!("{}/api/search", self.base_url))
            .query(&[("artist_name", &query.artist), ("track_name", &query.title)])
            .timeout(Duration::from_secs(15));
        let response = send_with_policy(request, "lrclib", "search").await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }
        if !response.status().is_success() {
            anyhow::bail!("LRCLIB answered {}", response.status());
        }
        response_json_limited(response, METADATA_RESPONSE_LIMIT)
            .await
            .context("LRCLIB sent search results that could not be read")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GET_FIXTURE: &str = include_str!("../../../tests/fixtures/lrclib/get.json");
    const SEARCH_FIXTURE: &str = include_str!("../../../tests/fixtures/lrclib/search.json");

    #[test]
    fn get_response_reads() {
        let record: LrclibRecord = serde_json::from_str(GET_FIXTURE).unwrap();
        assert_eq!(record.id, 3396226);
        assert_eq!(record.artist_name, "Borislav Slavov");
        assert!(
            record
                .synced_lyrics
                .as_deref()
                .unwrap()
                .starts_with("[00:17.12]")
        );
        assert!(!record.instrumental);
    }

    #[test]
    fn search_takes_the_synced_record_within_two_seconds() {
        let results: Vec<LrclibRecord> = serde_json::from_str(SEARCH_FIXTURE).unwrap();
        let query = LyricsQuery {
            artist: "Borislav Slavov".into(),
            title: "I Want to Live".into(),
            album: None,
            duration_secs: Some(233),
        };
        let picked = pick_search_result(&query, results.clone()).unwrap();
        assert_eq!(picked.id, 3396226);
        // Far from every duration: nothing.
        let far = LyricsQuery {
            duration_secs: Some(400),
            ..query.clone()
        };
        assert!(pick_search_result(&far, results.clone()).is_none());
        // No duration: only an exact name match.
        let unknown = LyricsQuery {
            duration_secs: None,
            ..query
        };
        assert_eq!(pick_search_result(&unknown, results).unwrap().id, 3396226);
    }

    #[tokio::test]
    async fn client_reads_a_recorded_server() {
        use axum::{Json, Router, extract::Query, routing::get};
        use std::collections::HashMap;
        let app = Router::new()
            .route(
                "/api/get",
                get(|Query(params): Query<HashMap<String, String>>| async move {
                    if params.get("track_name").map(String::as_str) == Some("I Want to Live") {
                        Ok(Json(
                            serde_json::from_str::<serde_json::Value>(GET_FIXTURE).unwrap(),
                        ))
                    } else {
                        Err(StatusCode::NOT_FOUND)
                    }
                }),
            )
            .route(
                "/api/search",
                get(|| async { Json(serde_json::from_str::<serde_json::Value>("[]").unwrap()) }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client = LrclibClient::new(format!("http://{address}")).unwrap();
        let query = LyricsQuery {
            artist: "Borislav Slavov".into(),
            title: "I Want to Live".into(),
            album: Some("Baldur's Gate 3 (Original Game Soundtrack)".into()),
            duration_secs: Some(233),
        };
        assert_eq!(client.find(&query).await.unwrap().unwrap().id, 3396226);
        let missing = LyricsQuery {
            title: "Nothing".into(),
            ..query
        };
        assert!(client.find(&missing).await.unwrap().is_none());
    }
}
