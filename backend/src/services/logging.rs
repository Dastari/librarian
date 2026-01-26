//! Logging service: database persistence and real-time log subscriptions.
//!
//! Implements [Service](crate::services::manager::Service) and depends on the database service.
//! The DB tracing layer is added as [OptionalDbLayer] in main (single subscriber init); the
//! logging service injects/removes the inner layer via [LoggingServiceConfig::db_layer_state]
//! when it starts/stops.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use time::OffsetDateTime;
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;
use uuid::Uuid;

use crate::db::operations;
use crate::db::Database;
use crate::services::graphql::entities::AppLog;
use crate::services::manager::{Service, ServiceHealth};

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
        if let Ok(guard) = self.0.lock() {
            if let Some(ref inner) = *guard {
                inner.on_event(event, ctx);
            }
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
}

impl std::fmt::Debug for LoggingServiceConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoggingServiceConfig")
            .field("min_level", &self.min_level)
            .field("batch_size", &self.batch_size)
            .field("flush_interval_ms", &self.flush_interval_ms)
            .field("broadcast_capacity", &self.broadcast_capacity)
            .field("db_layer_state", &self.db_layer_state.as_ref().map(|_| "..."))
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
        }
    }
}

/// Alias for backward compatibility.
pub type DatabaseLoggerConfig = LoggingServiceConfig;

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
        vec!["database".to_string()]
    }

    async fn start(&self) -> Result<()> {
        tracing::info!(service = "logging", "Logging service starting");
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

        tracing::info!(service = "logging", "Logging service started");
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
        tracing::info!(service = "logging", "Stopped");
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        if self.layer.read().is_some() {
            Ok(ServiceHealth::healthy())
        } else {
            Ok(ServiceHealth::unhealthy(
                "logging layer not initialized (start not called)",
            ))
        }
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

    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            Some(log) = rx.recv() => {
                batch.push(log);
                if batch.len() >= batch_size {
                    if let Err(e) = operations::insert_app_logs_batch(&pool, &batch).await {
                        tracing::error!(error = %e, "Failed to write logs to database");
                    }
                    batch.clear();
                }
            }
            _ = interval.tick() => {
                if !batch.is_empty() {
                    if let Err(e) = operations::insert_app_logs_batch(&pool, &batch).await {
                        tracing::error!(error = %e, "Failed to write logs to database");
                    }
                    batch.clear();
                }
            }
        }
    }
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
