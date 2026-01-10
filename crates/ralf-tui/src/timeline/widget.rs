//! Timeline widget for rendering events.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use super::event::{EventKind, ReviewResult, SystemLevel, TimelineEvent, MAX_EXPANDED_LINES};
use super::state::TimelineState;
use crate::text::{render_markdown, wrap_lines, wrap_text};
use crate::theme::Theme;

/// Timeline pane widget.
pub struct TimelineWidget<'a> {
    state: &'a TimelineState,
    theme: &'a Theme,
    focused: bool,
    /// Whether to render with a border (default: true).
    with_border: bool,
    /// Whether the canvas is showing spec content (auto-collapse assistant spec events).
    canvas_shows_spec: bool,
}

impl<'a> TimelineWidget<'a> {
    /// Create a new timeline widget.
    pub fn new(state: &'a TimelineState, theme: &'a Theme) -> Self {
        Self {
            state,
            theme,
            focused: false,
            with_border: true,
            canvas_shows_spec: false,
        }
    }

    /// Set whether the pane is focused.
    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Set whether to render with a border.
    ///
    /// When false, renders just the timeline content without a surrounding border.
    /// Useful when embedding in a composite widget like `ConversationPane`.
    #[must_use]
    pub fn with_border(mut self, border: bool) -> Self {
        self.with_border = border;
        self
    }

    /// Set whether the canvas is showing spec content.
    ///
    /// When true, assistant spec events (non-user) will be auto-collapsed to avoid
    /// duplicating content shown in the canvas pane.
    #[must_use]
    pub fn canvas_shows_spec(mut self, shows_spec: bool) -> Self {
        self.canvas_shows_spec = shows_spec;
        self
    }

    /// Get the badge color for an event.
    fn badge_color(&self, event: &TimelineEvent) -> ratatui::style::Color {
        match &event.kind {
            EventKind::Spec(_) => self.theme.primary,
            EventKind::Run(e) => self.model_color(&e.model),
            EventKind::Review(e) => match e.result {
                ReviewResult::Passed => self.theme.success,
                ReviewResult::Failed => self.theme.error,
                ReviewResult::Skipped => self.theme.muted,
            },
            EventKind::System(e) => match e.level {
                SystemLevel::Info => self.theme.info,
                SystemLevel::Warning => self.theme.warning,
                SystemLevel::Error => self.theme.error,
            },
        }
    }

    /// Get the color for a model name.
    fn model_color(&self, model: &str) -> ratatui::style::Color {
        match model {
            "claude" => self.theme.claude,
            "gemini" => self.theme.gemini,
            "codex" => self.theme.codex,
            _ => self.theme.info,
        }
    }

    /// Render a "[+N more]" truncation indicator.
    fn render_truncation_indicator(
        &self,
        y: &mut u16,
        area: Rect,
        buf: &mut Buffer,
        remaining_lines: usize,
    ) {
        if *y < area.y + area.height {
            let line = Line::from(vec![
                Span::raw("         "),
                Span::styled(
                    format!("[+{remaining_lines} more]"),
                    Style::default().fg(self.theme.muted),
                ),
            ]);
            Paragraph::new(line).render(Rect::new(area.x, *y, area.width, 1), buf);
            *y += 1;
        }
    }

