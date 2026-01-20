//! Torrents panel - displays active torrent progress

use std::sync::Arc;

use parking_lot::RwLock;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::services::{TorrentInfo, TorrentService, TorrentState};
use crate::tui::input::Action;
use crate::tui::panels::Panel;
use crate::tui::theme::{PanelKind, Theme};

/// Shared torrent list updated by background task
pub type SharedTorrentList = Arc<RwLock<Vec<TorrentInfo>>>;

/// Torrents panel showing active downloads
pub struct TorrentsPanel {
    /// Cached torrent list (shared with updater task)
    torrents: SharedTorrentList,
    /// List state for scrolling
    list_state: ListState,
}

impl TorrentsPanel {
    /// Create a new torrents panel
    pub fn new(torrents: SharedTorrentList) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            torrents,
            list_state,
        }
    }

    /// Get current torrent list
    fn get_torrents(&self) -> Vec<TorrentInfo> {
        self.torrents.read().clone()
    }
}

/// Spawn a background task to update torrent list
pub fn spawn_torrent_updater(
    torrent_service: Arc<TorrentService>,
    torrents: SharedTorrentList,
) {
    tokio::spawn(async move {
        loop {
            // Fetch torrents
            let mut list = torrent_service.list_torrents().await;
            
            // Sort by state (downloading first), then by name
            list.sort_by(|a, b| {
                let state_order = |s: &TorrentState| match s {
                    TorrentState::Downloading => 0,
                    TorrentState::Checking => 1,
                    TorrentState::Seeding => 2,
                    TorrentState::Paused => 3,
                    TorrentState::Error => 4,
                    TorrentState::Queued => 5,
                };
                state_order(&a.state)
                    .cmp(&state_order(&b.state))
                    .then(a.name.cmp(&b.name))
            });

            // Update shared list
            *torrents.write() = list;

            // Sleep for 1 second
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });
}

impl Panel for TorrentsPanel {
    fn title(&self) -> &str {
        "Torrents"
    }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let torrents = self.get_torrents();
        
        // Calculate available width for progress bar
        let content_width = area.width.saturating_sub(4); // borders + padding
        let name_width = content_width.saturating_sub(15) as usize; // leave space for progress
        let bar_width = 10;

        // Build list items
        let items: Vec<ListItem> = torrents
            .iter()
            .map(|torrent| {
                let name = truncate_str(&torrent.name, name_width);

                // Progress bar
                let progress = (torrent.progress * 100.0) as u8;
                let filled = ((progress as usize) * bar_width / 100).min(bar_width);
                let empty = bar_width - filled;
                let bar = format!(
                    "{}{}",
                    "█".repeat(filled),
                    "░".repeat(empty)
                );

                // Status indicator
                let (status_char, status_style) = match torrent.state {
                    TorrentState::Downloading => ("↓", Theme::progress_complete()),
                    TorrentState::Seeding => ("✓", Theme::progress_complete()),
                    TorrentState::Checking => ("⟳", Theme::dim()),
                    TorrentState::Paused => ("⏸", Theme::dim()),
                    TorrentState::Error => ("✗", Theme::log_level("ERROR")),
                    TorrentState::Queued => ("⏳", Theme::dim()),
                };

                let spans = vec![
                    Span::styled(status_char, status_style),
                    Span::raw(" "),
                    Span::styled(name, Theme::text()),
                    Span::raw(" "),
                    Span::styled(bar, Theme::progress_complete()),
                    Span::raw(" "),
                    Span::styled(format!("{:3}%", progress), Theme::dim()),
                ];

                ListItem::new(Line::from(spans))
            })
            .collect();

        let title = format!(" Torrents ({}) ", torrents.len());

        let border_style = if focused {
            Theme::border_focused()
        } else {
            Theme::border()
        };

        let block = Block::default()
            .title(Span::styled(title, Theme::panel_title(PanelKind::Torrents)))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(border_style);

        // Show empty state if no torrents
        if items.is_empty() {
            let empty_block = block;
            frame.render_widget(empty_block, area);

            // Render centered "No active torrents" text
            if area.height > 2 && area.width > 20 {
                let text_area = Rect {
                    x: area.x + 2,
                    y: area.y + area.height / 2,
                    width: area.width - 4,
                    height: 1,
                };
                let text = ratatui::widgets::Paragraph::new(Span::styled(
                    "No active torrents",
                    Theme::dim(),
                ));
                frame.render_widget(text, text_area);
            }
        } else {
            let list = List::new(items)
                .block(block)
                .highlight_style(Theme::selected());

            let mut state = self.list_state.clone();
            frame.render_stateful_widget(list, area, &mut state);
        }
    }

    fn handle_action(&mut self, action: &Action) {
        let len = self.get_torrents().len();
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
            Action::Home => {
                self.list_state.select(Some(0));
            }
            Action::End => {
                self.list_state.select(Some(len.saturating_sub(1)));
            }
            _ => {}
        }
    }

    fn update(&mut self) {
        // Data is updated by background task, nothing to do here
    }

    fn scroll_position(&self) -> Option<(usize, usize)> {
        let torrents = self.get_torrents();
        if torrents.is_empty() {
            None
        } else {
            self.list_state
                .selected()
                .map(|pos| (pos + 1, torrents.len()))
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
