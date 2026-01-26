//! GraphQL ORM Layer
//!
//! Provides traits and utilities for macro-generated GraphQL entities.
//! The `librarian-macros` crate generates implementations of these traits
//! from annotated Rust structs, creating a single source of truth for:
//! - GraphQL types (SimpleObject with PascalCase names)
//! - Filter inputs (WhereInput)
//! - Sort inputs (OrderByInput)
//! - SQL query generation (parameterized via sqlx)
//! - Row decoding (FromSqlRow)
//! - Relation loading (with look_ahead support)
//!
//! # Repository Pattern
//!
//! The repository module provides a unified interface for querying entities
//! that works for both GraphQL resolvers and internal service code:
//!
//! ```rust,ignore
//! use crate::graphql::entities::{MovieEntity, MovieEntityWhereInput};
//! use crate::graphql::filters::StringFilter;
//!
//! // Find movies in a library
//! let movies = MovieEntity::find_all(&pool)
//!     .filter(MovieEntityWhereInput {
//!         library_id: Some(StringFilter::eq(library_id)),
//!         ..Default::default()
//!     })
//!     .fetch_all()
//!     .await?;
//! ```

mod builder;
pub mod fuzzy;
mod repository;
mod traits;

pub use builder::*;
pub use fuzzy::{FuzzyMatcher, generate_candidate_pattern};
pub use repository::*;
pub use traits::*;

// Re-export filter types for use in generated code
pub use super::filters::StringFilter;
pub use super::pagination::{Connection, Edge, PageInfo, decode_cursor, encode_cursor};
