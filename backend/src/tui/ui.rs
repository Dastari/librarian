//! UI layout and rendering

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};

use crate::tui::panels::Panel;

/// Panel identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelId {
    Logs,
    Torrents,
    System,
    Libraries,
    Database,
}

impl PanelId {
    pub const ALL: [PanelId; 5] = [
        PanelId::Logs,
        PanelId::Torrents,
        PanelId::System,
        PanelId::Libraries,
        PanelId::Database,
    ];

    pub fn index(self) -> usize {
        match self {
            PanelId::Logs => 0,
            PanelId::Torrents => 1,
            PanelId::System => 2,
            PanelId::Libraries => 3,
            PanelId::Database => 4,
        }
    }

    pub fn from_index(index: usize) -> Option<PanelId> {
        match index {
            0 => Some(PanelId::Logs),
            1 => Some(PanelId::Torrents),
            2 => Some(PanelId::System),
            3 => Some(PanelId::Libraries),
            4 => Some(PanelId::Database),
            _ => None,
        }
    }
}

/// Layout configuration
pub struct UiLayout {
    /// Currently focused panel
    pub focused: PanelId,
}

impl Default for UiLayout {
    fn default() -> Self {
        Self {
            focused: PanelId::Logs,
        }
    }
}

impl UiLayout {
    /// Focus the next panel
    pub fn focus_next(&mut self) {
        let next_index = (self.focused.index() + 1) % PanelId::ALL.len();
        self.focused = PanelId::from_index(next_index).unwrap_or(PanelId::Logs);
    }

    /// Focus the previous panel
    pub fn focus_prev(&mut self) {
        let prev_index = if self.focused.index() == 0 {
            PanelId::ALL.len() - 1
        } else {
            self.focused.index() - 1
        };
        self.focused = PanelId::from_index(prev_index).unwrap_or(PanelId::Logs);
    }

    /// Focus a specific panel by index
    pub fn focus_panel(&mut self, index: usize) {
        if let Some(panel) = PanelId::from_index(index) {
            self.focused = panel;
        }
    }

    /// Calculate panel areas for the given frame size
    ///
    /// Layout (5 panels):
    /// ```text
    /// ┌─ ¹logs ──────────────────────────────────────────────────────────────┐
    /// │                                                                       │
    /// └───────────────────────────────────────────────────────────────────────┘
    /// ┌─ ²torrents ────────────────────────────────┐┌─ ³sys ─────────────────┐
    /// │                                            ││                        │
    /// └────────────────────────────────────────────┘└────────────────────────┘
    /// ┌─ ⁴libs ────────────────────────────────────┐┌─ ⁵db ──────────────────┐
    /// │                                            ││                        │
    /// └────────────────────────────────────────────┘└────────────────────────┘
    /// ```
    pub fn calculate_areas(&self, area: Rect) -> PanelAreas {
        let vertical = Layout::vertical([
            Constraint::Percentage(45), // Logs
            Constraint::Percentage(28), // Torrents + System
            Constraint::Percentage(27), // Libraries + Database
        ])
        .split(area);

        let logs_area = vertical[0];

        // Middle row: Torrents (75%), System (25%)
        let middle_row =
            Layout::horizontal([Constraint::Percentage(75), Constraint::Percentage(25)])
                .split(vertical[1]);

        let torrents_area = middle_row[0];
        let system_area = middle_row[1];

        // Bottom row: Libraries (75%), Database (25%)
        let bottom_row =
            Layout::horizontal([Constraint::Percentage(75), Constraint::Percentage(25)])
                .split(vertical[2]);

        let libraries_area = bottom_row[0];
        let database_area = bottom_row[1];

        PanelAreas {
            logs: logs_area,
            torrents: torrents_area,
            system: system_area,
            libraries: libraries_area,
            database: database_area,
        }
    }
}

/// Calculated areas for each panel
pub struct PanelAreas {
    pub logs: Rect,
    pub torrents: Rect,
    pub system: Rect,
    pub libraries: Rect,
    pub database: Rect,
}

/// Render all panels
pub fn render_panels(
    frame: &mut Frame,
    layout: &UiLayout,
    areas: &PanelAreas,
    logs_panel: &dyn Panel,
    torrents_panel: &dyn Panel,
    system_panel: &dyn Panel,
    libraries_panel: &dyn Panel,
    database_panel: &dyn Panel,
) {
    logs_panel.render(frame, areas.logs, layout.focused == PanelId::Logs);
    torrents_panel.render(frame, areas.torrents, layout.focused == PanelId::Torrents);
    system_panel.render(frame, areas.system, layout.focused == PanelId::System);
    libraries_panel.render(frame, areas.libraries, layout.focused == PanelId::Libraries);
    database_panel.render(frame, areas.database, layout.focused == PanelId::Database);
}
