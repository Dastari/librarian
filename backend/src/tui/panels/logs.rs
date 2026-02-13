//! Logs panel - displays live log stream with filtering

use std::cell::Cell;
use std::collections::VecDeque;

use chrono::{DateTime, Local};
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
        // Convert RFC3339 UTC timestamps from the logging layer into local time for display.
        let timestamp = DateTime::parse_from_rfc3339(&event.timestamp)
            .map(|dt| {
                dt.with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|_| {
                event
                    .timestamp
                    .split('T')
                    .nth(1)
                    .and_then(|t| t.split('.').next())
                    .unwrap_or(&event.timestamp)
                    .to_string()
            });

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
    /// Last rendered first visible item index (viewport offset).
    last_render_offset: Cell<usize>,
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
            last_render_offset: Cell::new(0),
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
            // Drain so we don't lag the channel
            while let Ok(_) = self.log_rx.try_recv() {}
            return;
        }

        use tokio::sync::broadcast::error::TryRecvError;
        loop {
            match self.log_rx.try_recv() {
                Ok(event) => {
                    self.logs.push_back(LogLine::from(event));
                    while self.logs.len() > MAX_LOGS {
                        self.logs.pop_front();
                    }
                }
                Err(TryRecvError::Lagged(n)) => {
                    // Skip lagged messages and continue receiving
                    let _ = n;
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => break,
            }
        }

        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Select a log entry by a screen y-coordinate inside the logs panel area.
    pub fn select_at_screen_row(&mut self, screen_y: u16, area: Rect) {
        // List content is inside the block border.
        let content_top = area.y.saturating_add(1);
        let content_height = area.height.saturating_sub(2);
        if content_height == 0 || screen_y < content_top || screen_y >= content_top + content_height
        {
            return;
        }

        let relative_row = (screen_y - content_top) as usize;
        let filtered = self.filtered_logs();
        if filtered.is_empty() {
            return;
        }

        let selected = self.list_state.selected();
        let content_width = area.width.saturating_sub(2) as usize;
        let start_index = self.last_render_offset.get();
        let previous_offset = start_index;

        let mut cursor = 0usize;
        for (idx, log) in filtered.iter().enumerate().skip(start_index) {
            let height = self.rendered_item_height(log, idx, selected, content_width);
            if relative_row < cursor + height {
                self.list_state.select(Some(idx));
                *self.list_state.offset_mut() = previous_offset;
                self.auto_scroll = false;
                return;
            }
            cursor += height;
        }
    }

    /// Compute how many terminal rows a log entry occupies in the list.
    fn rendered_item_height(
        &self,
        log: &LogLine,
        idx: usize,
        selected: Option<usize>,
        content_width: usize,
    ) -> usize {
        let timestamp = log.timestamp.as_str();
        let level = format!("{:5}", log.level);
        let target = format!("{:20}", truncate_str(&log.target, 20));
        let prefix = format!("{timestamp} {level} {target} ");
        let message_width = content_width.saturating_sub(prefix.chars().count());

        if selected == Some(idx) && message_width > 0 {
            wrap_text(&log.message, message_width).len().max(1)
        } else {
            1
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
        let selected = self.list_state.selected();
        let content_width = area.width.saturating_sub(2) as usize;

        // Build list items
        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(idx, log)| {
                let timestamp = log.timestamp.as_str();
                let level = format!("{:5}", log.level);
                let target = format!("{:20}", truncate_str(&log.target, 20));

                let prefix = format!("{timestamp} {level} {target} ");
                let message_width = content_width.saturating_sub(prefix.chars().count());

                if selected == Some(idx) && message_width > 0 {
                    let wrapped_message = wrap_text(&log.message, message_width);
                    let mut lines = Vec::with_capacity(wrapped_message.len().max(1));

                    if let Some(first_line) = wrapped_message.first() {
                        lines.push(Line::from(vec![
                            Span::styled(timestamp, Theme::dim()),
                            Span::raw(" "),
                            Span::styled(level.clone(), Theme::log_level(&log.level)),
                            Span::raw(" "),
                            Span::styled(target.clone(), Theme::dim()),
                            Span::raw(" "),
                            Span::styled(first_line.clone(), Theme::text()),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::styled(timestamp, Theme::dim()),
                            Span::raw(" "),
                            Span::styled(level.clone(), Theme::log_level(&log.level)),
                            Span::raw(" "),
                            Span::styled(target.clone(), Theme::dim()),
                            Span::raw(" "),
                        ]));
                    }

                    let continuation_prefix = " ".repeat(prefix.chars().count());
                    for continuation in wrapped_message.iter().skip(1) {
                        lines.push(Line::from(vec![
                            Span::styled(continuation_prefix.clone(), Theme::dim()),
                            Span::styled(continuation.clone(), Theme::text()),
                        ]));
                    }

                    ListItem::new(lines)
                } else {
                    let spans = vec![
                        Span::styled(timestamp, Theme::dim()),
                        Span::raw(" "),
                        Span::styled(level, Theme::log_level(&log.level)),
                        Span::raw(" "),
                        Span::styled(target, Theme::dim()),
                        Span::raw(" "),
                        Span::styled(&log.message, Theme::text()),
                    ];
                    ListItem::new(Line::from(spans))
                }
            })
            .collect();

        // Build title with decorators
        let status = if self.paused { " ⏸" } else { "" };
        let border_style = if focused {
            Theme::border(PanelKind::Logs)
        } else {
            Theme::border_dim()
        };

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
            Span::styled(
                format!(" ({}){}{}", filtered.len(), status, filter_indicator),
                Theme::dim(),
            ),
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
            Span::styled("┌─┐", border_style),
            Span::styled("m", Theme::keybind_key()),
            Span::styled("ouse", Theme::keybind()),
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
        self.last_render_offset.set(state.offset());
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

/// Wrap text to a maximum number of characters per line.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![String::new()];
    }

    let mut lines = Vec::new();

    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let word_len = word.chars().count();
            let current_len = current.chars().count();

            if current.is_empty() {
                if word_len <= max_width {
                    current.push_str(word);
                } else {
                    for chunk in chunk_by_chars(word, max_width) {
                        lines.push(chunk);
                    }
                }
                continue;
            }

            if current_len + 1 + word_len <= max_width {
                current.push(' ');
                current.push_str(word);
            } else {
                lines.push(current);
                current = String::new();

                if word_len <= max_width {
                    current.push_str(word);
                } else {
                    for chunk in chunk_by_chars(word, max_width) {
                        lines.push(chunk);
                    }
                }
            }
        }

        if !current.is_empty() {
            lines.push(current);
        }
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

/// Split a string into chunks by character count.
fn chunk_by_chars(s: &str, chunk_size: usize) -> Vec<String> {
    if chunk_size == 0 {
        return vec![String::new()];
    }

    let chars: Vec<char> = s.chars().collect();
    chars
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect()
}
