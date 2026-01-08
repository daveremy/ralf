//! Headless mode for the ralf TUI.
//!
//! This module provides a way to run the TUI without a real terminal,
//! enabling E2E testing and automation. Actions are sent via channels
//! and screen state is captured after each render.

use crate::app::{App, Screen};
use crate::event::Action;
use crate::screens::Screen as ScreenTrait;
use crate::{app, screens};
use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};
use std::path::Path;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

/// Default terminal dimensions for headless mode.
pub const DEFAULT_WIDTH: u16 = 80;
pub const DEFAULT_HEIGHT: u16 = 24;

/// State captured from the headless TUI after each render.
#[derive(Debug, Clone)]
pub struct HeadlessState {
    /// Current screen being displayed.
    pub screen: Screen,
    /// Text contents of the terminal buffer.
    pub screen_contents: String,
    /// Whether the TUI should quit.
    pub should_quit: bool,
    /// Whether help overlay is visible.
    pub show_help: bool,
}

impl Default for HeadlessState {
    fn default() -> Self {
        Self {
            screen: Screen::SpecStudio,
            screen_contents: String::new(),
            should_quit: false,
            show_help: false,
        }
    }
}

/// Handle to control a headless TUI instance.
///
/// Use this to send actions and observe state changes.
pub struct HeadlessHandle {
    action_tx: mpsc::UnboundedSender<Action>,
    state_rx: watch::Receiver<HeadlessState>,
}

impl HeadlessHandle {
    /// Send an action to the TUI.
    ///
    /// Returns `true` if the action was sent successfully.
    pub fn send_action(&self, action: Action) -> bool {
        self.action_tx.send(action).is_ok()
    }

    /// Get the current state of the TUI.
    pub fn state(&self) -> HeadlessState {
        self.state_rx.borrow().clone()
    }

    /// Wait for the state to change, with a timeout.
    ///
    /// Returns `true` if state changed, `false` if timed out.
    pub async fn wait_for_change(&mut self, timeout: std::time::Duration) -> bool {
        tokio::time::timeout(timeout, self.state_rx.changed())
            .await
            .is_ok()
    }

    /// Wait until a condition is met on the state.
    ///
    /// Returns the state when the condition is met, or `None` if timed out.
    pub async fn wait_for<F>(
        &mut self,
        condition: F,
        timeout: std::time::Duration,
    ) -> Option<HeadlessState>
    where
        F: Fn(&HeadlessState) -> bool,
    {
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            let state = self.state();
            if condition(&state) {
                return Some(state);
            }

            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return None;
            }

            if tokio::time::timeout(remaining, self.state_rx.changed())
                .await
                .is_err()
            {
                return None;
            }
        }
    }

    /// Wait for specific text to appear on screen.
    pub async fn wait_for_text(
        &mut self,
        text: &str,
        timeout: std::time::Duration,
    ) -> Option<HeadlessState> {
        let text = text.to_string();
        self.wait_for(|s| s.screen_contents.contains(&text), timeout)
            .await
    }

    /// Wait for a specific screen to be displayed.
    pub async fn wait_for_screen(
        &mut self,
        screen: Screen,
        timeout: std::time::Duration,
    ) -> Option<HeadlessState> {
        self.wait_for(|s| s.screen == screen, timeout).await
    }

    /// Check if the TUI has quit.
    pub fn has_quit(&self) -> bool {
        self.state().should_quit
    }
}

/// Configuration for headless mode.
#[derive(Debug, Clone)]
pub struct HeadlessConfig {
    /// Terminal width.
    pub width: u16,
    /// Terminal height.
    pub height: u16,
    /// Tick rate in milliseconds.
    pub tick_rate_ms: u64,
}

impl Default for HeadlessConfig {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            tick_rate_ms: 50, // Faster tick rate for testing
        }
    }
}

/// Run the TUI in headless mode.
///
/// Returns a handle to control the TUI and a join handle for the background task.
///
/// # Example
///
/// ```ignore
/// let (handle, task) = run_tui_headless(repo_path, HeadlessConfig::default()).await;
///
/// // Send actions
/// handle.send_action(Action::Tab(1));
///
/// // Wait for state changes
/// let state = handle.wait_for_text("Setup", Duration::from_secs(1)).await;
///
/// // Quit
/// handle.send_action(Action::Quit);
/// task.await.unwrap();
/// ```
pub fn run_tui_headless(
    repo_path: &Path,
    config: HeadlessConfig,
) -> (HeadlessHandle, JoinHandle<Result<(), String>>) {
    let (action_tx, action_rx) = mpsc::unbounded_channel();
    let (state_tx, state_rx) = watch::channel(HeadlessState::default());

    let repo_path = repo_path.to_path_buf();

    let task = tokio::spawn(async move {
        run_headless_loop(repo_path, config, action_rx, state_tx)
            .await
            .map_err(|e| e.to_string())
    });

    let handle = HeadlessHandle {
        action_tx,
        state_rx,
    };

    (handle, task)
}

