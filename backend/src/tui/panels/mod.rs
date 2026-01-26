//! TUI panel components
//!
//! Each panel renders a specific section of the dashboard.

mod database;
mod libraries;
mod logs;
mod system;
mod torrents;

pub use database::{DatabasePanel, create_shared_table_counts};
pub use libraries::{LibrariesPanel, create_shared_libraries};
pub use logs::LogsPanel;
pub use system::SystemPanel;
pub use torrents::{TorrentsPanel, create_shared_torrents};

use ratatui::Frame;
use ratatui::layout::Rect;

use crate::tui::input::Action;
use crate::tui::theme::PanelKind;

/// Trait for TUI panels
pub trait Panel {
    /// Get the panel title (short name like "logs", "net")
    fn title(&self) -> &str;

    /// Get the panel kind for theming
    fn kind(&self) -> PanelKind;

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

    /// Check if panel is visible (for toggle functionality)
    fn is_visible(&self) -> bool {
        true
    }
}
