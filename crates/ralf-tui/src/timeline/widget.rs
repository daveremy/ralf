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
    /// Format: `▸ › content...` or `▾ ● content...              claude`
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

        // Collapse/expand indicator (ASCII: > and v)
        let (collapsed_icon, expanded_icon) = if self.ascii_mode {
            (">", "v")
        } else {
            ("\u{25b8}", "\u{25be}") // ▸ ▾
        };

        let collapse_indicator = if event.is_collapsible() {
            if effectively_collapsed {
                collapsed_icon
            } else {
                expanded_icon
            }
        } else if selected {
            collapsed_icon
        } else {
            " "
        };

        // Speaker symbol and color
        let speaker_symbol = if self.ascii_mode {
            event.speaker_symbol_ascii()
        } else {
            event.speaker_symbol()
        };
        let symbol_color = self.speaker_color(event);

        // Model attribution (right-aligned, AI events only)
        let attribution = event.model_attribution();
        let attribution_width = attribution.as_ref().map_or(0, |a| a.len() + 2); // +2 for padding

        // Calculate content width
        // Layout: [collapse 1][space 1][symbol 1][space 1][content...][attribution]
        let prefix_width = 4; // "▸ › " = 4 chars
        let content_max_width = width.saturating_sub(prefix_width + attribution_width);

        // Get summary for first line
        let summary = event.summary();

        // Render first line: collapse + symbol + content + attribution
        self.render_first_line(
            y,
            area,
            buf,
            collapse_indicator,
            speaker_symbol,
            symbol_color,
            &summary,
            content_max_width,
            attribution.as_deref(),
            selected,
        );
        y += 1;

        if y >= area.y + area.height {
            return y - area.y;
        }

        // Render continuation lines if expanded
        if !effectively_collapsed && event.is_collapsible() {
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

    /// Render the first line of an event in compact format.
    #[allow(clippy::too_many_arguments)]
    fn render_first_line(
        &self,
        y: u16,
        area: Rect,
        buf: &mut Buffer,
        collapse_indicator: &str,
        speaker_symbol: &str,
        symbol_color: ratatui::style::Color,
        summary: &str,
        content_max_width: usize,
        attribution: Option<&str>,
        selected: bool,
    ) {
        let width = area.width as usize;

        // Truncate summary to fit using unicode-safe truncation
        let display_summary = truncate_to_width(summary, content_max_width);

        // Build spans
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
            // Content
            Span::styled(display_summary.clone(), Style::default().fg(self.theme.text)),
        ];

        // Add right-aligned attribution for AI events
        if let Some(attr) = attribution {
            // Calculate padding using visual width (not byte length)
            let prefix_width = 4; // "▸ ● " = 4 visual cells
            let content_visual_width = visual_width(&display_summary);
            let attr_visual_width = visual_width(attr);
            let used_width = prefix_width + content_visual_width;
            let padding = width.saturating_sub(used_width + attr_visual_width + 1);
            if padding > 0 {
                spans.push(Span::raw(" ".repeat(padding)));
            }
            spans.push(Span::styled(
                attr.to_string(),
                Style::default().fg(self.theme.subtext),
            ));
        }

        let line = Line::from(spans);
        Paragraph::new(line).render(Rect::new(area.x, y, area.width, 1), buf);
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
