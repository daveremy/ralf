//! Conversation pane widget.
//!
//! Combines the timeline (scrollable history) with an input area at the bottom.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    symbols::line,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use ralf_engine::thread::PhaseKind;

use crate::theme::Theme;
use crate::timeline::{TimelineState, TimelineWidget};
use crate::ui::widgets::TextInputState;

use super::placeholder::input_placeholder;

/// Fixed height for the input area (in lines).
const INPUT_HEIGHT: u16 = 3;

/// Height for the divider line.
const DIVIDER_HEIGHT: u16 = 1;

/// Conversation pane widget combining timeline and input.
///
/// ```text
/// ┌─ Conversation ──────────────────────┐
/// │                                      │
/// │  [SpecEvent] User: I want to build  │
/// │              a CLI that converts... │
/// │                                      │
/// │  [SpecEvent] claude: Here's a       │
/// │              draft specification... │
/// │                                      │
/// ├──────────────────────────────────────┤
/// │ > Type your message...            │
/// └──────────────────────────────────────┘
/// ```
pub struct ConversationPane<'a> {
    timeline: &'a TimelineState,
    input: &'a TextInputState,
    phase: Option<PhaseKind>,
    theme: &'a Theme,
    focused: bool,
}

impl<'a> ConversationPane<'a> {
    /// Create a new conversation pane.
    pub fn new(
        timeline: &'a TimelineState,
        input: &'a TextInputState,
        theme: &'a Theme,
    ) -> Self {
        Self {
            timeline,
            input,
            phase: None,
            theme,
            focused: false,
        }
    }

    /// Set the current phase (affects placeholder text).
    #[must_use]
    pub fn phase(mut self, phase: Option<PhaseKind>) -> Self {
        self.phase = phase;
        self
    }

    /// Set whether this pane is focused.
    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Render the input area.
    fn render_input(&self, area: Rect, buf: &mut Buffer) {
        // Get placeholder based on phase
        let placeholder = input_placeholder(self.phase);

        // Build prompt prefix
        let prompt = "> ";
        let prompt_len = prompt.len();

        // Determine what to display
        if self.input.is_empty() {
            // Show placeholder or cursor
            let mut spans = vec![Span::styled(
                prompt,
                Style::default().fg(self.theme.primary),
            )];

            if self.focused {
                // Show cursor
                spans.push(Span::styled("_", Style::default().fg(self.theme.text)));
            } else {
                // Show placeholder
                spans.push(Span::styled(
                    placeholder,
                    Style::default().fg(self.theme.muted),
                ));
            }

            let line = Line::from(spans);
            Paragraph::new(vec![line]).render(area, buf);
        } else {
            // Show content with cursor
            let content = self.input.content();
            let cursor_pos = self.input.cursor;

            let mut lines: Vec<Line<'_>> = Vec::new();
            let mut current_line_spans: Vec<Span<'_>> = Vec::new();
            let mut is_first_line = true;
            let mut cursor_drawn = false;

            // Add prompt to first line
            current_line_spans.push(Span::styled(
                prompt.to_string(),
                Style::default().fg(self.theme.primary),
            ));

            for (char_count, ch) in content.chars().enumerate() {
                // Insert cursor before this character if at position
                if self.focused && char_count == cursor_pos && !cursor_drawn {
                    current_line_spans.push(Span::styled(
                        "|",
                        Style::default().fg(self.theme.text),
                    ));
                    cursor_drawn = true;
                }

                if ch == '\n' {
                    // End current line
                    lines.push(Line::from(current_line_spans));
                    current_line_spans = Vec::new();

                    // Continuation lines get indentation
                    if is_first_line {
                        is_first_line = false;
                    }
                    current_line_spans.push(Span::raw(" ".repeat(prompt_len)));
                } else {
                    current_line_spans.push(Span::styled(
                        ch.to_string(),
                        Style::default().fg(self.theme.text),
                    ));
                }
            }

            // Cursor at the end
            if self.focused && !cursor_drawn {
                current_line_spans.push(Span::styled(
                    "_",
                    Style::default().fg(self.theme.text),
                ));
            }

            // Add remaining content
            if !current_line_spans.is_empty() {
                lines.push(Line::from(current_line_spans));
            }

            Paragraph::new(lines).render(area, buf);
        }
    }

