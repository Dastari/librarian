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

pub use app::{TuiApp, TuiConfig};
pub use layer::create_tui_layer;

use std::io::{self, IsTerminal};

/// Check if the TUI should be used based on terminal detection
pub fn should_use_tui() -> bool {
    // Check if stdout is a terminal (not piped/Docker)
    io::stdout().is_terminal()
        // And stdin is also a terminal (allows keyboard input)
        && io::stdin().is_terminal()
        // And not explicitly disabled via environment variable
        && std::env::var("LIBRARIAN_HEADLESS").is_err()
}
