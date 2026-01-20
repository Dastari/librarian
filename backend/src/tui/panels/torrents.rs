//! Torrents panel - displays active torrent progress with network rates and braille graph

use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::RwLock;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Color;
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, canvas::{Canvas, Points}};

use crate::services::{TorrentInfo, TorrentService, TorrentState};
use crate::tui::input::Action;
use crate::tui::panels::Panel;
use crate::tui::theme::{PanelKind, Theme};

/// Maximum rate history samples
const RATE_HISTORY_SIZE: usize = 120;

/// Torrent stats with rate history
#[derive(Debug, Clone, Default)]
pub struct TorrentStats {
    pub torrents: Vec<TorrentInfo>,
    pub download_history: VecDeque<f64>,
    pub upload_history: VecDeque<f64>,
    pub max_download_rate: u64,
    pub max_upload_rate: u64,
}

/// Shared torrent stats updated by background task
pub type SharedTorrentList = Arc<RwLock<TorrentStats>>;

/// Create shared torrent stats
pub fn create_shared_torrents() -> SharedTorrentList {
    Arc::new(RwLock::new(TorrentStats {
        torrents: Vec::new(),
        download_history: VecDeque::with_capacity(RATE_HISTORY_SIZE),
        upload_history: VecDeque::with_capacity(RATE_HISTORY_SIZE),
        max_download_rate: 1024 * 1024, // 1 MB/s default
        max_upload_rate: 1024 * 1024,
    }))
}

/// Torrents panel showing active downloads
pub struct TorrentsPanel {
    /// Cached torrent stats (shared with updater task)
    stats: SharedTorrentList,
    /// List state for scrolling
    list_state: ListState,
}

impl TorrentsPanel {
    /// Create a new torrents panel
    pub fn new(stats: SharedTorrentList) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self { stats, list_state }
    }

    fn get_stats(&self) -> TorrentStats {
        self.stats.read().clone()
    }
}

