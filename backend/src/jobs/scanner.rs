//! Library scanner job

use anyhow::Result;

/// Run a full library scan
pub async fn run_scan() -> Result<()> {
    // TODO: Implement library scanning
    // 1. Walk library paths
    // 2. Detect new/missing files
    // 3. Run ffprobe for media properties
    // 4. Identify content (parse filename)
    // 5. Fetch metadata from TheTVDB/TMDB
    // 6. Update database

    tracing::info!("Library scan completed");
    Ok(())
}
