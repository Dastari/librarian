//! Logging service: database persistence and real-time log subscriptions.
//!
//! Implements [Service](crate::services::manager::Service) and depends on the database service.
//! The DB tracing layer is added as [OptionalDbLayer] in main (single subscriber init); the
//! logging service injects/removes the inner layer via [LoggingServiceConfig::db_layer_state]
//! when it starts/stops.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use async_graphql::{Request, Variables};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use time::OffsetDateTime;
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use uuid::Uuid;

use crate::db::Database;
use crate::services::graphql::AuthUser;
use crate::services::graphql::entities::{AppLog, CreateAppLogInput};
use crate::services::manager::{Service, ServiceHealth};

/// Default retention window (in days) for `app_log` rows, used when
/// `LIBRARIAN_LOG_RETENTION_DAYS` is unset or unparsable. A value of `0` disables
/// the retention sweep entirely.
const DEFAULT_LOG_RETENTION_DAYS: u32 = 30;

/// How long to wait after the service starts before the first retention sweep
/// (avoids competing with startup work for DB/GraphQL resources).
const RETENTION_STARTUP_DELAY: Duration = Duration::from_secs(60);

/// How often the retention sweep runs once started.
const RETENTION_SWEEP_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Read `LIBRARIAN_LOG_RETENTION_DAYS` from the environment (default
/// [DEFAULT_LOG_RETENTION_DAYS]; `0` disables the retention sweep).
fn log_retention_days_from_env() -> u32 {
    std::env::var("LIBRARIAN_LOG_RETENTION_DAYS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(DEFAULT_LOG_RETENTION_DAYS)
}

/// Shared state for the optional DB layer. Main builds the subscriber with [OptionalDbLayer] using
/// this state; the logging service sets [Some] when it starts and [None] when it stops.
pub type DbLayerState = Arc<Mutex<Option<Arc<DatabaseLoggingLayer>>>>;

/// Wrapper that holds an optional [DatabaseLoggingLayer] via shared state so it can be set by the
/// logging service after the subscriber is initialized.
#[derive(Clone)]
pub struct OptionalDbLayer(pub DbLayerState);

impl OptionalDbLayer {
    pub fn new(state: DbLayerState) -> Self {
        Self(state)
    }
}

impl<S> Layer<S> for OptionalDbLayer
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        if let Ok(guard) = self.0.lock()
            && let Some(ref inner) = *guard
        {
            inner.on_event(event, ctx);
        }
    }
}

/// Configuration for the logging service (database batch size, levels, etc.).
#[derive(Clone)]
pub struct LoggingServiceConfig {
    pub min_level: Level,
    pub batch_size: usize,
    pub flush_interval_ms: u64,
    pub broadcast_capacity: usize,
    /// If set, the logging service will set the inner DB layer when it starts and clear it when
    /// it stops. Main adds [OptionalDbLayer] with this state to the subscriber at init.
    pub db_layer_state: Option<DbLayerState>,
    /// Days of `app_log` history to keep; rows older than this are purged by a background
    /// sweep every 24h (see [RETENTION_SWEEP_INTERVAL]). `0` disables the sweep. Defaults to
    /// [DEFAULT_LOG_RETENTION_DAYS], overridable via `LIBRARIAN_LOG_RETENTION_DAYS`.
    pub log_retention_days: u32,
}

impl std::fmt::Debug for LoggingServiceConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoggingServiceConfig")
            .field("min_level", &self.min_level)
            .field("batch_size", &self.batch_size)
            .field("flush_interval_ms", &self.flush_interval_ms)
            .field("broadcast_capacity", &self.broadcast_capacity)
            .field(
                "db_layer_state",
                &self.db_layer_state.as_ref().map(|_| "..."),
            )
            .field("log_retention_days", &self.log_retention_days)
            .finish()
    }
}

impl Default for LoggingServiceConfig {
    fn default() -> Self {
        Self {
            min_level: Level::INFO,
            batch_size: 100,
            flush_interval_ms: 2000,
            broadcast_capacity: 1000,
            db_layer_state: None,
            log_retention_days: log_retention_days_from_env(),
        }
    }
}

/// Log event for broadcasting to subscribers (e.g. GraphQL).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub id: Option<String>,
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: Option<JsonValue>,
    pub span_name: Option<String>,
}

