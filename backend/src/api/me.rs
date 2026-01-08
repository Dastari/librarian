//! User profile endpoints (DEPRECATED - use GraphQL)
//!
//! This module is kept for reference but is no longer exposed.
//! User profile operations are now handled via GraphQL.

use axum::Router;
use crate::AppState;

// This module is deprecated - all operations now go through GraphQL
pub fn router() -> Router<AppState> {
    Router::new()
}
