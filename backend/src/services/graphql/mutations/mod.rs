//! GraphQL mutation modules (auth, filesystem, etc.).

pub mod auth;
pub mod filesystem;

pub use auth::AuthMutations;
pub use filesystem::FilesystemMutations;