/// Logging service: depends on database, provides a tracing layer and broadcast for real-time logs.
pub struct LoggingService {
    services: Arc<crate::services::manager::ServicesManager>,
    config: LoggingServiceConfig,
    /// Set in start(); used by tracing_layer().
    layer: parking_lot::RwLock<Option<Arc<DatabaseLoggingLayer>>>,
    /// Used in stop() to signal the writer task.
    shutdown_tx: parking_lot::RwLock<Option<oneshot::Sender<()>>>,
    /// Writer task handle for orderly shutdown.
    writer_handle: parking_lot::RwLock<Option<tokio::task::JoinHandle<()>>>,
    /// Shared state to inject/clear the DB layer when this service starts/stops.
    db_layer_state: Option<DbLayerState>,
    /// Used in stop() to signal the retention sweep task.
    retention_shutdown_tx: parking_lot::RwLock<Option<oneshot::Sender<()>>>,
    /// Retention sweep task handle for orderly shutdown.
    retention_handle: parking_lot::RwLock<Option<tokio::task::JoinHandle<()>>>,
}

impl LoggingService {
    pub fn new(
        services: Arc<crate::services::manager::ServicesManager>,
        config: LoggingServiceConfig,
    ) -> Self {
        Self {
            db_layer_state: config.db_layer_state.clone(),
            services,
            config,
            layer: parking_lot::RwLock::new(None),
            shutdown_tx: parking_lot::RwLock::new(None),
            writer_handle: parking_lot::RwLock::new(None),
            retention_shutdown_tx: parking_lot::RwLock::new(None),
            retention_handle: parking_lot::RwLock::new(None),
        }
    }

    /// Returns the tracing layer to add to the subscriber. Only [Some] after [Service::start] has run.
    pub fn tracing_layer(&self) -> Option<Arc<DatabaseLoggingLayer>> {
        self.layer.read().clone()
    }

    /// Subscribe to real-time log events (e.g. for GraphQL subscriptions).
    pub fn subscribe(&self) -> Option<broadcast::Receiver<LogEvent>> {
        self.layer.read().as_ref().map(|l| l.subscribe())
    }
}

#[async_trait]
impl Service for LoggingService {
    fn name(&self) -> &str {
        "logging"
    }

    fn dependencies(&self) -> Vec<String> {
        // "graphql" is needed by the retention sweep, which deletes old app_log rows via the
        // deleteAppLogs mutation (see run_log_retention_sweep) rather than raw SQL.
        vec!["database".to_string(), "graphql".to_string()]
    }