    /// Render a horizontal divider line.
    fn render_divider(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 {
            return;
        }

        // Use box drawing characters for a clean divider
        let divider_char = line::HORIZONTAL;
        let divider_str = divider_char.repeat(area.width as usize);

        let line = Line::from(Span::styled(
            divider_str,
            Style::default().fg(self.theme.border),
        ));

        Paragraph::new(vec![line]).render(area, buf);
    }
}

impl Widget for ConversationPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render outer border
        let border_style = if self.focused {
            Style::default().fg(self.theme.border_focused)
        } else {
            Style::default().fg(self.theme.border)
        };

        let block = Block::default()
            .title(" Conversation ")
            .title_style(Style::default().fg(self.theme.text))
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::default().bg(self.theme.base));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < INPUT_HEIGHT + DIVIDER_HEIGHT + 1 {
            // Not enough space - just show input
            self.render_input(inner, buf);
            return;
        }

        // Calculate layout:
        // - Timeline takes all space except input and divider
        // - Divider is 1 line
        // - Input is INPUT_HEIGHT lines
        let timeline_height = inner.height.saturating_sub(INPUT_HEIGHT + DIVIDER_HEIGHT);
        let divider_y = inner.y + timeline_height;
        let input_y = divider_y + DIVIDER_HEIGHT;

        let timeline_area = Rect::new(inner.x, inner.y, inner.width, timeline_height);
        let divider_area = Rect::new(inner.x, divider_y, inner.width, DIVIDER_HEIGHT);
        let input_area = Rect::new(inner.x, input_y, inner.width, INPUT_HEIGHT);

        // Render timeline (without its own border)
        let timeline_widget = TimelineWidget::new(self.timeline, self.theme)
            .with_border(false)
            .focused(self.focused);
        timeline_widget.render(timeline_area, buf);

        // Render divider
        self.render_divider(divider_area, buf);

        // Render input
        self.render_input(input_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn create_test_terminal(width: u16, height: u16) -> Terminal<TestBackend> {
        let backend = TestBackend::new(width, height);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn test_conversation_pane_creation() {
        let timeline = TimelineState::new();
        let input = TextInputState::new();
        let theme = Theme::default();

        let pane = ConversationPane::new(&timeline, &input, &theme)
            .phase(Some(PhaseKind::Drafting))
            .focused(true);

        assert!(pane.focused);
        assert_eq!(pane.phase, Some(PhaseKind::Drafting));
    }

    #[test]
    fn test_conversation_pane_renders() {
        let timeline = TimelineState::new();
        let input = TextInputState::new();
        let theme = Theme::default();

        let mut terminal = create_test_terminal(60, 20);

        terminal
            .draw(|frame| {
                let pane = ConversationPane::new(&timeline, &input, &theme);
                frame.render_widget(pane, frame.area());
            })
            .unwrap();

        // Check that the title is rendered by collecting buffer content
        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
        assert!(
            content.contains("Conversation"),
            "Conversation title should be rendered"
        );
    }

    #[test]
    fn test_conversation_pane_placeholder_changes_with_phase() {
        // This test verifies that different phases produce different placeholders
        // by checking the input_placeholder function
        assert_ne!(
            input_placeholder(Some(PhaseKind::Drafting)),
            input_placeholder(Some(PhaseKind::Running))
        );
    }

    #[test]
    fn test_conversation_pane_minimum_size() {
        let timeline = TimelineState::new();
        let input = TextInputState::new();
        let theme = Theme::default();

        // Very small terminal - should not panic
        let mut terminal = create_test_terminal(20, 5);

        terminal
            .draw(|frame| {
                let pane = ConversationPane::new(&timeline, &input, &theme);
                frame.render_widget(pane, frame.area());
            })
            .unwrap();

        // Should complete without panic
    }
}
