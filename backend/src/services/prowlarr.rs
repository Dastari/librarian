//! Prowlarr/Jackett Torznab client

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Prowlarr API client for Torznab searches
pub struct ProwlarrClient {
    base_url: String,
    api_key: String,
    client: Client,
}

impl ProwlarrClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url,
            api_key,
            client: Client::new(),
        }
    }

    /// Search for releases using Torznab
    pub async fn search(&self, query: &str, categories: &[i32]) -> Result<Vec<TorznabResult>> {
        let cats: String = categories
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let url = format!(
            "{}/api/v1/indexer/all/torznab?apikey={}&t=search&q={}&cat={}",
            self.base_url, self.api_key, query, cats
        );

        let resp = self.client.get(&url).send().await?;
        let text = resp.text().await?;

        // Parse Torznab XML response
        self.parse_torznab_response(&text)
    }

    /// Search for TV episodes
    pub async fn search_tv(
        &self,
        query: &str,
        season: Option<i32>,
        episode: Option<i32>,
    ) -> Result<Vec<TorznabResult>> {
        let mut url = format!(
            "{}/api/v1/indexer/all/torznab?apikey={}&t=tvsearch&q={}",
            self.base_url, self.api_key, query
        );

        if let Some(s) = season {
            url.push_str(&format!("&season={}", s));
        }
        if let Some(e) = episode {
            url.push_str(&format!("&ep={}", e));
        }

        let resp = self.client.get(&url).send().await?;
        let text = resp.text().await?;

        self.parse_torznab_response(&text)
    }

    /// Search for movies
    pub async fn search_movie(&self, query: &str) -> Result<Vec<TorznabResult>> {
        let url = format!(
            "{}/api/v1/indexer/all/torznab?apikey={}&t=movie&q={}",
            self.base_url, self.api_key, query
        );

        let resp = self.client.get(&url).send().await?;
        let text = resp.text().await?;

        self.parse_torznab_response(&text)
    }

    fn parse_torznab_response(&self, _xml: &str) -> Result<Vec<TorznabResult>> {
        // TODO: Implement XML parsing for Torznab response
        // The response is RSS 2.0 with Torznab extensions
        Ok(vec![])
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TorznabResult {
    pub title: String,
    pub link: String,
    pub size: u64,
    pub seeders: i32,
    pub leechers: i32,
    pub indexer: String,
    pub category: String,
    pub pub_date: String,
}