    /// Render a single event.
    #[allow(clippy::too_many_lines)]
    fn render_event(
        &self,
        event: &TimelineEvent,
        selected: bool,
        area: Rect,
        buf: &mut Buffer,
    ) -> u16 {
        let mut y = area.y;
        let width = area.width as usize;

        // Selection indicator
        let selection_prefix = if selected { "\u{25b8} " } else { "  " }; // ▸ or space

        // Line 1: badge + attribution (no timestamp)
        let badge = event.badge();
        let attribution = event.attribution();
        let badge_color = self.badge_color(event);

        let mut spans = vec![
            Span::styled(
                selection_prefix,
                Style::default().fg(if selected {
                    self.theme.primary
                } else {
                    self.theme.base
                }),
            ),
            Span::styled("[", Style::default().fg(self.theme.muted)),
            Span::styled(badge, Style::default().fg(badge_color)),
            Span::styled("] ", Style::default().fg(self.theme.muted)),
        ];

        if !attribution.is_empty() {
            spans.push(Span::styled(
                attribution,
                Style::default().fg(self.theme.subtext),
            ));
        }

        let line1 = Line::from(spans);
        let para1 = Paragraph::new(line1);
        para1.render(Rect::new(area.x, y, area.width, 1), buf);
        y += 1;

        if y >= area.y + area.height {
            return y - area.y;
        }

        // Line 2+: content
        // Force collapse assistant spec events when canvas is showing spec
        let is_assistant_spec = matches!(&event.kind, EventKind::Spec(spec) if !spec.is_user);
        let force_collapse = self.canvas_shows_spec && is_assistant_spec;
        let effectively_collapsed = event.collapsed || force_collapse;

        let collapse_indicator = if event.is_collapsible() {
            if effectively_collapsed {
                "\u{25b8} " // ▸
            } else {
                "\u{25be} " // ▾
            }
        } else {
            "  "
        };

        if effectively_collapsed || !event.is_collapsible() {
            // Collapsed: wrap summary text instead of truncating
            let summary = event.summary();
            let content_width = width.saturating_sub(9); // Account for indent + prefix
            let wrapped = wrap_text(&summary, content_width);

            for (i, line_text) in wrapped.iter().enumerate() {
                if y >= area.y + area.height {
                    break;
                }

                let prefix = if i == 0 { collapse_indicator } else { "  " };
                let line = Line::from(vec![
                    Span::raw("       "), // Indent to align with content
                    Span::styled(prefix, Style::default().fg(self.theme.muted)),
                    Span::styled(line_text.clone(), Style::default().fg(self.theme.text)),
                ]);
                let para = Paragraph::new(line);
                para.render(Rect::new(area.x, y, area.width, 1), buf);
                y += 1;
            }
        } else {
            // Expanded content - check if assistant message for markdown rendering
            let is_assistant_message = matches!(
                &event.kind,
                EventKind::Spec(spec) if !spec.is_user
            );

            // Calculate available width for content (accounting for indent + prefix)
            let content_width = width.saturating_sub(9);

            // Selected events show all content; non-selected are truncated
            let max_lines = if selected { usize::MAX } else { MAX_EXPANDED_LINES };

            if is_assistant_message {
                // Render assistant messages with markdown styling
                let content = event.copyable_content();
                let md_lines = render_markdown(&content, content_width, self.theme);
                // Wrap lines to fit available width
                let wrapped_lines = wrap_lines(md_lines, content_width);
                let total_lines = wrapped_lines.len();
                let display_lines = total_lines.min(max_lines);
                let has_more = total_lines > max_lines;

                for (i, md_line) in wrapped_lines.into_iter().take(display_lines).enumerate() {
                    if y >= area.y + area.height {
                        break;
                    }

                    let prefix = if i == 0 { collapse_indicator } else { "  " };

                    // Prepend indentation and collapse indicator to the markdown line
                    let mut final_spans = vec![
                        Span::raw("       "),
                        Span::styled(prefix, Style::default().fg(self.theme.muted)),
                    ];
                    final_spans.extend(md_line.spans);

                    let line = Line::from(final_spans);
                    let para = Paragraph::new(line);
                    para.render(Rect::new(area.x, y, area.width, 1), buf);
                    y += 1;
                }

                if has_more {
                    self.render_truncation_indicator(&mut y, area, buf, total_lines - max_lines);
                }
            } else {
                // Plain text rendering for user/system messages
                let content_lines = event.content_lines();
                let wrapped_content: Vec<String> = content_lines
                    .iter()
                    .flat_map(|line| wrap_text(line, content_width))
                    .collect();

                let total_lines = wrapped_content.len();
                let display_lines = total_lines.min(max_lines);
                let has_more = total_lines > max_lines;

                for (i, content_line) in wrapped_content.iter().take(display_lines).enumerate() {
                    if y >= area.y + area.height {
                        break;
                    }

                    let prefix = if i == 0 { collapse_indicator } else { "  " };

                    let line = Line::from(vec![
                        Span::raw("       "),
                        Span::styled(prefix, Style::default().fg(self.theme.muted)),
                        Span::styled(content_line.clone(), Style::default().fg(self.theme.text)),
                    ]);
                    Paragraph::new(line).render(Rect::new(area.x, y, area.width, 1), buf);
                    y += 1;
                }

                if has_more {
                    self.render_truncation_indicator(&mut y, area, buf, total_lines - max_lines);
                }
            }
        }

        y - area.y
    }
}

impl Widget for TimelineWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Determine inner area based on border setting
        let inner = if self.with_border {
            // Render border
            let border_style = if self.focused {
                Style::default().fg(self.theme.border_focused)
            } else {
                Style::default().fg(self.theme.border)
            };

            let block = Block::default()
                .title(" Timeline ")
                .title_style(Style::default().fg(self.theme.text))
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Style::default().bg(self.theme.base));

            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            // No border - use full area
            area
        };

        if inner.height == 0 {
            return;
        }

        // Empty state
        if self.state.is_empty() {
            let empty_msg = Line::from(vec![Span::styled(
                "No events yet",
                Style::default().fg(self.theme.muted),
            )]);
            let para = Paragraph::new(empty_msg);
            para.render(
                Rect::new(inner.x + 2, inner.y + inner.height / 2, inner.width - 4, 1),
                buf,
            );
            return;
        }

        // Calculate visible events
        let visible_count = self.state.events_per_page(inner.height as usize);
        let visible = self.state.visible_events(visible_count);

        // Render events
        let mut y = inner.y;
        for (idx, event) in visible {
            if y >= inner.y + inner.height {
                break;
            }

            let is_selected = self.state.selected() == Some(idx);
            let remaining_height = (inner.y + inner.height).saturating_sub(y);
            let event_area = Rect::new(inner.x, y, inner.width, remaining_height);

            let lines_used = self.render_event(event, is_selected, event_area, buf);
            y += lines_used;

            // Add empty line between events if space
            if y < inner.y + inner.height {
                y += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_widget_creation() {
        let state = TimelineState::new();
        let theme = Theme::default();
        let widget = TimelineWidget::new(&state, &theme).focused(true);
        assert!(widget.focused);
    }
}
