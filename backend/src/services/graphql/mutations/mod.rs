//! GraphQL mutation modules (auth, filesystem, etc.).

pub mod artwork;
pub mod auth;
pub mod filesystem;
pub mod library_scan;

pub use artwork::ArtworkMutations;
pub use auth::AuthMutations;
pub use filesystem::FilesystemMutations;
pub use library_scan::LibraryScanMutations;
