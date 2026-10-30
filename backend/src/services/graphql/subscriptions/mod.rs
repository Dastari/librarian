//! GraphQL subscription modules (filesystem change events, etc.).

pub mod backup;
pub mod filesystem;
pub mod session;
pub mod source_capabilities;
pub mod torrents;

#[cfg(test)]
mod lag_tests;

pub use backup::BackupSubscriptions;
pub use filesystem::FilesystemSubscriptions;
pub use session::SessionSubscriptions;
pub use source_capabilities::SourceCapabilitySubscriptions;
pub use torrents::TorrentSubscriptions;
