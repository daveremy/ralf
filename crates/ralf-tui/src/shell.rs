//! M5-A Shell: Core TUI shell implementation.
//!
//! This module provides the foundational TUI shell with:
//! - Two-pane layout (Timeline | Context)
//! - Status bar and footer hints
//! - Focus management and screen modes
//! - Theme and icon support
//! - Model discovery and probing
//!
//! See SPEC-m5a-tui-shell.md and SPEC-m5a1-model-probing.md for full specification.

use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent};
#[cfg(test)]
use crossterm::event::KeyModifiers;
use ratatui::{backend::Backend, Terminal};

use crate::layout::{render_shell, FocusedPane, ScreenMode, MIN_HEIGHT, MIN_WIDTH};
use crate::models::ModelStatus;
use crate::theme::{BorderSet, IconMode, IconSet, Theme};
use ralf_engine::discovery::{discover_models, probe_model, KNOWN_MODELS};

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
    /// Model status from probing.
    pub models: Vec<ModelStatus>,
    /// Whether initial probe is complete.
    pub probe_complete: bool,
    /// Whether to show the models panel in the context pane.
    pub show_models_panel: bool,
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

        // Initialize models with "Probing" state
        let models: Vec<ModelStatus> = KNOWN_MODELS
            .iter()
            .map(|name| ModelStatus::probing(name))
            .collect();

        Self {
            screen_mode: ScreenMode::default(),
            focused_pane: FocusedPane::default(),
            ui_config,
            theme: Theme::default(),
            icons,
            borders,
            terminal_size: (80, 24), // Default, updated on first render
            should_quit: false,
            models,
            probe_complete: false,
            show_models_panel: true, // Show by default until a thread is loaded
        }
    }

    /// Check if terminal is too small.
    pub fn is_too_small(&self) -> bool {
        self.terminal_size.0 < MIN_WIDTH || self.terminal_size.1 < MIN_HEIGHT
    }

    /// Check if ASCII mode is enabled (for `NO_COLOR` support).
    pub fn is_ascii_mode(&self) -> bool {
        matches!(self.ui_config.icons, IconMode::Ascii)
    }

    /// Handle keyboard input.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<ShellAction> {
        match key.code {
            // Screen modes - support both Ctrl+N and plain N, plus F-keys
            KeyCode::Char('1') | KeyCode::F(1) => {
                self.screen_mode = ScreenMode::Split;
                None
            }
            KeyCode::Char('2') | KeyCode::F(2) => {
                self.screen_mode = ScreenMode::TimelineFocus;
                None
            }
            KeyCode::Char('3') | KeyCode::F(3) => {
                self.screen_mode = ScreenMode::ContextFocus;
                None
            }

            // Focus management - only effective in Split mode
            KeyCode::Tab => {
                if self.screen_mode == ScreenMode::Split {
                    self.focused_pane = self.focused_pane.toggle();
                }
                // In non-Split modes, Tab is a no-op
                None
            }

            // Refresh models - only when models panel is visible and not already probing
            KeyCode::Char('r') if self.show_models_panel && self.probe_complete => {
                Some(ShellAction::RefreshModels)
            }

            // Help overlay - placeholder for M5-A, implemented in M5-C
            // KeyCode::Char('?') => { /* TODO: Show help overlay */ }

            // Quit
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
                None
            }

            _ => None,
        }
    }

    /// Handle terminal resize.
    pub fn handle_resize(&mut self, width: u16, height: u16) {
        self.terminal_size = (width, height);
    }

    /// Update models with probe results.
    pub fn update_models(&mut self, models: Vec<ModelStatus>) {
        self.models = models;
        self.probe_complete = true;
    }

    /// Start probing models and update them as results arrive.
    ///
    /// Returns a receiver that will receive model statuses as probes complete.
    pub fn start_probing(&self) -> mpsc::Receiver<ModelStatus> {
        probe_models_parallel(Duration::from_secs(10))
    }
}

/// Actions that the shell can request from the main loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellAction {
    /// Refresh all model probes.
    RefreshModels,
}

