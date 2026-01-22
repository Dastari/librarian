//! Background job scheduling and workers
//!
//! This module provides:
//! - Cron-based job scheduling via tokio-cron-scheduler
//! - Job execution with retry logic
//! - Basic failure tracking and logging

pub mod artwork;
pub mod auto_download;
pub mod auto_hunt;
pub mod download_monitor;
pub mod rss_poller;
pub mod scanner;
pub mod schedule_sync;
pub mod transcode_gc;

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
use sqlx::SqlitePool;
use tokio_cron_scheduler::{Job, JobScheduler};

#[cfg(feature = "postgres")]
type DbPool = PgPool;
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
type DbPool = SqlitePool;
use tracing::{error, info, warn};

use crate::indexer::manager::IndexerManager;
use crate::services::{ScannerService, TorrentService};

/// Configuration for job retry behavior
#[derive(Debug, Clone)]
pub struct JobRetryConfig {
    /// Maximum number of retry attempts (0 = no retries)
    pub max_retries: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
}

impl Default for JobRetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            initial_delay: Duration::from_secs(5),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
        }
    }
}

impl JobRetryConfig {
    /// Create a config for critical jobs that should retry more aggressively
    pub fn critical() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(10),
            max_delay: Duration::from_secs(120),
            backoff_multiplier: 2.0,
        }
    }

    /// Create a config for jobs that should not retry
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            initial_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            backoff_multiplier: 1.0,
        }
    }
}

/// Execute a job with retry logic
///
/// This function wraps job execution with:
/// - Configurable retry attempts with exponential backoff
/// - Logging of failures and retry attempts
/// - Final error logging after all retries exhausted
///
/// # Arguments
/// * `job_name` - Human-readable name for logging
/// * `config` - Retry configuration
/// * `job_fn` - The async job function to execute
///
/// # Returns
/// Ok(()) if the job succeeded (possibly after retries), Err if all attempts failed
pub async fn run_with_retry<F, Fut>(
    job_name: &str,
    config: &JobRetryConfig,
    job_fn: F,
) -> anyhow::Result<()>
where
    F: Fn() -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let mut attempt = 0;
    let mut delay = config.initial_delay;

    loop {
        attempt += 1;

        match job_fn().await {
            Ok(()) => {
                if attempt > 1 {
                    info!(
                        job = %job_name,
                        attempt = attempt,
                        "Job succeeded after retry"
                    );
                }
                return Ok(());
            }
            Err(e) => {
                if attempt > config.max_retries {
                    error!(
                        job = %job_name,
                        attempts = attempt,
                        error = %e,
                        "Job failed after all retry attempts"
                    );
                    return Err(e);
                }

                warn!(
                    job = %job_name,
                    attempt = attempt,
                    max_attempts = config.max_retries + 1,
                    error = %e,
                    retry_delay_secs = delay.as_secs(),
                    "Job failed, scheduling retry"
                );

                tokio::time::sleep(delay).await;

                // Exponential backoff
                delay = Duration::from_secs_f64(
                    (delay.as_secs_f64() * config.backoff_multiplier)
                        .min(config.max_delay.as_secs_f64()),
                );
            }
        }
    }
}

/// Helper macro to create a job that runs with retry logic
#[macro_export]
macro_rules! job_with_retry {
    ($name:expr, $config:expr, $body:expr) => {{
        let config = $config.clone();
        let name = $name.to_string();
        Box::pin(async move {
            if let Err(e) = $crate::jobs::run_with_retry(&name, &config, || async { $body }).await {
                tracing::error!(job = %name, error = %e, "Job ultimately failed");
            }
        })
    }};
}

