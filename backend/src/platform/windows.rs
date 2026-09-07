//! Explicit qualification gate for service and tray modes.

use anyhow::{Result, anyhow};

use crate::app_mode::RunMode;

pub fn ensure_mode_supported(run_mode: RunMode) -> Result<()> {
    match run_mode {
        RunMode::Tray => Err(anyhow!(
            "Tray mode is not supported in this release; run with --server."
        )),
        RunMode::Service => Err(anyhow!(
            "Service mode is not supported in this release; use an external service manager with --server."
        )),
        RunMode::Server => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_qualified_server_mode_is_accepted() {
        assert!(ensure_mode_supported(RunMode::Server).is_ok());
        assert!(ensure_mode_supported(RunMode::Tray).is_err());
        assert!(ensure_mode_supported(RunMode::Service).is_err());
    }
}
