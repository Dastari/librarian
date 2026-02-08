//! Core types for the sources system
//!
//! These types are modeled after the Torznab specification and Jackett's implementation,
//! providing a unified interface for torrent indexers, usenet indexers, and RSS feeds.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::categories::CategoryMapping;

/// Type of search query
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum QueryType {
    /// General text search
    #[default]
    Search,
    /// TV show search (supports season/episode)
    TvSearch,
    /// Movie search
    MovieSearch,
    /// Music search
    MusicSearch,
    /// Book search
    BookSearch,
}

impl std::fmt::Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryType::Search => write!(f, "search"),
            QueryType::TvSearch => write!(f, "tvsearch"),
            QueryType::MovieSearch => write!(f, "movie"),
            QueryType::MusicSearch => write!(f, "music"),
            QueryType::BookSearch => write!(f, "book"),
        }
    }
}

/// TV search parameters supported by a source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TvSearchParam {
    Q,
    Season,
    Ep,
    ImdbId,
    TvdbId,
    RId,
    TmdbId,
    TvmazeId,
    TraktId,
    DoubanId,
    Year,
    Genre,
}

/// Movie search parameters supported by a source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum MovieSearchParam {
    Q,
    ImdbId,
    TmdbId,
    TraktId,
    DoubanId,
    Year,
    Genre,
}

/// Music search parameters supported by a source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum MusicSearchParam {
    Q,
    Album,
    Artist,
    Label,
    Track,
    Year,
    Genre,
}

/// Book search parameters supported by a source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum BookSearchParam {
    Q,
    Title,
    Author,
    Publisher,
    Year,
    Genre,
}

/// Capabilities of a source (Torznab-compatible)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SourceCapabilities {
    /// Whether basic search is available
    pub search_available: bool,

    /// TV search parameters supported
    pub tv_search_params: Vec<TvSearchParam>,

    /// Movie search parameters supported
    pub movie_search_params: Vec<MovieSearchParam>,

    /// Music search parameters supported
    pub music_search_params: Vec<MusicSearchParam>,

    /// Book search parameters supported
    pub book_search_params: Vec<BookSearchParam>,

    /// Category mappings (tracker category -> Torznab category)
    pub categories: Vec<CategoryMapping>,
}

impl SourceCapabilities {
    /// Whether TV search is available
    pub fn tv_search_available(&self) -> bool {
        !self.tv_search_params.is_empty()
    }

    /// Whether movie search is available
    pub fn movie_search_available(&self) -> bool {
        !self.movie_search_params.is_empty()
    }

    /// Whether music search is available
    pub fn music_search_available(&self) -> bool {
        !self.music_search_params.is_empty()
    }

    /// Whether book search is available
    pub fn book_search_available(&self) -> bool {
        !self.book_search_params.is_empty()
    }

    /// Map Torznab categories to tracker categories
    pub fn map_torznab_to_tracker(&self, cats: &[i32]) -> Vec<String> {
        if cats.is_empty() {
            return vec![];
        }

        self.categories
            .iter()
            .filter(|c| {
                cats.contains(&c.torznab_cat)
                    || cats.iter().any(|cat| c.torznab_cat / 1000 == cat / 1000)
            })
            .map(|c| c.tracker_id.clone())
            .collect()
    }

    /// Map tracker categories to Torznab categories
    pub fn map_tracker_to_torznab(&self, tracker_id: &str) -> Vec<i32> {
        self.categories
            .iter()
            .filter(|c| c.tracker_id == tracker_id)
            .map(|c| c.torznab_cat)
            .collect()
    }
}

/// A search query for sources
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SourceQuery {
    /// The type of search
    pub query_type: QueryType,

    /// Free-text search term
    pub search_term: Option<String>,

    /// Categories to search in (Torznab category IDs)
    pub categories: Vec<i32>,

    /// Maximum number of results
    pub limit: Option<i32>,

    /// Offset for pagination
    pub offset: Option<i32>,

    /// Whether to use caching
    pub cache: bool,

    // TV-specific fields
    pub season: Option<i32>,
    pub episode: Option<String>,
    pub imdb_id: Option<String>,
    pub tvdb_id: Option<i32>,
    pub tmdb_id: Option<i32>,
    pub tvmaze_id: Option<i32>,

    // Music-specific fields
    pub album: Option<String>,
    pub artist: Option<String>,

    // Book-specific fields
    pub title: Option<String>,
    pub author: Option<String>,

    // Common fields
    pub year: Option<i32>,
    pub genre: Option<String>,
}

impl SourceQuery {
    /// Create a new search query
    pub fn search(term: &str) -> Self {
        Self {
            query_type: QueryType::Search,
            search_term: Some(term.to_string()),
            cache: true,
            ..Default::default()
        }
    }

