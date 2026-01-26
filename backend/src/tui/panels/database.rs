//! Database panel - displays connection pool statistics and table counts
//! DB/entity access commented out; panel shows empty/disabled for now.

use std::sync::Arc;

use parking_lot::RwLock;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::tui::input::Action;
use crate::tui::panels::Panel;
use crate::tui::theme::{PanelKind, Theme};

/// Table record count entry
#[derive(Debug, Clone)]
pub struct TableCount {
    pub name: String,
    pub count: i64,
}

/// Table counts list
#[derive(Debug, Clone, Default)]
pub struct TableCounts {
    pub tables: Vec<TableCount>,
}

/// Shared table counts updated by background task
pub type SharedTableCounts = Arc<RwLock<TableCounts>>;

/// Create a new shared table counts
pub fn create_shared_table_counts() -> SharedTableCounts {
    Arc::new(RwLock::new(TableCounts::default()))
}

/// Spawn a background task to update table counts (disabled: no DB access)
#[allow(dead_code)]
pub fn spawn_table_counts_updater(_pool: crate::db::DbPool, counts: SharedTableCounts) {
    // Legacy: DB/entity access commented out; panel uses empty counts.
    let _ = counts;
    // tokio::spawn(async move {
    //     loop {
    //         let new_counts = fetch_all_table_counts(&pool).await;
    //         *counts.write() = new_counts;
    //         tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    //     }
    // });
}

// /// Fetch counts for all tables from database
// async fn fetch_all_table_counts(pool: &crate::db::DbPool) -> TableCounts { ... }

/// Database panel showing connection pool stats and table counts
pub struct DatabasePanel {
    table_counts: SharedTableCounts,
    list_state: ListState,
}

impl DatabasePanel {
    /// New panel with pool (legacy; not used when DB is disabled).
    #[allow(dead_code)]
    pub fn new(_pool: crate::db::DbPool, table_counts: SharedTableCounts) -> Self {
        Self::new_empty(table_counts)
    }

    /// New panel with no DB connection; shows empty/disabled.
    pub fn new_empty(table_counts: SharedTableCounts) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            table_counts,
            list_state,
        }
    }

    fn get_counts(&self) -> TableCounts {
        self.table_counts.read().clone()
    }
}

impl Panel for DatabasePanel {
    fn title(&self) -> &str {
        "db"
    }
    fn kind(&self) -> PanelKind {
        PanelKind::Database
    }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            Theme::border(PanelKind::Database)
        } else {
            Theme::border_dim()
        };

        let title = Line::from(vec![
            Span::styled("┐", border_style),
            Span::styled(PanelKind::Database.superscript(), Theme::panel_number()),
            Span::styled("db", Theme::panel_title(PanelKind::Database)),
            Span::styled("┌", border_style),
        ]);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 4 || inner.width < 25 {
            return;
        }

        let counts = self.get_counts();

        // Split vertically: pool stats at top (disabled), table list below
        let chunks = Layout::vertical([
            Constraint::Length(3), // Pool stats box
            Constraint::Min(3),    // Table list
        ])
        .split(inner);

        // Pool stats disabled (no DB connection in this build)
        render_pool_stats_disabled(frame, chunks[0], border_style);

        let items: Vec<ListItem> = counts
            .tables
            .iter()
            .map(|t| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{:<20}", t.name), Theme::dim()),
                    Span::styled(format!("{:>8}", format_number(t.count)), Theme::text()),
                ]))
            })
            .collect();

        let list = List::new(items).highlight_style(Theme::selected());
        let mut state = self.list_state.clone();
        frame.render_stateful_widget(list, chunks[1], &mut state);
    }

    fn handle_action(&mut self, action: &Action) {
        let len = self.get_counts().tables.len();
        if len == 0 {
            return;
        }
        match action {
            Action::ScrollUp => {
                if let Some(s) = self.list_state.selected() {
                    if s > 0 {
                        self.list_state.select(Some(s - 1));
                    }
                }
            }
            Action::ScrollDown => {
                if let Some(s) = self.list_state.selected() {
                    if s + 1 < len {
                        self.list_state.select(Some(s + 1));
                    }
                }
            }
            Action::Home => {
                self.list_state.select(Some(0));
            }
            Action::End => {
                self.list_state.select(Some(len.saturating_sub(1)));
            }
            _ => {}
        }
    }

    fn update(&mut self) {}

    fn scroll_position(&self) -> Option<(usize, usize)> {
        let counts = self.get_counts();
        if counts.tables.is_empty() {
            None
        } else {
            self.list_state
                .selected()
                .map(|p| (p + 1, counts.tables.len()))
        }
    }
}

/// Render pool stats as disabled (no DB in this build)
fn render_pool_stats_disabled(frame: &mut Frame, area: Rect, border_style: ratatui::style::Style) {
    let inner_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(border_style);

    let inner_area = inner_block.inner(area);
    frame.render_widget(inner_block, area);

    if inner_area.width < 20 || inner_area.height < 1 {
        return;
    }

    let stats_line = Line::from(vec![
        Span::styled("pool ", Theme::dim()),
        Span::styled("disabled (no DB)", Theme::dim()),
    ]);
    frame.render_widget(Paragraph::new(stats_line), inner_area);
}

/// Format a number with thousands separators
fn format_number(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
