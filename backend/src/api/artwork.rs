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
    // Map artwork_type to database format (normalize plural forms)
    let db_artwork_type = match artwork_type.as_str() {
        "poster" | "posters" => "poster",
        "backdrop" | "backdrops" => "backdrop",
        "thumbnail" | "thumbnails" => "thumbnail",
        "banner" | "banners" => "banner",
        "cover" => "cover",
        other => other,
    };

    // Query directly from database
    let result = sqlx::query_as::<_, (Vec<u8>, String, String)>(
        r#"
        SELECT data, mime_type, content_hash
        FROM artwork_cache
        WHERE entity_type = ?1 AND entity_id = ?2 AND artwork_type = ?3
        LIMIT 1
        "#,
    )
    .bind(&entity_type)
    .bind(&entity_id)
    .bind(db_artwork_type)
    .fetch_optional(&state.db)
    .await;

    match result {
        Ok(Some((data, mime_type, content_hash))) => {
            let headers = [
                (header::CONTENT_TYPE, mime_type),
                (header::CACHE_CONTROL, "public, max-age=86400".to_string()), // 24 hour cache
                (header::ETAG, format!("\"{}\"", content_hash)),
            ];
            (StatusCode::OK, headers, data).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Artwork not found").into_response(),
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
    // Get total count and size
    let count_result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM artwork_cache")
        .fetch_one(&state.db)
        .await;

    let size_result =
        sqlx::query_scalar::<_, i64>("SELECT COALESCE(SUM(size_bytes), 0) FROM artwork_cache")
            .fetch_one(&state.db)
            .await;

    let stats_result = sqlx::query_as::<_, (String, i64, i64)>(
        r#"
        SELECT entity_type, COUNT(*) as count, COALESCE(SUM(size_bytes), 0) as total_bytes
        FROM artwork_cache
        GROUP BY entity_type
        "#,
    )
    .fetch_all(&state.db)
    .await;

    match (count_result, size_result, stats_result) {
        (Ok(count), Ok(total_bytes), Ok(by_type)) => {
            let response = serde_json::json!({
                "total_count": count,
                "total_bytes": total_bytes,
                "total_mb": total_bytes as f64 / 1_048_576.0,
                "by_entity_type": by_type.iter().map(|(entity_type, c, b)| {
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
        _ => {
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