    /// Create a TV search query
    pub fn tv_search(term: &str) -> Self {
        Self {
            query_type: QueryType::TvSearch,
            search_term: Some(term.to_string()),
            cache: true,
            ..Default::default()
        }
    }

    /// Create a movie search query
    pub fn movie_search(term: &str) -> Self {
        Self {
            query_type: QueryType::MovieSearch,
            search_term: Some(term.to_string()),
            cache: true,
            ..Default::default()
        }
    }

    /// Create a music search query
    pub fn music_search(term: &str) -> Self {
        Self {
            query_type: QueryType::MusicSearch,
            search_term: Some(term.to_string()),
            cache: true,
            ..Default::default()
        }
    }

    /// Get the episode search string (e.g., "S01E05")
    pub fn get_episode_string(&self) -> Option<String> {
        self.season.map(|s| {
            if let Some(ref ep) = self.episode {
                format!("S{:02}E{}", s, ep)
            } else {
                format!("S{:02}", s)
            }
        })
    }

    /// Check if this is an ID-based search (IMDB, TVDB, etc.)
    pub fn is_id_search(&self) -> bool {
        self.imdb_id.is_some()
            || self.tvdb_id.is_some()
            || self.tmdb_id.is_some()
            || self.tvmaze_id.is_some()
    }

    /// Get IMDB ID without the "tt" prefix
    pub fn imdb_id_short(&self) -> Option<String> {
        self.imdb_id
            .as_ref()
            .map(|id| id.trim_start_matches("tt").to_string())
    }

    /// Create a cache key hash for this query
    pub fn cache_key(&self) -> String {
        use sha2::{Digest, Sha256};
        let json = serde_json::to_string(self).unwrap_or_default();
        let hash = Sha256::digest(json.as_bytes());
        format!("{:x}", hash)
    }
}

/// Information about a release from a source (unified shape)
///
/// This struct is the single return type for all source types (torrent indexers,
/// usenet indexers, RSS feeds). Not all fields are relevant for every source type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SourceRelease {
    /// Release title
    pub title: String,

    /// Unique identifier (usually the details URL)
    pub guid: String,

    /// Download link (torrent file or NZB)
    pub link: Option<String>,

    /// Magnet URI (torrent only)
    pub magnet_uri: Option<String>,

    /// InfoHash (torrent only)
    pub info_hash: Option<String>,

    /// Details page URL
    pub details: Option<String>,

    /// Publication date
    pub publish_date: DateTime<Utc>,

    /// Torznab category IDs
    pub categories: Vec<i32>,

    /// File size in bytes
    pub size: Option<i64>,

    /// Number of files
    pub files: Option<i32>,

    /// Number of times snatched/downloaded
    pub grabs: Option<i32>,

    /// Description
    pub description: Option<String>,

    // Metadata IDs
    pub imdb: Option<i64>,
    pub tmdb: Option<i64>,
    pub tvdb_id: Option<i64>,
    pub tvmaze_id: Option<i64>,

    /// Release year
    pub year: Option<i32>,

    // Peer info (torrent only)
    pub seeders: Option<i32>,
    pub peers: Option<i32>,

    /// Poster/cover image URL
    pub poster: Option<String>,

    /// Download volume factor (0 = freeleech, 1 = normal)
    pub download_volume_factor: f64,
    /// Upload volume factor (usually 1, can be 2 for double upload)
    pub upload_volume_factor: f64,

    /// Minimum ratio required
    pub minimum_ratio: Option<f64>,
    /// Minimum seed time in seconds
    pub minimum_seed_time: Option<i64>,

    /// The source that found this release
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
}

impl SourceRelease {
    /// Create a new release with minimal info
    pub fn new(title: String, guid: String, publish_date: DateTime<Utc>) -> Self {
        Self {
            title,
            guid,
            publish_date,
            link: None,
            magnet_uri: None,
            info_hash: None,
            details: None,
            categories: vec![],
            size: None,
            files: None,
            grabs: None,
            description: None,
            imdb: None,
            tmdb: None,
            tvdb_id: None,
            tvmaze_id: None,
            year: None,
            seeders: None,
            peers: None,
            poster: None,
            download_volume_factor: 1.0,
            upload_volume_factor: 1.0,
            minimum_ratio: None,
            minimum_seed_time: None,
            source_id: None,
            source_name: None,
        }
    }

    /// Check if this is a freeleech release
    pub fn is_freeleech(&self) -> bool {
        self.download_volume_factor == 0.0
    }

    /// Get the number of leechers
    pub fn leechers(&self) -> Option<i32> {
        match (self.peers, self.seeders) {
            (Some(peers), Some(seeders)) => Some(peers - seeders),
            _ => None,
        }
    }
}

impl Default for SourceRelease {
    fn default() -> Self {
        Self::new(String::new(), String::new(), Utc::now())
    }
}
