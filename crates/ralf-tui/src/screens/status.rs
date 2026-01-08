//! Status screen - displays and controls autonomous loop execution.
//!
//! Shows all panes simultaneously for real-time visibility into the run.

use crate::app::{App, CriterionStatus, RunStatus};
use crate::screens::Screen;
use crate::ui::main_layout;
use crate::ui::theme::Styles;
use crate::ui::widgets::{KeyHint, StatusBar};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

/// The Status screen (run dashboard).
pub struct StatusScreen;

impl Screen for StatusScreen {
    fn render(&self, app: &App, area: Rect, buf: &mut Buffer) {
        let (main_area, status_area) = main_layout(area);

        // Layout: Header | Middle (Output + Criteria) | Bottom (Events + Git)
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),  // Compact header
                Constraint::Min(10),    // Middle row (Output + Criteria sidebar)
                Constraint::Length(8),  // Bottom row (Events + Git)
            ])
            .split(main_area);

        // Render header with inline cooldowns
        render_header(app, main_chunks[0], buf);

        // Middle row: Output (left) | Criteria (right sidebar)
        let middle_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70), // Output
                Constraint::Percentage(30), // Criteria sidebar
            ])
            .split(main_chunks[1]);

        render_output_pane(app, middle_chunks[0], buf);
        render_criteria_pane(app, middle_chunks[1], buf);

        // Bottom row: Events | Git
        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60), // Events
                Constraint::Percentage(40), // Git
            ])
            .split(main_chunks[2]);

        render_events_pane(app, bottom_chunks[0], buf);
        render_git_pane(app, bottom_chunks[1], buf);

        // Render status bar
        let hints = if matches!(app.run_state.status, RunStatus::Running | RunStatus::Verifying) {
            vec![
                KeyHint::new("Esc/Ctrl+C", "Cancel"),
                KeyHint::new("f", "Toggle Follow"),
                KeyHint::new("?", "Help"),
            ]
        } else {
            vec![
                KeyHint::new("Enter", "Start"),
                KeyHint::new("Esc", "Back"),
                KeyHint::new("f", "Toggle Follow"),
                KeyHint::new("?", "Help"),
            ]
        };

        let status_text = match app.run_state.status {
            RunStatus::Running => "Running",
            RunStatus::Verifying => "Verifying",
            RunStatus::Completed => "Completed",
            RunStatus::Cancelled => "Cancelled",
            RunStatus::Failed => "Failed",
            RunStatus::Idle => "Ready",
        };

        let mut status_bar = StatusBar::new("Status").hints(hints);
        if let Some(notification) = &app.notification {
            status_bar = status_bar.right(notification);
        } else {
            status_bar = status_bar.right(status_text);
        }
        status_bar.render(status_area, buf);
    }
}

fn render_header(app: &App, area: Rect, buf: &mut Buffer) {
    // Determine border style based on status
    let border_style = match app.run_state.status {
        RunStatus::Running => Style::default().fg(Color::Cyan),
        RunStatus::Verifying => Style::default().fg(Color::Magenta),
        RunStatus::Completed => Style::default().fg(Color::Green),
        RunStatus::Failed | RunStatus::Cancelled => Style::default().fg(Color::Yellow),
        RunStatus::Idle => Styles::border(),
    };

    let block = Block::default()
        .title(" Run Status ")
        .title_style(Styles::title())
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Styles::default());

    let inner = block.inner(area);
    block.render(area, buf);

    // Build compact status line
    let status_style = match app.run_state.status {
        RunStatus::Running => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        RunStatus::Verifying => Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        RunStatus::Completed => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        RunStatus::Failed => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        RunStatus::Cancelled => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        RunStatus::Idle => Styles::dim(),
    };

    let status_text = match app.run_state.status {
        RunStatus::Running => "RUNNING",
        RunStatus::Verifying => "VERIFYING",
        RunStatus::Completed => "COMPLETED",
        RunStatus::Cancelled => "CANCELLED",
        RunStatus::Failed => "FAILED",
        RunStatus::Idle => "READY",
    };

    // First line: Status | Run ID | Elapsed
    let elapsed = if let Some(start) = app.run_state.started_at {
        let secs = start.elapsed().as_secs();
        format!("{}m {:02}s", secs / 60, secs % 60)
    } else {
        "-".to_string()
    };

    let line1 = Line::from(vec![
        Span::raw(" "),
        Span::styled(status_text, status_style),
        Span::styled("  Run: ", Styles::dim()),
        Span::styled(
            app.run_state.run_id.as_deref().unwrap_or("-"),
            Style::default().fg(Color::White),
        ),
        Span::styled("  Elapsed: ", Styles::dim()),
        Span::styled(&elapsed, Style::default().fg(Color::White)),
    ]);

    // Second line: Iteration | Model | Cooldowns
    let iteration_text = if app.run_state.max_iterations > 0 {
        format!(
            "Iter {}/{}",
            app.run_state.current_iteration, app.run_state.max_iterations
        )
    } else {
        format!("Iter {}", app.run_state.current_iteration)
    };

    let model_text = app.run_state.current_model.as_deref().unwrap_or("-");

    let mut line2_spans = vec![
        Span::raw(" "),
        Span::styled(&iteration_text, Style::default().fg(Color::Magenta)),
        Span::styled("  Model: ", Styles::dim()),
        Span::styled(model_text, Style::default().fg(Color::Cyan)),
    ];

    // Add cooldowns inline if any
    if !app.run_state.cooldowns.is_empty() {
        line2_spans.push(Span::styled("  Cooldowns: ", Styles::dim()));
        let cooldown_text: Vec<String> = app
            .run_state
            .cooldowns
            .iter()
            .map(|(m, s)| format!("{}:{}s", m, s))
            .collect();
        line2_spans.push(Span::styled(
            cooldown_text.join(", "),
            Style::default().fg(Color::Yellow),
        ));
    }

    let line2 = Line::from(line2_spans);

    let paragraph = Paragraph::new(vec![line1, line2]).style(Styles::default());
    paragraph.render(inner, buf);
}

