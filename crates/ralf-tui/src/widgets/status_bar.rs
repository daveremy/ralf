//! Status bar widget for the top of the TUI.
//!
//! Format: `● Phase │ "Title" │ model │ file:line │ metric │ → hint`

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::theme::Theme;

/// Status bar content.
#[derive(Debug, Clone, Default)]
pub struct StatusBarContent {
    /// Current phase (e.g., "Drafting", "Running").
    pub phase: String,
    /// Thread title.
    pub title: String,
    /// Current model (e.g., "claude", "gemini").
    pub model: Option<String>,
    /// Current file being edited (e.g., "src/main.rs:47").
    pub file: Option<String>,
    /// Progress metric (e.g., "2/5 criteria").
    pub metric: Option<String>,
    /// Next action hint (e.g., "→ Press Enter to send").
    pub hint: Option<String>,
}

impl StatusBarContent {
    /// Create placeholder content for M5-A testing.
    pub fn placeholder() -> Self {
        Self {
            phase: "Drafting".into(),
            title: "New Thread".into(),
            model: Some("claude".into()),
            file: None,
            metric: None,
            hint: None,
        }
    }

    /// Create a "terminal too small" warning.
    pub fn too_small() -> Self {
        Self {
            phase: "Warning".into(),
            title: "Terminal too small".into(),
            model: None,
            file: None,
            metric: None,
            hint: Some("Resize to at least 40x12".into()),
        }
    }
}

/// Status bar widget.
pub struct StatusBar<'a> {
    content: &'a StatusBarContent,
    theme: &'a Theme,
}

impl<'a> StatusBar<'a> {
    /// Create a new status bar widget.
    pub fn new(content: &'a StatusBarContent, theme: &'a Theme) -> Self {
        Self { content, theme }
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut spans = vec![
            Span::styled("● ", Style::default().fg(self.theme.primary)),
            Span::styled(&self.content.phase, Style::default().fg(self.theme.text)),
            Span::styled(" │ ", Style::default().fg(self.theme.muted)),
            Span::styled(
                format!("\"{}\"", self.content.title),
                Style::default().fg(self.theme.text),
            ),
        ];

        // Add optional model
        if let Some(ref model) = self.content.model {
            spans.push(Span::styled(" │ ", Style::default().fg(self.theme.muted)));
            spans.push(Span::styled(model, Style::default().fg(self.theme.subtext)));
        }

        // Add optional file
        if let Some(ref file) = self.content.file {
            spans.push(Span::styled(" │ ", Style::default().fg(self.theme.muted)));
            spans.push(Span::styled(file, Style::default().fg(self.theme.subtext)));
        }

        // Add optional metric
        if let Some(ref metric) = self.content.metric {
            spans.push(Span::styled(" │ ", Style::default().fg(self.theme.muted)));
            spans.push(Span::styled(metric, Style::default().fg(self.theme.info)));
        }

        // Add optional hint
        if let Some(ref hint) = self.content.hint {
            spans.push(Span::styled(" │ ", Style::default().fg(self.theme.muted)));
            spans.push(Span::styled(
                format!("→ {hint}"),
                Style::default().fg(self.theme.secondary),
            ));
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
    fn test_placeholder_content() {
        let content = StatusBarContent::placeholder();
        assert_eq!(content.phase, "Drafting");
        assert_eq!(content.title, "New Thread");
        assert_eq!(content.model, Some("claude".into()));
    }

    #[test]
    fn test_too_small_content() {
        let content = StatusBarContent::too_small();
        assert_eq!(content.phase, "Warning");
        assert!(content.title.contains("too small"));
    }
}
