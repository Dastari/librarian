//! External service integrations

pub mod artwork;
pub mod filename_parser;
pub mod metadata;
pub mod prowlarr;
pub mod scanner;
pub mod supabase_storage;
pub mod torrent;
pub mod tvmaze;

pub use artwork::ArtworkService;
pub use metadata::{
    create_metadata_service, create_metadata_service_with_artwork, AddTvShowOptions,
    MetadataProvider, MetadataService, MetadataServiceConfig,
};
pub use scanner::{create_scanner_service, ScannerService};
pub use supabase_storage::StorageClient;
pub use torrent::{TorrentEvent, TorrentInfo, TorrentService, TorrentServiceConfig, TorrentState};
