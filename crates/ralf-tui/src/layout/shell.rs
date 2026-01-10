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
    context::{ContextView, SpecPhase, SpecPreview},
    conversation::ConversationPane,
    models::ModelStatus,
    shell::{TimelinePaneBounds, Toast},
    theme::{BorderSet, Theme},
    thread_state::ThreadDisplay,
    timeline::TimelineState,
    ui::widgets::TextInputState,
    widgets::{FooterHints, InputBar, ModelsPanel, Pane, StatusBar, StatusBarContent},
};

/// Minimum terminal width.
pub const MIN_WIDTH: u16 = 40;
/// Minimum terminal height.
pub const MIN_HEIGHT: u16 = 12;

/// Render the main shell layout.
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
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
    input: &TextInputState,
    timeline_bounds: &mut TimelinePaneBounds,
    toast: Option<&Toast>,
    thread: Option<&ThreadDisplay>,
    chat_loading: bool,
    loading_model: Option<&str>,
    spec_content: Option<&str>,
    spec_scroll: u16,
    keyboard_enhanced: bool,
    split_ratio: u16,
    show_canvas: bool,
) {
    let area = frame.area();

    // Check for minimum size
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        render_too_small(frame, theme, models, ascii_mode);
        return;
    }

    // Calculate dynamic input bar height based on content
    // Minimum 3 (1 line + 2 for border), maximum 10 (8 lines + 2 for border)
    let input_lines = input.line_count();
    #[allow(clippy::cast_possible_truncation)]
    let input_height = (input_lines as u16 + 2).clamp(3, 10); // Safe: clamped to 3-10

    // Divide into: StatusBar | MainArea | InputBar | FooterHints
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),            // Status bar
            Constraint::Min(0),               // Main area (expands)
            Constraint::Length(input_height), // Input bar (dynamic height)
            Constraint::Length(1),            // Footer hints
        ])
        .split(area);

    // Status bar with thread-driven content
    let status_content = StatusBarContent::from_thread(thread);
    let status_bar = StatusBar::new(&status_content, models, theme).ascii_mode(ascii_mode);
    frame.render_widget(status_bar, chunks[0]);

    // Extract phase once for reuse
    let phase = thread.map(|t| t.phase_kind);

    // Main pane area (timeline and/or canvas)
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
        spec_content,
        spec_scroll,
        split_ratio,
        show_canvas,
    );

    // Full-width input bar (always visible)
    let input_bar = InputBar::new(input, theme)
        .focused(focused_pane == FocusedPane::Input)
        .loading(chat_loading, loading_model);
    frame.render_widget(input_bar, chunks[2]);

    // Footer with status bar format: Mode │ Focus │ Phase    [pane-specific hints]
    let hints = FooterHints::pane_hints(focused_pane, show_models_panel, keyboard_enhanced);
    let footer = FooterHints::new(&hints, theme)
        .screen_mode(screen_mode)
        .focused_pane(focused_pane)
        .phase(phase);
    frame.render_widget(footer, chunks[3]);

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

    // Red background for errors, green for success
    let is_error = toast.message.contains("failed") || toast.message.contains("unavailable");
    let bg_color = if is_error { Color::Red } else { Color::Green };

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
    spec_content: Option<&str>,
    spec_scroll: u16,
    split_ratio: u16,
    show_canvas: bool,
) {
    // Determine if canvas is showing spec (used to auto-collapse spec events in timeline)
    let canvas_shows_spec = show_canvas && spec_content.is_some();

    // If canvas is collapsed/hidden, show timeline full width regardless of screen mode
    if !show_canvas && screen_mode == ScreenMode::Split {
        render_timeline_pane(
            frame,
            area,
            focused_pane == FocusedPane::Timeline,
            theme,
            timeline,
            timeline_bounds,
            false, // Canvas not visible
        );
        return;
    }

    match screen_mode {
        ScreenMode::Split => {
            // Use configurable split ratio
            let timeline_pct = split_ratio;
            let canvas_pct = 100 - split_ratio;
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(timeline_pct),
                    Constraint::Percentage(canvas_pct),
                ])
                .split(area);

            render_timeline_pane(
                frame,
                chunks[0],
                focused_pane == FocusedPane::Timeline,
                theme,
                timeline,
                timeline_bounds,
                canvas_shows_spec,
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
                spec_content,
                spec_scroll,
            );
        }
        ScreenMode::TimelineFocus => {
            // Focus mode: only timeline visible
            render_timeline_pane(
                frame,
                area,
                focused_pane == FocusedPane::Timeline,
                theme,
                timeline,
                timeline_bounds,
                false, // Canvas not visible in focus mode
            );
        }
        ScreenMode::ContextFocus => {
            // Focus mode: only context/canvas visible
            render_context_pane(
                frame,
                area,
                focused_pane == FocusedPane::Context,
                theme,
                borders,
                models,
                ascii_mode,
                show_models_panel,
                phase,
                spec_content,
                spec_scroll,
            );
        }
    }
}

