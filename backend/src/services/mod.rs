//! Service integrations and utilities
//!
//! ## Services (lifecycle: start/stop, optional routes)
//!
//! These are long-running or background components that register with
//! [ServicesManager](manager::ServicesManager) and can start/stop:
//!
//! - **torrent** – librqbit session, progress monitor + DB sync loops
//! - **cast** – mDNS device discovery loop
//! - **logging** – DB writer task + broadcast (layer created at startup)
//! - **scanner** – invoked by job scheduler (no own loop)
//! - **notifications** – in-memory broadcast only (lightweight, no loop)
//! - **artwork** – cache + optional HTTP routes for serving images
//!
//! ## Utilities (no lifecycle; use where needed)
//!
//! Stateless or on-demand helpers, not registered as services:
//!
//! - **metadata** (tmdb, tvmaze, musicbrainz) – API clients
//! - **ffmpeg** – ffprobe runner for media analysis
//! - **filesystem** – path validation + operations; can expose routes
//! - **ollama** – LLM client for filename parsing
//! - **auth** – JWT and user validation
//! - **filename_parser**, **file_matcher**, **file_processor**, **organizer**
//! - **queues** (MediaAnalysisQueue, etc.) – worker pools, often owned by a service
//! - **torrent_completion_handler** – event-driven, started alongside torrent service
//!
//! Route wiring (e.g. `/api/artwork`, `/api/filesystem`) is done in `main` when
//! building the app router; services that provide routes set [Service::provides_routes].

#![allow(unused_imports)]

pub mod artwork;
pub mod auth;
pub mod backup;
pub mod bootstrap_defaults;
pub mod cast;
pub mod database;
pub mod extract;
pub mod graphql;
pub mod http_client;
pub mod http_server;
pub mod library_scan;
pub mod logging;
pub mod login_rate_limit;
pub mod manager;
pub mod metadata;
pub mod ollama;
pub mod quality;
pub mod rate_limiter;
pub mod sources;
pub mod storage;
pub mod torrent;
pub mod transcode;

pub use rate_limiter::{RateLimitConfig, RateLimitedClient, RetryConfig, retry_async};

pub use artwork::ArtworkService;
pub use auth::{
    AuthConfig, AuthService, AuthTokens, AuthenticatedUser, LoginResult, RegisterInput,
};
pub use cast::{CastDeviceType, CastService, CastServiceConfig, DiscoveredCastDevice};

pub use backup::{
    BackupCapabilities, BackupService, BackupServiceConfig, BackupSnapshotSummary,
    LibrarianBackupObjectIndex,
};
pub use database::{DatabaseService, DatabaseServiceConfig};
pub use graphql::{GraphqlService, GraphqlServiceConfig};
pub use http_server::{HttpServerConfig, HttpServerService};
pub use library_scan::{
    LibraryScanService, LibraryScanServiceConfig, MatchMethod, MatchRequest, MatchResult,
    OrganizeResult,
};
pub use logging::{LogEvent, LoggingService, LoggingServiceConfig};
pub use manager::{
    HealthStatus, IntoServiceRegistration, Service, ServiceHealth, ServicesManager,
    ServicesManagerBuilder,
};
pub use ollama::{OllamaClient, OllamaParsedHint, OllamaParserSettings};
pub use sources::service::{SourcesService, SourcesServiceConfig};
pub use storage::{ObjectStorageService, ObjectStorageServiceConfig};
pub use torrent::{
    TorrentEvent, TorrentFile, TorrentInfo, TorrentService, TorrentServiceConfig, TorrentState,
};
pub use transcode::{TranscodeKind, TranscodeService, TranscodeServiceConfig};

pub use metadata::tmdb::{
    TmdbClient, TmdbCollection, TmdbCredits, TmdbMovie, TmdbMovieSearchResult, TmdbReleaseDates,
    normalize_movie_status,
};
