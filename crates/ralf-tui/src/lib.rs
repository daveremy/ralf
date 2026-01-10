//! ralf-tui: Terminal UI for multi-model autonomous loops
//!
//! This crate provides the TUI layer for ralf, including:
//! - M5-A Shell: Core two-pane layout with status bar and footer
//! - Theme support with Catppuccin colors
//! - Icon support with Nerd/Unicode/ASCII modes
//! - Headless mode for testing and automation
//!
//! ## M5-A Shell (New)
//!
//! The new shell implementation provides:
//! - [`layout`] - Shell layout with 4 regions
//! - [`theme`] - Colors, icons, and borders
//! - [`widgets`] - Status bar, footer hints, panes
//! - [`shell`] - Main app and run function

mod app;
pub mod commands;
pub mod context;
pub mod conversation;
mod event;
pub mod headless;
pub mod layout;
pub mod models;
mod screens;
pub mod shell;
#[cfg(test)]
pub mod test_utils;
pub mod text;
pub mod theme;
pub mod thread_state;
pub mod timeline;
mod ui;
pub mod widgets;

use screens::Screen as ScreenTrait;

pub use app::{App, Screen};
pub use event::{Action, Event, EventHandler};
pub use ralf_engine;

// Re-export M5-A shell components
pub use context::{CompletionKind, ContextView};
pub use conversation::{input_placeholder, ConversationPane};
pub use layout::{FocusedPane, ScreenMode};
pub use models::{ModelState, ModelStatus, ModelsSummary};
pub use shell::{run_shell, ShellApp, UiConfig};
pub use text::{render_markdown, MarkdownStyles};
pub use theme::{BorderSet, IconMode, IconSet, Theme};
pub use thread_state::ThreadDisplay;
pub use timeline::{
    EventKind, ReviewEvent, ReviewResult, RunEvent, SpecEvent, SystemEvent, SystemLevel,
    TimelineEvent, TimelineState, TimelineWidget,
};
pub use ui::widgets::TextInputState;

use crossterm::{
    cursor::Show as ShowCursor,
    event::{
        DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, stdout};
use std::path::Path;

/// RAII guard for terminal state restoration.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

/// Restore terminal to normal mode.
///
/// Called by `TerminalGuard::drop` and panic hook to ensure terminal is usable.
fn restore_terminal() {
    // Pop keyboard enhancement (may fail on unsupported terminals, that's ok)
    let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), DisableMouseCapture, LeaveAlternateScreen, ShowCursor);
}

/// Install panic hook that restores terminal before printing panic info.
///
/// Without this, panics leave the terminal in raw mode and the error is garbled.
fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal first so panic message is readable
        restore_terminal();
        // Then run the original hook (prints backtrace, etc.)
        original_hook(panic_info);
    }));
}

/// Run the TUI application.
///
/// This is the main entry point for the TUI. It sets up the terminal,
/// runs the event loop, and restores the terminal on exit.
pub async fn run_tui(repo_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Install panic hook first so terminal is restored on panic
    install_panic_hook();

    // Setup terminal with RAII guard for cleanup
    enable_raw_mode()?;
    let _guard = TerminalGuard;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(repo_path.to_path_buf());

    // Start probing if we're on Settings screen (first-time setup)
    if app.screen == Screen::Settings {
        app.start_probing();
    }

    // Create event handler (4 Hz tick rate = 250ms)
    let mut events = EventHandler::new(250);

    // Main loop
    let result = run_loop(&mut terminal, &mut app, &mut events).await;

    // Restore cursor before guard drops
    terminal.show_cursor()?;

    result
}

