//! API route definitions
//!
//! The primary API is GraphQL at /graphql.
//! REST endpoints are provided only for operations that don't work well with GraphQL:
//! - File uploads (multipart form data)
//! - Filesystem browsing (simple REST is cleaner)
//! - Health checks

pub mod filesystem;
pub mod health;
pub mod torrents;