#[allow(clippy::too_many_lines)]
async fn run_headless_loop(
    repo_path: std::path::PathBuf,
    config: HeadlessConfig,
    mut action_rx: mpsc::UnboundedReceiver<Action>,
    state_tx: watch::Sender<HeadlessState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create test backend
    let backend = TestBackend::new(config.width, config.height);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(repo_path);

    // Start probing if we're on Setup screen
    if app.screen == Screen::Settings {
        app.start_probing();
    }

    // Probe task handles
    let mut probe_handles: Vec<tokio::task::JoinHandle<(String, ralf_engine::ProbeResult)>> =
        Vec::new();

    // Chat task handles
    let mut chat_handles: Vec<
        tokio::task::JoinHandle<Result<ralf_engine::ChatResult, ralf_engine::RunnerError>>,
    > = Vec::new();

    let tick_duration = std::time::Duration::from_millis(config.tick_rate_ms);

    loop {
        // Draw
        terminal.draw(|frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();

            // Render current screen
            match app.screen {
                app::Screen::Settings => {
                    screens::settings::SettingsScreen.render(&app, area, buf);
                }
                app::Screen::SpecStudio => {
                    screens::spec_studio::SpecStudioScreen.render(&app, area, buf);
                }
                app::Screen::FinalizeConfirm => {
                    screens::spec_studio::FinalizeConfirmScreen.render(&app, area, buf);
                }
                app::Screen::FinalizeError => {
                    screens::spec_studio::FinalizeErrorScreen.render(&app, area, buf);
                }
                app::Screen::QuitConfirm => {
                    screens::spec_studio::QuitConfirmScreen.render(&app, area, buf);
                }
                app::Screen::Status => {
                    screens::status::StatusScreen.render(&app, area, buf);
                }
            }

            // Render help overlay if visible
            if app.show_help {
                screens::render_help_overlay(area, buf);
            }
        })?;

        // Capture screen contents
        let screen_contents = buffer_to_string(terminal.backend().buffer());

        // Update state
        let _ = state_tx.send(HeadlessState {
            screen: app.screen,
            screen_contents,
            should_quit: app.should_quit,
            show_help: app.show_help,
        });

        // Check for quit
        if app.should_quit {
            break;
        }

        // Check for completed probes (non-blocking)
        let mut completed = Vec::new();
        for (i, handle) in probe_handles.iter().enumerate() {
            if handle.is_finished() {
                completed.push(i);
            }
        }
        for i in completed.into_iter().rev() {
            if let Ok((name, result)) = probe_handles.remove(i).await {
                app.update_probe_result(&name, result);
            }
        }

        // Start new probes if needed (only on Settings screen)
        if app.screen == app::Screen::Settings {
            let models_to_probe = app.models_to_probe();
            for name in models_to_probe {
                app.mark_probe_started(&name);

                let name_clone = name.clone();
                let handle = tokio::task::spawn_blocking(move || {
                    let timeout = std::time::Duration::from_secs(10);
                    let result = ralf_engine::probe_model(&name_clone, timeout);
                    (name_clone, result)
                });
                probe_handles.push(handle);
            }
        }

        // Check for completed chats (non-blocking)
        let mut completed = Vec::new();
        for (i, handle) in chat_handles.iter().enumerate() {
            if handle.is_finished() {
                completed.push(i);
            }
        }
        for i in completed.into_iter().rev() {
            if let Ok(result) = chat_handles.remove(i).await {
                match result {
                    Ok(chat_result) => {
                        app.add_assistant_message(chat_result.content, chat_result.model);
                    }
                    Err(e) => {
                        app.add_assistant_message(format!("Error: {e}"), "error".to_string());
                    }
                }
                app.chat_in_progress = false;
            }
        }

        // Wait for action or tick
        let action = tokio::select! {
            Some(action) = action_rx.recv() => action,
            () = tokio::time::sleep(tick_duration) => Action::None,
        };

        // Handle action
        if action != Action::None {
            app.handle_action(action);
        }
    }

    Ok(())
}

/// Convert a terminal buffer to a string representation.
fn buffer_to_string(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut result = String::new();

    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buffer.cell((x, y)) {
                result.push_str(cell.symbol());
            }
        }
        // Trim trailing whitespace from each line
        while result.ends_with(' ') {
            result.pop();
        }
        result.push('\n');
    }

    // Remove trailing newline
    if result.ends_with('\n') {
        result.pop();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headless_state_default() {
        let state = HeadlessState::default();
        assert_eq!(state.screen, Screen::SpecStudio);
        assert!(!state.should_quit);
        assert!(!state.show_help);
        assert!(state.screen_contents.is_empty());
    }

    #[test]
    fn test_headless_config_default() {
        let config = HeadlessConfig::default();
        assert_eq!(config.width, DEFAULT_WIDTH);
        assert_eq!(config.height, DEFAULT_HEIGHT);
        assert_eq!(config.tick_rate_ms, 50);
    }

    #[test]
    fn test_buffer_to_string() {
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;
        use ratatui::style::Style;

        let area = Rect::new(0, 0, 10, 2);
        let mut buffer = Buffer::empty(area);
        buffer.set_string(0, 0, "Hello", Style::default());
        buffer.set_string(0, 1, "World", Style::default());

        let result = buffer_to_string(&buffer);
        assert!(result.contains("Hello"));
        assert!(result.contains("World"));
    }
}
