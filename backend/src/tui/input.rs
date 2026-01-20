//! Input event handling for the TUI

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use std::time::Duration;

/// Actions that can be triggered by user input
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Quit the application
    Quit,
    /// Switch to the next panel (Tab)
    NextPanel,
    /// Switch to the previous panel (Shift+Tab)
    PrevPanel,
    /// Scroll up in current panel
    ScrollUp,
    /// Scroll down in current panel
    ScrollDown,
    /// Scroll up by a page
    PageUp,
    /// Scroll down by a page
    PageDown,
    /// Go to top
    Home,
    /// Go to bottom
    End,
    /// Toggle pause/resume for logs
    TogglePause,
    /// Open filter dialog
    OpenFilter,
    /// Open search dialog
    OpenSearch,
    /// Clear current panel content (e.g., clear logs)
    Clear,
    /// Refresh data
    Refresh,
    /// Focus a specific panel by index
    FocusPanel(usize),
    /// Toggle help overlay
    ToggleHelp,
    /// Mouse click at position
    Click(u16, u16),
    /// Mouse scroll at position
    MouseScroll(u16, u16, i32),
    /// No action (tick)
    Tick,
}

/// Input handler that converts terminal events to actions
pub struct InputHandler {
    /// Tick rate for polling events
    tick_rate: Duration,
}

impl InputHandler {
    /// Create a new input handler
    pub fn new(tick_rate_ms: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }

    /// Poll for the next action (blocks until event or timeout)
    pub fn next_action(&self) -> std::io::Result<Action> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                Event::Key(key) => Ok(self.handle_key(key)),
                Event::Mouse(mouse) => Ok(self.handle_mouse(mouse)),
                Event::Resize(_, _) => Ok(Action::Tick), // UI will redraw on resize
                _ => Ok(Action::Tick),
            }
        } else {
            Ok(Action::Tick)
        }
    }

    /// Convert a key event to an action
    fn handle_key(&self, key: KeyEvent) -> Action {
        // Handle Ctrl+C and q for quit
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Action::Quit;
        }

        match key.code {
            // Quit
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,

            // Panel navigation
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    Action::PrevPanel
                } else {
                    Action::NextPanel
                }
            }
            KeyCode::BackTab => Action::PrevPanel,

            // Number keys to focus panels directly
            KeyCode::Char('1') => Action::FocusPanel(0),
            KeyCode::Char('2') => Action::FocusPanel(1),
            KeyCode::Char('3') => Action::FocusPanel(2),
            KeyCode::Char('4') => Action::FocusPanel(3),
            KeyCode::Char('5') => Action::FocusPanel(4),

            // Scrolling
            KeyCode::Up | KeyCode::Char('k') => Action::ScrollUp,
            KeyCode::Down | KeyCode::Char('j') => Action::ScrollDown,
            KeyCode::PageUp => Action::PageUp,
            KeyCode::PageDown => Action::PageDown,
            KeyCode::Home | KeyCode::Char('g') => Action::Home,
            KeyCode::End | KeyCode::Char('G') => Action::End,

            // Actions
            KeyCode::Char(' ') | KeyCode::Char('p') => Action::TogglePause,
            KeyCode::Char('f') => Action::OpenFilter,
            KeyCode::Char('/') => Action::OpenSearch,
            KeyCode::Char('c') => Action::Clear,
            KeyCode::Char('r') => Action::Refresh,
            KeyCode::Char('?') | KeyCode::F(1) => Action::ToggleHelp,

            _ => Action::Tick,
        }
    }

    /// Convert a mouse event to an action
    fn handle_mouse(&self, mouse: MouseEvent) -> Action {
        match mouse.kind {
            MouseEventKind::Down(_) => Action::Click(mouse.column, mouse.row),
            MouseEventKind::ScrollUp => Action::MouseScroll(mouse.column, mouse.row, -1),
            MouseEventKind::ScrollDown => Action::MouseScroll(mouse.column, mouse.row, 1),
            _ => Action::Tick,
        }
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new(100) // 100ms tick rate = 10 FPS
    }
}