/// Run the M5-A shell TUI.
///
/// This is the new shell implementation with:
/// - Two-pane layout (Timeline | Context)
/// - Status bar and footer hints
/// - Focus management and screen modes
/// - Catppuccin theme and icon support
pub fn run_shell_tui() -> Result<(), Box<dyn std::error::Error>> {
    // Install panic hook first so terminal is restored on panic
    install_panic_hook();

    // Detect keyboard enhancement support BEFORE entering raw mode
    // (the check can conflict with event reading if done after)
    let keyboard_enhanced =
        crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false);

    // Create tokio runtime for async chat operations
    let rt = tokio::runtime::Runtime::new()?;
    let _guard_rt = rt.enter(); // Keep runtime active for tokio::spawn

    // Setup terminal with RAII guard for cleanup
    enable_raw_mode()?;
    let _guard = TerminalGuard;

    // Enable keyboard enhancement for proper Shift+Enter detection.
    // Only use DISAMBIGUATE_ESCAPE_CODES - REPORT_EVENT_TYPES causes double input
    // by reporting both press and release events.
    // Some terminals (legacy Windows) don't support this; ignore errors.
    let _ = execute!(
        stdout(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    );

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the shell with keyboard enhancement info
    shell::run_shell(&mut terminal, keyboard_enhanced)?;

    // Restore cursor before guard drops
    terminal.show_cursor()?;

    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    events: &mut EventHandler,
) -> Result<(), Box<dyn std::error::Error>> {
    // Probe task handles
    let mut probe_handles: Vec<tokio::task::JoinHandle<(String, ralf_engine::ProbeResult)>> =
        Vec::new();

    // Chat task handles
    let mut chat_handles: Vec<
        tokio::task::JoinHandle<Result<ralf_engine::ChatResult, ralf_engine::RunnerError>>,
    > = Vec::new();

    loop {
        // Draw
        terminal.draw(|frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();

            // Render current screen
            match app.screen {
                app::Screen::Settings => {
                    screens::settings::SettingsScreen.render(app, area, buf);
                }
                app::Screen::SpecStudio => {
                    screens::spec_studio::SpecStudioScreen.render(app, area, buf);
                }
                app::Screen::FinalizeConfirm => {
                    screens::spec_studio::FinalizeConfirmScreen.render(app, area, buf);
                }
                app::Screen::FinalizeError => {
                    screens::spec_studio::FinalizeErrorScreen.render(app, area, buf);
                }
                app::Screen::QuitConfirm => {
                    screens::spec_studio::QuitConfirmScreen.render(app, area, buf);
                }
                app::Screen::Status => {
                    screens::status::StatusScreen.render(app, area, buf);
                }
            }

            // Render help overlay if visible
            if app.show_help {
                screens::render_help_overlay(area, buf);
            }
        })?;

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
                // Mark as in-flight to prevent duplicate spawning
                app.mark_probe_started(&name);

                let name_clone = name.clone();
                // Use spawn_blocking since probe_model does blocking I/O
                let handle = tokio::task::spawn_blocking(move || {
                    let timeout = std::time::Duration::from_secs(10);
                    let result = ralf_engine::probe_model(&name_clone, timeout);
                    (name_clone, result)
                });
                probe_handles.push(handle);
            }
        }

        // Handle events
        if let Some(event) = events.next().await {
            match event {
                Event::Key(key) => {
                    // Special handling for SpecStudio text input
                    if app.screen == app::Screen::SpecStudio
                        && !app.chat_in_progress
                        && handle_spec_studio_key(app, key, &mut chat_handles)
                    {
                        continue; // Key was handled by text input
                    }
                    let action = event::key_to_action(key);
                    app.handle_action(action);
                }
                Event::Mouse(mouse) => {
                    use crossterm::event::MouseEventKind;
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            app.handle_action(Action::Up);
                        }
                        MouseEventKind::ScrollDown => {
                            app.handle_action(Action::Down);
                        }
                        _ => {}
                    }
                }
                Event::Tick => {
                    app.tick();
                    // Process any pending run events
                    app.process_run_events();
                }
                Event::Resize(_, _) => {
                    // Terminal will handle resize automatically
                }
            }
        }

        // Check for completed chat requests
        let mut completed_chats = Vec::new();
        for (i, handle) in chat_handles.iter().enumerate() {
            if handle.is_finished() {
                completed_chats.push(i);
            }
        }
        for i in completed_chats.into_iter().rev() {
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

        if app.should_quit {
            // Abort any remaining tasks
            for handle in probe_handles {
                handle.abort();
            }
            for handle in chat_handles {
                handle.abort();
            }
            break;
        }
    }

    Ok(())
}

