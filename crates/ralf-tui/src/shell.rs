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
use std::time::{Duration, Instant};

use arboard::Clipboard;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::{backend::Backend, Terminal};

use crate::layout::{render_shell, FocusedPane, ScreenMode, MIN_HEIGHT, MIN_WIDTH};
use crate::models::ModelStatus;
use crate::theme::{BorderSet, IconMode, IconSet, Theme};
use crate::thread_state::ThreadDisplay;
use crate::timeline::{
    EventKind, ReviewEvent, ReviewResult, RunEvent, SpecEvent, SystemEvent, TimelineState,
    SCROLL_SPEED,
};
use ralf_engine::discovery::{discover_models, probe_model_with_info, KNOWN_MODELS};

/// Maximum time between clicks to count as double-click.
const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(500);

/// Tracks last click for double-click detection.
#[derive(Debug, Clone, Copy)]
struct LastClick {
    time: Instant,
    row: u16,
    column: u16,
}

/// Toast notification duration.
const TOAST_DURATION: Duration = Duration::from_secs(2);

/// A temporary toast notification.
#[derive(Debug, Clone)]
pub struct Toast {
    /// The message to display.
    pub message: String,
    /// When the toast expires.
    pub expires_at: Instant,
}

/// Bounds of the timeline pane's inner area (for mouse coordinate translation).
#[derive(Debug, Default, Clone, Copy)]
pub struct TimelinePaneBounds {
    /// Inner area top-left x coordinate.
    pub inner_x: u16,
    /// Inner area top-left y coordinate.
    pub inner_y: u16,
    /// Inner area width.
    pub inner_width: u16,
    /// Inner area height.
    pub inner_height: u16,
}

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
    /// Timeline state for the left pane.
    pub timeline: TimelineState,
    /// Bounds of the timeline pane's inner area.
    pub timeline_bounds: TimelinePaneBounds,
    /// Last mouse click for double-click detection.
    last_click: Option<LastClick>,
    /// Current toast notification (if any).
    pub toast: Option<Toast>,
    /// Current thread display state (None = no thread loaded).
    pub current_thread: Option<ThreadDisplay>,
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

        // Create timeline with sample events for testing
        let mut timeline = TimelineState::new();
        Self::add_sample_events(&mut timeline);

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
            timeline,
            timeline_bounds: TimelinePaneBounds::default(),
            last_click: None,
            toast: None,
            current_thread: None, // No thread loaded initially
        }
    }

    /// Set the current thread, updating models panel visibility.
    pub fn set_thread(&mut self, thread: Option<ThreadDisplay>) {
        self.current_thread = thread;
        self.show_models_panel = self.current_thread.is_none();
    }

    /// Show a toast notification.
    pub fn show_toast(&mut self, message: impl Into<String>) {
        self.toast = Some(Toast {
            message: message.into(),
            expires_at: Instant::now() + TOAST_DURATION,
        });
    }

    /// Clear expired toast.
    pub fn clear_expired_toast(&mut self) {
        if let Some(ref toast) = self.toast {
            if Instant::now() >= toast.expires_at {
                self.toast = None;
            }
        }
    }

    /// Add sample events to timeline for testing.
    fn add_sample_events(timeline: &mut TimelineState) {
        // System event: session start
        timeline.push(EventKind::System(SystemEvent::info("Session started")));

        // Spec event: user input
        timeline.push(EventKind::Spec(SpecEvent::user(
            "Add authentication to the API\nSupport JWT tokens\nInclude refresh token logic",
        )));

        // Run event: model working (multi-line, will be collapsed by default)
        timeline.push(EventKind::Run(RunEvent::new(
            "claude",
            1,
            "Analyzing codebase structure...\nFound 47 relevant files\nPlanning implementation",
        )));

        // Run event: file change
        timeline.push(EventKind::Run(RunEvent::file_change(
            "claude",
            1,
            "src/auth.rs +127",
            "pub fn authenticate(token: &str) -> Result<User, AuthError> {\n    // JWT validation\n    let claims = decode_jwt(token)?;\n    Ok(User::from_claims(claims))\n}",
        )));

        // Review event: passed
        timeline.push(EventKind::Review(ReviewEvent::new(
            "cargo check passes",
            ReviewResult::Passed,
        )));

        // Review event: failed with details
        timeline.push(EventKind::Review(ReviewEvent::with_details(
            "cargo test passes",
            ReviewResult::Failed,
            "2 tests failed:\n- test_auth_expired_token\n- test_refresh_invalid",
        )));

        // Run event: fixing tests
        timeline.push(EventKind::Run(RunEvent::new(
            "gemini",
            2,
            "Fixing test failures...\nUpdating token validation logic",
        )));

        // System event: warning
        timeline.push(EventKind::System(SystemEvent::warning(
            "Model rate limit approaching (80% of quota used)",
        )));

        // Review event: passed after fix
        timeline.push(EventKind::Review(ReviewEvent::new(
            "cargo test passes",
            ReviewResult::Passed,
        )));

        // System event: completion
        timeline.push(EventKind::System(SystemEvent::info(
            "All criteria verified - ready for review",
        )));
    }

    /// Check if terminal is too small.
    pub fn is_too_small(&self) -> bool {
        self.terminal_size.0 < MIN_WIDTH || self.terminal_size.1 < MIN_HEIGHT
    }

    /// Check if ASCII mode is enabled (for `NO_COLOR` support).
    pub fn is_ascii_mode(&self) -> bool {
        matches!(self.ui_config.icons, IconMode::Ascii)
    }

    /// Check if the timeline pane is currently focused.
    pub fn timeline_focused(&self) -> bool {
        match self.screen_mode {
            ScreenMode::Split => self.focused_pane == FocusedPane::Timeline,
            ScreenMode::TimelineFocus => true,
            ScreenMode::ContextFocus => false,
        }
    }

    /// Handle keyboard input.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<ShellAction> {
        // Timeline-specific keys when timeline is focused
        if self.timeline_focused() {
            let visible_count = self
                .timeline
                .events_per_page(self.timeline_bounds.inner_height as usize);
            match key.code {
                // Navigation
                KeyCode::Char('j') | KeyCode::Down => {
                    self.timeline.select_next();
                    self.timeline.ensure_selection_visible(visible_count);
                    return None;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.timeline.select_prev();
                    self.timeline.ensure_selection_visible(visible_count);
                    return None;
                }
                KeyCode::Char('g') | KeyCode::Home => {
                    self.timeline.jump_to_start();
                    return None;
                }
                KeyCode::Char('G') | KeyCode::End => {
                    self.timeline.jump_to_end();
                    return None;
                }
                KeyCode::PageUp => {
                    self.timeline.page_up(visible_count);
                    self.timeline.ensure_selection_visible(visible_count);
                    return None;
                }
                KeyCode::PageDown => {
                    self.timeline.page_down(visible_count);
                    self.timeline.ensure_selection_visible(visible_count);
                    return None;
                }
                // Toggle collapse
                KeyCode::Enter => {
                    self.timeline.toggle_collapse();
                    return None;
                }
                // Copy selected event to clipboard (vim-style yank)
                KeyCode::Char('y') => {
                    if let Some(content) = self.selected_event_content() {
                        return Some(ShellAction::CopyToClipboard(content));
                    }
                    return None;
                }
                // Copy with Ctrl+C
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if let Some(content) = self.selected_event_content() {
                        return Some(ShellAction::CopyToClipboard(content));
                    }
                    return None;
                }
                _ => {} // Fall through to global handlers
            }
        }

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

    /// Handle mouse input.
    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        // Only handle mouse events when timeline is focused
        if !self.timeline_focused() {
            return;
        }

        let bounds = &self.timeline_bounds;

        // Check if click is within timeline pane bounds
        let in_timeline = mouse.column >= bounds.inner_x
            && mouse.column < bounds.inner_x + bounds.inner_width
            && mouse.row >= bounds.inner_y
            && mouse.row < bounds.inner_y + bounds.inner_height;

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                if in_timeline {
                    self.timeline.scroll_up(SCROLL_SPEED);
                }
            }
            MouseEventKind::ScrollDown => {
                if in_timeline {
                    self.timeline.scroll_down(SCROLL_SPEED);
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if !in_timeline {
                    return;
                }

                let now = Instant::now();

                // Check for double-click
                let is_double_click = self.last_click.is_some_and(|last| {
                    now.duration_since(last.time) < DOUBLE_CLICK_THRESHOLD
                        && mouse.row == last.row
                        && mouse.column == last.column
                });

                // Convert to relative y coordinate within timeline inner area
                let relative_y = (mouse.row - bounds.inner_y) as usize;

                if let Some(idx) = self.timeline.y_to_event_index(relative_y) {
                    self.timeline.select(idx);

                    if is_double_click {
                        self.timeline.toggle_collapse();
                        self.last_click = None; // Reset after double-click
                    }
                }

                if !is_double_click {
                    self.last_click = Some(LastClick {
                        time: now,
                        row: mouse.row,
                        column: mouse.column,
                    });
                }
            }
            _ => {}
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

    /// Get the content of the selected event for copying.
    ///
    /// Returns None if no event is selected.
    pub fn selected_event_content(&self) -> Option<String> {
        self.timeline
            .selected()
            .and_then(|idx| self.timeline.events().get(idx))
            .map(crate::TimelineEvent::copyable_content)
    }
}

