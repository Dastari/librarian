//! GraphQL subscription modules (filesystem change events, etc.).

pub mod filesystem;

pub use filesystem::FilesystemSubscriptions;
