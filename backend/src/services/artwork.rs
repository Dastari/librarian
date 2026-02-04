//! Artwork caching service
//!
//! Downloads and caches artwork (posters, backdrops) from external URLs (TMDB, etc.)
//! into the SQLite database for fast local serving.
//!
//! This is a utility service (not a lifecycle Service) - no background tasks,
//! just on-demand operations called during metadata operations.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::db::Database;

/// Artwork service for caching images from external URLs
#[derive(Clone)]
pub struct ArtworkService {
    db: Database,
}

impl ArtworkService {
    /// Create a new artwork service
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Cache an image from a source URL
    ///
    /// Downloads the image, computes its hash, stores it in the database,
    /// and returns the internal URL for serving.
    ///
    /// Returns the cached URL: `/api/artwork/{entity_type}/{entity_id}/{artwork_type}`
    pub async fn cache_image(
        &self,
        source_url: &str,
        entity_type: &str,
        entity_id: &str,
        artwork_type: &str,
    ) -> Result<String> {
        debug!(
            source_url = %source_url,
            entity_type = %entity_type,
            entity_id = %entity_id,
            artwork_type = %artwork_type,
            "Caching artwork from URL"
        );

        // Check if already cached
        if let Some(_existing) = self
            .get_cached_entry(entity_type, entity_id, artwork_type)
            .await?
        {
            debug!(
                entity_type = %entity_type,
                entity_id = %entity_id,
                artwork_type = %artwork_type,
                "Artwork already cached, skipping download"
            );
            return Ok(format!(
                "/api/artwork/{}/{}/{}",
                entity_type, entity_id, artwork_type
            ));
        }

        // Download the image
        let response = reqwest::get(source_url)
            .await
            .context("Failed to download artwork")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to download artwork: HTTP {}", response.status());
        }

        // Get content type
        let mime_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();

        // Download the bytes
        let bytes = response
            .bytes()
            .await
            .context("Failed to read artwork bytes")?;
        let data = bytes.to_vec();
        let size_bytes = data.len() as i64;

        // Compute content hash for deduplication
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let content_hash = format!("{:x}", hasher.finalize());

        // Try to detect dimensions using image crate
        let (width, height) = match image::load_from_memory(&data) {
            Ok(img) => (Some(img.width() as i32), Some(img.height() as i32)),
            Err(e) => {
                debug!(error = %e, "Could not detect image dimensions");
                (None, None)
            }
        };

        // Generate ID
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        // Insert into database
        sqlx::query(
            r#"
            INSERT INTO artwork_cache 
            (id, entity_type, entity_id, artwork_type, content_hash, mime_type, data, size_bytes, source_url, width, height, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
        )
        .bind(&id)
        .bind(entity_type)
        .bind(entity_id)
        .bind(artwork_type)
        .bind(&content_hash)
        .bind(&mime_type)
        .bind(&data)
        .bind(size_bytes)
        .bind(source_url)
        .bind(width)
        .bind(height)
        .bind(&now)
        .bind(&now)
        .execute(&self.db)
        .await
        .context("Failed to insert artwork into database")?;

        info!(
            entity_type = %entity_type,
            entity_id = %entity_id,
            artwork_type = %artwork_type,
            size_kb = size_bytes / 1024,
            dimensions = ?format!("{}x{}", width.unwrap_or(0), height.unwrap_or(0)),
            "Cached artwork from '{}'",
            source_url
        );

        Ok(format!(
            "/api/artwork/{}/{}/{}",
            entity_type, entity_id, artwork_type
        ))
    }

    /// Check if artwork is cached
    async fn get_cached_entry(
        &self,
        entity_type: &str,
        entity_id: &str,
        artwork_type: &str,
    ) -> Result<Option<String>> {
        let result: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT id FROM artwork_cache
            WHERE entity_type = ?1 AND entity_id = ?2 AND artwork_type = ?3
            LIMIT 1
            "#,
        )
        .bind(entity_type)
        .bind(entity_id)
        .bind(artwork_type)
        .fetch_optional(&self.db)
        .await?;