/// Handle key input for `SpecStudio` text input.
/// Returns true if the key was handled (should not be processed as action).
fn handle_spec_studio_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    chat_handles: &mut Vec<
        tokio::task::JoinHandle<Result<ralf_engine::ChatResult, ralf_engine::RunnerError>>,
    >,
) -> bool {
    use crossterm::event::{KeyCode, KeyModifiers};

    // Handle Ctrl+Enter or Shift+Enter to insert newline
    if (key.modifiers.contains(KeyModifiers::CONTROL)
        || key.modifiers.contains(KeyModifiers::SHIFT))
        && key.code == KeyCode::Enter
    {
        app.input_state.insert('\n');
        return true;
    }

    // Don't handle if control key is pressed (except for certain keys)
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return false; // Let action handler deal with Ctrl+C, Ctrl+F, etc.
    }

    match key.code {
        // Enter sends the message
        KeyCode::Enter => {
            if !app.input_state.is_empty() {
                let user_input = app.input_state.submit();
                app.add_user_message(user_input);

                // Start chat request
                if let Some(model_status) = app.current_chat_model() {
                    let model_config =
                        ralf_engine::ModelConfig::default_for(&model_status.info.name);
                    let chat_context = app.thread.to_context();

                    app.chat_in_progress = true;

                    // Use tokio::spawn for async function (not spawn_blocking)
                    let handle = tokio::spawn(async move {
                        ralf_engine::invoke_chat(&model_config, &chat_context, 300).await
                    });
                    chat_handles.push(handle);
                }
            }
            true
        }

        // Text input
        KeyCode::Char(c) => {
            app.input_state.insert(c);
            true
        }
        KeyCode::Backspace => {
            app.input_state.backspace();
            true
        }
        KeyCode::Delete => {
            app.input_state.delete();
            true
        }
        KeyCode::Left => {
            app.input_state.move_left();
            true
        }
        KeyCode::Right => {
            app.input_state.move_right();
            true
        }
        KeyCode::Home => {
            app.input_state.move_home();
            true
        }
        KeyCode::End => {
            app.input_state.move_end();
            true
        }
        KeyCode::Up => {
            // History navigation when input is empty
            if app.input_state.is_empty() {
                app.input_state.history_prev();
                true
            } else {
                false // Let action handler scroll transcript
            }
        }
        KeyCode::Down => {
            if app.input_state.is_empty() {
                app.input_state.history_next();
                true
            } else {
                false
            }
        }

        _ => false,
    }
}