    async fn start(&self) -> Result<()> {
        if self.config.batch_size == 0 {
            anyhow::bail!("Logging batch size must be greater than zero");
        }
        if self.config.flush_interval_ms == 0 {
            anyhow::bail!("Logging flush interval must be greater than zero");
        }
        if self.config.broadcast_capacity == 0 {
            anyhow::bail!("Logging broadcast capacity must be greater than zero");
        }
        tracing::info!(
            service = "logging",
            min_level = ?self.config.min_level,
            batch_size = self.config.batch_size,
            flush_interval_ms = self.config.flush_interval_ms,
            broadcast_capacity = self.config.broadcast_capacity,
            "Logging service starting: min_level={:?}, batch_size={}, flush_interval_ms={}, broadcast_capacity={}",
            self.config.min_level,
            self.config.batch_size,
            self.config.flush_interval_ms,
            self.config.broadcast_capacity
        );
        let db = self
            .services
            .get_database()
            .await
            .ok_or_else(|| anyhow::anyhow!("database service not started"))?;
        let pool = db.pool().clone();

        let (broadcast_tx, _) = broadcast::channel(self.config.broadcast_capacity);
        let (db_tx, db_rx) = mpsc::channel::<AppLog>(self.config.batch_size * 10);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let handle = tokio::spawn(database_writer_task(
            db_rx,
            pool,
            self.config.batch_size,
            self.config.flush_interval_ms,
            shutdown_rx,
        ));

        let layer = Arc::new(DatabaseLoggingLayer::new(
            self.config.min_level,
            broadcast_tx.clone(),
            db_tx,
        ));

        *self.layer.write() = Some(Arc::clone(&layer));
        *self.shutdown_tx.write() = Some(shutdown_tx);
        *self.writer_handle.write() = Some(handle);

        if let Some(ref state) = self.db_layer_state {
            let _ = state.lock().map(|mut g| *g = Some(Arc::clone(&layer)));
        }

        let (retention_shutdown_tx, retention_shutdown_rx) = oneshot::channel();
        let retention_handle = tokio::spawn(log_retention_task(
            Arc::clone(&self.services),
            self.config.log_retention_days,
            retention_shutdown_rx,
        ));
        *self.retention_shutdown_tx.write() = Some(retention_shutdown_tx);
        *self.retention_handle.write() = Some(retention_handle);

        tracing::info!(
            service = "logging",
            min_level = ?self.config.min_level,
            log_retention_days = self.config.log_retention_days,
            "Logging service started: min_level={:?}, log_retention_days={}",
            self.config.min_level,
            self.config.log_retention_days
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        if let Some(ref state) = self.db_layer_state {
            let _ = state.lock().map(|mut g| *g = None);
        }

        let handle = self.writer_handle.write().take();
        let _ = self.shutdown_tx.write().take();
        *self.layer.write() = None;

        if let Some(h) = handle {
            let _ = h.await;
        }

        let retention_handle = self.retention_handle.write().take();
        let _ = self.retention_shutdown_tx.write().take();
        if let Some(h) = retention_handle {
            let _ = h.await;
        }

        tracing::info!(
            service = "logging",
            "Logging service stopped: tracing layer detached, writer task and retention sweep shut down"
        );
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        if self.layer.read().is_none() {
            return Ok(ServiceHealth::unhealthy(
                "logging layer not initialized (start not called)",
            ));
        }
        if self
            .writer_handle
            .read()
            .as_ref()
            .is_none_or(tokio::task::JoinHandle::is_finished)
        {
            return Ok(ServiceHealth::unhealthy(
                "database log writer exited unexpectedly",
            ));
        }
        if self.config.log_retention_days > 0
            && self
                .retention_handle
                .read()
                .as_ref()
                .is_none_or(tokio::task::JoinHandle::is_finished)
        {
            return Ok(ServiceHealth::degraded(
                "log retention worker exited unexpectedly",
            ));
        }
        Ok(ServiceHealth::healthy())
    }
}

async fn database_writer_task(
    mut rx: mpsc::Receiver<AppLog>,
    pool: Database,
    batch_size: usize,
    flush_interval_ms: u64,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    let mut batch: Vec<AppLog> = Vec::with_capacity(batch_size);
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(flush_interval_ms));
    let mut last_fd_diag_log: Option<Instant> = None;

    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            Some(log) = rx.recv() => {
                batch.push(log);
                if batch.len() >= batch_size {
                    if let Err(e) = insert_app_logs_batch(&pool, &batch).await {
                        let fd_diag = open_fd_diagnostics().unwrap_or_else(|| "fd_diag=unavailable".to_string());
                        tracing::error!(
                            error = %e,
                            batch_size = batch.len(),
                            fd_diag = %fd_diag,
                            "Failed to write logs to database during size flush: batch_size={}, error={}, {}",
                            batch.len(),
                            e,
                            fd_diag
                        );
                        maybe_log_fd_diag(&mut last_fd_diag_log, &fd_diag);
                    }
                    batch.clear();
                }
            }
            _ = interval.tick() => {
                if !batch.is_empty() {
                    if let Err(e) = insert_app_logs_batch(&pool, &batch).await {
                        let fd_diag = open_fd_diagnostics().unwrap_or_else(|| "fd_diag=unavailable".to_string());
                        tracing::error!(
                            error = %e,
                            batch_size = batch.len(),
                            fd_diag = %fd_diag,
                            "Failed to write logs to database during interval flush: batch_size={}, error={}, {}",
                            batch.len(),
                            e,
                            fd_diag
                        );
                        maybe_log_fd_diag(&mut last_fd_diag_log, &fd_diag);
                    }
                    batch.clear();
                }
            }
        }
    }
}

async fn insert_app_logs_batch(db: &Database, logs: &[AppLog]) -> Result<()> {
    for log in logs {
        AppLog::insert(
            db,
            CreateAppLogInput {
                timestamp: log.timestamp.clone(),
                level: log.level.clone(),
                target: log.target.clone(),
                message: log.message.clone(),
                fields: log.fields.clone(),
                span_name: log.span_name.clone(),
                span_id: log.span_id.clone(),
            },
        )
        .await?;
    }

    Ok(())
}