fn render_output_pane(app: &App, area: Rect, buf: &mut Buffer) {
    let border_style = match app.run_state.status {
        RunStatus::Running => Style::default().fg(Color::Cyan),
        RunStatus::Verifying => Style::default().fg(Color::Magenta),
        _ => Styles::border(),
    };

    // Calculate scroll position for indicator
    let total_lines = app.run_state.model_output.lines().count();
    let scroll = app.run_state.output_scroll;

    // Build title with scroll indicator if there's content
    let title = if total_lines > 0 {
        // Calculate viewport height (approximate, will be refined after block.inner)
        let approx_height = area.height.saturating_sub(2) as usize; // borders
        let end_line = (scroll + approx_height).min(total_lines);
        let follow_indicator = if app.run_state.follow_output { " [F]" } else { "" };
        format!(" Output [{}-{}/{}]{} ", scroll + 1, end_line, total_lines, follow_indicator)
    } else {
        " Output ".to_string()
    };

    let block = Block::default()
        .title(title)
        .title_style(Styles::title())
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Styles::default());

    let inner = block.inner(area);
    block.render(area, buf);

    if app.run_state.model_output.is_empty() {
        let hint = match app.run_state.status {
            RunStatus::Running => "Waiting for model output...",
            RunStatus::Verifying => "Verifying completion criteria...",
            _ => "Press Enter to start the run",
        };
        let paragraph = Paragraph::new(Line::from(Span::styled(
            format!(" {hint}"),
            Styles::dim(),
        )));
        paragraph.render(inner, buf);
        return;
    }

    // Colorize output lines
    let lines: Vec<Line<'_>> = app
        .run_state
        .model_output
        .lines()
        .skip(scroll)
        .take(inner.height as usize)
        .map(|l| colorize_output_line(l))
        .collect();

    let paragraph = Paragraph::new(lines)
        .style(Styles::default())
        .wrap(Wrap { trim: false });
    paragraph.render(inner, buf);
}

