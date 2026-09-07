//! Artwork serving endpoint backed by object storage.
//!
//! Serves cached artwork using metadata stored in GraphQL ORM entities and bytes
//! stored through `graphql-orm-storage`.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::get,
};
use graphql_orm::graphql::filters::StringFilter;

use crate::AppState;
use crate::api::auth_guard::{require_authenticated_admin, require_authenticated_user};
use crate::graphql::AuthUser;
use crate::graphql::entities::{
    Album, Artist, ArtworkCache, ArtworkCacheWhereInput, Audiobook, Collection, Movie, Show,
    StorageObject,
};

/// Serve artwork from object storage.
///
/// GET /api/artwork/:entity_type/:entity_id/:artwork_type
async fn serve_artwork(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((entity_type, entity_id, artwork_type)): Path<(String, String, String)>,
) -> impl IntoResponse {
    let auth_user = match require_authenticated_user(&state, &headers, None).await {
        Ok(user) => user,
        Err(status) => return status.into_response(),
    };
    match can_access_artwork(&state, &auth_user, &entity_type, &entity_id).await {
        Ok(true) => {}
        Ok(false) => return (StatusCode::NOT_FOUND, "Artwork not found").into_response(),
        Err(error) => {
            tracing::error!(
                user_id = %auth_user.user_id,
                entity_type = %entity_type,
                entity_id = %entity_id,
                error = %error,
                "Failed to authorize artwork request"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to retrieve artwork",
            )
                .into_response();
        }
    }
    let db_artwork_type = normalize_artwork_type(&artwork_type);

    let entry = match find_artwork_cache(&state, &entity_type, &entity_id, db_artwork_type).await {
        Ok(Some(entry)) => entry,
        Ok(None) => return (StatusCode::NOT_FOUND, "Artwork not found").into_response(),
        Err(err) => {
            tracing::error!(
                entity_type = %entity_type,
                entity_id = %entity_id,
                artwork_type = %artwork_type,
                error = %err,
                "Failed to retrieve artwork metadata"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to retrieve artwork",
            )
                .into_response();
        }
    };

    let storage_row = match StorageObject::get(state.db.pool(), &entry.storage_object_id).await {
        Ok(Some(row)) => row,
        Ok(None) => {
            tracing::warn!(
                entity_type = %entity_type,
                entity_id = %entity_id,
                artwork_type = %artwork_type,
                storage_object_id = %entry.storage_object_id,
                "Artwork metadata references missing storage object"
            );
            return (StatusCode::NOT_FOUND, "Artwork not found").into_response();
        }
        Err(err) => {
            tracing::error!(
                entity_type = %entity_type,
                entity_id = %entity_id,
                artwork_type = %artwork_type,
                storage_object_id = %entry.storage_object_id,
                error = %err,
                "Failed to load artwork storage metadata"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to retrieve artwork",
            )
                .into_response();
        }
    };

    let Some(storage) = state.services.get_storage().await else {
        tracing::error!("Object storage service unavailable while serving artwork");
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Object storage unavailable",
        )
            .into_response();
    };

    let stored = match storage.stored_object_from_entity(&storage_row) {
        Ok(stored) => stored,
        Err(err) => {
            tracing::error!(
                entity_type = %entity_type,
                entity_id = %entity_id,
                artwork_type = %artwork_type,
                storage_key = %storage_row.storage_key,
                object_id = %storage_row.object_id,
                error = %err,
                "Invalid artwork storage metadata"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to retrieve artwork",
            )
                .into_response();
        }
    };

    match storage.storage().get_object(&stored).await {
        Ok(body) => {
            let headers = [
                (
                    header::CONTENT_TYPE,
                    storage_row
                        .mime_type
                        .clone()
                        .unwrap_or_else(|| "application/octet-stream".to_string()),
                ),
                (header::CACHE_CONTROL, "private, max-age=86400".to_string()),
                (header::ETAG, format!("\"{}\"", storage_row.sha256_hex)),
                (header::VARY, "Cookie, Authorization".to_string()),
                (
                    header::HeaderName::from_static("x-content-type-options"),
                    "nosniff".to_string(),
                ),
            ];
            (StatusCode::OK, headers, body.bytes).into_response()
        }
        Err(err) => {
            tracing::warn!(
                entity_type = %entity_type,
                entity_id = %entity_id,
                artwork_type = %artwork_type,
                storage_key = %storage_row.storage_key,
                object_id = %storage_row.object_id,
                error = %err,
                "Artwork object bytes are missing"
            );
            (StatusCode::NOT_FOUND, "Artwork not found").into_response()
        }
    }
}

