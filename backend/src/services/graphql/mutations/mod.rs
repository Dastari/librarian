//! GraphQL mutation modules (auth, filesystem, etc.).

pub mod artwork;
pub mod auth;
pub mod filesystem;

pub use artwork::ArtworkMutations;
pub use auth::AuthMutations;
pub use filesystem::FilesystemMutations;
