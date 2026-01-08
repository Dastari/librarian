//! API route definitions
//!
//! The primary API is GraphQL at /graphql.
//! REST endpoints are provided for operations that don't work well with GraphQL,
//! such as file uploads and filesystem browsing.

pub mod filesystem;
pub mod health;
pub mod torrents;

// Legacy REST modules - kept for reference but not exposed
// All operations now go through GraphQL
mod libraries;
mod media;
mod me;
mod subscriptions;