fn colorize_output_line(line: &str) -> Line<'_> {
    // Simple colorization based on content patterns
    let trimmed = line.trim();

    if trimmed.starts_with("error") || trimmed.starts_with("Error") || trimmed.contains("ERROR") {
        Line::from(Span::styled(line, Style::default().fg(Color::Red)))
    } else if trimmed.starts_with("warning") || trimmed.starts_with("Warning") || trimmed.contains("WARN") {
        Line::from(Span::styled(line, Style::default().fg(Color::Yellow)))
    } else if trimmed.starts_with(">>>") || trimmed.starts_with("===") {
        Line::from(Span::styled(line, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
    } else if trimmed.starts_with("✓") || trimmed.contains("success") || trimmed.contains("passed") {
        Line::from(Span::styled(line, Style::default().fg(Color::Green)))
    } else if trimmed.starts_with("•") || trimmed.starts_with("-") || trimmed.starts_with("*") {
        Line::from(Span::styled(line, Style::default().fg(Color::White)))
    } else {
        Line::from(Span::raw(line))
    }
}

fn render_events_pane(app: &App, area: Rect, buf: &mut Buffer) {
    let block = Block::default()
        .title(" Events ")
        .title_style(Styles::title())
        .borders(Borders::ALL)
        .border_style(Styles::border())
        .style(Styles::default());

    let inner = block.inner(area);
    block.render(area, buf);

    if app.run_state.events.is_empty() {
        let paragraph = Paragraph::new(Line::from(Span::styled(
            " No events yet",
            Styles::dim(),
        )));
        paragraph.render(inner, buf);
        return;
    }

    // Show most recent events, colorized
    let max_events = inner.height as usize;
    let events_to_show: Vec<&String> = app.run_state.events.iter().rev().take(max_events).collect();

    let lines: Vec<Line<'_>> = events_to_show
        .into_iter()
        .rev()
        .map(|e| colorize_event(e))
        .collect();

    let paragraph = Paragraph::new(lines).style(Styles::default());
    paragraph.render(inner, buf);
}

fn colorize_event(event: &str) -> Line<'_> {
    let style = if event.contains("Started") || event.contains("started") {
        Style::default().fg(Color::Cyan)
    } else if event.contains("Completed") || event.contains("completed") || event.contains("PASS") {
        Style::default().fg(Color::Green)
    } else if event.contains("Failed") || event.contains("failed") || event.contains("FAIL") {
        Style::default().fg(Color::Red)
    } else if event.contains("Cancelled") || event.contains("cancelled") || event.contains("Cancel") {
        Style::default().fg(Color::Yellow)
    } else if event.contains("cooldown") || event.contains("Cooldown") {
        Style::default().fg(Color::Yellow)
    } else if event.contains("Rate") || event.contains("rate") {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::Gray)
    };

    Line::from(Span::styled(format!(" {event}"), style))
}

fn render_criteria_pane(app: &App, area: Rect, buf: &mut Buffer) {
    let block = Block::default()
        .title(" Criteria ")
        .title_style(Styles::title())
        .borders(Borders::ALL)
        .border_style(Styles::border())
        .style(Styles::default());

    let inner = block.inner(area);
    block.render(area, buf);

    if app.run_state.criteria.is_empty() {
        let paragraph = Paragraph::new(Line::from(Span::styled(
            " No criteria found",
            Styles::dim(),
        )));
        paragraph.render(inner, buf);
        return;
    }

    // Build wrapped text with checkboxes based on verification status
    // Unicode symbols: ☐ (pending), ⏳ (verifying), ☑ (passed), ☒ (failed)
    let mut lines: Vec<Line<'_>> = Vec::new();
    for (i, criterion) in app.run_state.criteria.iter().enumerate() {
        let status = app
            .run_state
            .criteria_status
            .get(i)
            .copied()
            .unwrap_or(CriterionStatus::Pending);

        let (symbol, symbol_color, text_color) = match status {
            CriterionStatus::Pending => ("☐", Color::Gray, Color::White),
            CriterionStatus::Verifying => ("⏳", Color::Cyan, Color::Cyan),
            CriterionStatus::Passed => ("☑", Color::Green, Color::Green),
            CriterionStatus::Failed => ("☒", Color::Red, Color::Red),
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{symbol} "), Style::default().fg(symbol_color)),
            Span::styled(criterion.as_str(), Style::default().fg(text_color)),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .style(Style::default())
        .wrap(Wrap { trim: false });
    paragraph.render(inner, buf);
}

fn render_git_pane(app: &App, area: Rect, buf: &mut Buffer) {
    let block = Block::default()
        .title(" Git ")
        .title_style(Styles::title())
        .borders(Borders::ALL)
        .border_style(Styles::border())
        .style(Styles::default());

    let inner = block.inner(area);
    block.render(area, buf);

    let mut lines = vec![Line::from(vec![
        Span::styled(" Branch: ", Styles::dim()),
        Span::styled(&app.git_info.branch, Style::default().fg(Color::Magenta)),
    ])];

    if app.git_info.changed_files.is_empty() {
        lines.push(Line::from(Span::styled(" No changes", Styles::dim())));
    } else {
        let max_files = (inner.height as usize).saturating_sub(1);
        for file in app.git_info.changed_files.iter().take(max_files) {
            // Color based on file extension
            let style = if file.ends_with(".rs") {
                Style::default().fg(Color::Cyan)
            } else if file.ends_with(".md") || file.ends_with(".txt") {
                Style::default().fg(Color::Green)
            } else if file.ends_with(".json") || file.ends_with(".toml") {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(Span::styled(format!(" {file}"), style)));
        }
        if app.git_info.changed_files.len() > max_files {
            lines.push(Line::from(Span::styled(
                format!(" +{} more", app.git_info.changed_files.len() - max_files),
                Styles::dim(),
            )));
        }
    }

    let paragraph = Paragraph::new(lines).style(Styles::default());
    paragraph.render(inner, buf);
}

