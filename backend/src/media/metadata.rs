//! Metadata fetching service for TheTVDB and TMDB

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Metadata fetching service
pub struct MetadataService {
    tvdb_api_key: Option<String>,
    tmdb_api_key: Option<String>,
    client: reqwest::Client,
}

impl MetadataService {
    pub fn new(tvdb_api_key: Option<String>, tmdb_api_key: Option<String>) -> Self {
        Self {
            tvdb_api_key,
            tmdb_api_key,
            client: reqwest::Client::new(),
        }
    }

    /// Search for a TV show on TheTVDB
    pub async fn search_tvdb_show(&self, query: &str) -> Result<Vec<TvdbSearchResult>> {
        let api_key = self
            .tvdb_api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("TheTVDB API key not configured"))?;

        // TODO: Implement actual TheTVDB API call
        let _ = (api_key, query);
        Ok(vec![])
    }

    /// Get TV show details from TheTVDB
    pub async fn get_tvdb_show(&self, tvdb_id: i32) -> Result<Option<TvdbShow>> {
        let _ = tvdb_id;
        // TODO: Implement actual TheTVDB API call
        Ok(None)
    }

    /// Search for a movie on TMDB
    pub async fn search_tmdb_movie(&self, query: &str, year: Option<i32>) -> Result<Vec<TmdbSearchResult>> {
        let api_key = self
            .tmdb_api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("TMDB API key not configured"))?;

        // TODO: Implement actual TMDB API call
        let _ = (api_key, query, year);
        Ok(vec![])
    }

    /// Get movie details from TMDB
    pub async fn get_tmdb_movie(&self, tmdb_id: i32) -> Result<Option<TmdbMovie>> {
        let _ = tmdb_id;
        // TODO: Implement actual TMDB API call
        Ok(None)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TvdbSearchResult {
    pub tvdb_id: i32,
    pub name: String,
    pub year: Option<i32>,
    pub overview: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TvdbShow {
    pub tvdb_id: i32,
    pub name: String,
    pub year: Option<i32>,
    pub overview: Option<String>,
    pub poster: Option<String>,
    pub backdrop: Option<String>,
    pub seasons: Vec<TvdbSeason>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TvdbSeason {
    pub number: i32,
    pub episodes: Vec<TvdbEpisode>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TvdbEpisode {
    pub tvdb_id: i32,
    pub season: i32,
    pub episode: i32,
    pub name: String,
    pub overview: Option<String>,
    pub aired: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TmdbSearchResult {
    pub tmdb_id: i32,
    pub title: String,
    pub year: Option<i32>,
    pub overview: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TmdbMovie {
    pub tmdb_id: i32,
    pub title: String,
    pub year: Option<i32>,
    pub overview: Option<String>,
    pub runtime: Option<i32>,
    pub poster: Option<String>,
    pub backdrop: Option<String>,
}
