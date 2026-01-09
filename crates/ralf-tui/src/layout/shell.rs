//! Main shell layout with 4 regions.
//!
//! Regions:
//! 1. Status Bar (top, 1 line)
//! 2. Timeline Pane (left, 40%)
//! 3. Context Pane (right, 60%)
//! 4. Footer Hints (bottom, 1 line)

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use super::screen_modes::{FocusedPane, ScreenMode};
use crate::{
    models::ModelStatus,
    theme::{BorderSet, Theme},
    widgets::{FooterHints, KeyHint, ModelsPanel, Pane, StatusBar, StatusBarContent},
};

/// Minimum terminal width.
pub const MIN_WIDTH: u16 = 40;
/// Minimum terminal height.
pub const MIN_HEIGHT: u16 = 12;

/// Render the main shell layout.
#[allow(clippy::too_many_arguments)]
pub fn render_shell(
    frame: &mut Frame<'_>,
    screen_mode: ScreenMode,
    focused_pane: FocusedPane,
    theme: &Theme,
    borders: &BorderSet,
    models: &[ModelStatus],
    ascii_mode: bool,
    show_models_panel: bool,
) {
    let area = frame.area();

    // Check for minimum size
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        render_too_small(frame, theme, models, ascii_mode);
        return;
    }

    // Divide into: StatusBar | MainArea | FooterHints
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status bar
            Constraint::Min(0),    // Main area (expands)
            Constraint::Length(1), // Footer hints
        ])
        .split(area);

    // Status bar with placeholder content
    let status_content = StatusBarContent::placeholder();
    let status_bar = StatusBar::new(&status_content, models, theme).ascii_mode(ascii_mode);
    frame.render_widget(status_bar, chunks[0]);

    // Main pane area
    render_main_area(
        frame,
        chunks[1],
        screen_mode,
        focused_pane,
        theme,
        borders,
        models,
        ascii_mode,
        show_models_panel,
    );

    // Footer with keybinding hints (include 'r' for refresh when models panel is visible)
    let mut hints = FooterHints::default_hints();
    if show_models_panel {
        // Insert refresh hint before help hint
        hints.insert(hints.len().saturating_sub(2), KeyHint::new("r", "Refresh"));
    }
    let footer = FooterHints::new(&hints, theme);
    frame.render_widget(footer, chunks[2]);
}

/// Render the main two-pane area based on screen mode.
#[allow(clippy::too_many_arguments)]
fn render_main_area(
    frame: &mut Frame<'_>,
    area: Rect,
    screen_mode: ScreenMode,
    focused_pane: FocusedPane,
    theme: &Theme,
    borders: &BorderSet,
    models: &[ModelStatus],
    ascii_mode: bool,
    show_models_panel: bool,
) {
    match screen_mode {
        ScreenMode::Split => {
            // 40% Timeline | 60% Context
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(area);

            render_timeline_pane(
                frame,
                chunks[0],
                focused_pane == FocusedPane::Timeline,
                theme,
                borders,
            );
            render_context_pane(
                frame,
                chunks[1],
                focused_pane == FocusedPane::Context,
                theme,
                borders,
                models,
                ascii_mode,
                show_models_panel,
            );
        }
        ScreenMode::TimelineFocus => {
            // Focus mode: only timeline visible, always focused
            render_timeline_pane(frame, area, true, theme, borders);
        }
        ScreenMode::ContextFocus => {
            // Focus mode: only context visible, always focused
            render_context_pane(frame, area, true, theme, borders, models, ascii_mode, show_models_panel);
        }
    }
}

/// Render the timeline pane.
fn render_timeline_pane(
    frame: &mut Frame<'_>,
    area: Rect,
    focused: bool,
    theme: &Theme,
    borders: &BorderSet,
) {
    let pane = Pane::new(theme, borders)
        .title(" Timeline ")
        .focused(focused)
        .content("Timeline events will appear here...");

    frame.render_widget(pane, area);
}

/// Render the context pane.
#[allow(clippy::too_many_arguments)]
fn render_context_pane(
    frame: &mut Frame<'_>,
    area: Rect,
    focused: bool,
    theme: &Theme,
    borders: &BorderSet,
    models: &[ModelStatus],
    ascii_mode: bool,
    show_models_panel: bool,
) {
    if show_models_panel {
        // Render Models panel instead of generic Context pane
        let models_panel = ModelsPanel::new(models, theme).ascii_mode(ascii_mode);
        frame.render_widget(models_panel, area);
    } else {
        let pane = Pane::new(theme, borders)
            .title(" Context ")
            .focused(focused)
            .content("Context view will appear here...");

        frame.render_widget(pane, area);
    }
}

/// Render "terminal too small" warning.
fn render_too_small(
    frame: &mut Frame<'_>,
    theme: &Theme,
    models: &[ModelStatus],
    ascii_mode: bool,
) {
    let area = frame.area();

    // Just show the status bar with warning
    let status_content = StatusBarContent::too_small();
    let status_bar = StatusBar::new(&status_content, models, theme).ascii_mode(ascii_mode);
    frame.render_widget(status_bar, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_size_constants() {
        assert_eq!(MIN_WIDTH, 40);
        assert_eq!(MIN_HEIGHT, 12);
    }
}
