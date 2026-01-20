//! Libraries panel - displays library statistics

use std::sync::Arc;

use parking_lot::RwLock;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use sqlx::PgPool;

use crate::tui::input::Action;
use crate::tui::panels::Panel;
use crate::tui::theme::{PanelKind, Theme};

/// Library statistics for display
#[derive(Debug, Clone, Default)]
pub struct LibraryStats {
    pub name: String,
    pub library_type: String,
    pub path: String,
    pub item_count: i64,
    pub missing_count: i64,
    pub total_size_bytes: i64,
}

/// Shared library list updated by background task
pub type SharedLibraries = Arc<RwLock<Vec<LibraryStats>>>;

/// Create a new shared libraries list
pub fn create_shared_libraries() -> SharedLibraries {
    Arc::new(RwLock::new(Vec::new()))
}

/// Spawn a background task to update library stats
pub fn spawn_libraries_updater(pool: PgPool, libraries: SharedLibraries) {
    tokio::spawn(async move {
        loop {
            // Query library stats
            let stats = fetch_library_stats(&pool).await;
            *libraries.write() = stats;
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    });
}

/// Fetch library stats from database
async fn fetch_library_stats(pool: &PgPool) -> Vec<LibraryStats> {
    // Query libraries with id, name, type, path
    let result = sqlx::query_as::<_, (uuid::Uuid, String, String, String)>(
        r#"
        SELECT id, name, library_type, path
        FROM libraries
        ORDER BY name
        "#,
    )
    .fetch_all(pool)
    .await;

    let mut stats = Vec::new();

    if let Ok(libs) = result {
        for (library_id, name, library_type, path) in libs {
            // Get total size from media_files (same as frontend)
            let total_size = sqlx::query_scalar::<_, i64>(
                "SELECT COALESCE(SUM(size), 0)::BIGINT FROM media_files WHERE library_id = $1",
            )
            .bind(library_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

            // Get counts based on library type
            let (item_count, missing_count) = match library_type.as_str() {
                "movies" => {
                    let count = sqlx::query_scalar::<_, i64>(
                        "SELECT COUNT(*) FROM movies WHERE library_id = $1",
                    )
                    .bind(library_id)
                    .fetch_one(pool)
                    .await
                    .unwrap_or(0);

                    let missing = sqlx::query_scalar::<_, i64>(
                        "SELECT COUNT(*) FROM movies WHERE library_id = $1 AND has_file = false",
                    )
                    .bind(library_id)
                    .fetch_one(pool)
                    .await
                    .unwrap_or(0);

                    (count, missing)
                }
                "tv" => {
                    let count = sqlx::query_scalar::<_, i64>(
                        "SELECT COUNT(*) FROM tv_shows WHERE library_id = $1",
                    )
                    .bind(library_id)
                    .fetch_one(pool)
                    .await
                    .unwrap_or(0);

                    let missing = sqlx::query_scalar::<_, i64>(
                        "SELECT COUNT(DISTINCT e.id) FROM episodes e JOIN tv_shows s ON e.tv_show_id = s.id WHERE s.library_id = $1 AND NOT EXISTS (SELECT 1 FROM media_files WHERE episode_id = e.id)"
                    )
                    .bind(library_id)
                    .fetch_one(pool)
                    .await
                    .unwrap_or(0);

                    (count, missing)
                }
                "music" => {
                    let count = sqlx::query_scalar::<_, i64>(
                        "SELECT COUNT(*) FROM albums WHERE library_id = $1",
                    )
                    .bind(library_id)
                    .fetch_one(pool)
                    .await
                    .unwrap_or(0);

                    (count, 0)
                }
                "audiobooks" => {
                    let count = sqlx::query_scalar::<_, i64>(
                        "SELECT COUNT(*) FROM audiobooks WHERE library_id = $1",
                    )
                    .bind(library_id)
                    .fetch_one(pool)
                    .await
                    .unwrap_or(0);

                    (count, 0)
                }
                _ => (0, 0),
            };

            stats.push(LibraryStats {
                name,
                library_type,
                path,
                item_count,
                missing_count,
                total_size_bytes: total_size,
            });
        }
    }

    stats
}

/// Get icon for library type
fn library_icon(library_type: &str) -> &'static str {
    match library_type.to_lowercase().as_str() {
        "movies" => "\u{25B6}",     // ▶
        "tv" => "\u{25A3}",         // ▣
        "music" => "\u{266B}",      // ♫
        "audiobooks" => "\u{25C9}", // ◉
        _ => "\u{25CF}",            // ●
    }
}

/// Get item label for library type
fn item_label(library_type: &str) -> &'static str {
    match library_type.to_lowercase().as_str() {
        "movies" => "movies",
        "tv" => "shows",
        "music" => "albums",
        "audiobooks" => "books",
        _ => "items",
    }
}

