//! Footer hints widget for keybinding display.
//!
//! Format: `[Tab] Focus │ [1/2/3] Modes │ [?] Help │ [q] Quit`

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::theme::Theme;

/// A single keybinding hint.
#[derive(Debug, Clone)]
pub struct KeyHint {
    /// The key or key combination (e.g., "Tab", "Ctrl+Q").
    pub key: String,
    /// The action description (e.g., "Focus", "Quit").
    pub action: String,
}

impl KeyHint {
    /// Create a new key hint.
    pub fn new(key: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            action: action.into(),
        }
    }
}

/// Footer hints widget.
pub struct FooterHints<'a> {
    hints: &'a [KeyHint],
    theme: &'a Theme,
}

impl<'a> FooterHints<'a> {
    /// Create a new footer hints widget.
    pub fn new(hints: &'a [KeyHint], theme: &'a Theme) -> Self {
        Self { hints, theme }
    }

    /// Get the default hints for the shell (always includes help and quit).
    pub fn default_hints() -> Vec<KeyHint> {
        vec![
            KeyHint::new("Tab", "Focus"),
            KeyHint::new("1/2/3", "Modes"),
            KeyHint::new("?", "Help"),
            KeyHint::new("q", "Quit"),
        ]
    }
}

impl Widget for FooterHints<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut spans = Vec::new();

        for (i, hint) in self.hints.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" │ ", Style::default().fg(self.theme.muted)));
            }

            // Key in brackets
            spans.push(Span::styled("[", Style::default().fg(self.theme.muted)));
            spans.push(Span::styled(&hint.key, Style::default().fg(self.theme.primary)));
            spans.push(Span::styled("] ", Style::default().fg(self.theme.muted)));

            // Action
            spans.push(Span::styled(&hint.action, Style::default().fg(self.theme.subtext)));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line).style(Style::default().bg(self.theme.surface));
        paragraph.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_hint_creation() {
        let hint = KeyHint::new("Tab", "Focus");
        assert_eq!(hint.key, "Tab");
        assert_eq!(hint.action, "Focus");
    }

    #[test]
    fn test_default_hints() {
        let hints = FooterHints::default_hints();
        assert_eq!(hints.len(), 4);

        // Should include help and quit
        assert!(hints.iter().any(|h| h.key == "?" && h.action == "Help"));
        assert!(hints.iter().any(|h| h.key == "q" && h.action == "Quit"));
    }
}
