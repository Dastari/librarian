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
use sqlx::PgPool;
use tokio::sync::broadcast;

use crate::services::{LogEvent, SharedMetrics, TorrentService};
use crate::tui::input::{Action, InputHandler};
use crate::tui::panels::{DatabasePanel, LogsPanel, Panel, SystemPanel, TorrentsPanel, UsersPanel, spawn_torrent_updater};
use crate::tui::panels::users::{SessionTracker, create_session_tracker};
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
    /// Users panel
    users_panel: UsersPanel,
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

        // Create session tracker
        let sessions = create_session_tracker();

        // Create shared torrent list and spawn updater task
        let torrents = Arc::new(parking_lot::RwLock::new(Vec::new()));
        spawn_torrent_updater(torrent_service, torrents.clone());

        Ok(Self {
            terminal,
            input: InputHandler::new(config.tick_rate_ms),
            layout: UiLayout::default(),
            logs_panel: LogsPanel::new(log_rx),
            torrents_panel: TorrentsPanel::new(torrents),
            system_panel: SystemPanel::new(metrics),
            users_panel: UsersPanel::new(sessions),
            database_panel: DatabasePanel::new(pool),
            should_quit: false,
        })
    }

    /// Get the session tracker for registering user activity
    #[allow(dead_code)]
    pub fn session_tracker(&self) -> SessionTracker {
        // Note: This returns a clone, but the actual tracker is inside UsersPanel
        // In a real implementation, we'd want to share the tracker more broadly
        create_session_tracker()
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
                &self.users_panel,
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
            // Delegate to focused panel
            _ => {
                match self.layout.focused {
                    PanelId::Logs => self.logs_panel.handle_action(&action),
                    PanelId::Torrents => self.torrents_panel.handle_action(&action),
                    PanelId::System => self.system_panel.handle_action(&action),
                    PanelId::Users => self.users_panel.handle_action(&action),
                    PanelId::Database => self.database_panel.handle_action(&action),
                }
            }
        }
    }

    /// Update all panels
    fn update_panels(&mut self) {
        self.logs_panel.update();
        self.torrents_panel.update();
        self.system_panel.update();
        self.users_panel.update();
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