        Ok(result.map(|(id,)| id))
    }

    /// Cache movie artwork (poster and backdrop)
    ///
    /// Returns tuple of (cached_poster_url, cached_backdrop_url)
    pub async fn cache_movie_artwork(
        &self,
        movie_id: &str,
        poster_url: Option<&str>,
        backdrop_url: Option<&str>,
    ) -> (Option<String>, Option<String>) {
        let mut cached_poster = None;
        let mut cached_backdrop = None;

        if let Some(url) = poster_url {
            match self.cache_image(url, "movie", movie_id, "poster").await {
                Ok(cached_url) => cached_poster = Some(cached_url),
                Err(e) => {
                    warn!(
                        movie_id = %movie_id,
                        error = %e,
                        "Failed to cache movie poster"
                    );
                }
            }
        }

        if let Some(url) = backdrop_url {
            match self.cache_image(url, "movie", movie_id, "backdrop").await {
                Ok(cached_url) => cached_backdrop = Some(cached_url),
                Err(e) => {
                    warn!(
                        movie_id = %movie_id,
                        error = %e,
                        "Failed to cache movie backdrop"
                    );
                }
            }
        }

        (cached_poster, cached_backdrop)
    }

    /// Cache show artwork (poster and backdrop)
    pub async fn cache_show_artwork(
        &self,
        show_id: &str,
        poster_url: Option<&str>,
        backdrop_url: Option<&str>,
    ) -> (Option<String>, Option<String>) {
        let mut cached_poster = None;
        let mut cached_backdrop = None;

        if let Some(url) = poster_url {
            match self.cache_image(url, "show", show_id, "poster").await {
                Ok(cached_url) => cached_poster = Some(cached_url),
                Err(e) => {
                    warn!(
                        show_id = %show_id,
                        error = %e,
                        "Failed to cache show poster"
                    );
                }
            }
        }

        if let Some(url) = backdrop_url {
            match self.cache_image(url, "show", show_id, "backdrop").await {
                Ok(cached_url) => cached_backdrop = Some(cached_url),
                Err(e) => {
                    warn!(
                        show_id = %show_id,
                        error = %e,
                        "Failed to cache show backdrop"
                    );
                }
            }
        }

        (cached_poster, cached_backdrop)
    }

    /// Cache album artwork (cover)
    pub async fn cache_album_artwork(
        &self,
        album_id: &str,
        cover_url: Option<&str>,
    ) -> Option<String> {
        if let Some(url) = cover_url {
            match self.cache_image(url, "album", album_id, "cover").await {
                Ok(cached_url) => Some(cached_url),
                Err(e) => {
                    warn!(
                        album_id = %album_id,
                        error = %e,
                        "Failed to cache album cover"
                    );
                    None
                }
            }
        } else {
            None
        }
    }

    /// Cache audiobook artwork (cover)
    pub async fn cache_audiobook_artwork(
        &self,
        audiobook_id: &str,
        cover_url: Option<&str>,
    ) -> Option<String> {
        if let Some(url) = cover_url {
            match self
                .cache_image(url, "audiobook", audiobook_id, "cover")
                .await
            {
                Ok(cached_url) => Some(cached_url),
                Err(e) => {
                    warn!(
                        audiobook_id = %audiobook_id,
                        error = %e,
                        "Failed to cache audiobook cover"
                    );
                    None
                }
            }
        } else {
            None
        }
    }

    /// Get cached artwork URL if it exists, otherwise return the original URL
    pub async fn get_artwork_url(
        &self,
        entity_type: &str,
        entity_id: &str,
        artwork_type: &str,
        fallback_url: Option<&str>,
    ) -> Option<String> {
        match self
            .get_cached_entry(entity_type, entity_id, artwork_type)
            .await
        {
            Ok(Some(_)) => Some(format!(
                "/api/artwork/{}/{}/{}",
                entity_type, entity_id, artwork_type
            )),
            _ => fallback_url.map(|s| s.to_string()),
        }
    }
}
