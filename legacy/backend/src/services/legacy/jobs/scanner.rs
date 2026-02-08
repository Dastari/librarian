//! Library scanner job
//!
//! This module provides the scheduled scan functionality.
//! The actual scanning logic is in services::scanner.

use std::sync::Arc;

use anyhow::Result;

use crate::services::ScannerService;

/// Run a full library scan for all libraries with auto_scan enabled
pub async fn run_scan(scanner: Arc<ScannerService>) -> Result<()> {
    tracing::info!("Starting scheduled library scan");
    scanner.scan_all_libraries().await?;
    tracing::info!("Scheduled library scan completed");
    Ok(())
}
