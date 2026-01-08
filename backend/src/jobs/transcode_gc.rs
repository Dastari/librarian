//! Transcode cache garbage collection

use anyhow::Result;

/// Clean up stale transcode cache files
pub async fn cleanup_cache() -> Result<()> {
    // TODO: Implement cache cleanup
    // 1. Walk cache directory
    // 2. Find sessions older than threshold
    // 3. Delete stale session directories

    tracing::info!("Transcode cache cleanup completed");
    Ok(())
}
