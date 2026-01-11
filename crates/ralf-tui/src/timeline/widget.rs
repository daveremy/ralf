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
use crate::text::{render_markdown, truncate_to_width, visual_width, wrap_lines, wrap_text};
use crate::theme::Theme;

/// Spinner frames for pending indicator animation (Unicode braille).
const SPINNER: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

/// ASCII spinner frames for terminals without Unicode support.
const SPINNER_ASCII: [&str; 4] = ["-", "\\", "|", "/"];

/// Timeline pane widget.
#[allow(clippy::struct_excessive_bools)]
pub struct TimelineWidget<'a> {
    state: &'a TimelineState,
    theme: &'a Theme,
    focused: bool,
    /// Whether to render with a border (default: true).
    with_border: bool,
    /// Whether the canvas is showing spec content (auto-collapse assistant spec events).
    canvas_shows_spec: bool,
    /// Tick counter for animations.
    tick: usize,
    /// Whether to use ASCII-only symbols.
    ascii_mode: bool,
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
            tick: 0,
            ascii_mode: false,
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

    /// Set the tick counter for animations.
    #[must_use]
    pub fn tick(mut self, tick: usize) -> Self {
        self.tick = tick;
        self
    }

    /// Set whether to use ASCII-only symbols.
    #[must_use]
    pub fn ascii_mode(mut self, ascii: bool) -> Self {
        self.ascii_mode = ascii;
        self
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

    /// Render the pending response indicator with animated spinner.
    fn render_pending_indicator(&self, model: &str, y: u16, area: Rect, buf: &mut Buffer) {
        if y >= area.y + area.height {
            return;
        }

        // Animate spinner at ~2 frames per tick (4Hz tick = 2Hz spinner)
        let frame = if self.ascii_mode {
            SPINNER_ASCII[(self.tick / 2) % SPINNER_ASCII.len()]
        } else {
            SPINNER[(self.tick / 2) % SPINNER.len()]
        };
        let color = self.model_color(model);

        let line = Line::from(vec![
            Span::raw("  "),
            Span::styled(frame, Style::default().fg(color)),
            Span::raw(" "),
            Span::styled(model, Style::default().fg(self.theme.subtext)),
            Span::styled(" is thinking...", Style::default().fg(self.theme.muted)),
        ]);

        Paragraph::new(line).render(Rect::new(area.x, y, area.width, 1), buf);
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

    /// Render a single event in compact format.
    ///
    /// Format: `▸ › content...` or `▾ ● content...`
    /// Text wraps instead of truncating.
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

        // Force collapse assistant spec events when canvas is showing spec
        let is_assistant_spec = matches!(&event.kind, EventKind::Spec(spec) if !spec.is_user);
        let force_collapse = self.canvas_shows_spec && is_assistant_spec;
        let effectively_collapsed = event.collapsed || force_collapse;

        // Collapse/expand indicator
        let collapse_indicator = match (event.is_collapsible(), effectively_collapsed, selected) {
            (true, true, _) | (false, _, true) => {
                if self.ascii_mode { ">" } else { "▸" }
            }
            (true, false, _) => {
                if self.ascii_mode { "v" } else { "▾" }
            }
            (false, _, false) => " ",
        };

        // Speaker symbol and color
        let speaker_symbol = if self.ascii_mode {
            event.speaker_symbol_ascii()
        } else {
            event.speaker_symbol()
        };
        let symbol_color = self.speaker_color(event);

        // Model attribution (shown as colored prefix for AI events)
        let attribution = event.model_attribution();

        // Calculate content width (attribution prefix handled in render_first_line)
        // Layout: [collapse 1][space 1][symbol 1][space 1][attr: ][content...]
        let content_max_width = width.saturating_sub(4); // "▸ ● " = 4 chars base

        // Get summary for first line
        let summary = event.summary();

        // Render first line(s) with wrapping: collapse + symbol + content + attribution
        let lines_used = self.render_first_line_wrapped(
            &mut y,
            area,
            buf,
            collapse_indicator,
            speaker_symbol,
            symbol_color,
            &summary,
            content_max_width,
            attribution.as_deref(),
            selected,
            effectively_collapsed,
        );

        if y >= area.y + area.height {
            return y - area.y;
        }

        // Render continuation lines if expanded
        // (only if there are more lines beyond what we showed in the wrapped first line)
        if !effectively_collapsed && event.is_collapsible() {
            // For wrapped display, we've already shown the first line(s) of content
            // Now show remaining content lines
            let _ = lines_used; // Used for tracking what we've shown
            let is_assistant_message = matches!(
                &event.kind,
                EventKind::Spec(spec) if !spec.is_user
            );

            // Indent for continuation lines (4 spaces to align with content)
            let indent = "    ";
            let content_width = width.saturating_sub(indent.len());

            // Selected events show all content; non-selected are truncated
            let max_lines = if selected { usize::MAX } else { MAX_EXPANDED_LINES };

            if is_assistant_message {
                // Render with markdown
                let content = event.copyable_content();
                let md_lines = render_markdown(&content, content_width, self.theme);
                let wrapped_lines = wrap_lines(md_lines, content_width);

                // Skip first line (already rendered in summary)
                let remaining: Vec<_> = wrapped_lines.into_iter().skip(1).collect();
                let total_lines = remaining.len();
                let display_lines = total_lines.min(max_lines);
                let has_more = total_lines > max_lines;

                for md_line in remaining.into_iter().take(display_lines) {
                    if y >= area.y + area.height {
                        break;
                    }

                    let mut final_spans = vec![Span::raw(indent)];
                    final_spans.extend(md_line.spans);

                    let line = Line::from(final_spans);
                    Paragraph::new(line).render(Rect::new(area.x, y, area.width, 1), buf);
                    y += 1;
                }

                if has_more {
                    self.render_truncation_indicator(&mut y, area, buf, total_lines - max_lines);
                }
            } else {
                // Plain text for user/system messages
                let content_lines = event.content_lines();
                let wrapped: Vec<String> = content_lines
                    .iter()
                    .flat_map(|line| wrap_text(line, content_width))
                    .collect();

                // Skip first line
                let remaining: Vec<_> = wrapped.into_iter().skip(1).collect();
                let total_lines = remaining.len();
                let display_lines = total_lines.min(max_lines);
                let has_more = total_lines > max_lines;

                for content_line in remaining.into_iter().take(display_lines) {
                    if y >= area.y + area.height {
                        break;
                    }

                    let line = Line::from(vec![
                        Span::raw(indent),
                        Span::styled(content_line, Style::default().fg(self.theme.text)),
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

    /// Render the first line(s) of an event with text wrapping.
    ///
    /// Format: `▸ ● claude: I'll help you...` (attribution as colored prefix)
    /// When collapsed, shows up to 2 wrapped lines. When expanded, just shows the first line
    /// (continuation handled separately).
    ///
    /// Returns the number of content lines rendered from the summary.
    #[allow(clippy::too_many_arguments)]
    fn render_first_line_wrapped(
        &self,
        y: &mut u16,
        area: Rect,
        buf: &mut Buffer,
        collapse_indicator: &str,
        speaker_symbol: &str,
        symbol_color: ratatui::style::Color,
        summary: &str,
        content_max_width: usize,
        attribution: Option<&str>,
        selected: bool,
        collapsed: bool,
    ) -> usize {
        // Calculate prefix width for attribution
        let (attr_prefix, attr_width) = if let Some(attr) = attribution {
            let model_base = attr.split_whitespace().next().unwrap_or(attr);
            (Some((attr.to_string(), self.model_color(model_base))), visual_width(attr) + 2)
        } else {
            (None, 0)
        };

        // Available width for content on first line (after prefix)
        // Layout: [collapse 1][space 1][symbol 1][space 1][attr: ][content...]
        let first_line_content_width = content_max_width.saturating_sub(attr_width);

        // Continuation line indent: "    " (4 spaces to align with content)
        let continuation_indent = "    ";
        let continuation_width = content_max_width;

        // Wrap the summary text
        let wrapped_lines = wrap_text(summary, first_line_content_width);

        // For collapsed events, show up to 2 lines; for expanded, just show first line here
        let max_lines = if collapsed { 2 } else { 1 };
        let lines_to_show = wrapped_lines.len().min(max_lines);

        // Render first line with full prefix
        if !wrapped_lines.is_empty() && *y < area.y + area.height {
            let mut spans = vec![
                // Collapse indicator
                Span::styled(
                    collapse_indicator,
                    Style::default().fg(if selected {
                        self.theme.primary
                    } else {
                        self.theme.muted
                    }),
                ),
                Span::raw(" "),
                // Speaker symbol
                Span::styled(speaker_symbol, Style::default().fg(symbol_color)),
                Span::raw(" "),
            ];

            // Add attribution prefix for AI events (colored by model)
            if let Some((attr, attr_color)) = &attr_prefix {
                spans.push(Span::styled(
                    format!("{attr}: "),
                    Style::default().fg(*attr_color),
                ));
            }

            // Add first line of content
            spans.push(Span::styled(
                wrapped_lines[0].clone(),
                Style::default().fg(self.theme.text),
            ));

            let line = Line::from(spans);
            Paragraph::new(line).render(Rect::new(area.x, *y, area.width, 1), buf);
            *y += 1;
        }

        // Render continuation lines (for collapsed with wrapped content)
        for wrapped_line in wrapped_lines.iter().skip(1).take(lines_to_show - 1) {
            if *y >= area.y + area.height {
                break;
            }

            // Re-wrap for continuation width if needed
            let display_line = if visual_width(wrapped_line) > continuation_width {
                truncate_to_width(wrapped_line, continuation_width)
            } else {
                wrapped_line.clone()
            };

            let line = Line::from(vec![
                Span::raw(continuation_indent),
                Span::styled(display_line, Style::default().fg(self.theme.text)),
            ]);
            Paragraph::new(line).render(Rect::new(area.x, *y, area.width, 1), buf);
            *y += 1;
        }

        // Show "[+N more]" if collapsed and there's more content
        if collapsed && wrapped_lines.len() > max_lines {
            let remaining = wrapped_lines.len() - max_lines;
            if *y < area.y + area.height {
                let line = Line::from(vec![
                    Span::raw(continuation_indent),
                    Span::styled(
                        format!("[+{remaining} more lines]"),
                        Style::default().fg(self.theme.muted),
                    ),
                ]);
                Paragraph::new(line).render(Rect::new(area.x, *y, area.width, 1), buf);
                *y += 1;
            }
        }

        lines_to_show
    }

    /// Get the color for the speaker symbol.
    fn speaker_color(&self, event: &TimelineEvent) -> ratatui::style::Color {
        match &event.kind {
            EventKind::Spec(e) if e.is_user => self.theme.text,
            EventKind::Spec(e) => {
                if let Some(ref model) = e.model {
                    self.model_color(model)
                } else {
                    self.theme.info
                }
            }
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

        // Empty state (but may still have pending)
        if self.state.is_empty() {
            // Show pending indicator even when timeline is empty
            if let Some(model) = self.state.pending_response() {
                self.render_pending_indicator(model, inner.y, inner, buf);
            } else {
                let empty_msg = Line::from(vec![Span::styled(
                    "No events yet",
                    Style::default().fg(self.theme.muted),
                )]);
                let para = Paragraph::new(empty_msg);
                para.render(
                    Rect::new(inner.x + 2, inner.y + inner.height / 2, inner.width - 4, 1),
                    buf,
                );
            }
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

        // Render pending indicator if waiting for a response
        if let Some(model) = self.state.pending_response() {
            self.render_pending_indicator(model, y, inner, buf);
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
