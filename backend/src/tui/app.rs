//! Main TUI application

use std::io::{self, Stdout};
use std::sync::Arc;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use sqlx::PgPool;
use tokio::sync::broadcast;

use crate::services::{LogEvent, SharedMetrics, TorrentService};
use crate::tui::input::{Action, InputHandler};
use crate::tui::panels::{DatabasePanel, LibrariesPanel, LogsPanel, Panel, SystemPanel, TorrentsPanel, spawn_torrent_updater, spawn_libraries_updater, spawn_table_counts_updater, create_shared_libraries, create_shared_table_counts, create_shared_torrents};
use crate::tui::ui::{PanelId, UiLayout, render_panels};

/// TUI configuration
pub struct TuiConfig {
    /// Tick rate in milliseconds
    pub tick_rate_ms: u64,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self { tick_rate_ms: 100 }
    }
}

/// Main TUI application
pub struct TuiApp {
    /// Terminal instance
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// Input handler
    input: InputHandler,
    /// UI layout state
    layout: UiLayout,
    /// Logs panel
    logs_panel: LogsPanel,
    /// Torrents panel
    torrents_panel: TorrentsPanel,
    /// System panel
    system_panel: SystemPanel,
    /// Libraries panel
    libraries_panel: LibrariesPanel,
    /// Database panel
    database_panel: DatabasePanel,
    /// Whether the app should quit
    should_quit: bool,
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new(
        metrics: SharedMetrics,
        log_rx: broadcast::Receiver<LogEvent>,
        torrent_service: Arc<TorrentService>,
        pool: PgPool,
        config: TuiConfig,
    ) -> io::Result<Self> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        // Create shared torrent stats and spawn updater task
        let torrents = create_shared_torrents();
        spawn_torrent_updater(torrent_service, torrents.clone());

        // Create shared libraries list and spawn updater task
        let libraries = create_shared_libraries();
        spawn_libraries_updater(pool.clone(), libraries.clone());

        // Create shared table counts and spawn updater task
        let table_counts = create_shared_table_counts();
        spawn_table_counts_updater(pool.clone(), table_counts.clone());

        Ok(Self {
            terminal,
            input: InputHandler::new(config.tick_rate_ms),
            layout: UiLayout::default(),
            logs_panel: LogsPanel::new(log_rx),
            torrents_panel: TorrentsPanel::new(torrents),
            system_panel: SystemPanel::new(metrics),
            libraries_panel: LibrariesPanel::new(libraries),
            database_panel: DatabasePanel::new(pool, table_counts),
            should_quit: false,
        })
    }

    /// Run the TUI event loop
    pub async fn run(mut self) -> io::Result<()> {
        loop {
            // Draw UI
            self.draw()?;

            // Handle input (blocking with timeout)
            let action = tokio::task::spawn_blocking(move || {
                let result = self.input.next_action();
                (self, result)
            })
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            self = action.0;
            let action_result = action.1?;

            // Process action
            self.handle_action(action_result);

            // Update panels
            self.update_panels();

            // Check for quit
            if self.should_quit {
                break;
            }
        }

        // Cleanup
        self.cleanup()?;
        Ok(())
    }

    /// Draw the UI
    fn draw(&mut self) -> io::Result<()> {
        self.terminal.draw(|frame| {
            let areas = self.layout.calculate_areas(frame.area());
            render_panels(
                frame,
                &self.layout,
                &areas,
                &self.logs_panel,
                &self.torrents_panel,
                &self.system_panel,
                &self.libraries_panel,
                &self.database_panel,
            );
        })?;
        Ok(())
    }

    /// Handle an action
    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => {
                self.should_quit = true;
            }
            Action::NextPanel => {
                self.layout.focus_next();
            }
            Action::PrevPanel => {
                self.layout.focus_prev();
            }
            Action::FocusPanel(index) => {
                self.layout.focus_panel(index);
            }
            Action::Click(x, y) => {
                // Focus panel based on click position
                if let Some(panel) = self.panel_at_position(x, y) {
                    self.layout.focused = panel;
                }
            }
            Action::MouseScroll(x, y, delta) => {
                // Focus panel and scroll
                if let Some(panel) = self.panel_at_position(x, y) {
                    self.layout.focused = panel;
                    let scroll_action = if delta < 0 { Action::ScrollUp } else { Action::ScrollDown };
                    self.delegate_action(&scroll_action);
                }
            }
            // Delegate to focused panel
            _ => {
                self.delegate_action(&action);
            }
        }
    }

    /// Delegate an action to the focused panel
    fn delegate_action(&mut self, action: &Action) {
        match self.layout.focused {
            PanelId::Logs => self.logs_panel.handle_action(action),
            PanelId::Torrents => self.torrents_panel.handle_action(action),
            PanelId::System => self.system_panel.handle_action(action),
            PanelId::Libraries => self.libraries_panel.handle_action(action),
            PanelId::Database => self.database_panel.handle_action(action),
        }
    }

    /// Determine which panel is at the given screen position
    fn panel_at_position(&self, x: u16, y: u16) -> Option<PanelId> {
        // Get current terminal size and calculate areas
        let size = self.terminal.size().ok()?;
        let area = Rect::new(0, 0, size.width, size.height);
        let areas = self.layout.calculate_areas(area);

        // Check each panel area
        if contains_point(&areas.logs, x, y) { return Some(PanelId::Logs); }
        if contains_point(&areas.torrents, x, y) { return Some(PanelId::Torrents); }
        if contains_point(&areas.system, x, y) { return Some(PanelId::System); }
        if contains_point(&areas.libraries, x, y) { return Some(PanelId::Libraries); }
        if contains_point(&areas.database, x, y) { return Some(PanelId::Database); }
        None
    }

    /// Update all panels
    fn update_panels(&mut self) {
        self.logs_panel.update();
        self.torrents_panel.update();
        self.system_panel.update();
        self.libraries_panel.update();
        self.database_panel.update();
    }

    /// Cleanup terminal state
    fn cleanup(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        // Best effort cleanup on drop
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

/// Check if a point is within a rectangle
fn contains_point(rect: &Rect, x: u16, y: u16) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}
