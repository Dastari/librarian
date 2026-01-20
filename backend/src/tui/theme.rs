//! Theme and color definitions for the TUI

use ratatui::style::{Color, Modifier, Style};

/// Color theme for the TUI dashboard
pub struct Theme;

impl Theme {
    // Base colors
    pub const BG: Color = Color::Reset;
    pub const FG: Color = Color::White;
    pub const DIM: Color = Color::DarkGray;
    pub const BORDER: Color = Color::Rgb(80, 80, 80);
    pub const BORDER_FOCUSED: Color = Color::Rgb(100, 149, 237); // Cornflower blue

    // Log level colors
    pub const TRACE: Color = Color::DarkGray;
    pub const DEBUG: Color = Color::Gray;
    pub const INFO: Color = Color::Rgb(96, 165, 250); // Blue-400
    pub const WARN: Color = Color::Rgb(251, 191, 36); // Amber-400
    pub const ERROR: Color = Color::Rgb(248, 113, 113); // Red-400

    // Status colors
    pub const SUCCESS: Color = Color::Rgb(74, 222, 128); // Green-400
    pub const PROGRESS: Color = Color::Rgb(96, 165, 250); // Blue-400
    pub const PAUSED: Color = Color::Rgb(251, 191, 36); // Amber-400

    // Graph colors
    pub const CPU_GRAPH: Color = Color::Rgb(129, 140, 248); // Indigo-400
    pub const MEM_GRAPH: Color = Color::Rgb(52, 211, 153); // Emerald-400

    // Panel title colors
    pub const TITLE_LOGS: Color = Color::Rgb(96, 165, 250); // Blue
    pub const TITLE_TORRENTS: Color = Color::Rgb(167, 139, 250); // Purple
    pub const TITLE_SYSTEM: Color = Color::Rgb(52, 211, 153); // Green
    pub const TITLE_USERS: Color = Color::Rgb(251, 191, 36); // Amber
    pub const TITLE_DATABASE: Color = Color::Rgb(248, 113, 113); // Red

    /// Style for normal text
    pub fn text() -> Style {
        Style::default().fg(Self::FG)
    }

    /// Style for dimmed/secondary text
    pub fn dim() -> Style {
        Style::default().fg(Self::DIM)
    }

    /// Style for panel borders (unfocused)
    pub fn border() -> Style {
        Style::default().fg(Self::BORDER)
    }

    /// Style for panel borders (focused)
    pub fn border_focused() -> Style {
        Style::default().fg(Self::BORDER_FOCUSED)
    }

    /// Style for log level
    pub fn log_level(level: &str) -> Style {
        let color = match level.to_uppercase().as_str() {
            "TRACE" => Self::TRACE,
            "DEBUG" => Self::DEBUG,
            "INFO" => Self::INFO,
            "WARN" | "WARNING" => Self::WARN,
            "ERROR" => Self::ERROR,
            _ => Self::FG,
        };
        Style::default().fg(color)
    }

    /// Style for log level badge (inverted)
    pub fn log_level_badge(level: &str) -> Style {
        let color = match level.to_uppercase().as_str() {
            "TRACE" => Self::TRACE,
            "DEBUG" => Self::DEBUG,
            "INFO" => Self::INFO,
            "WARN" | "WARNING" => Self::WARN,
            "ERROR" => Self::ERROR,
            _ => Self::FG,
        };
        Style::default().fg(Color::Black).bg(color)
    }

    /// Style for selected/highlighted items
    pub fn selected() -> Style {
        Style::default()
            .bg(Color::Rgb(55, 65, 81)) // Gray-700
            .add_modifier(Modifier::BOLD)
    }

    /// Style for panel titles
    pub fn panel_title(panel: PanelKind) -> Style {
        let color = match panel {
            PanelKind::Logs => Self::TITLE_LOGS,
            PanelKind::Torrents => Self::TITLE_TORRENTS,
            PanelKind::System => Self::TITLE_SYSTEM,
            PanelKind::Users => Self::TITLE_USERS,
            PanelKind::Database => Self::TITLE_DATABASE,
        };
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    }

    /// Style for progress bar (complete portion)
    pub fn progress_complete() -> Style {
        Style::default().fg(Self::SUCCESS)
    }

    /// Style for progress bar (incomplete portion)
    pub fn progress_incomplete() -> Style {
        Style::default().fg(Self::DIM)
    }

    /// Style for sparkline graphs
    pub fn sparkline_cpu() -> Style {
        Style::default().fg(Self::CPU_GRAPH)
    }

    /// Style for memory sparkline
    pub fn sparkline_mem() -> Style {
        Style::default().fg(Self::MEM_GRAPH)
    }

    /// Style for keyboard shortcut hints
    pub fn keybind() -> Style {
        Style::default()
            .fg(Color::Rgb(156, 163, 175)) // Gray-400
    }

    /// Style for keyboard shortcut key
    pub fn keybind_key() -> Style {
        Style::default()
            .fg(Color::Rgb(96, 165, 250)) // Blue-400
            .add_modifier(Modifier::BOLD)
    }
}

/// Panel types for theming
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelKind {
    Logs,
    Torrents,
    System,
    Users,
    Database,
}
