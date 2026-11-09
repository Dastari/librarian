//! Fills in credits, title details and people for titles added before they were kept, or
//! whose first read failed (`docs/design.md` Q132). A few titles and people per pass, inside
//! TMDB's and MusicBrainz's rate limits; titles are never written, only their `Credit` and
//! `TitleDetail` rows, and locked credits and detail fields are left alone.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::services::ServicesManager;
use crate::services::manager::{Service, ServiceHealth};
use crate::services::metadata::providers::{
    CreditFillBudget, MetadataService, credit_fill_requested,
};

#[derive(Clone, Debug)]
pub struct CreditFillConfig {
    /// Time between passes when nothing wakes the job.
    pub interval: Duration,
    /// Wait after startup before the first pass, so it does not compete with startup scans.
    pub first_delay: Duration,
    pub budget: CreditFillBudget,
}

impl Default for CreditFillConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(10 * 60),
            first_delay: Duration::from_secs(90),
            budget: CreditFillBudget::default(),
        }
    }
}

pub struct CreditFillService {
    manager: Arc<ServicesManager>,
    config: CreditFillConfig,
    runtime: tokio::sync::Mutex<Option<(CancellationToken, tokio::task::JoinHandle<()>)>>,
    error: Arc<tokio::sync::RwLock<Option<String>>>,
}

impl CreditFillService {
    pub fn new(manager: Arc<ServicesManager>, config: CreditFillConfig) -> Self {
        Self {
            manager,
            config,
            runtime: Default::default(),
            error: Default::default(),
        }
    }
}

async fn run_pass(manager: &Arc<ServicesManager>, budget: CreditFillBudget) -> Result<()> {
    let Some(database) = manager.get_database().await else {
        return Ok(());
    };
    let metadata = MetadataService::new_default(database.pool().clone(), manager.clone());
    let report = metadata.fill_credits(budget).await?;
    if report.total() > 0 {
        tracing::info!(
            service = "credit_fill",
            movies = report.movies,
            shows = report.shows,
            albums = report.albums,
            audiobooks = report.audiobooks,
            people = report.people,
            carried_over = report.carried_over,
            "Filled credits and details: {} movies, {} shows, {} albums, {} audiobooks, {} people",
            report.movies,
            report.shows,
            report.albums,
            report.audiobooks,
            report.people
        );
    }
    Ok(())
}

#[async_trait]
impl Service for CreditFillService {
    fn name(&self) -> &str {
        "credit_fill"
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["database".into(), "graphql".into()]
    }

    async fn start(&self) -> Result<()> {
        let mut runtime = self.runtime.lock().await;
        if runtime.is_some() {
            return Ok(());
        }
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        let manager = self.manager.clone();
        let config = self.config.clone();
        let error = self.error.clone();
        let task = tokio::spawn(async move {
            tokio::select! {
                _ = token.cancelled() => return,
                _ = tokio::time::sleep(config.first_delay) => {}
            }
            loop {
                let result = tokio::select! {
                    _ = token.cancelled() => break,
                    result = run_pass(&manager, config.budget) => result,
                };
                *error.write().await = result.err().map(|e| {
                    tracing::warn!(service = "credit_fill", error = %e, "Credits fill pass failed; retrying next pass");
                    e.to_string()
                });
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = tokio::time::sleep(config.interval) => {}
                    _ = credit_fill_requested() => {
                        // Let the add that woke us finish its own writes first.
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });
        *runtime = Some((cancel, task));
        tracing::info!(
            service = "credit_fill",
            interval_secs = self.config.interval.as_secs(),
            "Started the credits and title details fill"
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        if let Some((cancel, task)) = self.runtime.lock().await.take() {
            cancel.cancel();
            let _ = task.await;
        }
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        if let Some(error) = self.error.read().await.as_ref() {
            return Ok(ServiceHealth::degraded(error));
        }
        let runtime = self.runtime.lock().await;
        Ok(
            if runtime
                .as_ref()
                .is_some_and(|(_, task)| !task.is_finished())
            {
                ServiceHealth::healthy()
            } else {
                ServiceHealth::degraded("credit_fill worker is not running")
            },
        )
    }
}
