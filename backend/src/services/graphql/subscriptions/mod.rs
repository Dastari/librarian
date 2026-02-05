//! GraphQL subscription modules (filesystem change events, etc.).

pub mod filesystem;
pub mod torrents;

pub use filesystem::FilesystemSubscriptions;
pub use torrents::TorrentSubscriptions;
