//! Full-width input bar widget.
//!
//! Always visible at the bottom of the screen for text entry.
//! Supports multi-line input with Ctrl+J for newlines.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::theme::Theme;
use crate::ui::widgets::TextInputState;

/// Full-width input bar for text entry.
pub struct InputBar<'a> {
    input: &'a TextInputState,
    theme: &'a Theme,
    focused: bool,
    loading: bool,
    loading_model: Option<&'a str>,
}

impl<'a> InputBar<'a> {
    /// Create a new input bar widget.
    pub fn new(input: &'a TextInputState, theme: &'a Theme) -> Self {
        Self {
            input,
            theme,
            focused: false,
            loading: false,
            loading_model: None,
        }
    }

    /// Set whether the input bar is focused.
    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Set loading state with optional model name.
    #[must_use]
    pub fn loading(mut self, loading: bool, model: Option<&'a str>) -> Self {
        self.loading = loading;
        self.loading_model = model;
        self
    }

    /// Build Lines for multi-line input display.
    /// Returns the lines to display and which line index contains the cursor.
    fn build_input_lines(&self) -> (Vec<Line<'static>>, usize) {
        let content = self.input.content();
        let cursor_pos = self.input.cursor;

        // Split content into lines
        let text_lines: Vec<&str> = if content.is_empty() {
            vec![""]
        } else {
            content.split('\n').collect()
        };

        // Find which line the cursor is on
        let mut char_count = 0;
        let mut cursor_line = 0;
        let mut cursor_col = 0;

        for (line_idx, line) in text_lines.iter().enumerate() {
            let line_len = line.chars().count();
            if cursor_pos <= char_count + line_len {
                cursor_line = line_idx;
                cursor_col = cursor_pos - char_count;
                break;
            }
            // +1 for the newline character
            char_count += line_len + 1;
            cursor_line = line_idx;
            cursor_col = 0; // Will be at start of next line
        }

        // Build display lines
        let mut lines = Vec::with_capacity(text_lines.len());

        for (line_idx, line_text) in text_lines.iter().enumerate() {
            let prefix = if line_idx == 0 { "> " } else { "  " };

            if self.focused && line_idx == cursor_line {
                // This line has the cursor - insert cursor block
                let mut spans = vec![Span::raw(prefix.to_string())];
                let chars: Vec<char> = line_text.chars().collect();

                if cursor_col < chars.len() {
                    // Cursor in middle of line
                    let before: String = chars[..cursor_col].iter().collect();
                    let after: String = chars[cursor_col..].iter().collect();
                    spans.push(Span::raw(before));
                    spans.push(Span::raw("█"));
                    spans.push(Span::raw(after));
                } else {
                    // Cursor at end of line
                    spans.push(Span::raw(line_text.to_string()));
                    spans.push(Span::raw("█"));
                }
                lines.push(Line::from(spans));
            } else {
                // Normal line without cursor
                let display = if line_idx == 0 && line_text.is_empty() && !self.focused {
                    format!("{prefix}_")
                } else {
                    format!("{prefix}{line_text}")
                };
                lines.push(Line::from(display));
            }
        }

        (lines, cursor_line)
    }
}

#[allow(clippy::cast_possible_truncation)]
impl Widget for InputBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Border style based on focus
        let border_style = if self.focused {
            Style::default().fg(self.theme.border_focused)
        } else {
            Style::default().fg(self.theme.border)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        // Calculate inner height (area minus borders)
        let inner_height = area.height.saturating_sub(2) as usize;

        // Build content - show loading indicator or normal input
        let paragraph = if self.loading {
            let model = self.loading_model.unwrap_or("model");
            let display = format!("● Waiting for {model}...");
            Paragraph::new(display)
                .block(block)
                .style(Style::default().fg(self.theme.muted))
        } else {
            let (lines, cursor_line) = self.build_input_lines();

            // Calculate scroll offset to keep cursor visible
            let scroll_offset = if lines.len() <= inner_height {
                0
            } else {
                // Scroll so cursor line is visible (show it at bottom if needed)
                cursor_line.saturating_sub(inner_height.saturating_sub(1))
            };

            Paragraph::new(lines)
                .block(block)
                .style(Style::default().fg(self.theme.text))
                .scroll((scroll_offset as u16, 0))
        };

        paragraph.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_bar_creation() {
        let input = TextInputState::new();
        let theme = Theme::default();
        let bar = InputBar::new(&input, &theme).focused(true);
        assert!(bar.focused);
    }
}
