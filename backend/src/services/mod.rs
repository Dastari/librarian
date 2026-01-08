//! External service integrations

pub mod prowlarr;
pub mod supabase_storage;
pub mod torrent;

pub use torrent::{TorrentEvent, TorrentInfo, TorrentService, TorrentServiceConfig, TorrentState};
