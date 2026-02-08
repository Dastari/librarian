// //! Torrent Indexer Module
// //!
// //! This module provides Jackett-like torrent indexer functionality directly in Rust.
// //! It supports native indexer implementations (like IPTorrents) and YAML-based
// //! Cardigann definitions for easy addition of new indexers.
// //!
// //! # Architecture
// //!
// //! - `Indexer` trait: Core abstraction for all indexer implementations
// //! - `IndexerManager`: Registry and orchestration for configured indexers
// //! - `types`: Torznab-compatible query and result types
// //! - `definitions`: Native indexer implementations
// //! - `torznab`: REST API for Torznab compatibility with external apps
// //!
// //! # Example
// //!
// //! ```ignore
// //! use crate::indexer::{IndexerManager, TorznabQuery, QueryType};
// //!
// //! let manager = IndexerManager::new(db, encryption_key).await?;
// //! let query = TorznabQuery {
// //!     query_type: QueryType::TvSearch,
// //!     search_term: Some("Breaking Bad".to_string()),
// //!     season: Some(1),
// //!     episode: Some("01".to_string()),
// //!     ..Default::default()
// //! };
// //! let results = manager.search_all(&query).await?;
// //! ```

// pub mod categories;
// pub mod definitions;
// pub mod encryption;
// pub mod manager;
// pub mod torznab;
// pub mod types;

// // Re-export commonly used types
// pub use types::{
//     BookSearchParam, IndexerType, MovieSearchParam, MusicSearchParam, QueryType, ReleaseInfo,
//     TorznabCapabilities, TorznabQuery, TvSearchParam,
// };

// use anyhow::Result;
// use async_graphql::async_trait::async_trait;

// /// Core trait for all indexer implementations
// ///
// /// This trait defines the interface that all indexers must implement,
// /// whether they are native Rust implementations or Cardigann YAML-based.
// #[async_trait]
// pub trait Indexer: Send + Sync {
//     /// Unique identifier for this indexer instance (database ID)
//     fn id(&self) -> &str;

//     /// Display name for the indexer
//     fn name(&self) -> &str;

//     /// Description of the indexer
//     fn description(&self) -> &str;

//     /// The type of indexer (native, cardigann, etc.)
//     fn indexer_type(&self) -> IndexerType;

//     /// The site URL
//     fn site_link(&self) -> &str;

//     /// Whether this is a private, public, or semi-private tracker
//     fn tracker_type(&self) -> TrackerType;

//     /// Language of the indexer (e.g., "en-US")
//     fn language(&self) -> &str;

//     /// Get the capabilities of this indexer
//     fn capabilities(&self) -> &TorznabCapabilities;

//     /// Whether this indexer is currently configured and ready to use
//     fn is_configured(&self) -> bool;

//     /// Whether pagination is supported
//     fn supports_pagination(&self) -> bool;

//     /// Test the connection/authentication
//     async fn test_connection(&self) -> Result<bool>;

//     /// Perform a search query
//     async fn search(&self, query: &TorznabQuery) -> Result<Vec<ReleaseInfo>>;

//     /// Check if this indexer can handle the given query
//     fn can_handle_query(&self, query: &TorznabQuery) -> bool {
//         let caps = self.capabilities();

//         match query.query_type {
//             QueryType::Search => caps.search_available,
//             QueryType::TvSearch => caps.tv_search_available(),
//             QueryType::MovieSearch => caps.movie_search_available(),
//             QueryType::MusicSearch => caps.music_search_available(),
//             QueryType::BookSearch => caps.book_search_available(),
//             QueryType::Caps => true,
//         }
//     }

//     /// Download a torrent file by its link
//     async fn download(&self, link: &str) -> Result<Vec<u8>>;
// }

// /// Type of tracker (affects how releases are handled)
// #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
// #[serde(rename_all = "lowercase")]
// pub enum TrackerType {
//     /// Private tracker - requires account, don't share magnets
//     Private,
//     /// Public tracker - no account needed
//     Public,
//     /// Semi-private - may require registration but is open
//     SemiPrivate,
// }

// impl Default for TrackerType {
//     fn default() -> Self {
//         Self::Private
//     }
// }

// impl std::fmt::Display for TrackerType {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             TrackerType::Private => write!(f, "private"),
//             TrackerType::Public => write!(f, "public"),
//             TrackerType::SemiPrivate => write!(f, "semi-private"),
//         }
//     }
// }

// impl std::str::FromStr for TrackerType {
//     type Err = anyhow::Error;

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         match s.to_lowercase().as_str() {
//             "private" => Ok(TrackerType::Private),
//             "public" => Ok(TrackerType::Public),
//             "semi-private" | "semiprivate" | "semi" => Ok(TrackerType::SemiPrivate),
//             _ => Err(anyhow::anyhow!("Unknown tracker type: {}", s)),
//         }
//     }
// }

// /// Result from an indexer search operation
// #[derive(Debug, Clone, serde::Serialize)]
// pub struct IndexerSearchResult {
//     /// The indexer that produced these results
//     pub indexer_id: String,
//     pub indexer_name: String,
//     /// The releases found
//     pub releases: Vec<ReleaseInfo>,
//     /// Time taken to search (milliseconds)
//     pub elapsed_ms: u64,
//     /// Whether results came from cache
//     pub from_cache: bool,
//     /// Any error that occurred (partial results may still be returned)
//     pub error: Option<String>,
// }
