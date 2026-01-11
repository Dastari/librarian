//! External service integrations

pub mod artwork;
pub mod cache;
pub mod filename_parser;
pub mod logging;
pub mod metadata;
pub mod organizer;
pub mod prowlarr;
pub mod rss;
pub mod scanner;
pub mod supabase_storage;
pub mod torrent;
pub mod tvmaze;

pub use artwork::ArtworkService;
pub use logging::{create_database_layer, DatabaseLoggerConfig, LogEvent};
pub use metadata::{
    create_metadata_service_with_artwork, AddTvShowOptions, MetadataProvider, MetadataService,
    MetadataServiceConfig,
};
pub use organizer::{OrganizerService, TorrentFileForOrganize};
pub use rss::{ParsedRssItem, RssService};
pub use scanner::{create_scanner_service, ScannerService};
pub use supabase_storage::StorageClient;
pub use torrent::{PeerStats, TorrentDetails, TorrentEvent, TorrentInfo, TorrentService, TorrentServiceConfig, TorrentState};
