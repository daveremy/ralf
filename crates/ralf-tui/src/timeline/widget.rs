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
use crate::text::render_markdown;
use crate::theme::Theme;

/// Timeline pane widget.
pub struct TimelineWidget<'a> {
    state: &'a TimelineState,
    theme: &'a Theme,
    focused: bool,
    /// Whether to render with a border (default: true).
    with_border: bool,
}

impl<'a> TimelineWidget<'a> {
    /// Create a new timeline widget.
    pub fn new(state: &'a TimelineState, theme: &'a Theme) -> Self {
        Self {
            state,
            theme,
            focused: false,
            with_border: true,
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
            _ => self.theme.info, // Fallback for unknown models
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

        // Line 1: timestamp + badge + attribution
        let time_str = event.time_str();
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
            Span::styled(&time_str, Style::default().fg(self.theme.muted)),
            Span::raw("  "),
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
        let collapse_indicator = if event.is_collapsible() {
            if event.collapsed {
                "\u{25b8} " // ▸
            } else {
                "\u{25be} " // ▾
            }
        } else {
            "  "
        };

        if event.collapsed || !event.is_collapsible() {
            // Single line summary
            let summary = event.summary();
            let max_len = width.saturating_sub(6); // Account for prefix
            let display = truncate_str(&summary, max_len);

            let line = Line::from(vec![
                Span::raw("       "), // Indent to align with content
                Span::styled(collapse_indicator, Style::default().fg(self.theme.muted)),
                Span::styled(display, Style::default().fg(self.theme.text)),
            ]);
            let para = Paragraph::new(line);
            para.render(Rect::new(area.x, y, area.width, 1), buf);
            y += 1;
        } else {
            // Expanded content - check if assistant message for markdown rendering
            let is_assistant_message = matches!(
                &event.kind,
                EventKind::Spec(spec) if !spec.is_user
            );

            if is_assistant_message {
                // Render assistant messages with markdown styling
                let content = event.copyable_content();
                let md_lines = render_markdown(&content, width.saturating_sub(9), self.theme);
                let total_lines = md_lines.len();
                let display_lines = total_lines.min(MAX_EXPANDED_LINES);
                let has_more = total_lines > MAX_EXPANDED_LINES;

                for (i, md_line) in md_lines.into_iter().take(display_lines).enumerate() {
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

                // Show "[+N more]" if truncated
                if has_more && y < area.y + area.height {
                    let more = total_lines - MAX_EXPANDED_LINES;
                    let line = Line::from(vec![
                        Span::raw("         "),
                        Span::styled(
                            format!("[+{more} more]"),
                            Style::default().fg(self.theme.muted),
                        ),
                    ]);
                    let para = Paragraph::new(line);
                    para.render(Rect::new(area.x, y, area.width, 1), buf);
                    y += 1;
                }
            } else {
                // Plain text rendering for user/system messages
                let content_lines = event.content_lines();
                let display_lines = content_lines.len().min(MAX_EXPANDED_LINES);
                let has_more = content_lines.len() > MAX_EXPANDED_LINES;

                for (i, content_line) in content_lines.iter().take(display_lines).enumerate() {
                    if y >= area.y + area.height {
                        break;
                    }

                    let prefix = if i == 0 { collapse_indicator } else { "  " };
                    let max_len = width.saturating_sub(9);
                    let display = truncate_str(content_line, max_len);

                    let line = Line::from(vec![
                        Span::raw("       "),
                        Span::styled(prefix, Style::default().fg(self.theme.muted)),
                        Span::styled(display, Style::default().fg(self.theme.text)),
                    ]);
                    let para = Paragraph::new(line);
                    para.render(Rect::new(area.x, y, area.width, 1), buf);
                    y += 1;
                }

                // Show "[+N more]" if truncated
                if has_more && y < area.y + area.height {
                    let more = content_lines.len() - MAX_EXPANDED_LINES;
                    let line = Line::from(vec![
                        Span::raw("         "),
                        Span::styled(
                            format!("[+{more} more]"),
                            Style::default().fg(self.theme.muted),
                        ),
                    ]);
                    let para = Paragraph::new(line);
                    para.render(Rect::new(area.x, y, area.width, 1), buf);
                    y += 1;
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

/// Truncate a string to `max_len`, adding ellipsis if needed.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", &s[..max_len - 3])
    } else {
        s[..max_len].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("short", 10), "short");
        assert_eq!(truncate_str("this is a long string", 10), "this is...");
        assert_eq!(truncate_str("abc", 3), "abc");
        assert_eq!(truncate_str("abcd", 3), "abc");
    }

    #[test]
    fn test_timeline_widget_creation() {
        let state = TimelineState::new();
        let theme = Theme::default();
        let widget = TimelineWidget::new(&state, &theme).focused(true);
        assert!(widget.focused);
    }
}
