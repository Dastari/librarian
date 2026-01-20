//! Database panel - displays connection pool statistics

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use sqlx::PgPool;

use crate::tui::input::Action;
use crate::tui::panels::Panel;
use crate::tui::theme::{PanelKind, Theme};

/// Database panel showing connection pool stats
pub struct DatabasePanel {
    /// Database pool reference
    pool: PgPool,
}

impl DatabasePanel {
    /// Create a new database panel
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl Panel for DatabasePanel {
    fn title(&self) -> &str {
        "Database"
    }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            Theme::border_focused()
        } else {
            Theme::border()
        };

        let block = Block::default()
            .title(Span::styled(" Database ", Theme::panel_title(PanelKind::Database)))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Skip if too small
        if inner.height < 3 || inner.width < 20 {
            return;
        }

        // Get pool stats
        let pool_size = self.pool.size();
        let idle = self.pool.num_idle();
        let active = pool_size - idle as u32;
        let max_connections = self.pool.options().get_max_connections();

        // Calculate progress bar
        let bar_width = 10;
        let filled = if max_connections > 0 {
            ((active as usize) * bar_width / max_connections as usize).min(bar_width)
        } else {
            0
        };
        let empty = bar_width - filled;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

        // Build display lines
        let lines = vec![
            Line::from(vec![
                Span::styled("Pool  ", Theme::dim()),
                Span::styled(bar, Theme::progress_complete()),
                Span::styled(
                    format!(" {}/{}", active, max_connections),
                    Theme::text(),
                ),
            ]),
            Line::from(vec![
                Span::styled("Idle  ", Theme::dim()),
                Span::styled(format!("{} connections", idle), Theme::text()),
            ]),
            Line::from(vec![
                Span::styled("Size  ", Theme::dim()),
                Span::styled(format!("{} current", pool_size), Theme::text()),
            ]),
        ];

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }

    fn handle_action(&mut self, _action: &Action) {
        // Database panel doesn't need special handling
    }

    fn update(&mut self) {
        // Pool stats are always live, no caching needed
    }
}
