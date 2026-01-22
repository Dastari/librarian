//! Logging service for database persistence and real-time subscriptions
//!
//! This module provides:
//! - A custom tracing layer that captures logs and stores them in the database
//! - A broadcast channel for real-time log subscriptions (for GraphQL)
//! - Batched database writes for performance

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;
use time::OffsetDateTime;

#[cfg(feature = "postgres")]
type DbPool = PgPool;
#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;
use tokio::sync::{RwLock, broadcast, mpsc};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

use crate::db::{CreateLog, LogsRepository};

/// Log event for broadcasting to subscribers
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

/// Configuration for the database logging layer
#[derive(Debug, Clone)]
pub struct DatabaseLoggerConfig {
    /// Minimum level to log to database (default: INFO)
    pub min_level: Level,
    /// Batch size for database writes (default: 50)
    pub batch_size: usize,
    /// Flush interval in milliseconds (default: 1000)
    pub flush_interval_ms: u64,
    /// Broadcast channel capacity (default: 1000)
    pub broadcast_capacity: usize,
}

impl Default for DatabaseLoggerConfig {
    fn default() -> Self {
        Self {
            min_level: Level::INFO,
            batch_size: 50,
            flush_interval_ms: 1000,
            broadcast_capacity: 1000,
        }
    }
}

/// Logging service that manages database persistence and subscriptions
pub struct LoggingService {
    /// Broadcast sender for real-time subscriptions
    broadcast_tx: broadcast::Sender<LogEvent>,
    /// Channel sender for batched database writes
    db_tx: mpsc::Sender<CreateLog>,
}

impl LoggingService {
    /// Create a new logging service
    pub fn new(pool: DbPool, config: DatabaseLoggerConfig) -> Self {
        let (broadcast_tx, _) = broadcast::channel(config.broadcast_capacity);
        let (db_tx, db_rx) = mpsc::channel::<CreateLog>(config.batch_size * 10);

        // Start the database writer task
        let logs_repo = LogsRepository::new(pool);
        tokio::spawn(database_writer_task(
            db_rx,
            logs_repo,
            config.batch_size,
            config.flush_interval_ms,
        ));

        Self {
            broadcast_tx,
            db_tx,
        }
    }

    /// Subscribe to real-time log events - for GraphQL subscriptions
    #[allow(dead_code)]
    pub fn subscribe(&self) -> broadcast::Receiver<LogEvent> {
        self.broadcast_tx.subscribe()
    }

    /// Get the broadcast sender for the tracing layer
    pub fn broadcast_sender(&self) -> broadcast::Sender<LogEvent> {
        self.broadcast_tx.clone()
    }

    /// Get the database sender for the tracing layer
    pub fn db_sender(&self) -> mpsc::Sender<CreateLog> {
        self.db_tx.clone()
    }
}

/// Background task that batches and writes logs to the database
async fn database_writer_task(
    mut rx: mpsc::Receiver<CreateLog>,
    logs_repo: LogsRepository,
    batch_size: usize,
    flush_interval_ms: u64,
) {
    let mut batch: Vec<CreateLog> = Vec::with_capacity(batch_size);
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(flush_interval_ms));

    loop {
        tokio::select! {
            // Receive a log entry
            Some(log) = rx.recv() => {
                batch.push(log);

                // Flush if batch is full
                if batch.len() >= batch_size {
                    if let Err(e) = logs_repo.create_batch(std::mem::take(&mut batch)).await {
                        eprintln!("Failed to write logs to database: {}", e);
                    }
                    batch = Vec::with_capacity(batch_size);
                }
            }
            // Periodic flush
            _ = interval.tick() => {
                if !batch.is_empty() {
                    if let Err(e) = logs_repo.create_batch(std::mem::take(&mut batch)).await {
                        eprintln!("Failed to write logs to database: {}", e);
                    }
                    batch = Vec::with_capacity(batch_size);
                }
            }
        }
    }
}

/// Custom tracing layer that captures logs for database storage and broadcasting
pub struct DatabaseLoggingLayer {
    min_level: Level,
    broadcast_tx: broadcast::Sender<LogEvent>,
    db_tx: mpsc::Sender<CreateLog>,
}

impl DatabaseLoggingLayer {
    pub fn new(
        min_level: Level,
        broadcast_tx: broadcast::Sender<LogEvent>,
        db_tx: mpsc::Sender<CreateLog>,
    ) -> Self {
        Self {
            min_level,
            broadcast_tx,
            db_tx,
        }
    }
}

/// Helper to extract fields from a tracing event
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
        // Check minimum level
        let level = *event.metadata().level();
        if level > self.min_level {
            return;
        }

        // Extract fields from the event
        let mut visitor = FieldVisitor::new();
        event.record(&mut visitor);

        let message = visitor.message.unwrap_or_default();
        let target = event.metadata().target().to_string();
        let level_str = level.as_str().to_uppercase();

        // Get span info if available
        let span_name = ctx.event_span(event).map(|s| s.name().to_string());
        let span_id = ctx.event_span(event).map(|s| format!("{:?}", s.id()));

        // Convert fields to JSON
        let fields = if visitor.fields.is_empty() {
            None
        } else {
            Some(serde_json::to_value(&visitor.fields).unwrap_or(JsonValue::Null))
        };

        let timestamp = OffsetDateTime::now_utc();
        let timestamp_str = timestamp
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();

        // Create log event for broadcasting
        let log_event = LogEvent {
            id: None,
            timestamp: timestamp_str.clone(),
            level: level_str.clone(),
            target: target.clone(),
            message: message.clone(),
            fields: fields.clone(),
            span_name: span_name.clone(),
        };

        // Broadcast to subscribers (non-blocking)
        let _ = self.broadcast_tx.send(log_event);

        // Send to database writer (non-blocking)
        let create_log = CreateLog {
            level: level_str,
            target,
            message,
            fields,
            span_name,
            span_id,
        };

        let _ = self.db_tx.try_send(create_log);
    }
}

/// Shared state for the logging service (thread-safe)
pub type SharedLoggingService = Arc<RwLock<Option<LoggingService>>>;

/// Global logging service accessor
static LOGGING_SERVICE: std::sync::OnceLock<SharedLoggingService> = std::sync::OnceLock::new();

/// Initialize the global logging service - for future global access pattern
#[allow(dead_code)]
pub fn init_logging_service(
    pool: DbPool,
    config: DatabaseLoggerConfig,
) -> &'static SharedLoggingService {
    LOGGING_SERVICE.get_or_init(|| {
        let service = LoggingService::new(pool, config);
        Arc::new(RwLock::new(Some(service)))
    })
}

/// Get the global logging service - for future global access pattern
#[allow(dead_code)]
pub fn get_logging_service() -> Option<&'static SharedLoggingService> {
    LOGGING_SERVICE.get()
}

/// Create a database logging layer for use with tracing_subscriber
/// Returns (layer, broadcast_sender) - the sender is needed for GraphQL subscriptions
pub fn create_database_layer(
    pool: DbPool,
    config: DatabaseLoggerConfig,
) -> (DatabaseLoggingLayer, broadcast::Sender<LogEvent>) {
    let min_level = config.min_level;
    let service = LoggingService::new(pool, config);
    let broadcast_sender = service.broadcast_sender();
    let layer =
        DatabaseLoggingLayer::new(min_level, service.broadcast_sender(), service.db_sender());
    (layer, broadcast_sender)
}
