//! Status bar widget for the top of the TUI.
//!
//! Format: `● Phase │ "Title" │ claude ● gemini ○ codex ○ │ file:line │ metric │ → hint`
//!
//! On narrow terminals (< 60 chars), model indicators collapse to: `2/3 models`

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::models::{ModelState, ModelStatus, ModelsSummary};
use crate::theme::Theme;

/// Width threshold below which model indicators collapse to summary format.
const NARROW_THRESHOLD: u16 = 60;

/// Status bar content.
#[derive(Debug, Clone, Default)]
pub struct StatusBarContent {
    /// Current phase (e.g., "Drafting", "Running").
    pub phase: String,
    /// Thread title.
    pub title: String,
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
            file: None,
            metric: None,
            hint: Some("Resize to at least 40x12".into()),
        }
    }

    /// Create a "checking models" status.
    pub fn checking_models() -> Self {
        Self {
            phase: "Starting".into(),
            title: "Checking models...".into(),
            file: None,
            metric: None,
            hint: None,
        }
    }
}

/// Status bar widget.
pub struct StatusBar<'a> {
    content: &'a StatusBarContent,
    models: &'a [ModelStatus],
    theme: &'a Theme,
    ascii_mode: bool,
}

impl<'a> StatusBar<'a> {
    /// Create a new status bar widget.
    pub fn new(content: &'a StatusBarContent, models: &'a [ModelStatus], theme: &'a Theme) -> Self {
        Self {
            content,
            models,
            theme,
            ascii_mode: false,
        }
    }

    /// Set ASCII mode for `NO_COLOR` environments.
    #[must_use]
    pub fn ascii_mode(mut self, ascii: bool) -> Self {
        self.ascii_mode = ascii;
        self
    }

    /// Get the color for a model state.
    fn state_color(&self, state: ModelState) -> ratatui::style::Color {
        match state {
            ModelState::Ready => self.theme.success,
            ModelState::Cooldown(_) => self.theme.warning,
            ModelState::Unavailable => self.theme.muted,
            ModelState::Probing => self.theme.info,
        }
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

        // Add model indicators
        if !self.models.is_empty() {
            spans.push(Span::styled(" │ ", Style::default().fg(self.theme.muted)));

            // Use narrow format on small terminals
            if area.width < NARROW_THRESHOLD {
                let summary = ModelsSummary::from_models(self.models);
                spans.push(Span::styled(
                    summary.narrow_format(),
                    Style::default().fg(self.theme.subtext),
                ));
            } else {
                // Show individual model indicators
                for (i, model) in self.models.iter().enumerate() {
                    if i > 0 {
                        spans.push(Span::raw(" "));
                    }
                    spans.push(Span::styled(
                        &model.name,
                        Style::default().fg(self.theme.subtext),
                    ));
                    spans.push(Span::raw(" "));
                    let indicator = model.indicator(self.ascii_mode);
                    let color = self.state_color(model.state);
                    spans.push(Span::styled(indicator, Style::default().fg(color)));
                }
            }
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
    }

    #[test]
    fn test_too_small_content() {
        let content = StatusBarContent::too_small();
        assert_eq!(content.phase, "Warning");
        assert!(content.title.contains("too small"));
    }

    #[test]
    fn test_checking_models_content() {
        let content = StatusBarContent::checking_models();
        assert_eq!(content.phase, "Starting");
        assert!(content.title.contains("Checking"));
    }

    #[test]
    fn test_status_bar_ascii_mode() {
        let content = StatusBarContent::placeholder();
        let theme = Theme::default();
        let models: Vec<ModelStatus> = vec![];
        let bar = StatusBar::new(&content, &models, &theme).ascii_mode(true);

        assert!(bar.ascii_mode);
    }

    #[test]
    fn test_state_colors() {
        let content = StatusBarContent::placeholder();
        let theme = Theme::default();
        let models: Vec<ModelStatus> = vec![];
        let bar = StatusBar::new(&content, &models, &theme);

        // Ready should use success color
        assert_eq!(bar.state_color(ModelState::Ready), theme.success);
        // Unavailable should use muted color
        assert_eq!(bar.state_color(ModelState::Unavailable), theme.muted);
    }
}