/// Get the TUI version.
pub fn tui_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_version() {
        let version = tui_version();
        assert!(!version.is_empty());
        assert!(version.starts_with("0."));
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use crate::app::{CriterionStatus, RunStatus};
    use crate::test_utils::*;
    use insta::assert_snapshot;

    // ========================================================================
    // Screen Snapshot Tests
    // ========================================================================

    #[test]
    fn test_snapshot_welcome_screen() {
        let app = create_test_app();
        let result = render_screen_to_string(&screens::spec_studio::SpecStudioScreen, &app);
        assert_snapshot!("welcome_screen", result);
    }

    #[test]
    fn test_snapshot_setup_screen() {
        let mut app = create_test_app();
        app.screen = app::Screen::Settings;
        let result = render_screen_to_string(&screens::settings::SettingsScreen, &app);
        assert_snapshot!("setup_screen", result);
    }

    #[test]
    fn test_snapshot_spec_studio_screen() {
        let mut app = create_test_app();
        app.screen = app::Screen::SpecStudio;
        let result = render_screen_to_string(&screens::spec_studio::SpecStudioScreen, &app);
        assert_snapshot!("spec_studio_screen", result);
    }

    #[test]
    fn test_snapshot_run_dashboard_idle() {
        let app = create_test_app_with_run_status(RunStatus::Idle);
        let result = render_screen_to_string(&screens::status::StatusScreen, &app);
        assert_snapshot!("run_dashboard_idle", result);
    }

    #[test]
    fn test_snapshot_run_dashboard_running() {
        let app = create_test_app_with_run_status(RunStatus::Running);
        let result = render_screen_to_string(&screens::status::StatusScreen, &app);
        assert_snapshot!("run_dashboard_running", result);
    }

    #[test]
    fn test_snapshot_run_dashboard_verifying() {
        let app = create_test_app_with_run_status(RunStatus::Verifying);
        let result = render_screen_to_string(&screens::status::StatusScreen, &app);
        assert_snapshot!("run_dashboard_verifying", result);
    }

    #[test]
    fn test_snapshot_run_dashboard_completed() {
        let app = create_test_app_with_run_status(RunStatus::Completed);
        let result = render_screen_to_string(&screens::status::StatusScreen, &app);
        assert_snapshot!("run_dashboard_completed", result);
    }

    #[test]
    fn test_snapshot_run_dashboard_failed() {
        let app = create_test_app_with_run_status(RunStatus::Failed);
        let result = render_screen_to_string(&screens::status::StatusScreen, &app);
        assert_snapshot!("run_dashboard_failed", result);
    }

    #[test]
    fn test_snapshot_run_dashboard_cancelled() {
        let app = create_test_app_with_run_status(RunStatus::Cancelled);
        let result = render_screen_to_string(&screens::status::StatusScreen, &app);
        assert_snapshot!("run_dashboard_cancelled", result);
    }

    // ========================================================================
    // Criteria Verification Display Tests
    // ========================================================================

    #[test]
    fn test_snapshot_criteria_all_pending() {
        let app = create_test_app_with_criteria(
            vec!["Test passes", "Code compiles", "No warnings"],
            vec![
                CriterionStatus::Pending,
                CriterionStatus::Pending,
                CriterionStatus::Pending,
            ],
        );
        let result = render_screen_to_string(&screens::status::StatusScreen, &app);
        assert_snapshot!("criteria_all_pending", result);
    }

    #[test]
    fn test_snapshot_criteria_mixed_status() {
        let app = create_test_app_with_criteria(
            vec![
                "All tests pass",
                "Code compiles without errors",
                "No new clippy warnings",
                "Documentation updated",
            ],
            vec![
                CriterionStatus::Passed,
                CriterionStatus::Passed,
                CriterionStatus::Verifying,
                CriterionStatus::Pending,
            ],
        );
        let result = render_screen_to_string(&screens::status::StatusScreen, &app);
        assert_snapshot!("criteria_mixed_status", result);
    }

    #[test]
    fn test_snapshot_criteria_with_failures() {
        let app = create_test_app_with_criteria(
            vec!["Tests pass", "Code compiles", "Linting clean"],
            vec![
                CriterionStatus::Passed,
                CriterionStatus::Failed,
                CriterionStatus::Failed,
            ],
        );
        let result = render_screen_to_string(&screens::status::StatusScreen, &app);
        assert_snapshot!("criteria_with_failures", result);
    }

    #[test]
    fn test_snapshot_criteria_all_passed() {
        let app = create_test_app_with_criteria(
            vec!["Tests pass", "Compiles", "No warnings"],
            vec![
                CriterionStatus::Passed,
                CriterionStatus::Passed,
                CriterionStatus::Passed,
            ],
        );
        let result = render_screen_to_string(&screens::status::StatusScreen, &app);
        assert_snapshot!("criteria_all_passed", result);
    }

    // ========================================================================
    // M5-A Shell Layout Snapshot Tests
    // ========================================================================

    /// Helper to render the M5-A shell layout to a string.
    fn render_shell_to_string(
        screen_mode: layout::ScreenMode,
        focused_pane: layout::FocusedPane,
        width: u16,
        height: u16,
    ) -> String {
        use ratatui::{backend::TestBackend, Terminal};

        let theme = theme::Theme::default();
        let borders = theme::BorderSet::new(theme::IconMode::Unicode);
        let models: Vec<models::ModelStatus> = vec![];
        let timeline_state = timeline::TimelineState::new();
        let input_state = ui::widgets::TextInputState::new();
        let mut timeline_bounds = shell::TimelinePaneBounds::default();

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

        terminal
            .draw(|frame| {
                layout::render_shell(
                    frame,
                    screen_mode,
                    focused_pane,
                    &theme,
                    &borders,
                    &models,
                    false, // ascii_mode
                    false, // show_models_panel
                    &timeline_state,
                    &input_state,
                    &mut timeline_bounds,
                    None,  // toast
                    None,  // thread (no thread loaded)
                    false, // chat_loading
                    None,  // loading_model
                    None,  // spec_content
                    0,     // spec_scroll
                    false, // keyboard_enhanced
                    40,    // split_ratio
                    true,  // show_canvas
                );
            })
            .expect("Failed to draw");

        buffer_to_string(terminal.backend().buffer())
    }

    #[test]
    fn test_snapshot_shell_split_mode() {
        let result = render_shell_to_string(
            layout::ScreenMode::Split,
            layout::FocusedPane::Timeline,
            TEST_WIDTH,
            TEST_HEIGHT,
        );
        assert_snapshot!("shell_split_mode", result);
    }

    #[test]
    fn test_snapshot_shell_split_mode_context_focused() {
        let result = render_shell_to_string(
            layout::ScreenMode::Split,
            layout::FocusedPane::Context,
            TEST_WIDTH,
            TEST_HEIGHT,
        );
        assert_snapshot!("shell_split_context_focused", result);
    }

    #[test]
    fn test_snapshot_shell_timeline_focus() {
        let result = render_shell_to_string(
            layout::ScreenMode::TimelineFocus,
            layout::FocusedPane::Timeline,
            TEST_WIDTH,
            TEST_HEIGHT,
        );
        assert_snapshot!("shell_timeline_focus", result);
    }

    #[test]
    fn test_snapshot_shell_context_focus() {
        let result = render_shell_to_string(
            layout::ScreenMode::ContextFocus,
            layout::FocusedPane::Context,
            TEST_WIDTH,
            TEST_HEIGHT,
        );
        assert_snapshot!("shell_context_focus", result);
    }

    #[test]
    fn test_snapshot_shell_too_small() {
        // Terminal smaller than MIN_WIDTH x MIN_HEIGHT (40x12)
        let result = render_shell_to_string(
            layout::ScreenMode::Split,
            layout::FocusedPane::Timeline,
            30, // Less than MIN_WIDTH (40)
            10, // Less than MIN_HEIGHT (12)
        );
        assert_snapshot!("shell_too_small", result);
    }
}

