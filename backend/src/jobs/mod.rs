//! Background job scheduling and workers

pub mod artwork;
pub mod download_monitor;
pub mod rss_poller;
pub mod scanner;
pub mod transcode_gc;

use std::sync::Arc;

use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

use crate::services::ScannerService;

/// Initialize and start the job scheduler
pub async fn start_scheduler(scanner_service: Arc<ScannerService>) -> anyhow::Result<JobScheduler> {
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

    // Download monitor - run every minute
    let download_job = Job::new_async("0 * * * * *", |_uuid, _l| {
        Box::pin(async move {
            if let Err(e) = download_monitor::check_downloads().await {
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
