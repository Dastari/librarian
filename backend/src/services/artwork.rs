//! Artwork service for downloading and caching images to Supabase Storage

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use super::supabase_storage::StorageClient;

/// Artwork service for managing poster/backdrop images
pub struct ArtworkService {
    storage: StorageClient,
    http_client: reqwest::Client,
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

    pub fn bucket(&self) -> &'static str {
        "artwork"
    }
}

impl ArtworkService {
    pub fn new(storage: StorageClient) -> Self {
        Self {
            storage,
            http_client: reqwest::Client::new(),
        }
    }

    /// Download an image from a URL and cache it in Supabase Storage
    ///
    /// Returns the public URL of the cached image
    pub async fn cache_image(
        &self,
        source_url: &str,
        artwork_type: ArtworkType,
        entity_type: &str, // "show", "movie", "episode"
        entity_id: &str,   // provider ID or UUID
    ) -> Result<String> {
        info!(
            url = %source_url,
            artwork_type = ?artwork_type,
            entity_type = %entity_type,
            entity_id = %entity_id,
            "Caching artwork"
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

        // Determine file extension from content type
        let extension = match content_type.as_str() {
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "jpg",
        };

        // Download the image bytes
        let bytes = response.bytes().await.context("Failed to read image bytes")?;

        // Generate a hash-based filename for deduplication
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash = format!("{:x}", hasher.finalize());
        let short_hash = &hash[..16];

        // Build the storage path
        let path = format!(
            "{}/{}/{}/{}.{}",
            artwork_type.as_str(),
            entity_type,
            entity_id,
            short_hash,
            extension
        );

        debug!(path = %path, size = bytes.len(), "Uploading to storage");

        // Upload to Supabase Storage
        let public_url = self
            .storage
            .upload(artwork_type.bucket(), &path, &bytes, &content_type)
            .await
            .context("Failed to upload to storage")?;

        info!(url = %public_url, "Artwork cached successfully");
        Ok(public_url)
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

        match self.cache_image(url, artwork_type, entity_type, entity_id).await {
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
        for artwork_type in [
            ArtworkType::Poster,
            ArtworkType::Backdrop,
            ArtworkType::Thumbnail,
            ArtworkType::Banner,
        ] {
            let path = format!("{}/{}/{}", artwork_type.as_str(), entity_type, entity_id);
            // Note: This would need a list + delete operation in the storage client
            // For now, we'll just log the intent
            debug!(path = %path, "Would delete artwork at path");
        }
        Ok(())
    }
}

/// Create the artwork bucket if it doesn't exist
pub async fn ensure_artwork_bucket(storage: &StorageClient) -> Result<()> {
    storage.ensure_bucket("artwork", true).await?;
    info!("Artwork bucket ready");
    Ok(())
}
