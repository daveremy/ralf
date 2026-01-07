//! Setup screen - model probing and configuration.

use crate::app::App;
use crate::screens::Screen;
use crate::ui::theme::{progress_bar, Styles, Symbols};
use crate::ui::widgets::{KeyHint, StatusBar};
use crate::ui::{centered_rect, main_layout};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

/// The setup screen.
pub struct SetupScreen;

impl Screen for SetupScreen {
    fn render(&self, app: &App, area: Rect, buf: &mut Buffer) {
        let (main_area, status_area) = main_layout(area);

        // Render main content
        render_setup_content(app, main_area, buf);

        // Render status bar
        let hints = vec![
            KeyHint::new("Enter", "Save"),
            KeyHint::new("d", "Toggle"),
            KeyHint::new("r", "Retry"),
            KeyHint::new("Esc", "Back"),
        ];
        let mut status_bar = StatusBar::new("Setup").hints(hints);
        if let Some(notification) = &app.notification {
            status_bar = status_bar.right(notification);
        }
        status_bar.render(status_area, buf);
    }
}

fn render_setup_content(app: &App, area: Rect, buf: &mut Buffer) {
    let content_area = centered_rect(80, 80, area);

    let block = Block::default()
        .title(" Setup ")
        .title_style(Styles::title())
        .borders(Borders::ALL)
        .border_style(Styles::border_active())
        .style(Styles::default());

    let inner = block.inner(content_area);
    block.render(content_area, buf);

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Header
            Constraint::Min(6),     // Model list
            Constraint::Length(1),  // Separator
            Constraint::Length(4),  // Options
            Constraint::Length(2),  // Footer
        ])
        .split(inner);

    // Header
    let header = if app.is_probing() {
        Line::from(Span::styled("  Probing models...", Styles::highlight()))
    } else {
        Line::from(Span::styled("  Model status:", Styles::dim()))
    };
    Paragraph::new(vec![Line::from(""), header])
        .render(chunks[0], buf);

    // Model list
    render_model_list(app, chunks[1], buf);

    // Separator
    let sep = Line::from(Span::styled(
        "  ".to_owned() + &"─".repeat((chunks[2].width as usize).saturating_sub(4)),
        Styles::dim(),
    ));
    Paragraph::new(vec![sep]).render(chunks[2], buf);

    // Options
    render_options(app, chunks[3], buf);

    // Footer hint
    let footer = Line::from(vec![
        Span::styled("  ", Styles::dim()),
        Span::styled("[Enter]", Styles::key_hint()),
        Span::styled(" Save config  ", Styles::dim()),
        Span::styled("[d]", Styles::key_hint()),
        Span::styled(" Toggle selected  ", Styles::dim()),
        Span::styled("[r]", Styles::key_hint()),
        Span::styled(" Retry probe", Styles::dim()),
    ]);
    Paragraph::new(vec![footer]).render(chunks[4], buf);
}

#[allow(clippy::cast_precision_loss)]
fn render_model_list(app: &App, area: Rect, buf: &mut Buffer) {
    let mut lines = Vec::new();

    for (i, model) in app.models.iter().enumerate() {
        let is_selected = i == app.selected_model;
        let prefix = if is_selected { "> " } else { "  " };

        // Determine status and progress
        let (status_str, style) = if model.probing {
            // Animated progress bar
            let progress = ((app.tick % 20) as f32) / 20.0;
            let bar = progress_bar(progress, 20);
            (format!("{bar} probing..."), Styles::dim())
        } else if let Some(result) = &model.probe_result {
            if result.success {
                let time = result.response_time_ms.map(|ms| format!("{ms}ms")).unwrap_or_default();
                (
                    format!("{} ready ({})", Symbols::CHECK, time),
                    Styles::success(),
                )
            } else {
                let err = result.issues.first().map_or("failed", String::as_str);
                (format!("{} {}", Symbols::WARN, err), Styles::warning())
            }
        } else {
            (format!("{} not probed", Symbols::PENDING), Styles::dim())
        };

        // Enabled/disabled indicator
        let enabled_indicator = if model.enabled {
            Span::styled("[+]", Styles::success())
        } else {
            Span::styled("[-]", Styles::dim())
        };

        let name_style = if is_selected {
            Styles::highlight()
        } else if !model.enabled {
            Styles::dim()
        } else {
            Styles::default()
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, if is_selected { Styles::highlight() } else { Styles::dim() }),
            enabled_indicator,
            Span::raw(" "),
            Span::styled(format!("{:<12}", model.info.name), name_style),
            Span::styled(status_str, style),
        ]));
    }

    if app.models.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No models detected. Install claude, codex, or gemini CLI.",
            Styles::warning(),
        )));
    }

    Paragraph::new(lines).render(area, buf);
}

fn render_options(app: &App, area: Rect, buf: &mut Buffer) {
    let mut lines = Vec::new();

    // Model selection strategy
    let rr_style = if app.round_robin {
        Styles::highlight()
    } else {
        Styles::dim()
    };
    let prio_style = if app.round_robin {
        Styles::dim()
    } else {
        Styles::highlight()
    };

    lines.push(Line::from(vec![
        Span::styled("  Model selection: ", Styles::dim()),
        Span::styled(if app.round_robin { "(*)" } else { "( )" }, rr_style),
        Span::styled(" Round-robin  ", rr_style),
        Span::styled(if app.round_robin { "( )" } else { "(*)" }, prio_style),
        Span::styled(" Priority", prio_style),
    ]));

    // Promise tag
    lines.push(Line::from(vec![
        Span::styled("  Promise tag: ", Styles::dim()),
        Span::styled(&app.promise_tag, Styles::default()),
    ]));

    // Arrow keys hint
    lines.push(Line::from(Span::styled(
        "  (Use Left/Right to change selection strategy)",
        Styles::dim(),
    )));

    Paragraph::new(lines).render(area, buf);
}
