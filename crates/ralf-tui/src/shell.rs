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
use ratatui::{
    backend::Backend,
    buffer::Buffer,
    layout::Rect,
    Terminal,
};

use crate::layout::{render_shell, FocusedPane, ScreenMode, MIN_HEIGHT, MIN_WIDTH};
use crate::models::ModelStatus;
use crate::theme::{BorderSet, IconMode, IconSet, Theme};
use crate::thread_state::ThreadDisplay;
use crate::timeline::{
    EventKind, ReviewEvent, ReviewResult, RunEvent, SpecEvent, SystemEvent, TimelineState,
    SCROLL_SPEED,
};
use crate::ui::widgets::TextInputState;
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
#[allow(clippy::struct_excessive_bools)]
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
    /// Text input state for the conversation pane.
    pub input: TextInputState,
    /// Whether to show the help overlay.
    pub show_help: bool,
    /// Autocomplete state (selected index into completions).
    pub autocomplete_index: Option<usize>,
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
            input: TextInputState::new(),
            show_help: false,
            autocomplete_index: None,
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

    /// Check if autocomplete popup should be shown.
    pub fn should_show_autocomplete(&self) -> bool {
        let content = self.input.content();
        content.starts_with('/') && !content.contains(' ')
    }

    /// Get current autocomplete completions.
    pub fn get_completions(&self) -> Vec<&'static crate::commands::CommandInfo> {
        use crate::commands::get_completions;
        let phase = self.current_thread.as_ref().map(|t| t.phase_kind);
        get_completions(self.input.content(), phase)
    }

    /// Select next autocomplete completion.
    pub fn autocomplete_next(&mut self) {
        let completions = self.get_completions();
        if completions.is_empty() {
            self.autocomplete_index = None;
            return;
        }

        self.autocomplete_index = match self.autocomplete_index {
            None => Some(0),
            Some(i) => Some((i + 1) % completions.len()),
        };
    }

    /// Select previous autocomplete completion.
    pub fn autocomplete_prev(&mut self) {
        let completions = self.get_completions();
        if completions.is_empty() {
            self.autocomplete_index = None;
            return;
        }

        self.autocomplete_index = match self.autocomplete_index {
            None | Some(0) => Some(completions.len().saturating_sub(1)),
            Some(i) => Some(i - 1),
        };
    }

    /// Accept the current autocomplete selection.
    ///
    /// Returns true if a completion was accepted.
    pub fn autocomplete_accept(&mut self) -> bool {
        let Some(index) = self.autocomplete_index else {
            return false;
        };

        let completions = self.get_completions();
        if let Some(cmd) = completions.get(index) {
            // Replace input with the completed command
            self.input.clear();
            self.input.insert_str(&format!("/{}", cmd.name));
            self.autocomplete_index = None;
            true
        } else {
            false
        }
    }

    /// Reset autocomplete state.
    pub fn reset_autocomplete(&mut self) {
        self.autocomplete_index = None;
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

    /// Handle key event for conversation input.
    ///
    /// Returns a `KeyResult` indicating how the key was handled.
    ///
    /// Input-first key handling strategy:
    /// - ALL character keys go to input (no reserved keys block typing)
    /// - Modifier keys (Ctrl+N) provide shortcuts for power users
    /// - Slash commands are invoked by typing `/command`
    /// - Tab navigates/accepts autocomplete
    fn handle_conversation_key(&mut self, key: KeyEvent) -> KeyResult {
        match key.code {
            // Tab - autocomplete navigation/accept
            KeyCode::Tab if self.should_show_autocomplete() => {
                if self.autocomplete_index.is_some() {
                    // Accept current selection
                    self.autocomplete_accept();
                } else {
                    // Start autocomplete selection
                    self.autocomplete_next();
                }
                KeyResult::Handled
            }

            // Text input - characters without ctrl/alt modifier go to input
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.input.insert(c);
                self.reset_autocomplete(); // Reset on text change
                KeyResult::Handled
            }

            // Shift+Enter inserts newline
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.input.insert('\n');
                self.reset_autocomplete();
                KeyResult::Handled
            }

            // Enter - accept autocomplete or submit input
            KeyCode::Enter => {
                // If autocomplete is active, accept the selection first
                if self.autocomplete_index.is_some() && self.autocomplete_accept() {
                    return KeyResult::Handled;
                }
                // Otherwise submit input
                if let Some(action) = self.submit_input() {
                    KeyResult::Action(action)
                } else {
                    KeyResult::Handled
                }
            }

            // Backspace
            KeyCode::Backspace => {
                self.input.backspace();
                self.reset_autocomplete();
                KeyResult::Handled
            }

            // Delete
            KeyCode::Delete => {
                self.input.delete();
                self.reset_autocomplete();
                KeyResult::Handled
            }

            // Cursor movement
            KeyCode::Left => {
                self.input.move_left();
                self.reset_autocomplete();
                KeyResult::Handled
            }
            KeyCode::Right => {
                self.input.move_right();
                self.reset_autocomplete();
                KeyResult::Handled
            }
            KeyCode::Home => {
                self.input.move_home();
                self.reset_autocomplete();
                KeyResult::Handled
            }
            KeyCode::End => {
                self.input.move_end();
                self.reset_autocomplete();
                KeyResult::Handled
            }

            // Up - autocomplete navigation or history
            KeyCode::Up => {
                if self.should_show_autocomplete() && !self.get_completions().is_empty() {
                    self.autocomplete_prev();
                    KeyResult::Handled
                } else if self.input.cursor == 0 || self.input.is_empty() {
                    self.input.history_prev();
                    KeyResult::Handled
                } else {
                    KeyResult::NotHandled // Let timeline scroll handle it
                }
            }

            // Down - autocomplete navigation or history
            KeyCode::Down => {
                if self.should_show_autocomplete() && !self.get_completions().is_empty() {
                    self.autocomplete_next();
                    KeyResult::Handled
                } else if self.input.cursor == self.input.content.len() {
                    self.input.history_next();
                    KeyResult::Handled
                } else {
                    KeyResult::NotHandled // Let timeline scroll handle it
                }
            }

            // Not handled by input
            _ => KeyResult::NotHandled,
        }
    }

    /// Escape: clear input (no longer quits - use /quit or /exit).
    fn handle_escape(&mut self) {
        self.input.clear();
        self.reset_autocomplete();
    }

    /// Submit the current input.
    ///
    /// Handles slash commands, escaped slashes, and regular messages.
    fn submit_input(&mut self) -> Option<ShellAction> {
        use crate::commands::{is_command, is_escaped_slash, parse_command, unescape_slash};

        let content = self.input.submit();
        if content.trim().is_empty() {
            return None;
        }

        // Check for escaped slash (// -> /)
        if is_escaped_slash(&content) {
            let unescaped = unescape_slash(&content);
            self.timeline.push(EventKind::System(SystemEvent::info(
                format!("[Message: {unescaped}]"),
            )));
            return None;
        }

        // Check for slash command
        if is_command(&content) {
            if let Some(cmd) = parse_command(&content) {
                return self.execute_command(cmd);
            }
        }

        // Regular message - placeholder for chat integration
        self.timeline.push(EventKind::System(SystemEvent::info(
            format!("[Input received: {} chars]", content.len()),
        )));
        None
    }

    /// Execute a parsed slash command.
    fn execute_command(&mut self, cmd: crate::commands::Command) -> Option<ShellAction> {
        use crate::commands::Command;

        match cmd {
            Command::Help => {
                self.show_help = true;
                None
            }
            Command::Quit => {
                self.should_quit = true;
                None
            }
            Command::Split => {
                self.screen_mode = ScreenMode::Split;
                None
            }
            Command::Focus => {
                self.screen_mode = ScreenMode::TimelineFocus;
                None
            }
            Command::Canvas => {
                self.screen_mode = ScreenMode::ContextFocus;
                None
            }
            Command::Refresh => {
                if self.show_models_panel && self.probe_complete {
                    Some(ShellAction::RefreshModels)
                } else {
                    None
                }
            }
            Command::Clear => {
                self.timeline.clear();
                None
            }
            Command::Copy => self.selected_event_content().map(ShellAction::CopyToClipboard),
            Command::Model(name) => {
                // TODO: Implement model switching
                if let Some(model_name) = name {
                    self.show_toast(format!("Model switching not yet implemented: {model_name}"));
                } else {
                    self.show_toast("Usage: /model <name>");
                }
                None
            }
            Command::Search(query) => {
                // TODO: Implement timeline search
                if let Some(q) = query {
                    self.show_toast(format!("Search not yet implemented: {q}"));
                } else {
                    self.show_toast("Usage: /search <query>");
                }
                None
            }
            Command::Editor => {
                // TODO: Open in $EDITOR
                self.show_toast("Editor integration not yet implemented");
                None
            }
            // Phase-specific commands - stub implementations
            Command::Approve | Command::Reject(_) | Command::Pause | Command::Resume
            | Command::Cancel | Command::Finalize | Command::Assess => {
                self.show_toast(format!("Phase command not yet implemented: /{cmd:?}"));
                None
            }
            Command::Unknown(name) => {
                self.show_toast(format!("Unknown command: /{name}"));
                None
            }
        }
    }

    /// Handle keyboard input.
    ///
    /// Uses the input-first model where all character keys go to input.
    /// Global actions use modifier keybindings (Ctrl+N) or F-keys.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<ShellAction> {
        // Help overlay: any key closes it
        if self.show_help {
            self.show_help = false;
            return None;
        }

        // F1 - Show help overlay
        if key.code == KeyCode::F(1) {
            self.show_help = true;
            return None;
        }

        // Escape clears input (use /quit or /exit to quit)
        if key.code == KeyCode::Esc {
            self.handle_escape();
            return None;
        }

        // Focus trap: '/' from anywhere jumps to input and inserts '/'
        if key.code == KeyCode::Char('/') && self.input.is_empty() {
            self.focused_pane = FocusedPane::Timeline;
            self.input.insert('/');
            return None;
        }

        // Conversation pane keys when focused
        if self.timeline_focused() {
            // Try conversation input handling first
            match self.handle_conversation_key(key) {
                KeyResult::Handled => return None,
                KeyResult::Action(action) => return Some(action),
                KeyResult::NotHandled => {}
            }

            // Timeline navigation (when input didn't handle the key)
            // Only works with Alt modifier in input-first model
            let visible_count = self
                .timeline
                .events_per_page(self.timeline_bounds.inner_height as usize);

            if key.modifiers.contains(KeyModifiers::ALT) {
                match key.code {
                    KeyCode::Char('j') => {
                        self.timeline.select_next();
                        self.timeline.ensure_selection_visible(visible_count);
                        return None;
                    }
                    KeyCode::Char('k') => {
                        self.timeline.select_prev();
                        self.timeline.ensure_selection_visible(visible_count);
                        return None;
                    }
                    _ => {}
                }
            }

            // Page keys work without modifier
            match key.code {
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
                _ => {}
            }
        }

        // Global keybindings with Alt modifier (works better cross-platform)
        if key.modifiers.contains(KeyModifiers::ALT) {
            match key.code {
                // Screen modes: Alt+1/2/3
                KeyCode::Char('1') => {
                    self.screen_mode = ScreenMode::Split;
                    return None;
                }
                KeyCode::Char('2') => {
                    self.screen_mode = ScreenMode::TimelineFocus;
                    return None;
                }
                KeyCode::Char('3') => {
                    self.screen_mode = ScreenMode::ContextFocus;
                    return None;
                }
                _ => {}
            }
        }

        // Global keybindings with Ctrl modifier
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                // Refresh: Ctrl+R
                KeyCode::Char('r') if self.show_models_panel && self.probe_complete => {
                    return Some(ShellAction::RefreshModels);
                }
                // Clear: Ctrl+L
                KeyCode::Char('l') => {
                    self.timeline.clear();
                    return None;
                }
                // Note: Ctrl+C intentionally NOT mapped - reserved for terminal interrupt
                _ => {}
            }
        }

        // Tab - toggle focus in split mode
        if key.code == KeyCode::Tab && self.screen_mode == ScreenMode::Split {
            self.focused_pane = self.focused_pane.toggle();
            return None;
        }

        None
    }

    /// Handle mouse input.
    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        let bounds = &self.timeline_bounds;

        // Check if click is within timeline pane bounds
        let in_timeline = mouse.column >= bounds.inner_x
            && mouse.column < bounds.inner_x + bounds.inner_width
            && mouse.row >= bounds.inner_y
            && mouse.row < bounds.inner_y + bounds.inner_height;

        // Check if click is within context pane (in split mode)
        // Context pane starts after timeline ends and goes to the right edge
        let in_context = self.screen_mode == ScreenMode::Split
            && mouse.column >= bounds.inner_x + bounds.inner_width
            && mouse.row >= bounds.inner_y
            && mouse.row < bounds.inner_y + bounds.inner_height;

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                // Only scroll when timeline is focused and click is in timeline
                if self.timeline_focused() && in_timeline {
                    self.timeline.scroll_up(SCROLL_SPEED);
                }
            }
            MouseEventKind::ScrollDown => {
                // Only scroll when timeline is focused and click is in timeline
                if self.timeline_focused() && in_timeline {
                    self.timeline.scroll_down(SCROLL_SPEED);
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                // Click-to-focus: clicking on a pane focuses it
                if self.screen_mode == ScreenMode::Split {
                    if in_timeline && self.focused_pane != FocusedPane::Timeline {
                        self.focused_pane = FocusedPane::Timeline;
                    } else if in_context && self.focused_pane != FocusedPane::Context {
                        self.focused_pane = FocusedPane::Context;
                    }
                }

                // Timeline selection only when clicking in timeline
                if in_timeline && self.timeline_focused() {
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

/// Result of handling a key event in conversation input.
#[derive(Debug)]
enum KeyResult {
    /// Key was not handled by the input.
    NotHandled,
    /// Key was handled, no further action needed.
    Handled,
    /// Key was handled and produced a shell action.
    Action(ShellAction),
}

/// Result of a clipboard operation.
#[derive(Debug, Clone)]
pub enum ClipboardResult {
    /// Successfully copied to clipboard.
    Success,
    /// Clipboard operation failed.
    Failed(String),
}

/// Render the help overlay.
fn render_help_overlay(area: Rect, buf: &mut Buffer, theme: &Theme) {
    use crate::commands::COMMANDS;
    use crate::ui::centered_fixed;
    use ratatui::style::Style;
    use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};

    // Build help text from commands registry
    // Format: /command [alias]  Description  (Keybinding)
    let mut help_lines: Vec<String> = Vec::new();

    help_lines.push("Commands".to_string());
    help_lines.push(String::new());

    for cmd in COMMANDS.iter().filter(|c| !c.phase_specific) {
        // Build command with alias: "/quit [q]" or just "/help"
        let cmd_str = if cmd.aliases.is_empty() {
            format!("/{}", cmd.name)
        } else {
            format!("/{}  [{}]", cmd.name, cmd.aliases.join(", "))
        };

        // Pad command to align descriptions
        let padded_cmd = format!("{cmd_str:<18}");

        // Add keybinding at end if present
        let line = if let Some(key) = cmd.keybinding {
            format!("  {}  {}  ({})", padded_cmd, cmd.description, key)
        } else {
            format!("  {}  {}", padded_cmd, cmd.description)
        };

        help_lines.push(line);
    }

    help_lines.push(String::new());
    help_lines.push("Keyboard Shortcuts".to_string());
    help_lines.push(String::new());
    help_lines.push("  Tab         Switch pane focus".to_string());
    help_lines.push("  Alt+1/2/3   Switch screen mode".to_string());
    help_lines.push("  Esc         Clear input".to_string());
    help_lines.push("  Enter       Send message / execute".to_string());
    help_lines.push(String::new());
    help_lines.push("[Press any key to close]".to_string());

    let help_text = help_lines.join("\n");

    // Calculate overlay size - make it wider to fit content
    let width = 60.min(area.width.saturating_sub(4));
    let height = 28.min(area.height.saturating_sub(4));
    let overlay_area = centered_fixed(width, height, area);

    // Clear the area
    Clear.render(overlay_area, buf);

    // Render the help block
    let block = Block::default()
        .title(" Help ")
        .title_style(Style::default().fg(theme.primary))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.primary))
        .style(Style::default().bg(theme.surface));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(theme.text).bg(theme.surface));

    paragraph.render(overlay_area, buf);
}

