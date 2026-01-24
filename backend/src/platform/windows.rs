//! Windows-only stubs for service and tray mode wiring.

use anyhow::{anyhow, Result};

use crate::app_mode::RunMode;

pub fn ensure_mode_supported(run_mode: RunMode) -> Result<()> {
    match run_mode {
        RunMode::Tray => Err(anyhow!("Tray mode is not wired yet.")),
        RunMode::Service => Err(anyhow!("Service mode is not wired yet.")),
        RunMode::Server => Ok(()),
    }
}
