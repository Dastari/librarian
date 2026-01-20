//! System panel - displays CPU, memory, and uptime statistics

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Sparkline};

use crate::services::{SharedMetrics, SystemSnapshot, format_uptime};
use crate::tui::input::Action;
use crate::tui::panels::Panel;
use crate::tui::theme::{PanelKind, Theme};

/// System panel showing CPU, memory, uptime
pub struct SystemPanel {
    /// Metrics collector reference
    metrics: SharedMetrics,
    /// Cached snapshot
    snapshot: SystemSnapshot,
}

impl SystemPanel {
    /// Create a new system panel
    pub fn new(metrics: SharedMetrics) -> Self {
        let snapshot = metrics.snapshot();
        Self { metrics, snapshot }
    }

    /// Refresh metrics snapshot
    fn refresh(&mut self) {
        self.metrics.refresh();
        self.snapshot = self.metrics.snapshot();
    }
}

impl Panel for SystemPanel {
    fn title(&self) -> &str {
        "System"
    }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            Theme::border_focused()
        } else {
            Theme::border()
        };

        let block = Block::default()
            .title(Span::styled(" System ", Theme::panel_title(PanelKind::System)))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Skip rendering if too small
        if inner.height < 4 || inner.width < 20 {
            return;
        }

        // Layout: CPU line, Memory line, Uptime line, Requests line
        let chunks = Layout::vertical([
            Constraint::Length(1), // CPU
            Constraint::Length(1), // Memory
            Constraint::Length(1), // Uptime
            Constraint::Length(1), // Requests
        ])
        .split(inner);

        // CPU line with sparkline
        let cpu_history: Vec<u64> = self
            .snapshot
            .cpu_history
            .iter()
            .map(|v| *v as u64)
            .collect();

        // Calculate sparkline width
        let label_width = 12;
        let sparkline_width = inner.width.saturating_sub(label_width + 1) as usize;

        // Take last N samples for sparkline
        let cpu_data: Vec<u64> = if cpu_history.len() > sparkline_width {
            cpu_history[cpu_history.len() - sparkline_width..].to_vec()
        } else {
            cpu_history
        };

        let cpu_row = chunks[0];
        let cpu_chunks = Layout::horizontal([
            Constraint::Length(label_width),
            Constraint::Min(10),
        ])
        .split(cpu_row);

        let cpu_text = Paragraph::new(Line::from(vec![
            Span::styled("CPU   ", Theme::dim()),
            Span::styled(format!("{:5.1}%", self.snapshot.cpu_percent), Theme::text()),
        ]));
        frame.render_widget(cpu_text, cpu_chunks[0]);

        let cpu_sparkline = Sparkline::default()
            .data(&cpu_data)
            .max(100)
            .style(Theme::sparkline_cpu());
        frame.render_widget(cpu_sparkline, cpu_chunks[1]);

        // Memory line with sparkline
        let mem_percent = if self.snapshot.memory_total > 0 {
            (self.snapshot.memory_used as f64 / self.snapshot.memory_total as f64) * 100.0
        } else {
            0.0
        };

        let mem_history: Vec<u64> = self
            .snapshot
            .mem_history
            .iter()
            .map(|v| *v as u64)
            .collect();

        let mem_data: Vec<u64> = if mem_history.len() > sparkline_width {
            mem_history[mem_history.len() - sparkline_width..].to_vec()
        } else {
            mem_history
        };

        let mem_row = chunks[1];
        let mem_chunks = Layout::horizontal([
            Constraint::Length(label_width),
            Constraint::Min(10),
        ])
        .split(mem_row);

        let mem_text = Paragraph::new(Line::from(vec![
            Span::styled("MEM   ", Theme::dim()),
            Span::styled(format!("{:5.1}%", mem_percent), Theme::text()),
        ]));
        frame.render_widget(mem_text, mem_chunks[0]);

        let mem_sparkline = Sparkline::default()
            .data(&mem_data)
            .max(100)
            .style(Theme::sparkline_mem());
        frame.render_widget(mem_sparkline, mem_chunks[1]);

        // Uptime line
        let uptime = format_uptime(self.snapshot.uptime_secs);
        let uptime_text = Paragraph::new(Line::from(vec![
            Span::styled("UP    ", Theme::dim()),
            Span::styled(uptime, Theme::text()),
        ]));
        frame.render_widget(uptime_text, chunks[2]);

        // Requests line
        let requests_text = Paragraph::new(Line::from(vec![
            Span::styled("REQ   ", Theme::dim()),
            Span::styled(
                format!("{} total", format_number(self.snapshot.total_requests)),
                Theme::text(),
            ),
            Span::styled("  ", Theme::dim()),
            Span::styled(
                format!("{} active", self.snapshot.active_requests),
                if self.snapshot.active_requests > 0 {
                    Theme::log_level("INFO")
                } else {
                    Theme::dim()
                },
            ),
        ]));
        frame.render_widget(requests_text, chunks[3]);
    }

    fn handle_action(&mut self, action: &Action) {
        if let Action::Refresh = action {
            self.refresh();
        }
    }

    fn update(&mut self) {
        self.refresh();
    }
}

/// Format a number with thousands separators
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }
    result
}
