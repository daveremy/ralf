//! Spec Studio screen - interactive chat for spec creation.

use crate::app::App;
use crate::screens::Screen;
use crate::ui::main_layout;
use crate::ui::theme::Styles;
use crate::ui::widgets::{KeyHint, StatusBar};
use ralf_engine::Role;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

/// The Spec Studio screen.
pub struct SpecStudioScreen;

impl Screen for SpecStudioScreen {
    fn render(&self, app: &App, area: Rect, buf: &mut Buffer) {
        let (main_area, status_area) = main_layout(area);

        // Split main area into content and input
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),   // Content area (transcript + draft)
                Constraint::Length(5), // Input area
            ])
            .split(main_area);

        // Split content into transcript (left) and draft (right)
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60), // Transcript
                Constraint::Percentage(40), // Draft
            ])
            .split(chunks[0]);

        // Render transcript
        render_transcript(app, content_chunks[0], buf);

        // Render draft
        render_draft(app, content_chunks[1], buf);

        // Render input
        render_input(app, chunks[1], buf);

        // Render status bar
        let model_name = app
            .current_chat_model()
            .map_or("none", |m| m.info.name.as_str());

        let hints = vec![
            KeyHint::new("Enter", "Send"),
            KeyHint::new("Tab", "Model"),
            KeyHint::new("Ctrl+E", "Export"),
            KeyHint::new("Ctrl+F", "Finalize"),
            KeyHint::new("Esc", "Quit"),
        ];
        let mut status_bar = StatusBar::new("Spec Studio").hints(hints);
        if let Some(notification) = &app.notification {
            status_bar = status_bar.right(notification);
        } else {
            status_bar = status_bar.right(model_name);
        }
        status_bar.render(status_area, buf);
    }
}

fn render_transcript(app: &App, area: Rect, buf: &mut Buffer) {
    let block = Block::default()
        .title(" Transcript ")
        .title_style(Styles::title())
        .borders(Borders::ALL)
        .border_style(Styles::border_active())
        .style(Styles::default());

    let inner = block.inner(area);
    block.render(area, buf);

    if app.thread.messages.is_empty() {
        let hint = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Describe what you want to build:",
                Styles::highlight(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Example: \"Add a /health endpoint that returns",
                Styles::dim(),
            )),
            Line::from(Span::styled(
                "  JSON with the server status and uptime.\"",
                Styles::dim(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  The assistant will help refine your spec,",
                Styles::dim(),
            )),
            Line::from(Span::styled(
                "  then press Ctrl+F to finalize.",
                Styles::dim(),
            )),
        ])
        .style(Styles::default());
        hint.render(inner, buf);
        return;
    }

    let mut lines = Vec::new();
    let mut last_was_blank = false;

    for msg in &app.thread.messages {
        let (prefix, style) = match msg.role {
            Role::User => ("You: ", Styles::highlight()),
            Role::Assistant => {
                let model = msg.model.as_deref().unwrap_or("Assistant");
                (model, Styles::active())
            }
            Role::System => ("System: ", Styles::dim()),
        };

        // Add prefix on first line
        let content_lines: Vec<&str> = msg.content.lines().collect();
        if let Some(first) = content_lines.first() {
            // First line has prefix, then render content with markdown
            let first_md = render_markdown_line(first);
            let mut spans = vec![Span::styled(format!("{prefix}: "), style)];
            spans.extend(first_md.spans);
            lines.push(Line::from(spans));
            last_was_blank = false;
        }
        // Add remaining lines with indent, with markdown styling
        // Collapse consecutive blank lines
        for line in content_lines.iter().skip(1) {
            let is_blank = line.trim().is_empty();
            if is_blank && last_was_blank {
                continue; // Skip consecutive blank lines
            }
            let md_line = render_markdown_line(line);
            // Add indent to the first span
            let mut spans: Vec<Span<'_>> = vec![Span::raw("  ")];
            spans.extend(md_line.spans);
            lines.push(Line::from(spans));
            last_was_blank = is_blank;
        }
        lines.push(Line::from("")); // Blank line between messages
        last_was_blank = true;
    }

    // Show loading indicator if chat in progress
    if app.chat_in_progress {
        lines.push(Line::from(Span::styled(
            "  Waiting for response...",
            Styles::dim(),
        )));
    }

    // Apply scroll offset
    let visible_lines: Vec<Line<'_>> = lines.into_iter().skip(app.transcript_scroll).collect();

    let paragraph = Paragraph::new(visible_lines)
        .style(Styles::default())
        .wrap(Wrap { trim: false });
    paragraph.render(inner, buf);
}

