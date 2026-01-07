//! Welcome screen - displayed on first run or when no config exists.

use crate::app::App;
use crate::screens::Screen;
use crate::ui::theme::Styles;
use crate::ui::widgets::{KeyHint, StatusBar};
use crate::ui::{centered_rect, main_layout};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

/// The welcome screen.
pub struct WelcomeScreen;

impl Screen for WelcomeScreen {
    fn render(&self, app: &App, area: Rect, buf: &mut Buffer) {
        let (main_area, status_area) = main_layout(area);

        // Render main content
        render_welcome_content(app, main_area, buf);

        // Render status bar
        let mut hints = vec![KeyHint::new("s", "Setup")];
        if app.config_exists {
            hints.push(KeyHint::new("c", "Chat"));
            if app.prompt_exists() {
                hints.push(KeyHint::new("r", "Run"));
            }
        }
        hints.push(KeyHint::new("?", "Help"));
        hints.push(KeyHint::new("q", "Quit"));

        let mut status_bar = StatusBar::new("Welcome").hints(hints);
        if let Some(notification) = &app.notification {
            status_bar = status_bar.right(notification);
        }
        status_bar.render(status_area, buf);
    }
}

fn render_welcome_content(app: &App, area: Rect, buf: &mut Buffer) {
    // Create centered content area
    let content_area = centered_rect(80, 70, area);

    let block = Block::default()
        .title(" ralf ")
        .title_style(Styles::title())
        .borders(Borders::ALL)
        .border_style(Styles::border())
        .style(Styles::default());

    let inner = block.inner(content_area);
    block.render(content_area, buf);

    let mut lines = Vec::new();

    // Title
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Welcome to ralf — multi-model autonomous loop engine",
        Styles::highlight(),
    )));
    lines.push(Line::from(""));

    // Repository info
    lines.push(Line::from(vec![
        Span::styled("  Repository: ", Styles::dim()),
        Span::styled(app.repo_path.display().to_string(), Styles::default()),
    ]));

    // Git status
    let git_status = if app.git_info.dirty {
        Span::styled("dirty", Styles::warning())
    } else {
        Span::styled("clean", Styles::success())
    };
    lines.push(Line::from(vec![
        Span::styled("  Git status: ", Styles::dim()),
        git_status,
        Span::styled(" (branch: ", Styles::dim()),
        Span::styled(&app.git_info.branch, Styles::default()),
        Span::styled(")", Styles::dim()),
    ]));

    lines.push(Line::from(""));

    // Config status
    let config_status = if app.config_exists {
        Span::styled("[ok] configured", Styles::success())
    } else {
        Span::styled("[!] not configured", Styles::warning())
    };
    lines.push(Line::from(vec![
        Span::styled("  Config: ", Styles::dim()),
        config_status,
    ]));

    lines.push(Line::from(""));

    // Model summary - show configured models from config, not probed status
    lines.push(Line::from(Span::styled("  Models:", Styles::dim())));

    if let Some(config) = &app.config {
        // Show models from config
        if config.model_priority.is_empty() {
            lines.push(Line::from(Span::styled(
                "    No models configured",
                Styles::warning(),
            )));
        } else {
            for model_name in &config.model_priority {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(model_name, Styles::default()),
                    Span::raw(" "),
                    Span::styled("[ok] configured", Styles::success()),
                ]));
            }
        }
    } else if app.models.is_empty() {
        lines.push(Line::from(Span::styled(
            "    No models detected",
            Styles::warning(),
        )));
    } else {
        // No config yet - show detected models
        for model in &app.models {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(&model.info.name, Styles::default()),
                Span::raw(" "),
                Span::styled("[!] not configured", Styles::warning()),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(""));

    // Instructions
    if app.config_exists {
        lines.push(Line::from(vec![
            Span::styled("  Press ", Styles::dim()),
            Span::styled("[c]", Styles::key_hint()),
            Span::styled(" for Spec Studio, ", Styles::dim()),
            Span::styled("[s]", Styles::key_hint()),
            Span::styled(" for setup", Styles::dim()),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  Press ", Styles::dim()),
            Span::styled("[s]", Styles::key_hint()),
            Span::styled(" to run setup, or ", Styles::dim()),
            Span::styled("[q]", Styles::key_hint()),
            Span::styled(" to quit", Styles::dim()),
        ]));
    }

    let paragraph = Paragraph::new(lines).style(Styles::default());
    paragraph.render(inner, buf);
}
