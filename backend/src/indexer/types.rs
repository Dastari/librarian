//! Core types for the indexer system
//!
//! These types are modeled after the Torznab specification and Jackett's implementation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::categories::CategoryMapping;

/// The type of indexer implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexerType {
    /// Native Rust implementation (e.g., IPTorrents)
    Native,
    /// Cardigann YAML-based definition
    Cardigann,
    /// RSS/Atom feed indexer
    Feed,
    /// Newznab-compatible indexer
    Newznab,
}

impl Default for IndexerType {
    fn default() -> Self {
        Self::Native
    }
}

impl std::fmt::Display for IndexerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexerType::Native => write!(f, "native"),
            IndexerType::Cardigann => write!(f, "cardigann"),
            IndexerType::Feed => write!(f, "feed"),
            IndexerType::Newznab => write!(f, "newznab"),
        }
    }
}

/// Type of search query
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
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
    /// Capabilities request
    Caps,
}

impl std::fmt::Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryType::Search => write!(f, "search"),
            QueryType::TvSearch => write!(f, "tvsearch"),
            QueryType::MovieSearch => write!(f, "movie"),
            QueryType::MusicSearch => write!(f, "music"),
            QueryType::BookSearch => write!(f, "book"),
            QueryType::Caps => write!(f, "caps"),
        }
    }
}

impl std::str::FromStr for QueryType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "search" | "q" => Ok(QueryType::Search),
            "tvsearch" | "tv-search" | "tv" => Ok(QueryType::TvSearch),
            "movie" | "movie-search" | "moviesearch" => Ok(QueryType::MovieSearch),
            "music" | "music-search" | "musicsearch" | "audio" => Ok(QueryType::MusicSearch),
            "book" | "book-search" | "booksearch" => Ok(QueryType::BookSearch),
            "caps" | "capabilities" => Ok(QueryType::Caps),
            _ => Err(anyhow::anyhow!("Unknown query type: {}", s)),
        }
    }
}

/// TV search parameters supported by an indexer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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

/// Movie search parameters supported by an indexer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MovieSearchParam {
    Q,
    ImdbId,
    TmdbId,
    TraktId,
    DoubanId,
    Year,
    Genre,
}

/// Music search parameters supported by an indexer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MusicSearchParam {
    Q,
    Album,
    Artist,
    Label,
    Track,
    Year,
    Genre,
}

/// Book search parameters supported by an indexer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BookSearchParam {
    Q,
    Title,
    Author,
    Publisher,
    Year,
    Genre,
}

/// Capabilities of a torrent indexer (Torznab-compatible)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TorznabCapabilities {
    /// Maximum results per page
    pub limits_max: Option<i32>,
    /// Default results per page
    pub limits_default: Option<i32>,

    /// Whether basic search is available
    pub search_available: bool,
    /// Whether raw search is supported
    pub supports_raw_search: bool,

    /// TV search parameters supported
    pub tv_search_params: Vec<TvSearchParam>,
    /// Whether IMDB search is available for TV
    pub tv_search_imdb_available: bool,

    /// Movie search parameters supported
    pub movie_search_params: Vec<MovieSearchParam>,

    /// Music search parameters supported
    pub music_search_params: Vec<MusicSearchParam>,

    /// Book search parameters supported
    pub book_search_params: Vec<BookSearchParam>,

    /// Category mappings (tracker category -> Torznab category)
    pub categories: Vec<CategoryMapping>,
}

impl TorznabCapabilities {
    /// Create default capabilities (search only)
    pub fn new() -> Self {
        Self {
            search_available: true,
            limits_default: Some(100),
            limits_max: Some(100),
            ..Default::default()
        }
    }

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

    /// Check if a specific TV search param is supported
    pub fn has_tv_param(&self, param: TvSearchParam) -> bool {
        self.tv_search_params.contains(&param)
    }

    /// Check if a specific movie search param is supported
    pub fn has_movie_param(&self, param: MovieSearchParam) -> bool {
        self.movie_search_params.contains(&param)
    }

    /// Add a category mapping
    pub fn add_category(&mut self, tracker_id: i32, torznab_cat: i32, desc: &str) {
        self.categories.push(CategoryMapping {
            tracker_id: tracker_id.to_string(),
            torznab_cat,
            description: Some(desc.to_string()),
        });
    }

    /// Map tracker categories to Torznab categories
    pub fn map_tracker_to_torznab(&self, tracker_id: &str) -> Vec<i32> {
        self.categories
            .iter()
            .filter(|c| c.tracker_id == tracker_id)
            .map(|c| c.torznab_cat)
            .collect()
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
}

/// A search query in Torznab format
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TorznabQuery {
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
    /// Season number
    pub season: Option<i32>,
    /// Episode number/identifier
    pub episode: Option<String>,
    /// IMDB ID (e.g., "tt1234567")
    pub imdb_id: Option<String>,
    /// TVDB ID
    pub tvdb_id: Option<i32>,
    /// TVRage ID
    pub rage_id: Option<i32>,
    /// TMDB ID
    pub tmdb_id: Option<i32>,
    /// TVMaze ID
    pub tvmaze_id: Option<i32>,
    /// Trakt ID
    pub trakt_id: Option<i32>,
    /// Douban ID
    pub douban_id: Option<i32>,

    // Music-specific fields
    /// Album name
    pub album: Option<String>,
    /// Artist name
    pub artist: Option<String>,
    /// Record label
    pub label: Option<String>,
    /// Track name
    pub track: Option<String>,

    // Book-specific fields
    /// Book title
    pub title: Option<String>,
    /// Author name
    pub author: Option<String>,
    /// Publisher name
    pub publisher: Option<String>,

    // Common fields
    /// Release year
    pub year: Option<i32>,
    /// Genre
    pub genre: Option<String>,
}

impl TorznabQuery {
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