/// E2E and navigation tests that test event handling and screen transitions.
#[cfg(test)]
mod navigation_tests {
    use crate::app::Screen;
    use crate::event::Action;
    use crate::test_utils::create_test_app;

    // ========================================================================
    // Navigation Flow Tests
    // ========================================================================

    #[test]
    fn test_welcome_to_setup_navigation() {
        let mut app = create_test_app();
        assert_eq!(app.screen, Screen::SpecStudio);

        // Press 's' to go to Setup
        app.handle_action(Action::Setup);
        assert_eq!(app.screen, Screen::Settings);
    }

    #[test]
    fn test_welcome_to_spec_studio_navigation() {
        let mut app = create_test_app();
        assert_eq!(app.screen, Screen::SpecStudio);

        // Press 'c' to go to Spec Studio (Chat)
        app.handle_action(Action::Chat);
        assert_eq!(app.screen, Screen::SpecStudio);
    }

    #[test]
    fn test_welcome_to_run_dashboard_requires_prompt() {
        let mut app = create_test_app();
        assert_eq!(app.screen, Screen::SpecStudio);

        // Press 'r' - but Run requires PROMPT.md to exist
        // Without PROMPT.md, we stay on Welcome
        app.handle_action(Action::Run);
        // This tests the guard condition - would go to RunDashboard if PROMPT.md existed
        assert_eq!(app.screen, Screen::SpecStudio);
    }

    #[test]
    fn test_back_from_setup_to_welcome() {
        let mut app = create_test_app();
        app.screen = Screen::Settings;

        // Press Escape to go back
        app.handle_action(Action::Back);
        assert_eq!(app.screen, Screen::SpecStudio);
    }

    #[test]
    fn test_back_from_spec_studio_shows_quit_confirm() {
        let mut app = create_test_app();
        app.screen = Screen::SpecStudio;
        assert!(!app.should_quit);

        // Press Escape from home screen - should show quit confirmation
        app.handle_action(Action::Back);
        assert_eq!(app.screen, Screen::QuitConfirm);
        assert!(!app.should_quit);

        // Press Enter to confirm quit
        app.handle_action(Action::Select);
        assert!(app.should_quit);
    }