fn render_draft(app: &App, area: Rect, buf: &mut Buffer) {
    let block = Block::default()
        .title(" Draft ")
        .title_style(Styles::title())
        .borders(Borders::ALL)
        .border_style(Styles::border())
        .style(Styles::default());

    let inner = block.inner(area);
    block.render(area, buf);

    if app.thread.draft.is_empty() {
        let hint = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  No draft yet.", Styles::dim())),
            Line::from(""),
            Line::from(Span::styled("  When the assistant produces", Styles::dim())),
            Line::from(Span::styled(
                "  a spec, it will appear here.",
                Styles::dim(),
            )),
            Line::from(""),
            Line::from(Span::styled("  Review it, then Ctrl+F to", Styles::dim())),
            Line::from(Span::styled("  save as PROMPT.md", Styles::dim())),
        ])
        .style(Styles::default());
        hint.render(inner, buf);
        return;
    }

    let lines: Vec<Line<'_>> = app
        .thread
        .draft
        .lines()
        .skip(app.draft_scroll)
        .map(render_markdown_line)
        .collect();

    let paragraph = Paragraph::new(lines)
        .style(Styles::default())
        .wrap(Wrap { trim: false });
    paragraph.render(inner, buf);
}

/// Render a line of markdown with basic styling.
fn render_markdown_line(line: &str) -> Line<'_> {
    let trimmed = line.trim();

    // Headers - render with color, but also parse inline markdown
    if trimmed.starts_with("# ") {
        let content = trimmed.strip_prefix("# ").unwrap_or(trimmed);
        let mut spans = vec![Span::styled(
            "# ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )];
        let inline = render_inline_markdown(content);
        for span in inline.spans {
            // Apply cyan color to non-styled spans
            let new_span = if span.style == Style::default() {
                Span::styled(span.content, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            } else {
                span
            };
            spans.push(new_span);
        }
        return Line::from(spans);
    }
    if trimmed.starts_with("## ") {
        let content = trimmed.strip_prefix("## ").unwrap_or(trimmed);
        let mut spans = vec![Span::styled(
            "## ",
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
        )];
        let inline = render_inline_markdown(content);
        for span in inline.spans {
            let new_span = if span.style == Style::default() {
                Span::styled(span.content, Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))
            } else {
                span
            };
            spans.push(new_span);
        }
        return Line::from(spans);
    }
    if trimmed.starts_with("### ") || trimmed.starts_with("#### ") {
        return Line::from(Span::styled(
            line,
            Style::default().fg(Color::Blue),
        ));
    }

    // Checkboxes - replace with actual symbols and parse inline markdown
    if trimmed.starts_with("- [ ]") || trimmed.starts_with("* [ ]") {
        let content = trimmed
            .strip_prefix("- [ ]")
            .or_else(|| trimmed.strip_prefix("* [ ]"))
            .unwrap_or("");
        let mut spans = vec![Span::styled("☐ ", Style::default().fg(Color::Yellow))];
        let inline = render_inline_markdown(content);
        for span in inline.spans {
            let new_span = if span.style == Style::default() {
                Span::styled(span.content, Style::default().fg(Color::Yellow))
            } else {
                span
            };
            spans.push(new_span);
        }
        return Line::from(spans);
    }
    if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]")
        || trimmed.starts_with("* [x]") || trimmed.starts_with("* [X]") {
        let content = trimmed
            .strip_prefix("- [x]")
            .or_else(|| trimmed.strip_prefix("- [X]"))
            .or_else(|| trimmed.strip_prefix("* [x]"))
            .or_else(|| trimmed.strip_prefix("* [X]"))
            .unwrap_or("");
        let mut spans = vec![Span::styled("☑ ", Style::default().fg(Color::Green))];
        let inline = render_inline_markdown(content);
        for span in inline.spans {
            let new_span = if span.style == Style::default() {
                Span::styled(span.content, Style::default().fg(Color::Green))
            } else {
                span
            };
            spans.push(new_span);
        }
        return Line::from(spans);
    }

    // List items - parse inline markdown
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        let content = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .unwrap_or(trimmed);
        let mut spans = vec![Span::styled("• ", Style::default().fg(Color::White))];
        spans.extend(render_inline_markdown(content).spans);
        return Line::from(spans);
    }

    // Inline bold/emphasis - parse and style
    if line.contains("**") || line.contains('`') {
        return render_inline_markdown(line);
    }

    Line::from(Span::raw(line))
}