/// Libraries panel showing library stats
pub struct LibrariesPanel {
    libraries: SharedLibraries,
    list_state: ListState,
}

impl LibrariesPanel {
    pub fn new(libraries: SharedLibraries) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            libraries,
            list_state,
        }
    }

    fn get_libraries(&self) -> Vec<LibraryStats> {
        self.libraries.read().clone()
    }
}

impl Panel for LibrariesPanel {
    fn title(&self) -> &str {
        "libs"
    }
    fn kind(&self) -> PanelKind {
        PanelKind::Libraries
    }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let libs = self.get_libraries();
        let border_style = if focused {
            Theme::border(PanelKind::Libraries)
        } else {
            Theme::border_dim()
        };

        let title = Line::from(vec![
            Span::styled("┐", border_style),
            Span::styled(PanelKind::Libraries.superscript(), Theme::panel_number()),
            Span::styled("libs", Theme::panel_title(PanelKind::Libraries)),
            Span::styled(format!(" ({})", libs.len()), Theme::dim()),
            Span::styled("┌", border_style),
        ]);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block.clone(), area);

        if libs.is_empty() {
            if inner.height > 0 && inner.width > 15 {
                let text_area = Rect {
                    x: inner.x + 1,
                    y: inner.y + inner.height / 2,
                    width: inner.width - 2,
                    height: 1,
                };
                frame.render_widget(
                    Paragraph::new(Span::styled("No libraries configured", Theme::dim())),
                    text_area,
                );
            }
            return;
        }

        // Calculate column widths
        let content_width = inner.width as usize;
        let icon_width = 2;
        let count_width = 12; // "123 movies"
        let missing_width = 9; // "45 miss"
        let size_width = 10; // "14.8 GB"
        let name_width = content_width
            .saturating_sub(icon_width + count_width + missing_width + size_width + 4)
            .min(20);
        let path_width = content_width
            .saturating_sub(icon_width + name_width + count_width + missing_width + size_width + 5);

        let items: Vec<ListItem> = libs
            .iter()
            .map(|lib| {
                let icon = library_icon(&lib.library_type);
                let label = item_label(&lib.library_type);
                let name = truncate_str(&lib.name, name_width);
                let count_str = format!("{} {}", lib.item_count, label);
                let missing_str = if lib.missing_count > 0 {
                    format!("{} miss", lib.missing_count)
                } else {
                    String::new()
                };
                let size_str = format_size(lib.total_size_bytes);
                let path = truncate_str(&lib.path, path_width);

                let mut spans = vec![
                    Span::styled(icon, Theme::panel_title(PanelKind::Libraries)),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:<width$}", name, width = name_width),
                        Theme::text(),
                    ),
                    Span::raw(" "),
                    Span::styled(format!("{:>12}", count_str), Theme::dim()),
                ];

                if !missing_str.is_empty() {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        format!("{:>9}", missing_str),
                        Theme::log_level("WARN"),
                    ));
                } else {
                    spans.push(Span::styled(format!("{:>10}", ""), Theme::dim()));
                }

                spans.push(Span::raw(" "));
                spans.push(Span::styled(format!("{:>10}", size_str), Theme::text()));
                spans.push(Span::raw(" "));
                spans.push(Span::styled(path, Theme::dim()));

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items).highlight_style(Theme::selected());
        let mut state = self.list_state.clone();
        frame.render_stateful_widget(list, inner, &mut state);
    }

    fn handle_action(&mut self, action: &Action) {
        let len = self.get_libraries().len();
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
        let libs = self.get_libraries();
        if libs.is_empty() {
            None
        } else {
            self.list_state.selected().map(|p| (p + 1, libs.len()))
        }
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", &s[..max_len - 3])
    } else {
        s[..max_len].to_string()
    }
}

fn format_size(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;
    const TB: i64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.0} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