/// Initialize and start the job scheduler
pub async fn start_scheduler(
    scanner_service: Arc<ScannerService>,
    torrent_service: Arc<TorrentService>,
    pool: DbPool,
    analysis_queue: Option<Arc<crate::services::MediaAnalysisQueue>>,
    metadata_service: Option<Arc<crate::services::MetadataService>>,
    indexer_manager: Option<Arc<IndexerManager>>,
) -> anyhow::Result<JobScheduler> {
    let scheduler = JobScheduler::new().await?;
    let default_retry = JobRetryConfig::default();
    let critical_retry = JobRetryConfig::critical();

    // Library scanner - run every hour (with retries for network issues)
    let scanner = scanner_service.clone();
    let scanner_retry = default_retry.clone();
    let scanner_job = Job::new_async("0 0 * * * *", move |_uuid, _l| {
        let scanner = scanner.clone();
        let retry_cfg = scanner_retry.clone();
        Box::pin(async move {
            info!("Running library scanner");
            let _ = run_with_retry("library_scanner", &retry_cfg, || {
                let s = scanner.clone();
                async move { scanner::run_scan(s).await }
            })
            .await;
        })
    })?;
    scheduler.add(scanner_job).await?;

    // RSS/Indexer poller - run every 15 minutes (with retries for network issues)
    let rss_retry = default_retry.clone();
    let rss_job = Job::new_async("0 */15 * * * *", move |_uuid, _l| {
        let retry_cfg = rss_retry.clone();
        Box::pin(async move {
            info!("Running RSS poller");
            let _ = run_with_retry("rss_poller", &retry_cfg, || async {
                rss_poller::poll_feeds().await
            })
            .await;
        })
    })?;
    scheduler.add(rss_job).await?;

    // Auto-download available episodes - run every 5 minutes (critical - use more retries)
    let torrent_svc = torrent_service.clone();
    let download_pool = pool.clone();
    let auto_dl_retry = critical_retry.clone();
    let auto_download_job = Job::new_async("0 */5 * * * *", move |_uuid, _l| {
        let svc = torrent_svc.clone();
        let p = download_pool.clone();
        let retry_cfg = auto_dl_retry.clone();
        Box::pin(async move {
            info!("Running auto-download check");
            let _ = run_with_retry("auto_download", &retry_cfg, || {
                let svc = svc.clone();
                let p = p.clone();
                async move { auto_download::process_available_episodes(p, svc).await }
            })
            .await;
        })
    })?;
    scheduler.add(auto_download_job).await?;

    // Download monitor - run every minute to process completed torrents (critical)
    let monitor_torrent_svc = torrent_service.clone();
    let monitor_pool = pool.clone();
    let monitor_retry = critical_retry.clone();
    let monitor_analysis_queue = analysis_queue.clone();
    let monitor_metadata_service = metadata_service.clone();
    let download_job = Job::new_async("0 * * * * *", move |_uuid, _l| {
        let svc = monitor_torrent_svc.clone();
        let p = monitor_pool.clone();
        let retry_cfg = monitor_retry.clone();
        let queue = monitor_analysis_queue.clone();
        let _metadata = monitor_metadata_service.clone(); // Kept for future use
        Box::pin(async move {
            let _ = run_with_retry("download_monitor", &retry_cfg, || {
                let svc = svc.clone();
                let p = p.clone();
                let q = queue.clone();
                async move { download_monitor::process_completed_torrents(p, svc, q).await }
            })
            .await;
        })
    })?;
    scheduler.add(download_job).await?;

    // Transcode cache cleanup - run daily at 3 AM (no retry needed - not critical)
    let gc_job = Job::new_async("0 0 3 * * *", |_uuid, _l| {
        Box::pin(async move {
            info!("Running transcode cache cleanup");
            if let Err(e) = transcode_gc::cleanup_cache().await {
                error!("Transcode GC error: {}", e);
            }
        })
    })?;
    scheduler.add(gc_job).await?;

    // TV Schedule sync - run every 6 hours (with retries for API issues)
    let schedule_pool = pool.clone();
    let schedule_retry = default_retry.clone();
    let schedule_job = Job::new_async("0 0 */6 * * *", move |_uuid, _l| {
        let p = schedule_pool.clone();
        let retry_cfg = schedule_retry.clone();
        Box::pin(async move {
            info!("Running TV schedule sync");
            let _ = run_with_retry("schedule_sync", &retry_cfg, || {
                let p = p.clone();
                async move { schedule_sync::sync_schedule(p).await }
            })
            .await;
        })
    })?;
    scheduler.add(schedule_job).await?;

    // NOTE: Auto-hunt no longer runs on an independent schedule.
    // It now runs in two scenarios:
    // 1. Immediately when a new movie is added (via add_movie mutation)
    // 2. After each library scan completes (via ScannerService)
    // This ensures auto-hunt runs on the same schedule as library scans
    // and provides immediate hunting for newly added content.
    if indexer_manager.is_some() {
        info!("Auto-hunt enabled: will run after library scans and when movies are added");
    } else {
        warn!("IndexerManager not provided - auto-hunt will be disabled");
    }

    scheduler.start().await?;

    info!("Job scheduler started with retry logic enabled");
    Ok(scheduler)
}
