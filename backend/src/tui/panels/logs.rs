//! Logs panel - displays live log stream with filtering

use std::collections::VecDeque;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use tokio::sync::broadcast;

use crate::services::LogEvent;
use crate::tui::input::Action;
use crate::tui::panels::Panel;
use crate::tui::theme::{PanelKind, Theme};

/// Maximum number of log entries to keep
const MAX_LOGS: usize = 1000;

/// A log entry with parsed display info
#[derive(Debug, Clone)]
struct LogLine {
    timestamp: String,
    level: String,
    target: String,
    message: String,
}

impl From<LogEvent> for LogLine {
    fn from(event: LogEvent) -> Self {
        // Parse timestamp to short format (HH:MM:SS)
        let timestamp = event
            .timestamp
            .split('T')
            .nth(1)
            .and_then(|t| t.split('.').next())
            .unwrap_or(&event.timestamp)
            .to_string();

        // Simplify target (keep last 2 parts)
        let target = {
            let parts: Vec<&str> = event.target.split("::").collect();
            if parts.len() <= 2 {
                event.target.clone()
            } else {
                parts[parts.len() - 2..].join("::")
            }
        };

        Self {
            timestamp,
            level: event.level,
            target,
            message: event.message,
        }
    }
}

/// Logs panel showing live log stream
pub struct LogsPanel {
    /// Log entries
    logs: VecDeque<LogLine>,
    /// List state for scrolling
    list_state: ListState,
    /// Whether auto-scroll is enabled (follow tail)
    auto_scroll: bool,
    /// Whether the log stream is paused
    paused: bool,
    /// Log level filter (None = all)
    level_filter: Option<String>,
    /// Search filter
    search_filter: Option<String>,
    /// Receiver for log events
    log_rx: broadcast::Receiver<LogEvent>,
}

