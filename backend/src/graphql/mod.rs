//! GraphQL API with subscriptions for real-time updates
//!
//! This module provides a GraphQL API using async-graphql with support for
//! queries, mutations, and subscriptions over WebSocket.
//!
//! This is the single API surface for the Librarian backend.

pub mod auth;
pub mod filters;
pub mod pagination;
mod schema;
mod subscriptions;
pub mod types;

pub use auth::{AuthUser, verify_token};
pub use filters::{StringFilter, IntFilter, BoolFilter, DateFilter, DateRange, OrderDirection};
pub use pagination::{Connection, Edge, PageInfo, encode_cursor, decode_cursor, parse_pagination_args};
pub use schema::{LibrarianSchema, build_schema};
pub use types::{Library, LibraryChangedEvent, LibraryChangeType, MediaFileUpdatedEvent};
