//! Status bar widget.

use crate::ui::theme::{Palette, Styles};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::Widget,
};

/// A key hint for the status bar.
#[derive(Debug, Clone)]
pub struct KeyHint {
    pub key: &'static str,
    pub label: &'static str,
}

impl KeyHint {
    pub const fn new(key: &'static str, label: &'static str) -> Self {
        Self { key, label }
    }
}

/// Status bar widget displayed at the bottom of the screen.
#[derive(Debug, Clone)]
pub struct StatusBar<'a> {
    mode: &'a str,
    hints: Vec<KeyHint>,
    right_text: Option<&'a str>,
}

impl<'a> StatusBar<'a> {
    /// Create a new status bar.
    pub fn new(mode: &'a str) -> Self {
        Self {
            mode,
            hints: Vec::new(),
            right_text: None,
        }
    }

    /// Add key hints.
    #[must_use]
    pub fn hints(mut self, hints: Vec<KeyHint>) -> Self {
        self.hints = hints;
        self
    }

    /// Set right-aligned text.
    #[must_use]
    pub fn right(mut self, text: &'a str) -> Self {
        self.right_text = Some(text);
        self
    }
}

impl Widget for StatusBar<'_> {
    #[allow(clippy::cast_possible_truncation)]
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 {
            return;
        }

        // Fill background with status bar color
        for x in area.x..area.x.saturating_add(area.width) {
            buf[(x, area.y)].set_char(' ').set_bg(Palette::STATUS_BG);
        }

        // Build left side: mode + hints
        let mut spans = Vec::new();

        // Mode indicator (bright accent background)
        spans.push(Span::styled(
            format!(" {} ", self.mode),
            Styles::default().bg(Palette::ACCENT).fg(Palette::BG),
        ));
        spans.push(Span::styled(" ", Styles::status_bar()));

        // Key hints with high contrast
        for hint in &self.hints {
            spans.push(Span::styled(format!(" {} ", hint.key), Styles::key_hint()));
            spans.push(Span::styled(
                format!(" {} ", hint.label),
                Styles::key_label(),
            ));
        }

        let left_line = Line::from(spans);
        buf.set_line(area.x, area.y, &left_line, area.width);

        // Right-aligned text
        if let Some(text) = self.right_text {
            let text_len = text.len() as u16;
            if text_len < area.width {
                let x = area.x + area.width - text_len - 1;
                buf.set_string(x, area.y, text, Styles::status_bar());
            }
        }
    }
}