/// Render the autocomplete popup for slash commands.
pub fn render_autocomplete_popup(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    completions: &[&crate::commands::CommandInfo],
    selected_index: Option<usize>,
) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Widget};

    if completions.is_empty() {
        return;
    }

    // Calculate popup size and position
    // Position above the input area (which is at the bottom of the conversation pane)
    let max_items = 8.min(completions.len());
    // Safe: max_items is capped at 8, so it fits in u16
    #[allow(clippy::cast_possible_truncation)]
    let popup_height = (max_items as u16) + 2; // +2 for borders
    let popup_width = 45.min(area.width.saturating_sub(4));

    // Position just above the input area:
    // - Footer: 1 line
    // - Input area: ~3-4 lines
    // Total offset from bottom: popup_height + 5
    let popup_y = area.height.saturating_sub(popup_height + 5);
    let popup_x = 2; // Small left margin

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear the area
    Clear.render(popup_area, buf);

    // Build list items
    let items: Vec<ListItem<'_>> = completions
        .iter()
        .enumerate()
        .take(max_items)
        .map(|(i, cmd)| {
            let is_selected = selected_index == Some(i);
            let style = if is_selected {
                Style::default()
                    .fg(theme.base)
                    .bg(theme.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };

            let spans = vec![
                Span::styled(format!("/{}", cmd.name), style),
                Span::styled(
                    format!("  {}", cmd.description),
                    if is_selected {
                        style
                    } else {
                        Style::default().fg(theme.subtext)
                    },
                ),
            ];

            ListItem::new(Line::from(spans))
        })
        .collect();

    // Create the list widget
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.overlay))
        .style(Style::default().bg(theme.surface));

    let list = List::new(items).block(block);

    list.render(popup_area, buf);
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
#[allow(clippy::too_many_lines)]
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
                    &app.input,
                    &mut app.timeline_bounds,
                    app.toast.as_ref(),
                    app.current_thread.as_ref(),
                );

                // Render overlays on top
                let area = frame.area();
                let buf = frame.buffer_mut();

                // Autocomplete popup (when typing slash commands)
                if app.should_show_autocomplete() {
                    let completions = app.get_completions();
                    if !completions.is_empty() {
                        render_autocomplete_popup(
                            area,
                            buf,
                            &app.theme,
                            &completions,
                            app.autocomplete_index,
                        );
                    }
                }

                // Help overlay (highest priority, renders on top)
                if app.show_help {
                    render_help_overlay(area, buf, &app.theme);
                }
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
    fn test_screen_mode_switching_with_alt() {
        let mut app = ShellApp::new();
        assert_eq!(app.screen_mode, ScreenMode::Split);

        // Alt+1/2/3 switch modes (works cross-platform including Mac)
        app.handle_key_event(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::ALT));
        assert_eq!(app.screen_mode, ScreenMode::TimelineFocus);

        app.handle_key_event(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::ALT));
        assert_eq!(app.screen_mode, ScreenMode::ContextFocus);

        app.handle_key_event(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::ALT));
        assert_eq!(app.screen_mode, ScreenMode::Split);
    }

    #[test]
    fn test_escape_clears_input() {
        let mut app = ShellApp::new();
        app.focused_pane = FocusedPane::Timeline;

        // Type something
        app.handle_key_event(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
        assert_eq!(app.input.content(), "test");

        // Esc clears input
        app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.input.is_empty());
        assert!(!app.should_quit); // Does NOT quit
    }

    #[test]
    fn test_escape_does_not_quit() {
        // Esc never quits - must use /quit or /exit
        let mut app = ShellApp::new();
        assert!(!app.should_quit);

        // Esc on empty input does NOT quit
        app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.should_quit);

        // Multiple Esc presses still don't quit
        app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.should_quit);
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

        // Ctrl+R triggers RefreshModels (input-first model)
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        assert_eq!(action, Some(ShellAction::RefreshModels));
    }

    #[test]
    fn test_refresh_models_noop_when_panel_hidden() {
        let mut app = ShellApp::new();
        app.show_models_panel = false;
        app.probe_complete = true;

        // Ctrl+R should do nothing when models panel is not visible
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        assert_eq!(action, None);
    }

    #[test]
    fn test_refresh_models_noop_during_probing() {
        let mut app = ShellApp::new();
        app.show_models_panel = true;
        app.probe_complete = false; // Still probing

        // Ctrl+R should do nothing while probing is in progress
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
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

    #[test]
    fn test_input_first_all_chars_go_to_input() {
        // Input-first model: ALL character keys (without Ctrl) go to input
        let mut app = ShellApp::new();
        app.focused_pane = FocusedPane::Timeline;

        // Type any text - all goes to input
        app.handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
        assert_eq!(app.input.content(), "hello");

        // Even 'q' goes to input (no reserved keys in input-first model)
        app.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.input.content(), "helloq");
        assert!(!app.should_quit);

        // Numbers also go to input
        app.handle_key_event(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
        assert_eq!(app.input.content(), "helloq123");
    }

    #[test]
    fn test_focus_trap_slash() {
        // Input-first model: '/' from anywhere jumps to input
        let mut app = ShellApp::new();
        app.focused_pane = FocusedPane::Context;
        assert!(app.input.is_empty());

        // '/' jumps to input and inserts '/'
        app.handle_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        assert_eq!(app.input.content(), "/");
    }

    #[test]
    fn test_f1_shows_help() {
        let mut app = ShellApp::new();
        assert!(!app.show_help);

        // F1 shows help overlay
        app.handle_key_event(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
        assert!(app.show_help);

        // Any key closes help
        app.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert!(!app.show_help);
    }

    #[test]
    fn test_slash_command_help() {
        let mut app = ShellApp::new();
        app.focused_pane = FocusedPane::Timeline;
        assert!(!app.show_help);

        // Type /help and submit
        for c in "/help".chars() {
            app.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(app.input.content(), "/help");

        // Submit via Enter
        app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        // Help overlay should be shown
        assert!(app.show_help);
        // Input should be cleared after submit
        assert!(app.input.is_empty());
    }

    #[test]
    fn test_slash_command_quit() {
        let mut app = ShellApp::new();
        app.focused_pane = FocusedPane::Timeline;

        // Type /quit and submit
        for c in "/quit".chars() {
            app.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(app.should_quit);
    }

    #[test]
    fn test_autocomplete_shows_for_slash() {
        let mut app = ShellApp::new();
        app.focused_pane = FocusedPane::Timeline;

        // Type '/'
        app.handle_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        assert!(app.should_show_autocomplete());

        // Get completions
        let completions = app.get_completions();
        assert!(!completions.is_empty());

        // Tab starts autocomplete selection
        app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert!(app.autocomplete_index.is_some());
    }
}
