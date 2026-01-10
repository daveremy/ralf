//! Spec preview widget for the context pane.
//!
//! Renders the spec draft with markdown styling and phase indicator.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};

use crate::text::render_markdown;
use crate::theme::Theme;

/// Phase badge to display in the spec preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecPhase {
    /// Spec is being drafted.
    Drafting,
    /// AI is assessing the spec.
    Assessing,
    /// Spec is finalized and ready.
    Ready,
}

impl SpecPhase {
    /// Get the display label for this phase.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Drafting => "Drafting",
            Self::Assessing => "Assessing",
            Self::Ready => "Ready",
        }
    }
}

/// Spec preview widget that renders markdown content with a phase badge.
pub struct SpecPreview<'a> {
    /// The spec content to render.
    content: &'a str,
    /// Current phase.
    phase: SpecPhase,
    /// Theme for styling.
    theme: &'a Theme,
    /// Scroll offset (lines from top).
    scroll: u16,
    /// Whether this pane is focused.
    focused: bool,
}

impl<'a> SpecPreview<'a> {
    /// Create a new spec preview.
    pub fn new(content: &'a str, phase: SpecPhase, theme: &'a Theme) -> Self {
        Self {
            content,
            phase,
            theme,
            scroll: 0,
            focused: false,
        }
    }

    /// Set the scroll offset.
    #[must_use]
    pub fn scroll(mut self, scroll: u16) -> Self {
        self.scroll = scroll;
        self
    }

    /// Set whether this pane is focused.
    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Build styled lines from the spec content.
    fn build_lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Phase badge at the top
        lines.push(self.build_phase_badge());
        lines.push(Line::from("")); // Spacing

        // Empty content message
        if self.content.trim().is_empty() {
            lines.push(Line::from(Span::styled(
                "No spec content yet.",
                Style::default().fg(self.theme.muted),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Start chatting to develop your spec.",
                Style::default().fg(self.theme.muted),
            )));
            return lines;
        }

        // Render markdown content using shared renderer
        let markdown_lines = render_markdown(self.content, 80, self.theme);
        lines.extend(markdown_lines);

        lines
    }

    /// Build the phase badge line.
    fn build_phase_badge(&self) -> Line<'static> {
        let badge_color = match self.phase {
            SpecPhase::Drafting => self.theme.info,
            SpecPhase::Assessing => self.theme.warning,
            SpecPhase::Ready => self.theme.success,
        };

        Line::from(vec![
            Span::styled("[".to_string(), Style::default().fg(self.theme.muted)),
            Span::styled(
                self.phase.label().to_string(),
                Style::default().fg(badge_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled("]".to_string(), Style::default().fg(self.theme.muted)),
        ])
    }
}

impl Widget for SpecPreview<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines = self.build_lines();

        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));

        paragraph.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_theme() -> Theme {
        Theme::default()
    }

    #[test]
    fn test_spec_phase_labels() {
        assert_eq!(SpecPhase::Drafting.label(), "Drafting");
        assert_eq!(SpecPhase::Assessing.label(), "Assessing");
        assert_eq!(SpecPhase::Ready.label(), "Ready");
    }

    #[test]
    fn test_empty_content() {
        let theme = test_theme();
        let preview = SpecPreview::new("", SpecPhase::Drafting, &theme);
        let lines = preview.build_lines();

        // Should have badge + spacing + empty message
        assert!(lines.len() >= 3);
    }

    #[test]
    fn test_header_rendering() {
        let theme = test_theme();
        let preview = SpecPreview::new("# Title\n## Subtitle", SpecPhase::Drafting, &theme);
        let lines = preview.build_lines();

        // Badge + spacing + markdown content
        assert!(lines.len() >= 4);
    }

    #[test]
    fn test_code_block_rendering() {
        let theme = test_theme();
        let content = "```rust\nfn main() {}\n```";
        let preview = SpecPreview::new(content, SpecPhase::Drafting, &theme);
        let lines = preview.build_lines();

        // Should include code line
        assert!(lines.len() >= 3);
    }

    #[test]
    fn test_list_rendering() {
        let theme = test_theme();
        let content = "- Item 1\n- Item 2";
        let preview = SpecPreview::new(content, SpecPhase::Drafting, &theme);
        let lines = preview.build_lines();

        // Badge + spacing + list items
        assert!(lines.len() >= 4);
    }

    #[test]
    fn test_checkbox_rendering() {
        let theme = test_theme();
        let content = "- [ ] Unchecked\n- [x] Checked";
        let preview = SpecPreview::new(content, SpecPhase::Drafting, &theme);
        let lines = preview.build_lines();

        // Badge + spacing + checkboxes
        assert!(lines.len() >= 4);
    }

    #[test]
    fn test_scroll_offset() {
        let theme = test_theme();
        let preview = SpecPreview::new("Content", SpecPhase::Drafting, &theme).scroll(5);
        assert_eq!(preview.scroll, 5);
    }

    #[test]
    fn test_focused_state() {
        let theme = test_theme();
        let preview = SpecPreview::new("Content", SpecPhase::Drafting, &theme).focused(true);
        assert!(preview.focused);
    }
}