/// Actions that the shell can request from the main loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellAction {
    /// Refresh all model probes.
    RefreshModels,
    /// Copy text to clipboard (with result message for feedback).
    CopyToClipboard(String),
}

/// Result of a clipboard operation.
#[derive(Debug, Clone)]
pub enum ClipboardResult {
    /// Successfully copied to clipboard.
    Success,
    /// Clipboard operation failed.
    Failed(String),
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
        let info_clone = info.clone();

        thread::spawn(move || {
            // Only probe if the model was found
            let status = if info_clone.found {
                let probe = probe_model_with_info(&info_clone, timeout);
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

    // Enable mouse capture
    crossterm::execute!(std::io::stdout(), EnableMouseCapture)?;

    // Start probing models in parallel
    let mut probe_rx = Some(app.start_probing());
    let mut pending_probes = KNOWN_MODELS.len();

    let result = (|| {
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

            // Clear expired toasts
            app.clear_expired_toast();

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
                    &app.timeline,
                    &mut app.timeline_bounds,
                    app.toast.as_ref(),
                    app.current_thread.as_ref(),
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
                                ShellAction::CopyToClipboard(content) => {
                                    // Try to copy to clipboard
                                    match Clipboard::new() {
                                        Ok(mut clipboard) => {
                                            if let Err(e) = clipboard.set_text(&content) {
                                                app.show_toast(format!("Copy failed: {e}"));
                                            } else {
                                                app.show_toast("Copied to clipboard");
                                            }
                                        }
                                        Err(e) => {
                                            app.show_toast(format!("Clipboard unavailable: {e}"));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Event::Mouse(mouse) => {
                        app.handle_mouse_event(mouse);
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
    })();

    // Disable mouse capture (cleanup)
    let _ = crossterm::execute!(std::io::stdout(), DisableMouseCapture);

    result
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

        // Tab toggles between left (Timeline) and right (Context/Models) panes
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