impl LogsPanel {
    /// Create a new logs panel
    pub fn new(log_rx: broadcast::Receiver<LogEvent>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            logs: VecDeque::with_capacity(MAX_LOGS),
            list_state,
            auto_scroll: true,
            paused: false,
            level_filter: None,
            search_filter: None,
            log_rx,
        }
    }

    /// Get filtered logs based on current filters
    fn filtered_logs(&self) -> Vec<&LogLine> {
        self.logs
            .iter()
            .filter(|log| {
                // Level filter
                if let Some(ref level) = self.level_filter {
                    if !log.level.eq_ignore_ascii_case(level) {
                        return false;
                    }
                }
                // Search filter
                if let Some(ref search) = self.search_filter {
                    let search_lower = search.to_lowercase();
                    if !log.message.to_lowercase().contains(&search_lower)
                        && !log.target.to_lowercase().contains(&search_lower)
                    {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    /// Scroll to the bottom (most recent)
    fn scroll_to_bottom(&mut self) {
        let filtered = self.filtered_logs();
        if !filtered.is_empty() {
            self.list_state.select(Some(filtered.len() - 1));
        }
    }

    /// Toggle pause state
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        if !self.paused && self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Set level filter
    #[allow(dead_code)]
    pub fn set_level_filter(&mut self, level: Option<String>) {
        self.level_filter = level;
    }

    /// Toggle a level filter (on if off, off if on)
    pub fn toggle_level_filter(&mut self, level: &str) {
        if self.level_filter.as_deref() == Some(level) {
            self.level_filter = None;
        } else {
            self.level_filter = Some(level.to_string());
        }
        self.list_state.select(Some(0));
    }

    /// Set search filter
    #[allow(dead_code)]
    pub fn set_search_filter(&mut self, search: Option<String>) {
        self.search_filter = search;
    }

    /// Clear all logs
    pub fn clear(&mut self) {
        self.logs.clear();
        self.list_state.select(Some(0));
    }

    /// Poll for new log events (non-blocking)
    fn poll_logs(&mut self) {
        if self.paused {
            // Still receive but don't display
            while self.log_rx.try_recv().is_ok() {}
            return;
        }

        // Receive all pending log events
        while let Ok(event) = self.log_rx.try_recv() {
            self.logs.push_back(LogLine::from(event));

            // Trim if over capacity
            while self.logs.len() > MAX_LOGS {
                self.logs.pop_front();
            }
        }

        // Auto-scroll to bottom if enabled
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }
}

impl Panel for LogsPanel {
    fn title(&self) -> &str {
        "logs"
    }

    fn kind(&self) -> PanelKind {
        PanelKind::Logs
    }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let filtered = self.filtered_logs();

        // Build list items
        let items: Vec<ListItem> = filtered
            .iter()
            .map(|log| {
                let spans = vec![
                    Span::styled(&log.timestamp, Theme::dim()),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:5}", log.level),
                        Theme::log_level(&log.level),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:20}", truncate_str(&log.target, 20)),
                        Theme::dim(),
                    ),
                    Span::raw(" "),
                    Span::styled(&log.message, Theme::text()),
                ];
                ListItem::new(Line::from(spans))
            })
            .collect();

        // Build title with decorators
        let status = if self.paused { " ⏸" } else { "" };
        let border_style = if focused { Theme::border(PanelKind::Logs) } else { Theme::border_dim() };
        
        // Current time
        let now = chrono::Local::now();
        let time_str = now.format("%H:%M:%S").to_string();
        
        // Level filter indicator
        let filter_indicator = match self.level_filter.as_deref() {
            Some("WARN") => " [W]",
            Some("INFO") => " [I]",
            Some("ERROR") => " [E]",
            _ => "",
        };
        
        // Left title: panel name and options
        let left_title = Line::from(vec![
            Span::styled("┐", border_style),
            Span::styled(PanelKind::Logs.superscript(), Theme::panel_number()),
            Span::styled("logs", Theme::panel_title(PanelKind::Logs)),
            Span::styled(format!(" ({}){}{}", filtered.len(), status, filter_indicator), Theme::dim()),
            Span::styled("┌─┐", border_style),
            Span::styled("p", Theme::keybind_key()),
            Span::styled("ause", Theme::keybind()),
            Span::styled("┌─┐", border_style),
            Span::styled("c", Theme::keybind_key()),
            Span::styled("lear", Theme::keybind()),
            Span::styled("┌─┐", border_style),
            Span::styled("w", Theme::keybind_key()),
            Span::styled("arn", Theme::keybind()),
            Span::styled("┌─┐", border_style),
            Span::styled("i", Theme::keybind_key()),
            Span::styled("nfo", Theme::keybind()),
            Span::styled("┌─┐", border_style),
            Span::styled("e", Theme::keybind_key()),
            Span::styled("rror", Theme::keybind()),
            Span::styled("┌", border_style),
        ]);

        // Center title: clock
        let center_title = Line::from(vec![
            Span::styled("┐", border_style),
            Span::styled(&time_str, Theme::text()),
            Span::styled("┌", border_style),
        ]);

        let block = Block::default()
            .title(left_title)
            .title_top(center_title.alignment(ratatui::layout::Alignment::Center))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(border_style);

        let list = List::new(items)
            .block(block)
            .highlight_style(Theme::selected());

        // Clone state for rendering
        let mut state = self.list_state.clone();
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn handle_action(&mut self, action: &Action) {
        let filtered_len = self.filtered_logs().len();

        match action {
            Action::ScrollUp => {
                self.auto_scroll = false;
                if let Some(selected) = self.list_state.selected() {
                    if selected > 0 {
                        self.list_state.select(Some(selected - 1));
                    }
                }
            }
            Action::ScrollDown => {
                if let Some(selected) = self.list_state.selected() {
                    if selected + 1 < filtered_len {
                        self.list_state.select(Some(selected + 1));
                    } else {
                        // At bottom, enable auto-scroll
                        self.auto_scroll = true;
                    }
                }
            }
            Action::PageUp => {
                self.auto_scroll = false;
                if let Some(selected) = self.list_state.selected() {
                    let new_pos = selected.saturating_sub(20);
                    self.list_state.select(Some(new_pos));
                }
            }
            Action::PageDown => {
                if let Some(selected) = self.list_state.selected() {
                    let new_pos = (selected + 20).min(filtered_len.saturating_sub(1));
                    self.list_state.select(Some(new_pos));
                    if new_pos + 1 >= filtered_len {
                        self.auto_scroll = true;
                    }
                }
            }
            Action::Home => {
                self.auto_scroll = false;
                self.list_state.select(Some(0));
            }
            Action::End => {
                self.auto_scroll = true;
                self.scroll_to_bottom();
            }
            Action::TogglePause => {
                self.toggle_pause();
            }
            Action::Clear => {
                self.clear();
            }
            Action::FilterWarn => {
                self.toggle_level_filter("WARN");
            }
            Action::FilterInfo => {
                self.toggle_level_filter("INFO");
            }
            Action::FilterError => {
                self.toggle_level_filter("ERROR");
            }
            _ => {}
        }
    }

    fn update(&mut self) {
        self.poll_logs();
    }

    fn scroll_position(&self) -> Option<(usize, usize)> {
        let total = self.filtered_logs().len();
        self.list_state.selected().map(|pos| (pos + 1, total))
    }
}

/// Truncate a string to max length, adding ellipsis if needed
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", &s[..max_len - 3])
    } else {
        s[..max_len].to_string()
    }
}
