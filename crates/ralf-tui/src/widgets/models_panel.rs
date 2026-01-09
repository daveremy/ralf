//! Models panel widget for displaying model status.
//!
//! Shows model status when no thread is loaded (initial state):
//! ```text
//! ┏ Models ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
//! ┃                                                      ┃
//! ┃  claude    ● Ready         v1.2.3                    ┃
//! ┃  gemini    ○ Not found     Install: gemini.google... ┃
//! ┃  codex     ○ Auth needed   Run: codex auth login     ┃
//! ┃                                                      ┃
//! ┃  [r] Refresh                                         ┃
//! ┃                                                      ┃
//! ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
//! ```

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::models::{ModelState, ModelStatus};
use crate::theme::Theme;

/// Models panel widget.
pub struct ModelsPanel<'a> {
    models: &'a [ModelStatus],
    theme: &'a Theme,
    ascii_mode: bool,
    focused: bool,
}

impl<'a> ModelsPanel<'a> {
    /// Create a new models panel.
    pub fn new(models: &'a [ModelStatus], theme: &'a Theme) -> Self {
        Self {
            models,
            theme,
            ascii_mode: false,
            focused: false,
        }
    }

    /// Set ASCII mode for `NO_COLOR` environments.
    #[must_use]
    pub fn ascii_mode(mut self, ascii: bool) -> Self {
        self.ascii_mode = ascii;
        self
    }

    /// Set whether this panel is focused.
    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
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

impl Widget for ModelsPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Create border with title (highlight border when focused)
        let border_color = if self.focused {
            self.theme.primary
        } else {
            self.theme.border
        };
        let block = Block::default()
            .title(" Models ")
            .title_style(Style::default().fg(self.theme.text))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(self.theme.base));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 3 {
            return;
        }

        // Build lines for each model
        let mut lines: Vec<Line<'_>> = Vec::new();

        // Empty line at top for spacing
        lines.push(Line::from(""));

        for model in self.models {
            let indicator = model.indicator(self.ascii_mode);
            let color = self.state_color(model.state);

            let mut spans = vec![
                Span::raw("  "),
                Span::styled(
                    format!("{:<10}", model.name),
                    Style::default().fg(self.theme.text),
                ),
                Span::styled(format!("{indicator} "), Style::default().fg(color)),
            ];

            // Add status message
            if let Some(ref msg) = model.message {
                // Truncate message if needed
                let max_msg_len = inner.width.saturating_sub(20) as usize;
                let display_msg = if msg.len() > max_msg_len && max_msg_len > 3 {
                    format!("{}...", &msg[..max_msg_len.saturating_sub(3)])
                } else {
                    msg.clone()
                };
                spans.push(Span::styled(display_msg, Style::default().fg(self.theme.subtext)));
            }

            // Add version if available and there's room
            if let Some(ref version) = model.version {
                if model.state == ModelState::Ready {
                    spans.push(Span::styled(
                        format!("  v{version}"),
                        Style::default().fg(self.theme.muted),
                    ));
                }
            }

            lines.push(Line::from(spans));
        }

        // Empty line before footer
        lines.push(Line::from(""));

        // Footer hint
        let footer_spans = vec![
            Span::raw("  "),
            Span::styled("[", Style::default().fg(self.theme.muted)),
            Span::styled("r", Style::default().fg(self.theme.primary)),
            Span::styled("] ", Style::default().fg(self.theme.muted)),
            Span::styled("Refresh", Style::default().fg(self.theme.subtext)),
        ];
        lines.push(Line::from(footer_spans));

        // Render
        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ModelState;

    fn mock_ready_model(name: &str) -> ModelStatus {
        ModelStatus {
            name: name.to_string(),
            state: ModelState::Ready,
            version: Some("1.0.0".to_string()),
            message: Some("Ready".to_string()),
        }
    }

    fn mock_unavailable_model(name: &str, message: &str) -> ModelStatus {
        ModelStatus {
            name: name.to_string(),
            state: ModelState::Unavailable,
            version: None,
            message: Some(message.to_string()),
        }
    }

    #[test]
    fn test_models_panel_creation() {
        let theme = Theme::default();
        let models = vec![mock_ready_model("claude")];
        let panel = ModelsPanel::new(&models, &theme);

        assert!(!panel.ascii_mode);
    }

    #[test]
    fn test_models_panel_ascii_mode() {
        let theme = Theme::default();
        let models = vec![mock_ready_model("claude")];
        let panel = ModelsPanel::new(&models, &theme).ascii_mode(true);

        assert!(panel.ascii_mode);
    }

    #[test]
    fn test_state_colors() {
        let theme = Theme::default();
        let models: Vec<ModelStatus> = vec![];
        let panel = ModelsPanel::new(&models, &theme);

        // Ready should use success color
        assert_eq!(panel.state_color(ModelState::Ready), theme.success);
        // Unavailable should use muted color
        assert_eq!(panel.state_color(ModelState::Unavailable), theme.muted);
    }
}
