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

use ratatui::{
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::screen_modes::{FocusedPane, ScreenMode};
use crate::{
    context::ContextView,
    models::ModelStatus,
    shell::{TimelinePaneBounds, Toast},
    theme::{BorderSet, Theme},
    thread_state::ThreadDisplay,
    timeline::{TimelineState, TimelineWidget},
    widgets::{hints_for_state, FooterHints, ModelsPanel, Pane, StatusBar, StatusBarContent},
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
    timeline: &TimelineState,
    timeline_bounds: &mut TimelinePaneBounds,
    toast: Option<&Toast>,
    thread: Option<&ThreadDisplay>,
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

    // Status bar with thread-driven content
    let status_content = StatusBarContent::from_thread(thread);
    let status_bar = StatusBar::new(&status_content, models, theme).ascii_mode(ascii_mode);
    frame.render_widget(status_bar, chunks[0]);

    // Main pane area
    let phase = thread.map(|t| t.phase_kind);
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
        timeline,
        timeline_bounds,
        phase,
    );

    // Footer with phase-aware hints
    let phase = thread.map(|t| t.phase_kind);
    let hints = hints_for_state(phase, screen_mode, focused_pane, show_models_panel);
    let footer = FooterHints::new(&hints, theme);
    frame.render_widget(footer, chunks[2]);

    // Render toast notification if present
    if let Some(toast) = toast {
        render_toast(frame, area, toast);
    }
}

/// Render a toast notification centered at the bottom of the screen.
fn render_toast(frame: &mut Frame<'_>, area: Rect, toast: &Toast) {
    // Calculate toast dimensions (cap at terminal width)
    #[allow(clippy::cast_possible_truncation)]
    let text_len = toast.message.len().min(200) as u16; // cap at 200 chars
    let toast_width = (text_len + 4).min(area.width.saturating_sub(4)); // padding, constrain to area
    let toast_height = 3; // border + text + border

    // Position: centered horizontally, above footer
    let x = area.x + (area.width.saturating_sub(toast_width)) / 2;
    let y = area.y + area.height.saturating_sub(toast_height + 2); // +2 for footer

    let toast_area = Rect::new(x, y, toast_width, toast_height);

    // Clear the area behind the toast
    frame.render_widget(Clear, toast_area);

    // Render toast with green background for success
    let is_error = toast.message.contains("failed") || toast.message.contains("unavailable");
    let bg_color = if is_error {
        Color::Red
    } else {
        Color::Green
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(bg_color).fg(Color::White));

    let paragraph = Paragraph::new(Span::raw(&toast.message))
        .block(block)
        .style(Style::default().bg(bg_color).fg(Color::White));

    frame.render_widget(paragraph, toast_area);
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
    timeline: &TimelineState,
    timeline_bounds: &mut TimelinePaneBounds,
    phase: Option<ralf_engine::thread::PhaseKind>,
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
                timeline,
                timeline_bounds,
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
                phase,
            );
        }
        ScreenMode::TimelineFocus => {
            // Focus mode: only timeline visible, always focused
            render_timeline_pane(frame, area, true, theme, timeline, timeline_bounds);
        }
        ScreenMode::ContextFocus => {
            // Focus mode: only context visible, always focused
            render_context_pane(
                frame,
                area,
                true,
                theme,
                borders,
                models,
                ascii_mode,
                show_models_panel,
                phase,
            );
        }
    }
}

/// Render the timeline pane.
fn render_timeline_pane(
    frame: &mut Frame<'_>,
    area: Rect,
    focused: bool,
    theme: &Theme,
    timeline: &TimelineState,
    timeline_bounds: &mut TimelinePaneBounds,
) {
    // Calculate inner area (accounting for 1-pixel border on all sides)
    // This is used for mouse coordinate translation
    timeline_bounds.inner_x = area.x.saturating_add(1);
    timeline_bounds.inner_y = area.y.saturating_add(1);
    timeline_bounds.inner_width = area.width.saturating_sub(2);
    timeline_bounds.inner_height = area.height.saturating_sub(2);

    let widget = TimelineWidget::new(timeline, theme).focused(focused);
    frame.render_widget(widget, area);
}

/// Render the context pane (right side - shows phase-routed content).
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
    phase: Option<ralf_engine::thread::PhaseKind>,
) {
    // Route to appropriate view based on phase
    let view = ContextView::from_phase(phase);

    // NoThread view shows ModelsPanel when models panel is enabled
    if matches!(view, ContextView::NoThread) && show_models_panel {
        let models_panel = ModelsPanel::new(models, theme)
            .ascii_mode(ascii_mode)
            .focused(focused);
        frame.render_widget(models_panel, area);
    } else {
        // Render placeholder for all other views (real implementations in M5-B.3/B.4)
        render_context_placeholder(frame, view, area, focused, theme, borders);
    }
}

/// Render placeholder content for context views.
fn render_context_placeholder(
    frame: &mut Frame<'_>,
    view: ContextView,
    area: Rect,
    focused: bool,
    theme: &Theme,
    borders: &BorderSet,
) {
    let pane = Pane::new(theme, borders)
        .title(view.title())
        .focused(focused)
        .content(view.placeholder_text());

    frame.render_widget(pane, area);
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