    #[test]
    fn test_quit_confirm_cancel_returns_to_spec_studio() {
        let mut app = create_test_app();
        app.screen = Screen::QuitConfirm;

        // Press Escape to cancel
        app.handle_action(Action::Back);
        assert_eq!(app.screen, Screen::SpecStudio);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_back_from_run_dashboard_to_welcome() {
        let mut app = create_test_app();
        app.screen = Screen::Status;

        // When idle, Escape goes back to Welcome
        app.handle_action(Action::Back);
        assert_eq!(app.screen, Screen::SpecStudio);
    }

    #[test]
    fn test_help_overlay_toggle() {
        let mut app = create_test_app();
        assert!(!app.show_help);

        // Press '?' to show help
        app.handle_action(Action::Help);
        assert!(app.show_help);

        // Press '?' or Escape to hide help
        app.handle_action(Action::Back);
        assert!(!app.show_help);
    }

    #[test]
    fn test_quit_action_from_spec_studio_shows_confirm() {
        let mut app = create_test_app();
        app.screen = Screen::SpecStudio;
        assert!(!app.should_quit);

        // Press 'q' to quit - should show confirmation first
        app.handle_action(Action::Quit);
        assert_eq!(app.screen, Screen::QuitConfirm);
        assert!(!app.should_quit);
    }

    // ========================================================================
    // Run Dashboard State Tests
    // ========================================================================

    #[test]
    fn test_run_dashboard_toggle_follow() {
        let mut app = create_test_app();
        app.screen = Screen::Status;

        let initial_follow = app.run_state.follow_output;

        // Press 'f' to toggle follow
        app.handle_action(Action::ToggleFollow);
        assert_ne!(app.run_state.follow_output, initial_follow);

        // Toggle back
        app.handle_action(Action::ToggleFollow);
        assert_eq!(app.run_state.follow_output, initial_follow);
    }

    #[test]
    fn test_setup_screen_model_selection() {
        let mut app = create_test_app();
        app.screen = Screen::Settings;

        // Press Down to move selection
        app.handle_action(Action::Down);

        // If there are multiple models, selection should change
        // (In test app we only have one model, so this tests bounds checking)
        assert!(app.selected_model < app.models.len());
    }

    // ========================================================================
    // Event Handling Edge Cases
    // ========================================================================

    #[test]
    fn test_action_none_does_nothing() {
        let mut app = create_test_app();
        let initial_screen = app.screen;

        app.handle_action(Action::None);
        assert_eq!(app.screen, initial_screen);
    }

    #[test]
    fn test_help_closes_before_quit() {
        let mut app = create_test_app();
        app.show_help = true;

        // When help is open, Quit should close help first
        app.handle_action(Action::Quit);
        assert!(!app.show_help);
        assert!(!app.should_quit); // Should not quit yet
    }
}

/// PTY-based E2E tests using ratatui-testlib for Playwright-like testing.
/// These tests spawn the actual ralf binary and interact with it through a PTY.
///
/// NOTE: These tests are currently experimental. Crossterm-based TUIs have
/// challenges with PTY-based testing due to how raw mode and input handling
/// work. The tests are preserved for future development when the PTY interaction
/// issues are resolved.
///
/// For now, we rely on:
/// - Snapshot tests (test rendering correctness)
/// - Navigation tests (test event handling logic)
/// - Manual E2E testing
#[cfg(test)]
mod pty_e2e_tests {
    #[allow(unused_imports)]
    use portable_pty::CommandBuilder;
    #[allow(unused_imports)]
    use ratatui_testlib::TuiTestHarness;
    #[allow(unused_imports)]
    use std::time::Duration;
    use tempfile::TempDir;

    /// Helper to get the path to the ralf binary.
    #[allow(dead_code)]
    fn ralf_binary_path() -> String {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let workspace_root = std::path::Path::new(manifest_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        workspace_root
            .join("target/release/ralf")
            .to_string_lossy()
            .to_string()
    }

    /// Create a temporary project directory with a PROMPT.md and .ralf directory
    #[allow(dead_code)]
    fn create_test_project(prompt_content: &str) -> TempDir {
        let dir = TempDir::new().expect("Failed to create temp dir");

        // Create PROMPT.md
        let prompt_path = dir.path().join("PROMPT.md");
        std::fs::write(&prompt_path, prompt_content).expect("Failed to write PROMPT.md");

        // Create .ralf directory (initialized state)
        let ralf_dir = dir.path().join(".ralf");
        std::fs::create_dir(&ralf_dir).expect("Failed to create .ralf dir");

        // Create minimal config.json
        let config = r#"{"primary_model":"claude","verifier_model":"gemini","max_iterations":10}"#;
        std::fs::write(ralf_dir.join("config.json"), config).expect("Failed to write config");

        dir
    }

    // PTY-based E2E tests are disabled due to crossterm PTY interaction issues.
    // The TUI uses raw mode which doesn't play well with PTY-based testing.
    // TODO: Investigate alternative approaches:
    // 1. Use a mock terminal backend for E2E tests
    // 2. Add a headless/test mode to the TUI
    // 3. Wait for improvements in ratatui-testlib

    #[test]
    fn test_pty_infrastructure_available() {
        // Verify the test infrastructure works
        let project_dir = create_test_project("# Test\n\nTest.\n");
        assert!(project_dir.path().join("PROMPT.md").exists());
        assert!(project_dir.path().join(".ralf/config.json").exists());
    }
}
