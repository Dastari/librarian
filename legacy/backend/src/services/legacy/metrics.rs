//! Metrics collection service for system monitoring
//!
//! Collects CPU, memory, uptime, and request statistics for the TUI dashboard
//! and GraphQL API.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use parking_lot::RwLock;
use sysinfo::System;

/// Maximum history samples to keep for sparkline graphs (2 minutes at 1 sample/sec)
const HISTORY_SIZE: usize = 120;

/// Snapshot of current system metrics
#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    /// Current CPU usage percentage (0-100)
    pub cpu_percent: f32,
    /// Memory currently used in bytes
    pub memory_used: u64,
    /// Total system memory in bytes
    pub memory_total: u64,
    /// Server uptime in seconds
    pub uptime_secs: u64,
    /// CPU usage history (last 60 samples)
    pub cpu_history: Vec<f32>,
    /// Memory usage history as percentage (last 60 samples)
    pub mem_history: Vec<f64>,
    /// Number of active HTTP requests
    pub active_requests: usize,
    /// Total HTTP requests served since startup
    pub total_requests: u64,
}

/// Database connection pool statistics
#[derive(Debug, Clone)]
pub struct DatabaseSnapshot {
    /// Number of active (in-use) connections
    pub active_connections: u32,
    /// Number of idle connections in the pool
    pub idle_connections: u32,
    /// Maximum pool size
    pub max_connections: u32,
}

/// Inner state for MetricsCollector (behind RwLock)
struct MetricsInner {
    /// System info collector
    sys: System,
    /// CPU usage history for sparkline graphs
    cpu_history: VecDeque<f32>,
    /// Memory usage history (as percentage)
    mem_history: VecDeque<f64>,
    /// Last recorded CPU percentage
    last_cpu: f32,
}

/// Service for collecting and exposing system metrics
pub struct MetricsCollector {
    /// Inner state protected by RwLock
    inner: RwLock<MetricsInner>,
    /// Active HTTP request counter
    active_requests: AtomicUsize,
    /// Total request counter
    total_requests: AtomicU64,
    /// Server start time
    start_time: Instant,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        Self {
            inner: RwLock::new(MetricsInner {
                sys,
                cpu_history: VecDeque::with_capacity(HISTORY_SIZE),
                mem_history: VecDeque::with_capacity(HISTORY_SIZE),
                last_cpu: 0.0,
            }),
            active_requests: AtomicUsize::new(0),
            total_requests: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Refresh system metrics (call periodically, e.g., every second)
    pub fn refresh(&self) {
        let mut inner = self.inner.write();

        // Refresh CPU and memory
        inner.sys.refresh_cpu_all();
        inner.sys.refresh_memory();

        // Calculate global CPU usage
        let cpu_usage = inner.sys.global_cpu_usage();
        inner.last_cpu = cpu_usage;

        // Add to history
        if inner.cpu_history.len() >= HISTORY_SIZE {
            inner.cpu_history.pop_front();
        }
        inner.cpu_history.push_back(cpu_usage);

        // Memory as percentage
        let mem_total = inner.sys.total_memory();
        let mem_used = inner.sys.used_memory();
        let mem_percent = if mem_total > 0 {
            (mem_used as f64 / mem_total as f64) * 100.0
        } else {
            0.0
        };

        if inner.mem_history.len() >= HISTORY_SIZE {
            inner.mem_history.pop_front();
        }
        inner.mem_history.push_back(mem_percent);
    }

    /// Get a snapshot of current system metrics
    pub fn snapshot(&self) -> SystemSnapshot {
        let inner = self.inner.read();

        SystemSnapshot {
            cpu_percent: inner.last_cpu,
            memory_used: inner.sys.used_memory(),
            memory_total: inner.sys.total_memory(),
            uptime_secs: self.start_time.elapsed().as_secs(),
            cpu_history: inner.cpu_history.iter().copied().collect(),
            mem_history: inner.mem_history.iter().copied().collect(),
            active_requests: self.active_requests.load(Ordering::Relaxed),
            total_requests: self.total_requests.load(Ordering::Relaxed),
        }
    }

    /// Record that a request has started
    pub fn request_started(&self) {
        self.active_requests.fetch_add(1, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Record that a request has completed
    pub fn request_completed(&self) {
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get current active request count
    pub fn active_request_count(&self) -> usize {
        self.active_requests.load(Ordering::Relaxed)
    }

    /// Get total request count
    pub fn total_request_count(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    /// Get server uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared metrics collector for use across the application
pub type SharedMetrics = Arc<MetricsCollector>;

/// Create a shared metrics collector
pub fn create_metrics_collector() -> SharedMetrics {
    Arc::new(MetricsCollector::new())
}

/// Format bytes as human-readable string for TUI display (e.g., "2.5G")
pub fn format_bytes_short(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;

    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format duration as human-readable uptime string (e.g., "1d 20h 32m")
pub fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes_short() {
        assert_eq!(format_bytes_short(500), "500 B");
        assert_eq!(format_bytes_short(1024), "1.0 KiB");
        assert_eq!(format_bytes_short(1536), "1.5 KiB");
        assert_eq!(format_bytes_short(1048576), "1.0 MiB");
        assert_eq!(format_bytes_short(1073741824), "1.0 GiB");
    }

    #[test]
    fn test_format_uptime() {
        assert_eq!(format_uptime(30), "0m");
        assert_eq!(format_uptime(90), "1m");
        assert_eq!(format_uptime(3661), "1h 1m");
        assert_eq!(format_uptime(90061), "1d 1h 1m");
    }

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        // Test request tracking
        collector.request_started();
        assert_eq!(collector.active_request_count(), 1);
        assert_eq!(collector.total_request_count(), 1);

        collector.request_started();
        assert_eq!(collector.active_request_count(), 2);
        assert_eq!(collector.total_request_count(), 2);

        collector.request_completed();
        assert_eq!(collector.active_request_count(), 1);
        assert_eq!(collector.total_request_count(), 2);

        // Test snapshot
        let snapshot = collector.snapshot();
        assert!(snapshot.memory_total > 0);
        assert!(snapshot.uptime_secs < 10); // Should be very recent
    }
}
