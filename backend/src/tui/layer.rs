//! Custom tracing layer for TUI mode
//!
//! Instead of writing to stdout, this layer forwards log events
//! to a channel that the TUI can display.

use std::collections::HashMap;

use serde_json::Value as JsonValue;
use time::OffsetDateTime;
use tokio::sync::broadcast;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

use crate::services::LogEvent;

/// Custom tracing layer that sends logs to the TUI via broadcast channel
pub struct TuiLoggingLayer {
    min_level: Level,
    tx: broadcast::Sender<LogEvent>,
}

impl TuiLoggingLayer {
    /// Create a new TUI logging layer
    pub fn new(min_level: Level, tx: broadcast::Sender<LogEvent>) -> Self {
        Self { min_level, tx }
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

impl<S> Layer<S> for TuiLoggingLayer
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

        // Create log event
        let log_event = LogEvent {
            id: None,
            timestamp: timestamp_str,
            level: level_str,
            target,
            message,
            fields,
            span_name,
        };

        // Send to TUI (non-blocking, ignore errors if no receivers)
        let _ = self.tx.send(log_event);
    }
}

/// Create a TUI logging layer
pub fn create_tui_layer(min_level: Level) -> (TuiLoggingLayer, broadcast::Receiver<LogEvent>) {
    let (tx, rx) = broadcast::channel(1000);
    (TuiLoggingLayer::new(min_level, tx), rx)
}
