//! GraphQL API with subscriptions for real-time updates
//!
//! This module provides a GraphQL API using async-graphql with support for
//! queries, mutations, and subscriptions over WebSocket.
//!
//! This is the single API surface for the Librarian backend.
//!
//! ## Architecture Note
//!
//! The `schema.rs` file is large (~7000 lines) and could benefit from being split
//! into domain-specific modules. The recommended approach for future refactoring:
//!
//! 1. Create `queries/` and `mutations/` subdirectories
//! 2. Create domain-specific files (e.g., `queries/tv_shows.rs`, `mutations/libraries.rs`)
//! 3. Each file defines a struct with `#[derive(Default)]` and `#[Object]` impl
//! 4. Use `#[derive(MergedObject)]` in schema.rs to combine them into QueryRoot/MutationRoot
//!
//! Example migration pattern:
//! ```rust,ignore
//! // queries/tv_shows.rs
//! #[derive(Default)]
//! pub struct TvShowQueries;
//!
//! #[Object]
//! impl TvShowQueries {
//!     async fn tv_shows(&self, ctx: &Context<'_>, library_id: String) -> Result<Vec<TvShow>> { ... }
//! }
//!
//! // schema.rs
//! #[derive(MergedObject, Default)]
//! pub struct QueryRoot(TvShowQueries, MovieQueries, LibraryQueries, ...);
//! ```

pub mod auth;
pub mod filters;
pub mod helpers;
pub mod mutations;
pub mod pagination;
pub mod queries;
mod schema;
mod subscriptions;
pub mod types;

pub use auth::{AuthUser, verify_token};
pub use schema::{LibrarianSchema, build_schema};
pub use types::{Library, LibraryChangeType, LibraryChangedEvent, MediaFileUpdatedEvent, ContentDownloadProgressEvent};
