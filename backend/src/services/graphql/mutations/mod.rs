//! GraphQL mutation modules (auth, filesystem, etc.).

pub mod artwork;
pub mod auth;
pub mod auto_download;
pub mod backup;
pub mod filesystem;
pub mod library_scan;
pub mod notifications;
pub mod quality;

pub use artwork::ArtworkMutations;
pub use auth::AuthMutations;
pub use auto_download::AutoDownloadMutations;
pub use backup::BackupMutations;
pub use filesystem::FilesystemMutations;
pub use library_scan::LibraryScanMutations;
pub use notifications::NotificationFeedMutations;
pub use quality::QualityMutations;

#[cfg(test)]
mod auth_session_tests;

#[cfg(test)]
mod notification_tests;
