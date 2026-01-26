//! Terminal User Interface (TUI) dashboard for Librarian
//!
//! Provides a btop-style interactive console interface showing:
//! - Live log stream with filtering
//! - Active torrent progress
//! - System metrics (CPU, memory, uptime)
//! - Active user sessions
//! - Database connection stats

mod app;
mod input;
mod layer;
mod panels;
mod theme;
mod ui;

use std::io::{self, IsTerminal};

pub use app::{TuiApp, TuiConfig};
pub use layer::create_tui_layer;

/// Check if the TUI should be used based on terminal detection and env overrides.
pub fn should_use_tui() -> bool {
    // Explicitly disabled (e.g. Docker, CI)
    if std::env::var("LIBRARIAN_HEADLESS").is_ok() {
        return false;
    }
    // Explicitly enabled (e.g. LIBRARIAN_TUI=1)
    if std::env::var("LIBRARIAN_TUI").as_deref() == Ok("1") {
        return io::stdout().is_terminal();
    }
    // Auto: stdout and stdin must be a terminal (allows keyboard input)
    io::stdout().is_terminal() && io::stdin().is_terminal()
}
