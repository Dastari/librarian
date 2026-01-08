//! RSS/Torznab feed poller

use anyhow::Result;

/// Poll RSS feeds for new releases
pub async fn poll_feeds() -> Result<()> {
    // TODO: Implement RSS polling
    // 1. Get active subscriptions
    // 2. Query Prowlarr/Jackett Torznab feeds
    // 3. Apply quality filters
    // 4. Enqueue matching downloads

    tracing::info!("RSS polling completed");
    Ok(())
}
