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

/// Serve artwork from SQLite cache
///
/// GET /api/artwork/:entity_type/:entity_id/:artwork_type
#[cfg(feature = "sqlite")]
async fn serve_artwork(
    State(state): State<AppState>,
    Path((entity_type, entity_id, artwork_type)): Path<(String, String, String)>,
) -> impl IntoResponse {
    // Map artwork_type to database format
    let db_artwork_type = match artwork_type.as_str() {
        "poster" | "posters" => "posters",
        "backdrop" | "backdrops" => "backdrops",
        "thumbnail" | "thumbnails" => "thumbnails",
        "banner" | "banners" => "banners",
        "cover" => "cover",
        other => other,
    };

    match state.db.artwork().get_with_data(&entity_type, &entity_id, db_artwork_type).await {
        Ok(Some(artwork)) => {
            let headers = [
                (header::CONTENT_TYPE, artwork.record.mime_type),
                (header::CACHE_CONTROL, "public, max-age=86400".to_string()), // 24 hour cache
                (header::ETAG, format!("\"{}\"", artwork.record.content_hash)),
            ];
            (StatusCode::OK, headers, artwork.data).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, "Artwork not found").into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to retrieve artwork");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to retrieve artwork").into_response()
        }
    }
}

/// Get artwork storage statistics
#[cfg(feature = "sqlite")]
async fn storage_stats(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let artwork_repo = state.db.artwork();
    
    let count = match artwork_repo.count().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get artwork count");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get stats").into_response();
        }
    };
    
    let total_bytes = match artwork_repo.total_storage_bytes().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get artwork size");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get stats").into_response();
        }
    };
    
    let by_type = match artwork_repo.storage_stats().await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get artwork stats by type");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get stats").into_response();
        }
    };
    
    let response = serde_json::json!({
        "total_count": count,
        "total_bytes": total_bytes,
        "total_mb": total_bytes as f64 / 1_048_576.0,
        "by_entity_type": by_type.iter().map(|(t, c, b)| {
            serde_json::json!({
                "entity_type": t,
                "count": c,
                "bytes": b,
                "mb": *b as f64 / 1_048_576.0
            })
        }).collect::<Vec<_>>()
    });
    (StatusCode::OK, axum::Json(response)).into_response()
}

/// Create the artwork router (SQLite mode)
#[cfg(feature = "sqlite")]
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/artwork/{entity_type}/{entity_id}/{artwork_type}", get(serve_artwork))
        .route("/artwork/stats", get(storage_stats))
}

/// Placeholder router for PostgreSQL mode (Supabase handles artwork)
#[cfg(feature = "postgres")]
pub fn router() -> Router<crate::AppState> {
    Router::new()
}
