//! Artwork fetching job

use anyhow::Result;

/// Fetch artwork for media items
pub async fn fetch_artwork(_media_id: &str) -> Result<()> {
    // TODO: Implement artwork fetching
    // 1. Query TheTVDB/TMDB for artwork URLs
    // 2. Download posters/backdrops
    // 3. Store in Supabase Storage
    // 4. Update artwork table

    tracing::info!("Artwork fetch completed");
    Ok(())
}
