//! External service integrations

pub mod artwork;
pub mod cache;
pub mod cast;
pub mod filename_parser;
pub mod filesystem;
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
pub use cast::{CastDeviceType, CastPlayerState, CastService, CastServiceConfig, CastSessionEvent, CastDevicesEvent};
pub use filesystem::{
    DirectoryChangeEvent, FilesystemService, FilesystemServiceConfig,
    FileEntry as FsFileEntry, QuickPath as FsQuickPath, BrowseResult, PathValidation,
};
pub use logging::{DatabaseLoggerConfig, LogEvent, create_database_layer};
pub use metadata::{
    AddTvShowOptions, MetadataProvider, MetadataService, MetadataServiceConfig,
    create_metadata_service_with_artwork,
};
pub use organizer::{OrganizerService, TorrentFileForOrganize};
pub use rss::{ParsedRssItem, RssService};
pub use scanner::{ScannerService, create_scanner_service};
pub use supabase_storage::StorageClient;
pub use torrent::{
    PeerStats, TorrentDetails, TorrentEvent, TorrentInfo, TorrentService, TorrentServiceConfig,
    TorrentState,
};