/// Render inline markdown (bold, code) with styling.
fn render_inline_markdown(line: &str) -> Line<'_> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_bold = false;
    let mut in_code = false;

    while let Some(ch) = chars.next() {
        if ch == '*' && chars.peek() == Some(&'*') && !in_code {
            // Toggle bold
            chars.next(); // consume second *
            if !current.is_empty() {
                if in_bold {
                    spans.push(Span::styled(
                        std::mem::take(&mut current),
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                } else {
                    spans.push(Span::raw(std::mem::take(&mut current)));
                }
            }
            in_bold = !in_bold;
        } else if ch == '`' && !in_bold {
            // Toggle code
            if !current.is_empty() {
                if in_code {
                    spans.push(Span::styled(
                        std::mem::take(&mut current),
                        Style::default().fg(Color::Magenta),
                    ));
                } else {
                    spans.push(Span::raw(std::mem::take(&mut current)));
                }
            }
            in_code = !in_code;
        } else {
            current.push(ch);
        }
    }

    // Flush remaining
    if !current.is_empty() {
        if in_bold {
            spans.push(Span::styled(
                current,
                Style::default().add_modifier(Modifier::BOLD),
            ));
        } else if in_code {
            spans.push(Span::styled(current, Style::default().fg(Color::Magenta)));
        } else {
            spans.push(Span::raw(current));
        }
    }

    Line::from(spans)
}

fn render_input(app: &App, area: Rect, buf: &mut Buffer) {
    let block = Block::default()
        .title(" Input ")
        .title_style(Styles::title())
        .borders(Borders::ALL)
        .border_style(if app.chat_in_progress {
            Styles::dim()
        } else {
            Styles::border_active()
        })
        .style(Styles::default());

    let inner = block.inner(area);
    block.render(area, buf);

    let input = app
        .input_state
        .widget()
        .focused(!app.chat_in_progress)
        .placeholder("Type your message here...");

    input.render(inner, buf);
}

/// Finalize confirmation overlay.
pub struct FinalizeConfirmScreen;

impl Screen for FinalizeConfirmScreen {
    fn render(&self, app: &App, area: Rect, buf: &mut Buffer) {
        // First render the spec studio behind
        SpecStudioScreen.render(app, area, buf);

        // Then render overlay
        render_finalize_confirm_overlay(app, area, buf);
    }
}

fn render_finalize_confirm_overlay(app: &App, area: Rect, buf: &mut Buffer) {
    use crate::ui::centered_fixed;
    use ratatui::widgets::Clear;

    let width = 70.min(area.width.saturating_sub(4));
    let height = 16.min(area.height.saturating_sub(4));
    let overlay_area = centered_fixed(width, height, area);

    Clear.render(overlay_area, buf);

    let block = Block::default()
        .title(" Finalize Specification ")
        .title_style(Styles::title())
        .borders(Borders::ALL)
        .border_style(Styles::border_active())
        .style(Styles::default());

    let inner = block.inner(overlay_area);
    block.render(overlay_area, buf);

    // Preview content
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Your specification will be saved to PROMPT.md",
            Styles::default(),
        )),
        Line::from(""),
        Line::from(Span::styled("  Preview:", Styles::dim())),
        Line::from(Span::styled(
            "  ─────────────────────────────────────────────────",
            Styles::dim(),
        )),
    ];

    // Show first few lines of draft
    for line in app.thread.draft.lines().take(5) {
        lines.push(Line::from(Span::styled(
            format!("  {line}"),
            Styles::default(),
        )));
    }
    if app.thread.draft.lines().count() > 5 {
        lines.push(Line::from(Span::styled("  ...", Styles::dim())));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  ", Styles::default()),
        Span::styled("[Enter]", Styles::key_hint()),
        Span::styled(" Confirm   ", Styles::default()),
        Span::styled("[Esc]", Styles::key_hint()),
        Span::styled(" Cancel", Styles::default()),
    ]));

    let paragraph = Paragraph::new(lines).style(Styles::default());
    paragraph.render(inner, buf);
}