/// Spawn a background task to update torrent list and rate history
pub fn spawn_torrent_updater(torrent_service: Arc<TorrentService>, stats: SharedTorrentList) {
    tokio::spawn(async move {
        loop {
            let mut list = torrent_service.list_torrents().await;
            list.sort_by(|a, b| {
                let state_order = |s: &TorrentState| match s {
                    TorrentState::Downloading => 0,
                    TorrentState::Checking => 1,
                    TorrentState::Seeding => 2,
                    TorrentState::Paused => 3,
                    TorrentState::Error => 4,
                    TorrentState::Queued => 5,
                };
                state_order(&a.state).cmp(&state_order(&b.state)).then(a.name.cmp(&b.name))
            });

            // Calculate total rates
            let total_down: u64 = list.iter().map(|t| t.download_speed).sum();
            let total_up: u64 = list.iter().map(|t| t.upload_speed).sum();

            // Update stats in a block to ensure lock is dropped before await
            {
                let mut s = stats.write();
                
                // Update rate history
                if s.download_history.len() >= RATE_HISTORY_SIZE {
                    s.download_history.pop_front();
                }
                s.download_history.push_back(total_down as f64);

                if s.upload_history.len() >= RATE_HISTORY_SIZE {
                    s.upload_history.pop_front();
                }
                s.upload_history.push_back(total_up as f64);

                // Update max rates (with some decay for better scaling)
                let current_max_down = s.download_history.iter().cloned().fold(0.0_f64, f64::max) as u64;
                let current_max_up = s.upload_history.iter().cloned().fold(0.0_f64, f64::max) as u64;
                s.max_download_rate = current_max_down.max(1024 * 100); // Min 100 KB/s
                s.max_upload_rate = current_max_up.max(1024 * 100);

                s.torrents = list;
            } // Lock dropped here

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });
}

impl Panel for TorrentsPanel {
    fn title(&self) -> &str { "torrents" }
    fn kind(&self) -> PanelKind { PanelKind::Torrents }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let stats = self.get_stats();
        let torrents = &stats.torrents;

        // Calculate total rates for title
        let total_down: u64 = torrents.iter().map(|t| t.download_speed).sum();
        let total_up: u64 = torrents.iter().map(|t| t.upload_speed).sum();

        let border_style = if focused { Theme::border(PanelKind::Torrents) } else { Theme::border_dim() };

        // Build title with decorators
        let title_spans = vec![
            Span::styled("┐", border_style),
            Span::styled(PanelKind::Torrents.superscript(), Theme::panel_number()),
            Span::styled("torrents", Theme::panel_title(PanelKind::Torrents)),
            Span::styled(format!(" ({})", torrents.len()), Theme::dim()),
            Span::styled("┌", border_style),
        ];

        let block = Block::default()
            .title(Line::from(title_spans))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block.clone(), area);

        if inner.height < 3 || inner.width < 20 { return; }

        // Split: graph area at top (8 rows for larger graph), list below
        // Show graph when there are torrents OR when there's history data
        let has_torrents = !torrents.is_empty();
        let has_history = !stats.download_history.is_empty() || !stats.upload_history.is_empty();
        let show_graph = has_torrents || has_history;
        let graph_height = if show_graph && inner.height > 12 { 8 } else if show_graph && inner.height > 8 { 6 } else { 0 };
        
        let chunks = if graph_height > 0 {
            Layout::vertical([
                Constraint::Length(graph_height),
                Constraint::Min(3),
            ]).split(inner)
        } else {
            Layout::vertical([Constraint::Min(3)]).split(inner)
        };

        // Render graph area with stats panel on right
        if graph_height > 0 {
            render_graph_with_stats(frame, chunks[0], &stats, total_down, total_up, border_style);
        }

        let list_area = if graph_height > 0 { chunks[1] } else { chunks[0] };

        if torrents.is_empty() {
            if list_area.height > 0 && list_area.width > 15 {
                let text_area = Rect { x: list_area.x + 1, y: list_area.y + list_area.height / 2, width: list_area.width - 2, height: 1 };
                frame.render_widget(Paragraph::new(Span::styled("No active torrents", Theme::dim())), text_area);
            }
            return;
        }

        // Calculate column widths
        let content_width = list_area.width as usize;
        let percent_width = 5;
        let bar_width = 12;
        let status_width = 2;
        let name_width = content_width.saturating_sub(percent_width + bar_width + status_width + 3);

        let items: Vec<ListItem> = torrents.iter().map(|torrent| {
            let name = truncate_str(&torrent.name, name_width);
            let progress = (torrent.progress * 100.0) as u8;
            let filled = ((progress as usize) * bar_width / 100).min(bar_width);
            let empty = bar_width - filled;
            let bar = format!("{}{}", "\u{28FF}".repeat(filled), "\u{2880}".repeat(empty));

            let (status_char, status_style, bar_style) = match torrent.state {
                TorrentState::Downloading => ("\u{2193}", Theme::progress_active(), Theme::progress_active()),
                TorrentState::Seeding => ("\u{2713}", Theme::progress_complete(), Theme::progress_complete()),
                TorrentState::Checking => ("\u{27F3}", Theme::progress_active(), Theme::progress_active()),
                TorrentState::Paused => ("\u{23F8}", Theme::dim(), Theme::dim()),
                TorrentState::Error => ("\u{2717}", Theme::log_level("ERROR"), Theme::log_level("ERROR")),
                TorrentState::Queued => ("\u{23F3}", Theme::dim(), Theme::dim()),
            };

            let padded_name = format!("{:<width$}", name, width = name_width);
            ListItem::new(Line::from(vec![
                Span::styled(status_char, status_style),
                Span::raw(" "),
                Span::styled(padded_name, Theme::text()),
                Span::raw(" "),
                Span::styled(bar, bar_style),
                Span::styled(format!("{:>4}%", progress), Theme::dim()),
            ]))
        }).collect();

        let list = List::new(items).highlight_style(Theme::selected());
        let mut state = self.list_state.clone();
        frame.render_stateful_widget(list, list_area, &mut state);
    }

    fn handle_action(&mut self, action: &Action) {
        let len = self.get_stats().torrents.len();
        if len == 0 { return; }
        match action {
            Action::ScrollUp => { if let Some(s) = self.list_state.selected() { if s > 0 { self.list_state.select(Some(s - 1)); } } }
            Action::ScrollDown => { if let Some(s) = self.list_state.selected() { if s + 1 < len { self.list_state.select(Some(s + 1)); } } }
            Action::Home => { self.list_state.select(Some(0)); }
            Action::End => { self.list_state.select(Some(len.saturating_sub(1))); }
            _ => {}
        }
    }

    fn update(&mut self) {}

    fn scroll_position(&self) -> Option<(usize, usize)> {
        let t = self.get_stats().torrents;
        if t.is_empty() { None } else { self.list_state.selected().map(|p| (p + 1, t.len())) }
    }
}

