//! GraphQL API with subscriptions for real-time updates
//!
//! This module provides a GraphQL API using async-graphql with support for
//! queries, mutations, and subscriptions over WebSocket.
//!
//! This is the single API surface for the Librarian backend.

pub mod auth;
mod schema;
mod subscriptions;
mod types;

pub use auth::{verify_token, AuthGuard, AuthUser, RoleGuard};
pub use schema::{build_schema, LibrarianSchema};
