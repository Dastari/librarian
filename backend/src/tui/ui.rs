//! UI layout and rendering

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::tui::panels::Panel;
use crate::tui::theme::Theme;

/// Panel identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelId {
    Logs,
    Torrents,
    System,
    Users,
    Database,
}

impl PanelId {
    pub const ALL: [PanelId; 5] = [
        PanelId::Logs,
        PanelId::Torrents,
        PanelId::System,
        PanelId::Users,
        PanelId::Database,
    ];

    pub fn index(self) -> usize {
        match self {
            PanelId::Logs => 0,
            PanelId::Torrents => 1,
            PanelId::System => 2,
            PanelId::Users => 3,
            PanelId::Database => 4,
        }
    }

    pub fn from_index(index: usize) -> Option<PanelId> {
        match index {
            0 => Some(PanelId::Logs),
            1 => Some(PanelId::Torrents),
            2 => Some(PanelId::System),
            3 => Some(PanelId::Users),
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
    /// Layout:
    /// ```text
    /// ┌─ Logs ──────────────────────────────────────────────────────────────┐
    /// │                                                                      │
    /// │                                                                      │
    /// └──────────────────────────────────────────────────────────────────────┘
    /// ┌─ Torrents ─────────────────────┐┌─ System ───────────────────────────┐
    /// │                                ││                                    │
    /// └────────────────────────────────┘└────────────────────────────────────┘
    /// ┌─ Users ────────────────────────┐┌─ Database ─────────────────────────┐
    /// │                                ││                                    │
    /// └────────────────────────────────┘└────────────────────────────────────┘
    /// [Tab] Switch Panel  [q] Quit  [Space] Pause  [/] Search
    /// ```
    pub fn calculate_areas(&self, area: Rect) -> PanelAreas {
        // Reserve 1 line for status bar
        let main_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height.saturating_sub(1),
        };

        let status_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: 1,
        };

        // Split vertically: logs (larger), then two rows of panels
        let vertical = Layout::vertical([
            Constraint::Percentage(50), // Logs
            Constraint::Percentage(25), // Torrents + System
            Constraint::Percentage(25), // Users + Database
        ])
        .split(main_area);

        let logs_area = vertical[0];

        // Split middle row horizontally
        let middle_row = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(vertical[1]);

        let torrents_area = middle_row[0];
        let system_area = middle_row[1];

        // Split bottom row horizontally
        let bottom_row = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(vertical[2]);

        let users_area = bottom_row[0];
        let database_area = bottom_row[1];

        PanelAreas {
            logs: logs_area,
            torrents: torrents_area,
            system: system_area,
            users: users_area,
            database: database_area,
            status: status_area,
        }
    }

    /// Render the status bar
    pub fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let keybinds = vec![
            ("Tab", "Switch Panel"),
            ("q", "Quit"),
            ("Space", "Pause"),
            ("↑↓", "Scroll"),
            ("c", "Clear"),
            ("r", "Refresh"),
        ];

        let mut spans = Vec::new();
        for (i, (key, desc)) in keybinds.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("  ", Theme::dim()));
            }
            spans.push(Span::styled(format!("[{}]", key), Theme::keybind_key()));
            spans.push(Span::styled(format!(" {}", desc), Theme::keybind()));
        }

        let status = Paragraph::new(Line::from(spans));
        frame.render_widget(status, area);
    }
}

/// Calculated areas for each panel
pub struct PanelAreas {
    pub logs: Rect,
    pub torrents: Rect,
    pub system: Rect,
    pub users: Rect,
    pub database: Rect,
    pub status: Rect,
}

/// Render all panels
pub fn render_panels(
    frame: &mut Frame,
    layout: &UiLayout,
    areas: &PanelAreas,
    logs_panel: &dyn Panel,
    torrents_panel: &dyn Panel,
    system_panel: &dyn Panel,
    users_panel: &dyn Panel,
    database_panel: &dyn Panel,
) {
    logs_panel.render(frame, areas.logs, layout.focused == PanelId::Logs);
    torrents_panel.render(frame, areas.torrents, layout.focused == PanelId::Torrents);
    system_panel.render(frame, areas.system, layout.focused == PanelId::System);
    users_panel.render(frame, areas.users, layout.focused == PanelId::Users);
    database_panel.render(frame, areas.database, layout.focused == PanelId::Database);
    layout.render_status_bar(frame, areas.status);
}
