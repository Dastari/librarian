//! Artwork serving endpoint backed by object storage.
//!
//! Serves cached artwork using metadata stored in GraphQL ORM entities and bytes
//! stored through `graphql-orm-storage`.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::get,
};
use graphql_orm::graphql::filters::{IntFilter, StringFilter};
use serde::Deserialize;

use crate::AppState;
use crate::api::auth_guard::{require_authenticated_admin, require_authenticated_user};
use crate::graphql::AuthUser;
use crate::graphql::entities::{
    Album, Artist, ArtworkCache, Audiobook, Collection, CollectionWhereInput, Episode, Movie,
    Season, Show, StorageObject,
};
use crate::services::ArtworkService;
use crate::services::artwork::normalize_artwork_width;

#[derive(Debug, Default, Deserialize)]
struct ArtworkQuery {
    /// Desired display width in pixels; snapped to a supported step and never upscaled.
    w: Option<u32>,
}

/// Serve artwork from object storage.
///
/// GET /api/artwork/:entity_type/:entity_id/:artwork_type
///
/// Artwork is cache-through: the first request for an image the cache does not hold yet
/// downloads it from the entity's provider URL into object storage, so clients only ever
/// load local copies. `?w=` serves a stored, resized variant.
async fn serve_artwork(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((entity_type, entity_id, artwork_type)): Path<(String, String, String)>,
    Query(query): Query<ArtworkQuery>,
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
    let entity_kind = entity_type.to_ascii_lowercase();

    let Some(storage) = state.services.get_storage().await else {
        tracing::error!("Object storage service unavailable while serving artwork");
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Object storage unavailable",
        )
            .into_response();
    };
    let artwork = ArtworkService::new(state.db.clone(), storage.clone());

    let entry = match artwork
        .ensure_cached(&entity_kind, &entity_id, db_artwork_type)
        .await
    {
        Ok(Some(entry)) => entry,
        Ok(None) => return (StatusCode::NOT_FOUND, "Artwork not found").into_response(),
        Err(err) => {
            // A provider outage or a refused download is not a server fault for the client.
            tracing::warn!(
                entity_type = %entity_type,
                entity_id = %entity_id,
                artwork_type = %artwork_type,
                error = %err,
                "Artwork is not cached and could not be downloaded"
            );
            return (StatusCode::NOT_FOUND, "Artwork not found").into_response();
        }
    };
    let entry = match normalize_artwork_width(query.w) {
        Some(width) => match artwork.cached_variant(&entry, width).await {
            Ok(variant) => variant,
            Err(err) => {
                tracing::warn!(
                    entity_type = %entity_type,
                    entity_id = %entity_id,
                    width,
                    error = %err,
                    "Could not produce resized artwork; serving the original"
                );
                entry
            }
        },
        None => entry,
    };

    let storage_row = match StorageObject::get(state.db.pool(), &entry.storage_object_id).await {
        Ok(Some(row)) => row,
        Ok(None) => {
            tracing::warn!(
                storage_object_id = %entry.storage_object_id,
                "Artwork metadata references missing storage object"
            );
            return (StatusCode::NOT_FOUND, "Artwork not found").into_response();
        }
        Err(err) => {
            tracing::error!(
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

    // Unchanged bytes: let the client reuse its copy.
    let etag = format!("\"{}\"", storage_row.sha256_hex);
    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.split(',').any(|tag| tag.trim() == etag))
    {
        return (
            StatusCode::NOT_MODIFIED,
            [
                (header::ETAG, etag),
                (header::CACHE_CONTROL, "private, max-age=86400".to_string()),
            ],
        )
            .into_response();
    }

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
                (header::ETAG, etag),
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
        // Collection artwork is addressed by TMDB collection id: readable when the user owns
        // any library collection with that id.
        "collection" => {
            let Ok(tmdb_id) = entity_id.parse::<i32>() else {
                return Ok(false);
            };
            let owned = Collection::query(state.db.pool())
                .filter(CollectionWhereInput {
                    tmdb_collection_id: Some(IntFilter {
                        eq: Some(tmdb_id),
                        ..Default::default()
                    }),
                    user_id: Some(StringFilter {
                        eq: Some(user.user_id.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .fetch_all()
                .await?;
            return Ok(!owned.is_empty());
        }
        "season" => match Season::get(state.db.pool(), &entity_id).await? {
            Some(season) => Show::get(state.db.pool(), &season.show_id)
                .await?
                .map(|show| show.user_id),
            None => None,
        },
        "episode" => match Episode::get(state.db.pool(), &entity_id).await? {
            Some(episode) => Show::get(state.db.pool(), &episode.show_id)
                .await?
                .map(|show| show.user_id),
            None => None,
        },
        // People and the TV guide are shared catalogue data, readable by every member.
        "person" | "guide" => return Ok(true),
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

fn normalize_artwork_type(artwork_type: &str) -> &str {
    match artwork_type {
        "poster" | "posters" => "poster",
        "backdrop" | "backdrops" => "backdrop",
        "thumbnail" | "thumbnails" => "thumbnail",
        "banner" | "banners" => "banner",
        "cover" => "cover",
        "profile" | "profiles" => "profile",
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
