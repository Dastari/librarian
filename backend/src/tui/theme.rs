//! Theme and color definitions for the TUI

use ratatui::style::{Color, Modifier, Style};

/// Color theme for the TUI dashboard
pub struct Theme;

impl Theme {
    // Base colors
    pub const BG: Color = Color::Reset;
    pub const FG: Color = Color::White;
    pub const DIM: Color = Color::DarkGray;
    pub const BORDER: Color = Color::Rgb(60, 60, 60);

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

    // Graph colors - pastel versions
    pub const CPU_GRAPH: Color = Color::Rgb(199, 210, 254); // Indigo-200 pastel
    pub const MEM_GRAPH: Color = Color::Rgb(167, 243, 208); // Emerald-200 pastel

    // Panel border colors - unique pastel for each panel
    pub const BORDER_LOGS: Color = Color::Rgb(147, 197, 253); // Blue pastel
    pub const BORDER_TORRENTS: Color = Color::Rgb(196, 181, 253); // Purple pastel
    pub const BORDER_SYSTEM: Color = Color::Rgb(167, 243, 208); // Green pastel
    pub const BORDER_LIBRARIES: Color = Color::Rgb(253, 230, 138); // Amber pastel
    pub const BORDER_DATABASE: Color = Color::Rgb(252, 165, 165); // Red pastel

    // Panel title colors (same as borders but slightly brighter)
    pub const TITLE_LOGS: Color = Color::Rgb(96, 165, 250); // Blue
    pub const TITLE_TORRENTS: Color = Color::Rgb(167, 139, 250); // Purple
    pub const TITLE_SYSTEM: Color = Color::Rgb(52, 211, 153); // Green
    pub const TITLE_LIBRARIES: Color = Color::Rgb(251, 191, 36); // Amber
    pub const TITLE_DATABASE: Color = Color::Rgb(248, 113, 113); // Red

    /// Style for normal text
    pub fn text() -> Style {
        Style::default().fg(Self::FG)
    }

    /// Style for dimmed/secondary text
    pub fn dim() -> Style {
        Style::default().fg(Self::DIM)
    }

    /// Style for panel borders based on panel kind
    pub fn border(panel: PanelKind) -> Style {
        let color = match panel {
            PanelKind::Logs => Self::BORDER_LOGS,
            PanelKind::Torrents => Self::BORDER_TORRENTS,
            PanelKind::System => Self::BORDER_SYSTEM,
            PanelKind::Libraries => Self::BORDER_LIBRARIES,
            PanelKind::Database => Self::BORDER_DATABASE,
        };
        Style::default().fg(color)
    }

    /// Style for panel borders (dimmed when not focused)
    pub fn border_dim() -> Style {
        Style::default().fg(Self::BORDER)
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
            PanelKind::Libraries => Self::TITLE_LIBRARIES,
            PanelKind::Database => Self::TITLE_DATABASE,
        };
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    }

    /// Style for panel number indicator (superscript)
    pub fn panel_number() -> Style {
        Style::default().fg(Color::Rgb(156, 163, 175)) // Gray-400
    }

    /// Style for progress bar (complete/seeding)
    pub fn progress_complete() -> Style {
        Style::default().fg(Self::SUCCESS)
    }

    /// Style for progress bar (downloading/checking - blue)
    pub fn progress_active() -> Style {
        Style::default().fg(Color::Rgb(100, 149, 237)) // Cornflower blue
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
        Style::default().fg(Color::Rgb(156, 163, 175)) // Gray-400
    }

    /// Style for keyboard shortcut key (pastel red for action keys)
    pub fn keybind_key() -> Style {
        Style::default().fg(Color::Rgb(251, 146, 146)) // Pastel red
    }

    /// Style for title decorators (┓ ┍)
    pub fn title_decorator(panel: PanelKind) -> Style {
        Self::border(panel)
    }

    /// Download graph color (magenta/pink like btop)
    pub fn graph_download() -> Style {
        Style::default().fg(Color::Rgb(236, 72, 153)) // Pink-500
    }

    /// Upload graph color (purple like btop)  
    pub fn graph_upload() -> Style {
        Style::default().fg(Color::Rgb(139, 92, 246)) // Violet-500
    }
}

/// Panel types for theming
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelKind {
    Logs,
    Torrents,
    System,
    Libraries,
    Database,
}

impl PanelKind {
    /// Get the superscript number for this panel (for btop-style ¹panel display)
    pub fn superscript(&self) -> &'static str {
        match self {
            PanelKind::Logs => "¹",
            PanelKind::Torrents => "²",
            PanelKind::System => "³",
            PanelKind::Libraries => "⁴",
            PanelKind::Database => "⁵",
        }
    }

    /// Get the number key for this panel
    pub fn key(&self) -> char {
        match self {
            PanelKind::Logs => '1',
            PanelKind::Torrents => '2',
            PanelKind::System => '3',
            PanelKind::Libraries => '4',
            PanelKind::Database => '5',
        }
    }
}