/// Background task: waits [RETENTION_STARTUP_DELAY], then runs the `app_log` retention sweep
/// every [RETENTION_SWEEP_INTERVAL] until `shutdown_rx` fires. A `retention_days` of `0`
/// disables the sweep (the task still runs so it observes shutdown, but never deletes).
async fn log_retention_task(
    services: Arc<crate::services::manager::ServicesManager>,
    retention_days: u32,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    tokio::select! {
        _ = &mut shutdown_rx => return,
        _ = tokio::time::sleep(RETENTION_STARTUP_DELAY) => {}
    }

    if retention_days == 0 {
        tracing::info!(
            service = "logging",
            "App log retention sweep disabled (LIBRARIAN_LOG_RETENTION_DAYS=0)"
        );
        return;
    }

    let mut interval = tokio::time::interval(RETENTION_SWEEP_INTERVAL);
    // The first tick fires immediately; consume it since RETENTION_STARTUP_DELAY already
    // served as the initial delay, then run the sweep once before waiting a full interval.
    interval.tick().await;

    loop {
        if let Err(e) = run_log_retention_sweep(&services, retention_days).await {
            tracing::error!(
                error = %e,
                retention_days,
                "App log retention sweep failed: retention_days={}, error={}",
                retention_days,
                e
            );
        }

        tokio::select! {
            _ = &mut shutdown_rx => break,
            _ = interval.tick() => {}
        }
    }
}

