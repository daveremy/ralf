//! Generic pane widget with border and optional title.
//!
//! Supports focused/unfocused states with different border styles.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::theme::{BorderSet, Theme};

/// Generic pane with border and optional title.
pub struct Pane<'a> {
    title: Option<&'a str>,
    focused: bool,
    content: Option<&'a str>,
    theme: &'a Theme,
    borders: &'a BorderSet,
}

impl<'a> Pane<'a> {
    /// Create a new pane widget.
    pub fn new(theme: &'a Theme, borders: &'a BorderSet) -> Self {
        Self {
            title: None,
            focused: false,
            content: None,
            theme,
            borders,
        }
    }

    /// Set the pane title.
    #[must_use]
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// Set whether the pane is focused.
    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Set the pane content (placeholder text for M5-A).
    #[must_use]
    pub fn content(mut self, content: &'a str) -> Self {
        self.content = Some(content);
        self
    }
}

impl Widget for Pane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Select border set based on focus
        let border_set = if self.focused {
            self.borders.focused()
        } else {
            self.borders.normal()
        };

        let border_style = if self.focused {
            Style::default().fg(self.theme.border_focused)
        } else {
            Style::default().fg(self.theme.border)
        };

        let title_style = if self.focused {
            Style::default().fg(self.theme.primary)
        } else {
            Style::default().fg(self.theme.subtext)
        };

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_set(border_set)
            .border_style(border_style);

        if let Some(title) = self.title {
            block = block.title(title).title_style(title_style);
        }

        let inner = block.inner(area);
        block.render(area, buf);

        // Render placeholder content
        if let Some(text) = self.content {
            let paragraph =
                Paragraph::new(text).style(Style::default().fg(self.theme.subtext));
            paragraph.render(inner, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::IconMode;

    #[test]
    fn test_pane_builder() {
        let theme = Theme::default();
        let borders = BorderSet::new(IconMode::Unicode);

        let pane = Pane::new(&theme, &borders)
            .title("Test Pane")
            .focused(true)
            .content("Hello, world!");

        assert_eq!(pane.title, Some("Test Pane"));
        assert!(pane.focused);
        assert_eq!(pane.content, Some("Hello, world!"));
    }
}