/// Render the timeline pane (events only, input is rendered separately).
fn render_timeline_pane(
    frame: &mut Frame<'_>,
    area: Rect,
    focused: bool,
    theme: &Theme,
    timeline: &TimelineState,
    timeline_bounds: &mut TimelinePaneBounds,
    canvas_shows_spec: bool,
) {
    // Calculate inner area (accounting for 1-pixel border on all sides)
    // This is used for mouse coordinate translation
    timeline_bounds.inner_x = area.x.saturating_add(1);
    timeline_bounds.inner_y = area.y.saturating_add(1);
    timeline_bounds.inner_width = area.width.saturating_sub(2);
    timeline_bounds.inner_height = area.height.saturating_sub(2);

    let widget = ConversationPane::from_timeline(timeline, theme)
        .focused(focused)
        .canvas_shows_spec(canvas_shows_spec);
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
    spec_content: Option<&str>,
    spec_scroll: u16,
) {
    use ralf_engine::thread::PhaseKind;

    // Route to appropriate view based on phase
    let view = ContextView::from_phase(phase);

    // NoThread view shows ModelsPanel when models panel is enabled
    if matches!(view, ContextView::NoThread) && show_models_panel {
        let models_panel = ModelsPanel::new(models, theme)
            .ascii_mode(ascii_mode)
            .focused(focused);
        frame.render_widget(models_panel, area);
    } else if matches!(view, ContextView::SpecEditor) {
        // Render SpecPreview for spec editing phases (Drafting is the default)
        let spec_phase = match phase {
            Some(PhaseKind::Assessing) => SpecPhase::Assessing,
            Some(PhaseKind::Finalized) => SpecPhase::Ready,
            _ => SpecPhase::Drafting,
        };

        // Render spec preview inside a bordered pane
        render_spec_pane(frame, area, focused, theme, borders, spec_content.unwrap_or(""), spec_phase, spec_scroll);
    } else {
        // Render placeholder for all other views (real implementations in M5-B.4)
        render_context_placeholder(frame, view, area, focused, theme, borders);
    }
}

/// Render spec preview inside a bordered pane.
#[allow(clippy::too_many_arguments)]
fn render_spec_pane(
    frame: &mut Frame<'_>,
    area: Rect,
    focused: bool,
    theme: &Theme,
    borders: &BorderSet,
    content: &str,
    phase: SpecPhase,
    scroll: u16,
) {
    let (border_set, border_color) = if focused {
        (borders.focused(), theme.border_focused)
    } else {
        (borders.normal(), theme.border)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(border_set)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(" Spec ", Style::default().fg(theme.text)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let preview = SpecPreview::new(content, phase, theme)
        .focused(focused)
        .scroll(scroll);
    frame.render_widget(preview, inner);
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
