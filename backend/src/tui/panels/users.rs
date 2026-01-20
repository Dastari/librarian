//! Users panel - displays active user sessions

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::RwLock;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::tui::input::Action;
use crate::tui::panels::Panel;
use crate::tui::theme::{PanelKind, Theme};

/// An active user session
#[derive(Debug, Clone)]
pub struct ActiveSession {
    pub user_id: String,
    pub email: String,
    pub ip_address: Option<String>,
    pub last_activity: Instant,
    pub user_agent: Option<String>,
}

/// Shared session tracker
pub type SessionTracker = Arc<RwLock<HashMap<String, ActiveSession>>>;

/// Create a new session tracker
pub fn create_session_tracker() -> SessionTracker {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Users panel showing active sessions
pub struct UsersPanel {
    /// Session tracker reference
    sessions: SessionTracker,
    /// Cached session list
    session_list: Vec<ActiveSession>,
    /// List state for scrolling
    list_state: ListState,
}

impl UsersPanel {
    /// Create a new users panel
    pub fn new(sessions: SessionTracker) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            sessions,
            session_list: Vec::new(),
            list_state,
        }
    }

    /// Refresh session list
    fn refresh(&mut self) {
        let sessions = self.sessions.read();
        self.session_list = sessions.values().cloned().collect();

        // Sort by most recent activity
        self.session_list
            .sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
    }
}

impl Panel for UsersPanel {
    fn title(&self) -> &str {
        "Users"
    }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            Theme::border_focused()
        } else {
            Theme::border()
        };

        let title = format!(" Users ({}) ", self.session_list.len());

        let block = Block::default()
            .title(Span::styled(title, Theme::panel_title(PanelKind::Users)))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(border_style);

        if self.session_list.is_empty() {
            let inner = block.inner(area);
            frame.render_widget(block, area);

            // Show "No active users" centered
            if inner.height > 0 && inner.width > 15 {
                let text = Paragraph::new(Span::styled("No active users", Theme::dim()));
                let text_area = Rect {
                    x: inner.x + 1,
                    y: inner.y + inner.height / 2,
                    width: inner.width.saturating_sub(2),
                    height: 1,
                };
                frame.render_widget(text, text_area);
            }
            return;
        }

        // Calculate column widths
        let content_width = area.width.saturating_sub(4) as usize;
        let email_width = content_width.saturating_sub(25).min(30);

        // Build list items
        let items: Vec<ListItem> = self
            .session_list
            .iter()
            .map(|session| {
                let email = truncate_str(&session.email, email_width);
                let ip = session
                    .ip_address
                    .as_deref()
                    .unwrap_or("unknown")
                    .to_string();
                let ago = format_time_ago(session.last_activity.elapsed().as_secs());

                let spans = vec![
                    Span::styled(format!("{:<width$}", email, width = email_width), Theme::text()),
                    Span::raw("  "),
                    Span::styled(format!("{:<15}", ip), Theme::dim()),
                    Span::raw(" "),
                    Span::styled(ago, Theme::dim()),
                ];

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Theme::selected());

        let mut state = self.list_state.clone();
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn handle_action(&mut self, action: &Action) {
        let len = self.session_list.len();
        if len == 0 {
            return;
        }

        match action {
            Action::ScrollUp => {
                if let Some(selected) = self.list_state.selected() {
                    if selected > 0 {
                        self.list_state.select(Some(selected - 1));
                    }
                }
            }
            Action::ScrollDown => {
                if let Some(selected) = self.list_state.selected() {
                    if selected + 1 < len {
                        self.list_state.select(Some(selected + 1));
                    }
                }
            }
            Action::Refresh => {
                self.refresh();
            }
            _ => {}
        }
    }

    fn update(&mut self) {
        self.refresh();
    }

    fn scroll_position(&self) -> Option<(usize, usize)> {
        if self.session_list.is_empty() {
            None
        } else {
            self.list_state
                .selected()
                .map(|pos| (pos + 1, self.session_list.len()))
        }
    }
}

/// Truncate a string to max length
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", &s[..max_len - 3])
    } else {
        s[..max_len].to_string()
    }
}

/// Format seconds as "Xm ago" or "Xh ago"
fn format_time_ago(secs: u64) -> String {
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}
