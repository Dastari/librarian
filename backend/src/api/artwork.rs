//! Artwork serving endpoint (SQLite mode only)
//!
//! Serves cached artwork images from the SQLite database.
//! Only compiled when the `sqlite` feature is enabled.

#[cfg(feature = "sqlite")]
use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
    routing::get,
};

#[cfg(feature = "sqlite")]
use crate::AppState;
#[cfg(feature = "sqlite")]
use crate::graphql::entities::ArtworkCache;

/// Serve artwork from SQLite cache
///
/// GET /api/artwork/:entity_type/:entity_id/:artwork_type
#[cfg(feature = "sqlite")]
async fn serve_artwork(
    State(state): State<AppState>,
    Path((entity_type, entity_id, artwork_type)): Path<(String, String, String)>,
) -> impl IntoResponse {
    // Map artwork_type to database format (normalize plural forms)
    let db_artwork_type = match artwork_type.as_str() {
        "poster" | "posters" => "poster",
        "backdrop" | "backdrops" => "backdrop",
        "thumbnail" | "thumbnails" => "thumbnail",
        "banner" | "banners" => "banner",
        "cover" => "cover",
        other => other,
    };

    let result = ArtworkCache::query(state.db.pool()).fetch_all().await;

    match result {
        Ok(entries) => match entries.into_iter().find(|entry| {
            entry.entity_type == entity_type
                && entry.entity_id == entity_id
                && entry.artwork_type == db_artwork_type
        }) {
            Some(entry) => {
                let headers = [
                    (header::CONTENT_TYPE, entry.mime_type),
                    (header::CACHE_CONTROL, "public, max-age=86400".to_string()), // 24 hour cache
                    (header::ETAG, format!("\"{}\"", entry.content_hash)),
                ];
                (StatusCode::OK, headers, entry.data).into_response()
            }
            None => (StatusCode::NOT_FOUND, "Artwork not found").into_response(),
        },
        Err(e) => {
            tracing::error!(
                entity_type = %entity_type,
                entity_id = %entity_id,
                artwork_type = %artwork_type,
                error = %e,
                "Failed to retrieve artwork from database"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to retrieve artwork",
            )
                .into_response()
        }
    }
}

/// Get artwork storage statistics
#[cfg(feature = "sqlite")]
async fn storage_stats(State(state): State<AppState>) -> impl IntoResponse {
    match ArtworkCache::query(state.db.pool()).fetch_all().await {
        Ok(entries) => {
            let count = entries.len() as i64;
            let total_bytes = entries.iter().map(|entry| entry.size_bytes).sum::<i64>();
            let mut by_type = std::collections::HashMap::<String, (i64, i64)>::new();
            for entry in entries {
                let totals = by_type.entry(entry.entity_type).or_default();
                totals.0 += 1;
                totals.1 += entry.size_bytes;
            }
            let response = serde_json::json!({
                "total_count": count,
                "total_bytes": total_bytes,
                "total_mb": total_bytes as f64 / 1_048_576.0,
                "by_entity_type": by_type.iter().map(|(entity_type, (c, b))| {
                    serde_json::json!({
                        "entity_type": entity_type,
                        "count": c,
                        "bytes": b,
                        "mb": *b as f64 / 1_048_576.0
                    })
                }).collect::<Vec<_>>()
            });
            (StatusCode::OK, axum::Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get artwork storage statistics");
            tracing::error!("Failed to get artwork storage statistics");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get stats").into_response()
        }
    }
}

/// Create the artwork router
#[cfg(feature = "sqlite")]
pub fn router() -> Router<AppState> {
    Router::new().nest(
        "/artwork",
        Router::new()
            .route(
                "/{entity_type}/{entity_id}/{artwork_type}",
                get(serve_artwork),
            )
            .route("/stats", get(storage_stats)),
    )
}

/// No-op router for non-SQLite builds
#[cfg(not(feature = "sqlite"))]
pub fn router() -> Router<AppState> {
    Router::new()
}
