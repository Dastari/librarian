//! Artwork service for downloading and caching images
//!
//! Stores artwork in SQLite BLOB storage for self-hosted deployment.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use crate::db::Database;

/// Artwork service for managing poster/backdrop images
pub struct ArtworkService {
    db: Database,
    http_client: reqwest::Client,
    /// Base URL for serving artwork (e.g., "http://localhost:3001")
    base_url: String,
}

/// Artwork type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtworkType {
    Poster,
    Backdrop,
    Thumbnail,
    Banner,
}

impl ArtworkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArtworkType::Poster => "posters",
            ArtworkType::Backdrop => "backdrops",
            ArtworkType::Thumbnail => "thumbnails",
            ArtworkType::Banner => "banners",
        }
    }
}

impl ArtworkService {
    pub fn new(db: Database, base_url: String) -> Self {
        Self {
            db,
            http_client: reqwest::Client::new(),
            base_url,
        }
    }

    /// Create with default base URL from environment
    pub fn with_env(db: Database) -> Self {
        let base_url = std::env::var("PUBLIC_URL")
            .unwrap_or_else(|_| "http://localhost:3001".to_string());
        Self::new(db, base_url)
    }

    /// Download an image from a URL and cache it in SQLite
    ///
    /// Returns the internal URL for serving the cached image
    pub async fn cache_image(
        &self,
        source_url: &str,
        artwork_type: ArtworkType,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<String> {
        use crate::db::UpsertArtwork;

        info!(
            url = %source_url,
            artwork_type = ?artwork_type,
            entity_type = %entity_type,
            entity_id = %entity_id,
            "Caching artwork to SQLite"
        );

        // Download the image
        let response = self
            .http_client
            .get(source_url)
            .send()
            .await
            .context("Failed to download image")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to download image: {}", response.status());
        }

        // Get content type
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();

        // Download the image bytes
        let bytes = response
            .bytes()
            .await
            .context("Failed to read image bytes")?;

        // Generate hash for deduplication
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash = format!("{:x}", hasher.finalize());

        // Try to detect image dimensions (optional)
        let (width, height) = self.detect_image_dimensions(&bytes);

        debug!(
            hash = %hash,
            size = bytes.len(),
            width = ?width,
            height = ?height,
            "Storing artwork in database"
        );

        // Store in database
        self.db.artwork().upsert(UpsertArtwork {
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            artwork_type: artwork_type.as_str().to_string(),
            content_hash: hash,
            mime_type: content_type,
            data: bytes.to_vec(),
            source_url: Some(source_url.to_string()),
            width,
            height,
        }).await?;

        // Return internal URL
        let internal_url = format!(
            "{}/api/artwork/{}/{}/{}",
            self.base_url,
            entity_type,
            entity_id,
            artwork_type.as_str()
        );

        info!(url = %internal_url, "Artwork cached successfully");
        Ok(internal_url)
    }

    /// Try to detect image dimensions (basic implementation)
    fn detect_image_dimensions(&self, data: &[u8]) -> (Option<i32>, Option<i32>) {
        // PNG signature check
        if data.len() > 24 && data[0..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
            // PNG: width at bytes 16-19, height at bytes 20-23
            let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
            let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
            return (Some(width as i32), Some(height as i32));
        }

        // JPEG SOI marker
        if data.len() > 2 && data[0] == 0xFF && data[1] == 0xD8 {
            // JPEG dimension detection is more complex, skip for now
            return (None, None);
        }

        (None, None)
    }

    /// Cache a poster image for a TV show
    pub async fn cache_show_poster(&self, source_url: &str, show_id: &str) -> Result<String> {
        self.cache_image(source_url, ArtworkType::Poster, "show", show_id)
            .await
    }

    /// Cache a backdrop image for a TV show
    pub async fn cache_show_backdrop(&self, source_url: &str, show_id: &str) -> Result<String> {
        self.cache_image(source_url, ArtworkType::Backdrop, "show", show_id)
            .await
    }

    /// Cache artwork with fallback (returns None if caching fails)
    pub async fn cache_image_optional(
        &self,
        source_url: Option<&str>,
        artwork_type: ArtworkType,
        entity_type: &str,
        entity_id: &str,
    ) -> Option<String> {
        let url = source_url?;

        match self
            .cache_image(url, artwork_type, entity_type, entity_id)
            .await
        {
            Ok(cached_url) => Some(cached_url),
            Err(e) => {
                warn!(
                    error = %e,
                    url = %url,
                    "Failed to cache artwork, using original URL"
                );
                Some(url.to_string())
            }
        }
    }

    /// Delete cached artwork for an entity
    pub async fn delete_entity_artwork(&self, entity_type: &str, entity_id: &str) -> Result<()> {
        let deleted = self.db.artwork().delete_for_entity(entity_type, entity_id).await?;
        debug!(entity_type = %entity_type, entity_id = %entity_id, count = deleted, "Deleted artwork");
        Ok(())
    }

    /// Get artwork from cache (returns image data and mime type)
    pub async fn get_artwork(
        &self,
        entity_type: &str,
        entity_id: &str,
        artwork_type: &str,
    ) -> Result<Option<(Vec<u8>, String)>> {
        if let Some(artwork) = self.db.artwork().get_with_data(entity_type, entity_id, artwork_type).await? {
            Ok(Some((artwork.data, artwork.record.mime_type)))
        } else {
            Ok(None)
        }
    }

    /// Get storage statistics
    pub async fn storage_stats(&self) -> Result<ArtworkStorageStats> {
        let count = self.db.artwork().count().await?;
        let total_bytes = self.db.artwork().total_storage_bytes().await?;
        let by_type = self.db.artwork().storage_stats().await?;

        Ok(ArtworkStorageStats {
            total_count: count,
            total_bytes,
            by_entity_type: by_type.into_iter().map(|(t, c, b)| (t, c, b)).collect(),
        })
    }
}

/// Artwork storage statistics
#[derive(Debug, Clone)]
pub struct ArtworkStorageStats {
    pub total_count: i64,
    pub total_bytes: i64,
    pub by_entity_type: Vec<(String, i64, i64)>,
}

/// No-op for SQLite (table is created via migration)
pub async fn ensure_artwork_storage(_db: &Database) -> Result<()> {
    info!("Artwork storage ready (SQLite)");
    Ok(())
}