/// Render braille rate graph with stats panel on right
fn render_graph_with_stats(
    frame: &mut Frame, 
    area: Rect, 
    stats: &TorrentStats,
    current_down: u64,
    current_up: u64,
    border_style: ratatui::style::Style,
) {
    if area.width < 30 || area.height < 4 { return; }

    // Split: graph on left (with scale labels), stats on right
    let stats_width = 22;
    let chunks = Layout::horizontal([
        Constraint::Min(20),
        Constraint::Length(stats_width),
    ]).split(area);

    let graph_area = chunks[0];
    let stats_area = chunks[1];

    // Scale label width
    let label_width = 6;
    let canvas_area = Rect {
        x: graph_area.x + label_width,
        y: graph_area.y,
        width: graph_area.width.saturating_sub(label_width),
        height: graph_area.height,
    };

    // Render max labels on left
    let max_down_str = format!("{}/s", format_bytes(stats.max_download_rate));
    let max_up_str = format!("{}/s", format_bytes(stats.max_upload_rate));
    
    if graph_area.height >= 1 {
        frame.render_widget(
            Paragraph::new(Span::styled(&max_down_str, Theme::dim())),
            Rect { x: graph_area.x, y: graph_area.y, width: label_width, height: 1 }
        );
    }
    if graph_area.height >= 3 {
        frame.render_widget(
            Paragraph::new(Span::styled(&max_up_str, Theme::dim())),
            Rect { x: graph_area.x, y: graph_area.y + graph_area.height - 1, width: label_width, height: 1 }
        );
    }

    // Draw braille canvas
    let graph_width = canvas_area.width as usize;
    let max_down = stats.max_download_rate.max(1) as f64;
    let max_up = stats.max_upload_rate.max(1) as f64;

    // Create download points (top half)
    let down_points: Vec<(f64, f64)> = stats.download_history.iter().enumerate()
        .rev()
        .take(graph_width * 2) // More points for braille density
        .map(|(i, &rate)| {
            let x = (graph_width.saturating_sub((stats.download_history.len().saturating_sub(i)) / 2)) as f64;
            let y = (rate / max_down).min(1.0) * (area.height as f64 / 2.0);
            (x, y + (area.height as f64 / 2.0))
        })
        .collect();

    // Create upload points (bottom half)
    let up_points: Vec<(f64, f64)> = stats.upload_history.iter().enumerate()
        .rev()
        .take(graph_width * 2)
        .map(|(i, &rate)| {
            let x = (graph_width.saturating_sub((stats.upload_history.len().saturating_sub(i)) / 2)) as f64;
            let y = (area.height as f64 / 2.0) - (rate / max_up).min(1.0) * (area.height as f64 / 2.0);
            (x, y)
        })
        .collect();

    let canvas = Canvas::default()
        .marker(Marker::Braille)
        .x_bounds([0.0, graph_width as f64])
        .y_bounds([0.0, area.height as f64])
        .paint(|ctx| {
            if !down_points.is_empty() {
                ctx.draw(&Points {
                    coords: &down_points,
                    color: Color::Rgb(236, 72, 153), // Pink
                });
            }
            if !up_points.is_empty() {
                ctx.draw(&Points {
                    coords: &up_points,
                    color: Color::Rgb(139, 92, 246), // Violet
                });
            }
        });

    frame.render_widget(canvas, canvas_area);

    // Render stats panel on right with internal border
    let stats_block = Block::default()
        .borders(Borders::LEFT)
        .border_style(border_style);
    
    let stats_inner = stats_block.inner(stats_area);
    frame.render_widget(stats_block, stats_area);

    // Calculate peaks for display (totals reserved for future use)
    let _total_down: u64 = stats.download_history.iter().map(|&v| v as u64).sum();
    let _total_up: u64 = stats.upload_history.iter().map(|&v| v as u64).sum();
    let peak_down = stats.download_history.iter().cloned().fold(0.0_f64, f64::max) as u64;
    let peak_up = stats.upload_history.iter().cloned().fold(0.0_f64, f64::max) as u64;

    // Build stats lines like btop
    let mut lines = Vec::new();
    
    // Download section
    lines.push(Line::from(Span::styled("download", Theme::text())));
    lines.push(Line::from(vec![
        Span::styled("▼ ", Theme::graph_download()),
        Span::styled(format!("{}/s", format_bytes(current_down)), Theme::text()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("▼ Top: ", Theme::dim()),
        Span::styled(format!("{}/s", format_bytes(peak_down)), Theme::dim()),
    ]));
    
    // Upload section  
    if stats_inner.height > 4 {
        lines.push(Line::from(Span::styled("upload", Theme::text())));
        lines.push(Line::from(vec![
            Span::styled("▲ ", Theme::graph_upload()),
            Span::styled(format!("{}/s", format_bytes(current_up)), Theme::text()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("▲ Top: ", Theme::dim()),
            Span::styled(format!("{}/s", format_bytes(peak_up)), Theme::dim()),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), stats_inner);
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len { s.to_string() }
    else if max_len > 3 { format!("{}...", &s[..max_len - 3]) }
    else { s[..max_len].to_string() }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB { format!("{:.1}G", bytes as f64 / GB as f64) }
    else if bytes >= MB { format!("{:.1}M", bytes as f64 / MB as f64) }
    else if bytes >= KB { format!("{:.0}K", bytes as f64 / KB as f64) }
    else { format!("{}B", bytes) }
}
