//! Torznab request parsing
//!
//! Parses query parameters into TorznabQuery structs.

use anyhow::{Result, anyhow};
use serde::Deserialize;

use crate::indexer::{QueryType, TorznabQuery};

/// Torznab API request parameters
#[derive(Debug, Deserialize, Default)]
pub struct TorznabRequest {
    /// Query type: search, tvsearch, movie, music, book, caps
    pub t: Option<String>,
    /// Search query
    pub q: Option<String>,
    /// API key
    pub apikey: Option<String>,
    /// Categories (comma-separated)
    pub cat: Option<String>,
    /// Extended attributes
    pub extended: Option<String>,
    /// Result limit
    pub limit: Option<String>,
    /// Result offset
    pub offset: Option<String>,
    /// Whether to use cache
    pub cache: Option<String>,

    // TV-specific
    /// Season number
    pub season: Option<String>,
    /// Episode number
    pub ep: Option<String>,
    /// IMDB ID
    pub imdbid: Option<String>,
    /// TVDB ID
    pub tvdbid: Option<String>,
    /// TVRage ID
    pub rid: Option<String>,
    /// TMDB ID
    pub tmdbid: Option<String>,
    /// TVMaze ID
    pub tvmazeid: Option<String>,
    /// Trakt ID
    pub traktid: Option<String>,
    /// Douban ID
    pub doubanid: Option<String>,

    // Music-specific
    pub album: Option<String>,
    pub artist: Option<String>,
    pub label: Option<String>,
    pub track: Option<String>,

    // Book-specific
    pub title: Option<String>,
    pub author: Option<String>,
    pub publisher: Option<String>,

    // Common
    pub year: Option<String>,
    pub genre: Option<String>,
}

impl TorznabRequest {
    /// Convert to a TorznabQuery
    pub fn to_query(&self) -> Result<TorznabQuery> {
        let query_type = match self.t.as_deref() {
            Some("search") | None => QueryType::Search,
            Some("tvsearch") | Some("tv-search") => QueryType::TvSearch,
            Some("movie") | Some("movie-search") => QueryType::MovieSearch,
            Some("music") | Some("music-search") => QueryType::MusicSearch,
            Some("book") | Some("book-search") => QueryType::BookSearch,
            Some("caps") => QueryType::Caps,
            Some(t) => return Err(anyhow!("Unknown query type: {}", t)),
        };

        // Parse categories
        let categories: Vec<i32> = self
            .cat
            .as_ref()
            .map(|c| c.split(',').filter_map(|s| s.trim().parse().ok()).collect())
            .unwrap_or_default();

        // Parse limit/offset
        let limit = self.limit.as_ref().and_then(|s| s.parse().ok());
        let offset = self.offset.as_ref().and_then(|s| s.parse().ok());

        // Parse cache flag
        let cache = self
            .cache
            .as_ref()
            .map(|s| s.to_lowercase() != "false" && s != "0")
            .unwrap_or(true);

        Ok(TorznabQuery {
            query_type,
            search_term: self.q.clone(),
            categories,
            limit,
            offset,
            cache,

            // TV-specific
            season: self.season.as_ref().and_then(|s| s.parse().ok()),
            episode: self.ep.clone(),
            imdb_id: self.normalize_imdb_id(),
            tvdb_id: self.tvdbid.as_ref().and_then(|s| s.parse().ok()),
            rage_id: self.rid.as_ref().and_then(|s| s.parse().ok()),
            tmdb_id: self.tmdbid.as_ref().and_then(|s| s.parse().ok()),
            tvmaze_id: self.tvmazeid.as_ref().and_then(|s| s.parse().ok()),
            trakt_id: self.traktid.as_ref().and_then(|s| s.parse().ok()),
            douban_id: self.doubanid.as_ref().and_then(|s| s.parse().ok()),

            // Music-specific
            album: self.album.clone(),
            artist: self.artist.clone(),
            label: self.label.clone(),
            track: self.track.clone(),

            // Book-specific
            title: self.title.clone(),
            author: self.author.clone(),
            publisher: self.publisher.clone(),

            // Common
            year: self.year.as_ref().and_then(|s| s.parse().ok()),
            genre: self.genre.clone(),
        })
    }

    /// Normalize IMDB ID to include "tt" prefix
    fn normalize_imdb_id(&self) -> Option<String> {
        self.imdbid.as_ref().map(|id| {
            let id = id.trim();
            if id.starts_with("tt") {
                id.to_string()
            } else {
                format!("tt{}", id.trim_start_matches('0'))
            }
        })
    }
}
