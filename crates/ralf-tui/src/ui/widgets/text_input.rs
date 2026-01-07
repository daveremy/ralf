//! Multi-line text input widget.

use crate::ui::theme::Styles;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
};

/// A multi-line text input widget.
#[derive(Debug, Clone)]
pub struct TextInput<'a> {
    /// The text content.
    content: String,
    /// Cursor position (character index).
    cursor: usize,
    /// Optional block for borders/title.
    block: Option<Block<'a>>,
    /// Whether the input is focused.
    focused: bool,
    /// Placeholder text.
    placeholder: Option<&'a str>,
    /// Prompt prefix (e.g., "> ").
    prompt: &'a str,
}

impl<'a> TextInput<'a> {
    /// Create a new text input.
    pub fn new(content: impl Into<String>) -> Self {
        let content = content.into();
        let cursor = content.len();
        Self {
            content,
            cursor,
            block: None,
            focused: true,
            placeholder: None,
            prompt: "> ",
        }
    }

    /// Set the block for the text input.
    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set focus state.
    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Set placeholder text.
    #[must_use]
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = Some(placeholder);
        self
    }
}

impl Widget for TextInput<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner = if let Some(block) = &self.block {
            let inner = block.inner(area);
            block.clone().render(area, buf);
            inner
        } else {
            area
        };

        if inner.height < 1 || inner.width < 1 {
            return;
        }

        let prompt_len = self.prompt.len();

        // Show placeholder if empty
        if self.content.is_empty() {
            let mut spans = vec![Span::styled(self.prompt, Styles::active())];
            if self.focused {
                // Show cursor after prompt
                spans.push(Span::styled("_", Styles::active()));
                if let Some(placeholder) = self.placeholder {
                    spans.push(Span::styled(placeholder, Styles::dim()));
                }
            } else if let Some(placeholder) = self.placeholder {
                spans.push(Span::styled(placeholder, Styles::dim()));
            }
            let line = Line::from(spans);
            Paragraph::new(vec![line]).render(inner, buf);
            return;
        }

        // Render content with cursor
        let mut lines = Vec::new();
        let mut current_line = self.prompt.to_string(); // Start first line with prompt
        let mut cursor_drawn = false;
        let mut char_count = 0;
        let mut is_first_line = true;

        for ch in self.content.chars() {
            if ch == '\n' {
                // Check if cursor is at end of this line
                if self.focused && char_count == self.cursor && !cursor_drawn {
                    current_line.push('_');
                    cursor_drawn = true;
                }
                lines.push(Line::from(current_line.clone()));
                current_line.clear();
                // Continuation lines get indentation matching prompt length
                if is_first_line {
                    is_first_line = false;
                }
                current_line.push_str(&" ".repeat(prompt_len));
            } else {
                // Insert cursor before this character if position matches
                if self.focused && char_count == self.cursor && !cursor_drawn {
                    current_line.push('|');
                    cursor_drawn = true;
                }
                current_line.push(ch);
            }
            char_count += 1;
        }

        // Cursor at the end
        if self.focused && !cursor_drawn {
            current_line.push('_');
        }

        if !current_line.is_empty() || lines.is_empty() {
            lines.push(Line::from(current_line));
        }

        let paragraph = Paragraph::new(lines).style(Styles::default());
        paragraph.render(inner, buf);
    }
}

/// State for a text input, managing content and cursor position.
#[derive(Debug, Clone, Default)]
pub struct TextInputState {
    /// The text content.
    pub content: String,
    /// Cursor position (character index).
    pub cursor: usize,
    /// Input history for up/down navigation.
    history: Vec<String>,
    /// Current history index (-1 = current input).
    history_index: isize,
    /// Saved current input when navigating history.
    saved_input: String,
}

impl TextInputState {
    /// Create a new empty text input state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the current content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Check if the content is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Clear the content.
    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
    }

    /// Take the content, clearing the state.
    pub fn take(&mut self) -> String {
        let content = std::mem::take(&mut self.content);
        self.cursor = 0;
        content
    }

    /// Insert a character at the cursor position.
    pub fn insert(&mut self, ch: char) {
        // Handle newline
        if ch == '\n' {
            self.content.insert(self.cursor, ch);
            self.cursor += 1;
            return;
        }

        self.content.insert(self.cursor, ch);
        self.cursor += 1;
    }

    /// Insert a string at the cursor position.
    pub fn insert_str(&mut self, s: &str) {
        self.content.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    /// Delete the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.content.remove(self.cursor);
        }
    }

    /// Delete the character at the cursor (delete).
    pub fn delete(&mut self) {
        if self.cursor < self.content.len() {
            self.content.remove(self.cursor);
        }
    }

    /// Move cursor left.
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor right.
    pub fn move_right(&mut self) {
        if self.cursor < self.content.len() {
            self.cursor += 1;
        }
    }

    /// Move cursor to start.
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end.
    pub fn move_end(&mut self) {
        self.cursor = self.content.len();
    }

    /// Add current content to history and clear.
    pub fn submit(&mut self) -> String {
        let content = self.take();
        if !content.trim().is_empty() {
            self.history.push(content.clone());
        }
        self.history_index = -1;
        self.saved_input.clear();
        content
    }

    /// Navigate to previous history entry.
    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }

        // Save current input if at the bottom
        if self.history_index == -1 {
            self.saved_input = self.content.clone();
        }

        // Move up in history
        let new_index = self.history_index + 1;
        #[allow(clippy::cast_sign_loss)]
        if (new_index as usize) < self.history.len() {
            self.history_index = new_index;
            #[allow(clippy::cast_sign_loss)]
            {
                self.content = self.history[self.history.len() - 1 - new_index as usize].clone();
            }
            self.cursor = self.content.len();
        }
    }

    /// Navigate to next history entry.
    pub fn history_next(&mut self) {
        if self.history_index <= 0 {
            // Restore saved input
            if self.history_index == 0 {
                self.content = std::mem::take(&mut self.saved_input);
                self.cursor = self.content.len();
            }
            self.history_index = -1;
            return;
        }

        self.history_index -= 1;
        #[allow(clippy::cast_sign_loss)]
        {
            self.content =
                self.history[self.history.len() - 1 - self.history_index as usize].clone();
        }
        self.cursor = self.content.len();
    }

    /// Create a widget from this state.
    pub fn widget(&self) -> TextInput<'_> {
        let mut input = TextInput::new(self.content.clone());
        input.cursor = self.cursor;
        input
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_input_state_basic() {
        let mut state = TextInputState::new();
        assert!(state.is_empty());

        state.insert('H');
        state.insert('i');
        assert_eq!(state.content(), "Hi");
        assert_eq!(state.cursor, 2);

        state.backspace();
        assert_eq!(state.content(), "H");

        state.clear();
        assert!(state.is_empty());
    }

    #[test]
    fn test_text_input_state_cursor_movement() {
        let mut state = TextInputState::new();
        state.insert_str("Hello");

        state.move_left();
        state.move_left();
        assert_eq!(state.cursor, 3);

        state.insert('X');
        assert_eq!(state.content(), "HelXlo");

        state.move_home();
        assert_eq!(state.cursor, 0);

        state.move_end();
        assert_eq!(state.cursor, 6);
    }

    #[test]
    fn test_text_input_state_history() {
        let mut state = TextInputState::new();

        state.insert_str("first");
        state.submit();
        assert!(state.is_empty());

        state.insert_str("second");
        state.submit();

        state.history_prev();
        assert_eq!(state.content(), "second");

        state.history_prev();
        assert_eq!(state.content(), "first");

        state.history_next();
        assert_eq!(state.content(), "second");
    }
}