async fn can_access_artwork(
    state: &AppState,
    user: &AuthUser,
    entity_type: &str,
    entity_id: &str,
) -> anyhow::Result<bool> {
    if user.is_admin() {
        return Ok(true);
    }

    let entity_id = entity_id.to_string();
    let owner = match entity_type.to_ascii_lowercase().as_str() {
        "movie" => Movie::get(state.db.pool(), &entity_id)
            .await?
            .map(|entity| entity.user_id),
        "show" => Show::get(state.db.pool(), &entity_id)
            .await?
            .map(|entity| entity.user_id),
        "album" => Album::get(state.db.pool(), &entity_id)
            .await?
            .map(|entity| entity.user_id),
        "audiobook" => Audiobook::get(state.db.pool(), &entity_id)
            .await?
            .map(|entity| entity.user_id),
        "collection" => Collection::get(state.db.pool(), &entity_id)
            .await?
            .map(|entity| entity.user_id),
        "artist" => Artist::get(state.db.pool(), &entity_id)
            .await?
            .map(|entity| entity.user_id),
        _ => None,
    };
    Ok(owner.as_deref() == Some(user.user_id.as_str()))
}

/// Get artwork storage statistics. Admin-only: reveals total library size/composition.
async fn storage_stats(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(status) = require_authenticated_admin(&state, &headers).await {
        return status.into_response();
    }

    let entries = match ArtworkCache::query(state.db.pool()).fetch_all().await {
        Ok(entries) => entries,
        Err(err) => {
            tracing::error!(error = %err, "Failed to get artwork storage statistics");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get stats").into_response();
        }
    };

    let mut total_bytes = 0_i64;
    let mut by_type = std::collections::HashMap::<String, (i64, i64)>::new();
    for entry in &entries {
        let size = match StorageObject::get(state.db.pool(), &entry.storage_object_id).await {
            Ok(Some(row)) => row.size_bytes,
            Ok(None) => 0,
            Err(err) => {
                tracing::warn!(
                    storage_object_id = %entry.storage_object_id,
                    error = %err,
                    "Failed to load storage object for artwork stats"
                );
                0
            }
        };
        total_bytes += size;
        let totals = by_type.entry(entry.entity_type.clone()).or_default();
        totals.0 += 1;
        totals.1 += size;
    }

    let response = serde_json::json!({
        "total_count": entries.len() as i64,
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
    (StatusCode::OK, Json(response)).into_response()
}

async fn find_artwork_cache(
    state: &AppState,
    entity_type: &str,
    entity_id: &str,
    artwork_type: &str,
) -> anyhow::Result<Option<ArtworkCache>> {
    let rows = ArtworkCache::query(state.db.pool())
        .filter(ArtworkCacheWhereInput {
            entity_type: Some(StringFilter {
                eq: Some(entity_type.to_string()),
                ..Default::default()
            }),
            entity_id: Some(StringFilter {
                eq: Some(entity_id.to_string()),
                ..Default::default()
            }),
            artwork_type: Some(StringFilter {
                eq: Some(artwork_type.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?;
    Ok(rows.into_iter().next())
}

fn normalize_artwork_type(artwork_type: &str) -> &str {
    match artwork_type {
        "poster" | "posters" => "poster",
        "backdrop" | "backdrops" => "backdrop",
        "thumbnail" | "thumbnails" => "thumbnail",
        "banner" | "banners" => "banner",
        "cover" => "cover",
        other => other,
    }
}

/// Create the artwork router.
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