    /// Add season/episode to a TV search
    pub fn with_season_episode(mut self, season: i32, episode: Option<&str>) -> Self {
        self.season = Some(season);
        self.episode = episode.map(|s| s.to_string());
        self
    }

    /// Add IMDB ID to the query
    pub fn with_imdb(mut self, imdb_id: &str) -> Self {
        self.imdb_id = Some(imdb_id.to_string());
        self
    }

    /// Add categories to the query
    pub fn with_categories(mut self, cats: Vec<i32>) -> Self {
        self.categories = cats;
        self
    }

    /// Get the query string for display/logging
    pub fn get_query_string(&self) -> String {
        let mut parts = vec![];

        if let Some(ref term) = self.search_term {
            parts.push(term.clone());
        }

        if let Some(season) = self.season {
            if let Some(ref ep) = self.episode {
                parts.push(format!("S{:02}E{}", season, ep));
            } else {
                parts.push(format!("S{:02}", season));
            }
        }

        parts.join(" ")
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
            || self.rage_id.is_some()
            || self.tvmaze_id.is_some()
            || self.trakt_id.is_some()
            || self.douban_id.is_some()
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

/// Information about a torrent release (Torznab-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    /// Release title
    pub title: String,

    /// Unique identifier (usually the details URL)
    pub guid: String,

    /// Download link (torrent file)
    pub link: Option<String>,

    /// Magnet URI
    pub magnet_uri: Option<String>,

    /// InfoHash
    pub info_hash: Option<String>,

    /// Details page URL
    pub details: Option<String>,

    /// Publication date
    pub publish_date: DateTime<Utc>,

    /// Torznab category IDs
    pub categories: Vec<i32>,

    /// File size in bytes
    pub size: Option<i64>,

    /// Number of files in the torrent
    pub files: Option<i32>,

    /// Number of times snatched/downloaded
    pub grabs: Option<i32>,

    /// Description
    pub description: Option<String>,

    // Metadata IDs
    /// TVRage ID
    pub rage_id: Option<i64>,
    /// TVDB ID
    pub tvdb_id: Option<i64>,
    /// IMDB ID (numeric part)
    pub imdb: Option<i64>,
    /// TMDB ID
    pub tmdb: Option<i64>,
    /// TVMaze ID
    pub tvmaze_id: Option<i64>,
    /// Trakt ID
    pub trakt_id: Option<i64>,
    /// Douban ID
    pub douban_id: Option<i64>,

    /// Genres
    pub genres: Vec<String>,
    /// Languages
    pub languages: Vec<String>,
    /// Subtitles
    pub subs: Vec<String>,

    /// Release year
    pub year: Option<i32>,

    // Book/Music fields
    pub author: Option<String>,
    pub book_title: Option<String>,
    pub publisher: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub label: Option<String>,
    pub track: Option<String>,

    // Peer info
    /// Number of seeders
    pub seeders: Option<i32>,
    /// Number of peers (seeders + leechers)
    pub peers: Option<i32>,

    /// Poster/cover image URL
    pub poster: Option<String>,

    // Download factors
    /// Download volume factor (0 = freeleech, 1 = normal)
    pub download_volume_factor: f64,
    /// Upload volume factor (usually 1, can be 2 for double upload)
    pub upload_volume_factor: f64,

    /// Minimum ratio required
    pub minimum_ratio: Option<f64>,
    /// Minimum seed time in seconds
    pub minimum_seed_time: Option<i64>,

    /// The indexer that found this release
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexer_name: Option<String>,
}

impl ReleaseInfo {
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
            rage_id: None,
            tvdb_id: None,
            imdb: None,
            tmdb: None,
            tvmaze_id: None,
            trakt_id: None,
            douban_id: None,
            genres: vec![],
            languages: vec![],
            subs: vec![],
            year: None,
            author: None,
            book_title: None,
            publisher: None,
            artist: None,
            album: None,
            label: None,
            track: None,
            seeders: None,
            peers: None,
            poster: None,
            download_volume_factor: 1.0,
            upload_volume_factor: 1.0,
            minimum_ratio: None,
            minimum_seed_time: None,
            indexer_id: None,
            indexer_name: None,
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

    /// Calculate a "gain" score (seeders * size in GB)
    pub fn gain(&self) -> Option<f64> {
        match (self.seeders, self.size) {
            (Some(seeders), Some(size)) => {
                let gb = size as f64 / (1024.0 * 1024.0 * 1024.0);
                Some(seeders as f64 * gb)
            }
            _ => None,
        }
    }
}

impl Default for ReleaseInfo {
    fn default() -> Self {
        Self::new(String::new(), String::new(), Utc::now())
    }
}
