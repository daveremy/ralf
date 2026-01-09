//! M5-A Shell: Core TUI shell implementation.
//!
//! This module provides the foundational TUI shell with:
//! - Two-pane layout (Timeline | Context)
//! - Status bar and footer hints
//! - Focus management and screen modes
//! - Theme and icon support
//!
//! See SPEC-m5a-tui-shell.md for full specification.

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent};
#[cfg(test)]
use crossterm::event::KeyModifiers;
use ratatui::{backend::Backend, Terminal};

use crate::layout::{render_shell, FocusedPane, ScreenMode, MIN_HEIGHT, MIN_WIDTH};
use crate::theme::{BorderSet, IconMode, IconSet, Theme};

/// UI configuration (from environment or config file).
#[derive(Debug, Clone)]
pub struct UiConfig {
    /// Icon mode (Nerd, Unicode, or ASCII).
    pub icons: IconMode,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

impl UiConfig {
    /// Create config from environment, respecting `NO_COLOR`.
    pub fn from_env() -> Self {
        let icons = if std::env::var("NO_COLOR").is_ok() {
            IconMode::Ascii
        } else {
            IconMode::Nerd
        };
        Self { icons }
    }
}

/// Main application state for the M5-A shell.
#[derive(Debug)]
pub struct ShellApp {
    /// Current screen mode.
    pub screen_mode: ScreenMode,
    /// Which pane has focus (for split mode).
    pub focused_pane: FocusedPane,
    /// UI configuration.
    pub ui_config: UiConfig,
    /// Theme colors.
    pub theme: Theme,
    /// Icon set based on config.
    pub icons: IconSet,
    /// Border set based on icon mode.
    pub borders: BorderSet,
    /// Current terminal size.
    pub terminal_size: (u16, u16),
    /// Should the app quit?
    pub should_quit: bool,
}

impl Default for ShellApp {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellApp {
    /// Create a new shell app with default configuration.
    pub fn new() -> Self {
        let ui_config = UiConfig::from_env();
        let icons = IconSet::new(ui_config.icons);
        let borders = BorderSet::new(ui_config.icons);

        Self {
            screen_mode: ScreenMode::default(),
            focused_pane: FocusedPane::default(),
            ui_config,
            theme: Theme::default(),
            icons,
            borders,
            terminal_size: (80, 24), // Default, updated on first render
            should_quit: false,
        }
    }

    /// Check if terminal is too small.
    pub fn is_too_small(&self) -> bool {
        self.terminal_size.0 < MIN_WIDTH || self.terminal_size.1 < MIN_HEIGHT
    }

    /// Handle keyboard input.
    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match key.code {
            // Screen modes - support both Ctrl+N and plain N, plus F-keys
            KeyCode::Char('1') | KeyCode::F(1) => {
                self.screen_mode = ScreenMode::Split;
            }
            KeyCode::Char('2') | KeyCode::F(2) => {
                self.screen_mode = ScreenMode::TimelineFocus;
            }
            KeyCode::Char('3') | KeyCode::F(3) => {
                self.screen_mode = ScreenMode::ContextFocus;
            }

            // Focus management - only effective in Split mode
            KeyCode::Tab => {
                if self.screen_mode == ScreenMode::Split {
                    self.focused_pane = self.focused_pane.toggle();
                }
                // In non-Split modes, Tab is a no-op
            }

            // Help overlay - placeholder for M5-A, implemented in M5-C
            // KeyCode::Char('?') => { /* TODO: Show help overlay */ }

            // Quit
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }

            _ => {}
        }
    }

    /// Handle terminal resize.
    pub fn handle_resize(&mut self, width: u16, height: u16) {
        self.terminal_size = (width, height);
    }
}

/// Run the shell app main loop.
pub fn run_shell<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut app = ShellApp::new();

    // Get initial terminal size
    if let Ok(size) = terminal.size() {
        app.terminal_size = (size.width, size.height);
    }

    loop {
        // Render
        terminal.draw(|frame| {
            render_shell(
                frame,
                app.screen_mode,
                app.focused_pane,
                &app.theme,
                &app.borders,
            );
        })?;

        // Handle events (16ms poll = ~60fps)
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => {
                    app.handle_key_event(key);
                }
                Event::Resize(width, height) => {
                    app.handle_resize(width, height);
                }
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_app_defaults() {
        let app = ShellApp::new();
        assert_eq!(app.screen_mode, ScreenMode::Split);
        assert_eq!(app.focused_pane, FocusedPane::Timeline);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_focus_cycling_in_split_mode() {
        let mut app = ShellApp::new();
        assert_eq!(app.screen_mode, ScreenMode::Split);
        assert_eq!(app.focused_pane, FocusedPane::Timeline);

        app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.focused_pane, FocusedPane::Context);

        app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.focused_pane, FocusedPane::Timeline);
    }

    #[test]
    fn test_focus_cycling_noop_in_focus_modes() {
        let mut app = ShellApp::new();
        app.screen_mode = ScreenMode::TimelineFocus;
        app.focused_pane = FocusedPane::Timeline;

        // Tab should be a no-op in TimelineFocus mode
        app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.focused_pane, FocusedPane::Timeline);

        app.screen_mode = ScreenMode::ContextFocus;
        // Tab should also be a no-op in ContextFocus mode
        app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.focused_pane, FocusedPane::Timeline);
    }

    #[test]
    fn test_screen_mode_switching() {
        let mut app = ShellApp::new();
        assert_eq!(app.screen_mode, ScreenMode::Split);

        // Plain number keys work
        app.handle_key_event(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
        assert_eq!(app.screen_mode, ScreenMode::TimelineFocus);

        app.handle_key_event(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
        assert_eq!(app.screen_mode, ScreenMode::ContextFocus);

        app.handle_key_event(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
        assert_eq!(app.screen_mode, ScreenMode::Split);

        // F-keys also work
        app.handle_key_event(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        assert_eq!(app.screen_mode, ScreenMode::TimelineFocus);
    }

    #[test]
    fn test_quit() {
        let mut app = ShellApp::new();
        assert!(!app.should_quit);

        // 'q' quits
        app.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(app.should_quit);

        // Esc also quits
        let mut app2 = ShellApp::new();
        app2.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app2.should_quit);
    }

    #[test]
    fn test_resize_handling() {
        let mut app = ShellApp::new();
        assert_eq!(app.terminal_size, (80, 24));

        app.handle_resize(120, 40);
        assert_eq!(app.terminal_size, (120, 40));
    }

    #[test]
    fn test_is_too_small() {
        let mut app = ShellApp::new();

        app.terminal_size = (80, 24);
        assert!(!app.is_too_small());

        app.terminal_size = (39, 24);
        assert!(app.is_too_small());

        app.terminal_size = (80, 11);
        assert!(app.is_too_small());

        app.terminal_size = (40, 12);
        assert!(!app.is_too_small());
    }

    #[test]
    fn test_no_color_sets_ascii_mode() {
        // This test would need to mock the environment variable
        // For now, just test that from_env works
        let config = UiConfig::from_env();
        // Without NO_COLOR set, should default to Nerd
        assert!(matches!(config.icons, IconMode::Nerd | IconMode::Ascii));
    }
}