/// Compute the RFC3339 cutoff timestamp for `app_log` retention: rows with `timestamp` older
/// than the returned value are outside the retention window and eligible for deletion.
fn retention_cutoff_iso(now: OffsetDateTime, retention_days: u32) -> String {
    let cutoff = now - time::Duration::days(retention_days as i64);
    cutoff
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

/// Delete `app_log` rows older than `retention_days` via the generated `deleteAppLogs`
/// mutation (never raw SQL, per the entity single-source-of-truth rule), logging the
/// deleted count on success.
async fn run_log_retention_sweep(
    services: &Arc<crate::services::manager::ServicesManager>,
    retention_days: u32,
) -> Result<()> {
    let cutoff = retention_cutoff_iso(OffsetDateTime::now_utc(), retention_days);

    let graphql = services
        .get_graphql()
        .await
        .ok_or_else(|| anyhow::anyhow!("graphql service not available"))?;
    let schema = graphql
        .schema()
        .await
        .ok_or_else(|| anyhow::anyhow!("graphql schema not available"))?;

    let auth_user = AuthUser::system_admin();
    let request = Request::new(
        r#"mutation DeleteOldAppLogs($where: AppLogWhereInput!) {
            deleteAppLogs(where: $where) { success error deletedCount }
        }"#,
    )
    .variables(Variables::from_json(serde_json::json!({
        "where": { "timestamp": { "lt": cutoff } }
    })))
    .data(auth_user.clone())
    .data(auth_user.user_id.clone());

    let response = schema.execute(request).await;
    if !response.errors.is_empty() {
        let msg = response
            .errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
            .join("; ");
        anyhow::bail!("deleteAppLogs mutation failed: {msg}");
    }

    let data = serde_json::to_value(&response.data)?;
    let payload = data.get("deleteAppLogs");
    let success = payload
        .and_then(|v| v.get("success"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !success {
        let err = payload
            .and_then(|v| v.get("error"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        anyhow::bail!("deleteAppLogs mutation returned failure: {err}");
    }
    let deleted_count = payload
        .and_then(|v| v.get("deletedCount"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    tracing::info!(
        deleted_count,
        retention_days,
        cutoff = %cutoff,
        "App log retention sweep deleted {} log row(s) older than {} day(s) (cutoff: {})",
        deleted_count,
        retention_days,
        cutoff
    );

    Ok(())
}

fn maybe_log_fd_diag(last_fd_diag_log: &mut Option<Instant>, fd_diag: &str) {
    let now = Instant::now();
    let should_log = last_fd_diag_log
        .map(|last| now.duration_since(last) >= Duration::from_secs(60))
        .unwrap_or(true);
    if should_log {
        tracing::debug!(
            fd_diag = %fd_diag,
            "Open-file diagnostics snapshot during DB log write failures: {}",
            fd_diag
        );
        *last_fd_diag_log = Some(now);
    }
}

fn open_fd_diagnostics() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let open_fd_count = fs::read_dir("/proc/self/fd").ok()?.count();
        let limits = fs::read_to_string("/proc/self/limits").ok()?;
        let mut soft = None::<String>;
        let mut hard = None::<String>;
        for line in limits.lines() {
            if line.starts_with("Max open files") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    soft = Some(parts[3].to_string());
                    hard = Some(parts[4].to_string());
                }
                break;
            }
        }
        let soft = soft.unwrap_or_else(|| "unknown".to_string());
        let hard = hard.unwrap_or_else(|| "unknown".to_string());
        return Some(format!(
            "open_fds={}, max_open_files_soft={}, max_open_files_hard={}",
            open_fd_count, soft, hard
        ));
    }

    #[allow(unreachable_code)]
    None
}

/// Tracing layer that sends events to the logging service (DB + broadcast).
#[derive(Clone)]
pub struct DatabaseLoggingLayer {
    min_level: Level,
    broadcast_tx: broadcast::Sender<LogEvent>,
    db_tx: mpsc::Sender<AppLog>,
}

impl DatabaseLoggingLayer {
    pub fn new(
        min_level: Level,
        broadcast_tx: broadcast::Sender<LogEvent>,
        db_tx: mpsc::Sender<AppLog>,
    ) -> Self {
        Self {
            min_level,
            broadcast_tx,
            db_tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LogEvent> {
        self.broadcast_tx.subscribe()
    }
}

struct FieldVisitor {
    fields: HashMap<String, JsonValue>,
    message: Option<String>,
}

impl FieldVisitor {
    fn new() -> Self {
        Self {
            fields: HashMap::new(),
            message: None,
        }
    }
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let value_str = format!("{:?}", value);
        if field.name() == "message" {
            self.message = Some(value_str);
        } else {
            self.fields
                .insert(field.name().to_string(), JsonValue::String(value_str));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields.insert(
                field.name().to_string(),
                JsonValue::String(value.to_string()),
            );
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), JsonValue::Number(value.into()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), JsonValue::Number(value.into()));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), JsonValue::Bool(value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        if let Some(n) = serde_json::Number::from_f64(value) {
            self.fields
                .insert(field.name().to_string(), JsonValue::Number(n));
        }
    }
}

impl<S> Layer<S> for DatabaseLoggingLayer
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let level = *event.metadata().level();
        if level > self.min_level {
            return;
        }

        let mut visitor = FieldVisitor::new();
        event.record(&mut visitor);

        let message = visitor.message.unwrap_or_default();
        let target = event.metadata().target().to_string();
        let level_str = level.as_str().to_uppercase();

        let span_name = ctx.event_span(event).map(|s| s.name().to_string());
        let span_id = ctx.event_span(event).map(|s| format!("{:?}", s.id()));

        let fields = if visitor.fields.is_empty() {
            None
        } else {
            Some(serde_json::to_value(&visitor.fields).unwrap_or(JsonValue::Null))
        };

        let timestamp = OffsetDateTime::now_utc();
        let timestamp_str = timestamp
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();

        let log_event = LogEvent {
            id: None,
            timestamp: timestamp_str.clone(),
            level: level_str.clone(),
            target: target.clone(),
            message: message.clone(),
            fields: fields.clone(),
            span_name: span_name.clone(),
        };

        let _ = self.broadcast_tx.send(log_event);

        let fields_str = fields.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let app_log = AppLog {
            id: Uuid::new_v4().to_string(),
            timestamp: timestamp_str.clone(),
            level: level_str,
            target,
            message,
            fields: fields_str,
            span_name,
            span_id,
            created_at: timestamp_str,
        };

        let _ = self.db_tx.try_send(app_log);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::{Date, Month, Time};

    fn utc_datetime(year: i32, month: Month, day: u8, hour: u8, minute: u8) -> OffsetDateTime {
        Date::from_calendar_date(year, month, day)
            .expect("valid test date")
            .with_time(Time::from_hms(hour, minute, 0).expect("valid test time"))
            .assume_utc()
    }

    #[test]
    fn retention_cutoff_iso_subtracts_retention_days() {
        let now = utc_datetime(2026, Month::January, 31, 12, 30);
        let cutoff = retention_cutoff_iso(now, 30);
        assert_eq!(cutoff, "2026-01-01T12:30:00Z");
    }

    #[test]
    fn retention_cutoff_iso_zero_days_is_now() {
        let now = utc_datetime(2026, Month::July, 5, 0, 0);
        let cutoff = retention_cutoff_iso(now, 0);
        assert_eq!(cutoff, "2026-07-05T00:00:00Z");
    }

    #[test]
    fn retention_cutoff_iso_crosses_month_and_year_boundary() {
        let now = utc_datetime(2026, Month::January, 10, 0, 0);
        let cutoff = retention_cutoff_iso(now, 15);
        assert_eq!(cutoff, "2025-12-26T00:00:00Z");
    }
}