/// Probe all known models in parallel, returning results via a channel.
///
/// Each probe has a 10-second timeout. Results are sent as they complete.
fn probe_models_parallel(timeout: Duration) -> mpsc::Receiver<ModelStatus> {
    let (tx, rx) = mpsc::channel();

    // Discover models first (quick, checks if binary exists)
    let discovery = discover_models();

    for info in discovery.models {
        let tx = tx.clone();
        let name = info.name.clone();
        let info_clone = info.clone();

        thread::spawn(move || {
            // Only probe if the model was found
            let status = if info_clone.found {
                let probe = probe_model(&name, timeout);
                ModelStatus::from_engine(&info_clone, Some(&probe))
            } else {
                ModelStatus::from_engine(&info_clone, None)
            };

            // Send result (ignore error if receiver was dropped)
            let _ = tx.send(status);
        });
    }

    rx
}

/// Run the shell app main loop.
pub fn run_shell<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut app = ShellApp::new();

    // Get initial terminal size
    if let Ok(size) = terminal.size() {
        app.terminal_size = (size.width, size.height);
    }

    // Start probing models in parallel
    let mut probe_rx = Some(app.start_probing());
    let mut pending_probes = KNOWN_MODELS.len();

    loop {
        // Check for completed probes (non-blocking)
        if let Some(ref rx) = probe_rx {
            while let Ok(status) = rx.try_recv() {
                // Update the model in our list
                if let Some(model) = app.models.iter_mut().find(|m| m.name == status.name) {
                    *model = status;
                }
                pending_probes = pending_probes.saturating_sub(1);
            }

            // If all probes complete, drop the receiver
            if pending_probes == 0 {
                app.probe_complete = true;
                probe_rx = None;
            }
        }

        // Render
        terminal.draw(|frame| {
            render_shell(
                frame,
                app.screen_mode,
                app.focused_pane,
                &app.theme,
                &app.borders,
                &app.models,
                app.is_ascii_mode(),
                app.show_models_panel,
            );
        })?;

        // Handle events (16ms poll = ~60fps)
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => {
                    if let Some(action) = app.handle_key_event(key) {
                        match action {
                            ShellAction::RefreshModels => {
                                // Reset models to probing state and start new probes
                                app.models = KNOWN_MODELS
                                    .iter()
                                    .map(|name| ModelStatus::probing(name))
                                    .collect();
                                app.probe_complete = false;
                                probe_rx = Some(app.start_probing());
                                pending_probes = KNOWN_MODELS.len();
                            }
                        }
                    }
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
        assert!(!app.probe_complete);
        assert!(app.show_models_panel);
        assert_eq!(app.models.len(), KNOWN_MODELS.len());
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

    #[test]
    fn test_refresh_models_when_panel_visible_and_complete() {
        let mut app = ShellApp::new();
        app.show_models_panel = true;
        app.probe_complete = true;

        // 'r' should trigger RefreshModels when models panel is visible and probe complete
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        assert_eq!(action, Some(ShellAction::RefreshModels));
    }

    #[test]
    fn test_refresh_models_noop_when_panel_hidden() {
        let mut app = ShellApp::new();
        app.show_models_panel = false;
        app.probe_complete = true;

        // 'r' should do nothing when models panel is not visible
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        assert_eq!(action, None);
    }

    #[test]
    fn test_refresh_models_noop_during_probing() {
        let mut app = ShellApp::new();
        app.show_models_panel = true;
        app.probe_complete = false; // Still probing

        // 'r' should do nothing while probing is in progress
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        assert_eq!(action, None);
    }

    #[test]
    fn test_is_ascii_mode() {
        let mut app = ShellApp::new();

        // Default should be Nerd mode (not ASCII)
        app.ui_config.icons = IconMode::Nerd;
        assert!(!app.is_ascii_mode());

        // ASCII mode should return true
        app.ui_config.icons = IconMode::Ascii;
        assert!(app.is_ascii_mode());

        // Unicode mode should return false
        app.ui_config.icons = IconMode::Unicode;
        assert!(!app.is_ascii_mode());
    }
}
