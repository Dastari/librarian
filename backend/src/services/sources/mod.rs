//! Sources Module
//!
//! This module provides a unified interface for content acquisition sources:
//! torrent indexers (e.g., IPTorrents), usenet indexers (Newznab), and RSS feeds.
//!
//! # Architecture
//!
//! - `Source` trait: Core abstraction for all source implementations
//! - `SourcesManager`: Registry and orchestration for configured sources
//! - `types`: Query and release types (Torznab-compatible)
//! - `definitions`: Source definitions and implementations
//! - `encryption`: AES-256-GCM credential encryption
//! - `categories`: Torznab category definitions and mappings
//!
//! # Example
//!
//! ```ignore
//! use crate::services::sources::{SourcesManager, SourceQuery};
//!
//! let manager = SourcesManager::new(encryption_key).await?;
//! let query = SourceQuery::tv_search("Breaking Bad");
//! let results = manager.search_all(&query).await;
//! ```

pub mod categories;
pub mod definitions;
pub mod encryption;
pub mod manager;
pub mod service;
pub mod types;

pub use types::{
    BookSearchParam, MovieSearchParam, MusicSearchParam, QueryType, SourceCapabilities,
    SourceQuery, SourceRelease, TvSearchParam,
};

use anyhow::Result;
use async_graphql::async_trait::async_trait;

/// The type of source
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SourceType {
    /// Torrent indexer (e.g., IPTorrents, public trackers)
    TorrentIndexer,
    /// Usenet indexer (Newznab-compatible)
    UsenetIndexer,
    /// RSS/Atom feed
    RssFeed,
}

impl Default for SourceType {
    fn default() -> Self {
        Self::TorrentIndexer
    }
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::TorrentIndexer => write!(f, "TorrentIndexer"),
            SourceType::UsenetIndexer => write!(f, "UsenetIndexer"),
            SourceType::RssFeed => write!(f, "RssFeed"),
        }
    }
}

impl std::str::FromStr for SourceType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TorrentIndexer" => Ok(SourceType::TorrentIndexer),
            "UsenetIndexer" => Ok(SourceType::UsenetIndexer),
            "RssFeed" => Ok(SourceType::RssFeed),
            _ => Err(anyhow::anyhow!("Unknown source type: {}", s)),
        }
    }
}

/// Type of tracker (affects how releases are handled)
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TrackerType {
    /// Private tracker - requires account, don't share magnets
    Private,
    /// Public tracker - no account needed
    Public,
    /// Semi-private - may require registration but is open
    SemiPrivate,
}

impl Default for TrackerType {
    fn default() -> Self {
        Self::Private
    }
}

impl std::fmt::Display for TrackerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackerType::Private => write!(f, "Private"),
            TrackerType::Public => write!(f, "Public"),
            TrackerType::SemiPrivate => write!(f, "SemiPrivate"),
        }
    }
}

/// Core trait for all source implementations
///
/// This trait defines the interface that all sources must implement,
/// whether they are native Rust implementations or external APIs.
#[async_trait]
pub trait Source: Send + Sync {
    /// Unique identifier for this source instance (database ID)
    fn id(&self) -> &str;

    /// Display name for the source
    fn name(&self) -> &str;

    /// The type of source
    fn source_type(&self) -> SourceType;

    /// The definition ID (e.g., "iptorrents", "newznab")
    fn definition_id(&self) -> &str;

    /// The site URL
    fn site_link(&self) -> &str;

    /// Whether this is a private, public, or semi-private tracker
    fn tracker_type(&self) -> TrackerType;

    /// Get the capabilities of this source
    fn capabilities(&self) -> &SourceCapabilities;

    /// Whether this source is currently configured and ready to use
    fn is_configured(&self) -> bool;

    /// Check if this source can handle the given query
    fn can_handle_query(&self, query: &SourceQuery) -> bool {
        let caps = self.capabilities();

        match query.query_type {
            QueryType::Search => caps.search_available,
            QueryType::TvSearch => caps.tv_search_available(),
            QueryType::MovieSearch => caps.movie_search_available(),
            QueryType::MusicSearch => caps.music_search_available(),
            QueryType::BookSearch => caps.book_search_available(),
        }
    }

    /// Test the connection/authentication
    async fn test_connection(&self) -> Result<bool>;

    /// Perform a search query
    async fn search(&self, query: &SourceQuery) -> Result<Vec<SourceRelease>>;

    /// Download a file by its link (torrent file, NZB, etc.)
    async fn download(&self, link: &str) -> Result<Vec<u8>>;
}

/// Result from a source search operation
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SourceSearchResult {
    /// The source that produced these results
    pub source_id: String,
    pub source_name: String,
    /// The releases found
    pub releases: Vec<SourceRelease>,
    /// Time taken to search (milliseconds)
    pub elapsed_ms: u64,
    /// Whether results came from cache
    pub from_cache: bool,
    /// Any error that occurred (partial results may still be returned)
    pub error: Option<String>,
}
