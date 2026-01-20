//! TUI panel components
//!
//! Each panel renders a specific section of the dashboard.

mod database;
mod logs;
mod system;
mod torrents;
pub mod users;

pub use database::DatabasePanel;
pub use logs::LogsPanel;
pub use system::SystemPanel;
pub use torrents::{TorrentsPanel, spawn_torrent_updater};
pub use users::UsersPanel;

use ratatui::Frame;
use ratatui::layout::Rect;

use crate::tui::input::Action;

/// Trait for TUI panels
pub trait Panel {
    /// Get the panel title
    fn title(&self) -> &str;

    /// Render the panel content
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool);

    /// Handle an action when this panel is focused
    fn handle_action(&mut self, action: &Action);

    /// Update panel data (called on tick)
    fn update(&mut self);

    /// Get current scroll position (for status display)
    fn scroll_position(&self) -> Option<(usize, usize)> {
        None
    }
}
