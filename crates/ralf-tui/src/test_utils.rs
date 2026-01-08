//! Test utilities for ralf-tui snapshot and integration testing.
//!
//! This module provides helper functions for creating test terminals,
//! rendering screens, and converting buffers to strings for snapshot testing.

use crate::app::{App, CriterionStatus, RunStatus, Screen};
use crate::screens::Screen as ScreenTrait;
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};

/// Default terminal width for tests.
pub const TEST_WIDTH: u16 = 80;

/// Default terminal height for tests.
pub const TEST_HEIGHT: u16 = 24;

/// Create a test terminal with the default dimensions (80x24).
pub fn create_test_terminal() -> Terminal<TestBackend> {
    create_test_terminal_sized(TEST_WIDTH, TEST_HEIGHT)
}

/// Create a test terminal with custom dimensions.
pub fn create_test_terminal_sized(width: u16, height: u16) -> Terminal<TestBackend> {
    let backend = TestBackend::new(width, height);
    Terminal::new(backend).expect("Failed to create test terminal")
}

/// Create a test app with default mock data.
pub fn create_test_app() -> App {
    App::new_for_test()
}

/// Create a test app positioned at a specific screen.
pub fn create_test_app_at_screen(screen: Screen) -> App {
    let mut app = App::new_for_test();
    app.screen = screen;
    app
}

/// Create a test app with run state configured for a specific status.
pub fn create_test_app_with_run_status(status: RunStatus) -> App {
    let mut app = App::new_for_test();
    app.screen = Screen::Status;
    app.run_state.status = status;

    // Add some sample data based on status
    match status {
        RunStatus::Running | RunStatus::Verifying => {
            app.run_state.run_id = Some("test-run-123".to_string());
            app.run_state.current_iteration = 3;
            app.run_state.max_iterations = 10;
            app.run_state.current_model = Some("claude".to_string());
            app.run_state.started_at = Some(std::time::Instant::now());
            app.run_state.model_output = "Working on the task...\n>>> Processing files".to_string();
        }
        RunStatus::Completed => {
            app.run_state.run_id = Some("test-run-123".to_string());
            app.run_state.current_iteration = 5;
            app.run_state.max_iterations = 10;
            app.run_state.completion_reason = Some("All criteria verified".to_string());
        }
        RunStatus::Failed => {
            app.run_state.run_id = Some("test-run-123".to_string());
            app.run_state.current_iteration = 2;
            app.run_state.error_message = Some("Model invocation failed".to_string());
        }
        RunStatus::Cancelled => {
            app.run_state.run_id = Some("test-run-123".to_string());
            app.run_state.current_iteration = 1;
        }
        RunStatus::Idle => {}
    }

    app
}

/// Create a test app with criteria configured for verification display.
pub fn create_test_app_with_criteria(criteria: Vec<&str>, statuses: Vec<CriterionStatus>) -> App {
    let mut app = App::new_for_test();
    app.screen = Screen::Status;
    app.run_state.status = RunStatus::Verifying;
    app.run_state.run_id = Some("test-run-123".to_string());
    app.run_state.current_iteration = 1;
    app.run_state.current_model = Some("claude".to_string());
    app.run_state.criteria = criteria.into_iter().map(String::from).collect();
    app.run_state.criteria_status = statuses;
    app.run_state.verifier_model = Some("gemini".to_string());
    app
}

/// Convert a buffer to a string representation for snapshot testing.
///
/// This produces a simple text representation of the buffer content,
/// suitable for snapshot comparison.
pub fn buffer_to_string(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut result = String::new();

    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = buffer.cell((x, y)).unwrap();
            result.push_str(cell.symbol());
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

/// Render a screen to a buffer and return it as a string.
pub fn render_screen_to_string<S: ScreenTrait>(screen: &S, app: &App) -> String {
    let area = Rect::new(0, 0, TEST_WIDTH, TEST_HEIGHT);
    let mut buffer = Buffer::empty(area);
    screen.render(app, area, &mut buffer);
    buffer_to_string(&buffer)
}

/// Render a screen to a buffer and return it as a string with custom dimensions.
pub fn render_screen_to_string_sized<S: ScreenTrait>(
    screen: &S,
    app: &App,
    width: u16,
    height: u16,
) -> String {
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    screen.render(app, area, &mut buffer);
    buffer_to_string(&buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_terminal() {
        let terminal = create_test_terminal();
        let size = terminal.size().unwrap();
        assert_eq!(size.width, TEST_WIDTH);
        assert_eq!(size.height, TEST_HEIGHT);
    }

    #[test]
    fn test_create_test_app() {
        let app = create_test_app();
        assert_eq!(app.screen, Screen::SpecStudio);
        assert!(!app.models.is_empty());
    }

    #[test]
    fn test_create_test_app_at_screen() {
        let app = create_test_app_at_screen(Screen::Status);
        assert_eq!(app.screen, Screen::Status);
    }

    #[test]
    fn test_buffer_to_string() {
        let area = Rect::new(0, 0, 10, 3);
        let mut buffer = Buffer::empty(area);
        buffer.set_string(0, 0, "Hello", ratatui::style::Style::default());
        buffer.set_string(0, 1, "World", ratatui::style::Style::default());

        let result = buffer_to_string(&buffer);
        assert!(result.contains("Hello"));
        assert!(result.contains("World"));
    }
}
