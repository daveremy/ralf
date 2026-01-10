//! Full-width input bar widget.
//!
//! Always visible at the bottom of the screen for text entry.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
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

    /// Build the display string for normal (non-loading) input.
    fn build_input_display(&self) -> String {
        let content = self.input.content();
        let cursor_pos = self.input.cursor;
        let mut display = String::with_capacity(content.len() + 4);
        display.push_str("> ");

        if self.focused {
            // Insert characters with cursor block at cursor position
            for (i, ch) in content.chars().enumerate() {
                if i == cursor_pos {
                    display.push('█');
                }
                display.push(ch);
            }
            // Cursor at end if we haven't drawn it yet
            if cursor_pos >= content.chars().count() {
                display.push('█');
            }
        } else {
            display.push_str(content);
            if content.is_empty() {
                display.push('_');
            }
        }

        display
    }
}

impl Widget for InputBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Border style based on focus
        let border_style = if self.focused {
            Style::default().fg(self.theme.border_focused)
        } else {
            Style::default().fg(self.theme.border)
        };

        // Build input line - show loading indicator or normal input
        let display = if self.loading {
            let model = self.loading_model.unwrap_or("model");
            format!("● Waiting for {model}...")
        } else {
            self.build_input_display()
        };

        // Text style - dimmed when loading
        let text_style = if self.loading {
            Style::default().fg(self.theme.muted)
        } else {
            Style::default().fg(self.theme.text)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        let paragraph = Paragraph::new(display)
            .block(block)
            .style(text_style);

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