/// Finalize error overlay (missing promise tag).
pub struct FinalizeErrorScreen;

impl Screen for FinalizeErrorScreen {
    fn render(&self, app: &App, area: Rect, buf: &mut Buffer) {
        // First render the spec studio behind
        SpecStudioScreen.render(app, area, buf);

        // Then render overlay
        render_finalize_error_overlay(area, buf);
    }
}

fn render_finalize_error_overlay(area: Rect, buf: &mut Buffer) {
    use crate::ui::centered_fixed;
    use ratatui::widgets::Clear;

    let width = 60.min(area.width.saturating_sub(4));
    let height = 12.min(area.height.saturating_sub(4));
    let overlay_area = centered_fixed(width, height, area);

    Clear.render(overlay_area, buf);

    let block = Block::default()
        .title(" Cannot Finalize ")
        .title_style(Styles::warning())
        .borders(Borders::ALL)
        .border_style(Styles::border_active())
        .style(Styles::default());

    let inner = block.inner(overlay_area);
    block.render(overlay_area, buf);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Your specification is missing a promise tag.",
            Styles::warning(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  The autonomous loop needs a promise tag to know",
            Styles::default(),
        )),
        Line::from(Span::styled(
            "  when the task is complete. Add this to your spec:",
            Styles::default(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "    <promise>COMPLETE</promise>",
            Styles::highlight(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Styles::default()),
            Span::styled("[Enter]", Styles::key_hint()),
            Span::styled(" Continue editing", Styles::default()),
        ]),
    ];

    let paragraph = Paragraph::new(lines).style(Styles::default());
    paragraph.render(inner, buf);
}

/// Quit confirmation overlay.
pub struct QuitConfirmScreen;

impl Screen for QuitConfirmScreen {
    fn render(&self, app: &App, area: Rect, buf: &mut Buffer) {
        // First render the spec studio behind
        SpecStudioScreen.render(app, area, buf);

        // Then render overlay
        render_quit_confirm_overlay(area, buf);
    }
}

fn render_quit_confirm_overlay(area: Rect, buf: &mut Buffer) {
    use crate::ui::centered_fixed;
    use ratatui::widgets::Clear;

    let width = 50.min(area.width.saturating_sub(4));
    let height = 8.min(area.height.saturating_sub(4));
    let overlay_area = centered_fixed(width, height, area);

    Clear.render(overlay_area, buf);

    let block = Block::default()
        .title(" Exit Ralf TUI? ")
        .title_style(Styles::title())
        .borders(Borders::ALL)
        .border_style(Styles::border_active())
        .style(Styles::default());

    let inner = block.inner(overlay_area);
    block.render(overlay_area, buf);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Your conversation will be saved ", Styles::default()),
            Span::styled("(coming soon)", Styles::dim()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Styles::default()),
            Span::styled("[Enter]", Styles::key_hint()),
            Span::styled(" Exit   ", Styles::default()),
            Span::styled("[Esc]", Styles::key_hint()),
            Span::styled(" Cancel", Styles::default()),
        ]),
    ];

    let paragraph = Paragraph::new(lines).style(Styles::default());
    paragraph.render(inner, buf);
}
