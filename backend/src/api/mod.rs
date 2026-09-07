//! API route definitions
//!
//! The primary API is GraphQL at /graphql.
//! REST endpoints are provided only for operations that don't work well with GraphQL:
//! - Health checks
//! - Media streaming for cast devices and browser playback
//! - Artwork serving (SQLite mode only - images are stored as BLOBs)

pub mod artwork;
pub mod auth_guard;
pub mod health;
pub mod media;
