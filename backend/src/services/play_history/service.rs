//! Play history upkeep: the one-time import from watch progress and the retention sweep.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use graphql_orm::graphql::filters::StringFilter;
use tokio_util::sync::CancellationToken;

use crate::db::Database;
use crate::graphql::entities::{AppSetting, AppSettingWhereInput, CreateAppSettingInput};
use crate::services::ServicesManager;
use crate::services::manager::{Service, ServiceHealth};

use super::{IMPORTED_SETTING, RETENTION_SETTING, import_from_progress, prune, retention_days};

/// How often retention runs.
const SWEEP_INTERVAL: Duration = Duration::from_secs(6 * 3600);

#[derive(Clone, Debug, Default)]
pub struct PlayHistoryConfig;

pub struct PlayHistoryService {
    manager: Arc<ServicesManager>,
    runtime: tokio::sync::Mutex<Option<(CancellationToken, tokio::task::JoinHandle<()>)>>,
    error: Arc<tokio::sync::RwLock<Option<String>>>,
}

impl PlayHistoryService {
    pub fn new(manager: Arc<ServicesManager>, _config: PlayHistoryConfig) -> Self {
        Self {
            manager,
            runtime: Default::default(),
            error: Default::default(),
        }
    }
}

pub async fn setting(db: &Database, key: &str) -> Result<Option<String>> {
    Ok(AppSetting::query(db.pool())
        .filter(AppSettingWhereInput {
            key: Some(StringFilter {
                eq: Some(key.to_owned()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?
        .into_iter()
        .next()
        .map(|setting| setting.value))
}

/// One pass: import once, then apply the retention setting.
pub async fn run_once(db: &Database) -> Result<()> {
    if setting(db, IMPORTED_SETTING).await?.is_none() {
        let outcome = import_from_progress(db, None).await?;
        AppSetting::insert(
            db,
            CreateAppSettingInput {
                key: IMPORTED_SETTING.into(),
                value: serde_json::to_string(&super::now())?,
                description: Some(
                    "When watched progress was first imported into play history".into(),
                ),
                category: "playback".into(),
            },
        )
        .await?;
        tracing::info!(
            service = "play_history",
            imported = outcome.imported,
            skipped = outcome.skipped,
            "Imported {} watched items from watch progress into play history ({} skipped)",
            outcome.imported,
            outcome.skipped
        );
    }
    let days = retention_days(setting(db, RETENTION_SETTING).await?.as_deref());
    let removed = prune(db, days, chrono::Utc::now()).await?;
    if removed > 0 {
        tracing::info!(
            service = "play_history",
            removed,
            retention_days = days,
            "Removed {removed} plays older than {days} days from play history"
        );
    }
    Ok(())
}

#[async_trait]
impl Service for PlayHistoryService {
    fn name(&self) -> &str {
        "play_history"
    }
    fn dependencies(&self) -> Vec<String> {
        vec!["database".into()]
    }
    async fn start(&self) -> Result<()> {
        let mut runtime = self.runtime.lock().await;
        if runtime.is_some() {
            return Ok(());
        }
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        let manager = self.manager.clone();
        let error = self.error.clone();
        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(SWEEP_INTERVAL);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! { _ = token.cancelled() => break, _ = interval.tick() => {} }
                let Some(database) = manager.get_database().await else {
                    continue;
                };
                let db = database.pool().clone();
                let result = tokio::select! {
                    _ = token.cancelled() => break,
                    result = run_once(&db) => result,
                };
                *error.write().await = result.err().map(|error| {
                    tracing::warn!(service = "play_history", error = %error, "Play history import or retention sweep failed; retrying in six hours");
                    error.to_string()
                });
            }
        });
        *runtime = Some((cancel, task));
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
        Ok(ServiceHealth::healthy())
    }
}
