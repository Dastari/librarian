//! API route definitions
//!
//! The primary API is GraphQL at /graphql.
//! REST endpoints are provided only for operations that don't work well with GraphQL:
//! - File uploads (multipart form data)
//! - Filesystem browsing (simple REST is cleaner)
//! - Health checks
//! - Torznab API for external app compatibility (Sonarr, Radarr)
//! - Media streaming for cast devices and browser playback

pub mod filesystem;
pub mod health;
pub mod media;
pub mod torrents;
pub mod torznab;
