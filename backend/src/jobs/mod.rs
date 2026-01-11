//! Background job scheduling and workers

pub mod artwork;
pub mod auto_download;
pub mod download_monitor;
pub mod rss_poller;
pub mod scanner;
pub mod transcode_gc;

use std::sync::Arc;

use sqlx::PgPool;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

use crate::services::{ScannerService, TorrentService};

/// Initialize and start the job scheduler
pub async fn start_scheduler(
    scanner_service: Arc<ScannerService>,
    torrent_service: Arc<TorrentService>,
    pool: PgPool,
) -> anyhow::Result<JobScheduler> {
    let scheduler = JobScheduler::new().await?;

    // Library scanner - run every hour
    let scanner = scanner_service.clone();
    let scanner_job = Job::new_async("0 0 * * * *", move |_uuid, _l| {
        let scanner = scanner.clone();
        Box::pin(async move {
            info!("Running library scanner");
            if let Err(e) = scanner::run_scan(scanner).await {
                tracing::error!("Scanner error: {}", e);
            }
        })
    })?;
    scheduler.add(scanner_job).await?;

    // RSS/Indexer poller - run every 15 minutes
    let rss_job = Job::new_async("0 */15 * * * *", |_uuid, _l| {
        Box::pin(async move {
            info!("Running RSS poller");
            if let Err(e) = rss_poller::poll_feeds().await {
                tracing::error!("RSS poller error: {}", e);
            }
        })
    })?;
    scheduler.add(rss_job).await?;

    // Auto-download available episodes - run every 5 minutes
    let torrent_svc = torrent_service.clone();
    let download_pool = pool.clone();
    let auto_download_job = Job::new_async("0 */5 * * * *", move |_uuid, _l| {
        let svc = torrent_svc.clone();
        let p = download_pool.clone();
        Box::pin(async move {
            info!("Running auto-download check");
            if let Err(e) = auto_download::process_available_episodes(p, svc).await {
                tracing::error!("Auto-download error: {}", e);
            }
        })
    })?;
    scheduler.add(auto_download_job).await?;

    // Download monitor - run every minute to process completed torrents
    let monitor_torrent_svc = torrent_service.clone();
    let monitor_pool = pool.clone();
    let download_job = Job::new_async("0 * * * * *", move |_uuid, _l| {
        let svc = monitor_torrent_svc.clone();
        let p = monitor_pool.clone();
        Box::pin(async move {
            if let Err(e) = download_monitor::process_completed_torrents(p, svc).await {
                tracing::error!("Download monitor error: {}", e);
            }
        })
    })?;
    scheduler.add(download_job).await?;

    // Transcode cache cleanup - run daily at 3 AM
    let gc_job = Job::new_async("0 0 3 * * *", |_uuid, _l| {
        Box::pin(async move {
            info!("Running transcode cache cleanup");
            if let Err(e) = transcode_gc::cleanup_cache().await {
                tracing::error!("Transcode GC error: {}", e);
            }
        })
    })?;
    scheduler.add(gc_job).await?;

    scheduler.start().await?;

    info!("Job scheduler started");
    Ok(scheduler)
}
