//! ralf-tui: Terminal UI for multi-model autonomous loops
//!
//! This crate provides the TUI layer for ralf, including:
//! - Welcome screen with model detection
//! - Setup screen for configuration
//! - Shared widgets (tabs, log viewers)

mod app;
mod event;
mod screens;
mod ui;

use screens::Screen as ScreenTrait;

pub use app::{App, Screen};
pub use event::{Action, Event, EventHandler};
pub use ralf_engine;

use crossterm::{
    cursor::Show as ShowCursor,
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
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, ShowCursor);
    }
}

/// Run the TUI application.
///
/// This is the main entry point for the TUI. It sets up the terminal,
/// runs the event loop, and restores the terminal on exit.
pub async fn run_tui(repo_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal with RAII guard for cleanup
    enable_raw_mode()?;
    let _guard = TerminalGuard;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(repo_path.to_path_buf());

    // Start probing if we're on Setup screen (first-time user)
    if app.screen == Screen::Setup {
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
                app::Screen::Welcome => {
                    screens::welcome::WelcomeScreen.render(app, area, buf);
                }
                app::Screen::Setup => {
                    screens::setup::SetupScreen.render(app, area, buf);
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

        // Start new probes if needed (only on Setup screen)
        if app.screen == app::Screen::Setup {
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
                    if app.screen == app::Screen::SpecStudio && !app.chat_in_progress {
                        if handle_spec_studio_key(app, key, &mut chat_handles).await {
                            continue; // Key was handled by text input
                        }
                    }
                    let action = event::key_to_action(key);
                    app.handle_action(action);
                }
                Event::Tick => {
                    app.tick();
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

/// Handle key input for SpecStudio text input.
/// Returns true if the key was handled (should not be processed as action).
async fn handle_spec_studio_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    chat_handles: &mut Vec<
        tokio::task::JoinHandle<Result<ralf_engine::ChatResult, ralf_engine::RunnerError>>,
    >,
) -> bool {
    use crossterm::event::{KeyCode, KeyModifiers};

    // Don't handle if control key is pressed (except for certain keys)
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return false; // Let action handler deal with Ctrl+C, Ctrl+F, etc.
    }

    match key.code {
        // Special keys that should be handled as actions
        KeyCode::Esc | KeyCode::Tab => false,

        // Enter sends the message
        KeyCode::Enter => {
            if !app.input_state.is_empty() {
                let content = app.input_state.submit();
                app.add_user_message(content);

                // Start chat request
                if let Some(model_status) = app.current_chat_model() {
                    let model_config = ralf_engine::ModelConfig::default_for(&model_status.info.name);
                    let context = app.thread.to_context();

                    app.chat_in_progress = true;

                    // Use tokio::spawn for async function (not spawn_blocking)
                    let handle = tokio::spawn(async move {
                        ralf_engine::invoke_chat(&model_config, &context, 300).await
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
