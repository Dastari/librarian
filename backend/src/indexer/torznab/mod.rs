//! Torznab REST API
//!
//! This module provides a Torznab-compatible REST API that allows external applications
//! like Sonarr and Radarr to use Librarian as a torrent indexer source.
//!
//! # Endpoints
//!
//! - `GET /api/torznab/{indexer_id}?t=caps` - Get indexer capabilities
//! - `GET /api/torznab/{indexer_id}?t=search&q=...` - General search
//! - `GET /api/torznab/{indexer_id}?t=tvsearch&q=...` - TV search
//! - `GET /api/torznab/{indexer_id}?t=movie&q=...` - Movie search
//!
//! All endpoints require an `apikey` parameter for authentication.

pub mod request;
pub mod response;

use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    routing::get,
};
use uuid::Uuid;

use crate::indexer::manager::IndexerManager;
use request::TorznabRequest;
use response::{TorznabError, TorznabResponse};

/// Application state for Torznab routes
#[derive(Clone)]
pub struct TorznabState {
    pub indexer_manager: Arc<IndexerManager>,
    pub api_key: String,
}

/// Create the Torznab router
pub fn router(state: TorznabState) -> Router {
    Router::new()
        .route("/torznab/:indexer_id", get(torznab_handler))
        .route("/torznab/:indexer_id/api", get(torznab_handler))
        .with_state(state)
}

/// Main Torznab endpoint handler
async fn torznab_handler(
    State(state): State<TorznabState>,
    Path(indexer_id): Path<String>,
    Query(params): Query<TorznabRequest>,
) -> Response {
    // Validate API key
    if params.apikey.as_deref() != Some(&state.api_key) {
        return TorznabError::unauthorized("Invalid API Key").into_response();
    }

    // Parse indexer ID
    let config_id = match Uuid::parse_str(&indexer_id) {
        Ok(id) => id,
        Err(_) => {
            return TorznabError::not_found("Invalid indexer ID").into_response();
        }
    };

    // Get the indexer
    let indexer = match state.indexer_manager.get_indexer(config_id) {
        Some(idx) => idx,
        None => {
            return TorznabError::not_found("Indexer not found or not configured").into_response();
        }
    };

    // Handle request based on type
    let query_type = params.t.as_deref().unwrap_or("search");

    match query_type {
        "caps" | "capabilities" => {
            // Return capabilities XML
            let caps = indexer.capabilities();
            TorznabResponse::capabilities(indexer.name(), caps).into_response()
        }
        "search" | "tvsearch" | "movie" | "music" | "book" => {
            // Convert to TorznabQuery
            let query = match params.to_query() {
                Ok(q) => q,
                Err(e) => {
                    return TorznabError::bad_request(&e.to_string()).into_response();
                }
            };

            // Check if indexer can handle this query
            if !indexer.can_handle_query(&query) {
                return TorznabError::function_not_available(&format!(
                    "{} is not supported by this indexer",
                    query_type
                ))
                .into_response();
            }

            // Perform search
            match indexer.search(&query).await {
                Ok(releases) => TorznabResponse::search_results(
                    indexer.name(),
                    indexer.description(),
                    indexer.site_link(),
                    releases,
                )
                .into_response(),
                Err(e) => {
                    tracing::error!(
                        indexer_id = indexer.id(),
                        error = %e,
                        "Torznab search failed"
                    );
                    TorznabError::indexer_error(&e.to_string()).into_response()
                }
            }
        }
        _ => TorznabError::bad_request(&format!("Unknown query type: {}", query_type))
            .into_response(),
    }
}
