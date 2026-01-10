//! Markdown styling configuration.
//!
//! Provides [`MarkdownStyles`] which maps markdown elements to ratatui styles.

use ratatui::style::{Modifier, Style};

use crate::theme::Theme;

/// Styles for rendering markdown elements.
#[derive(Debug, Clone)]
pub struct MarkdownStyles {
    /// H1 header style.
    pub h1: Style,
    /// H2 header style.
    pub h2: Style,
    /// H3+ header style.
    pub h3: Style,
    /// Inline code style.
    pub code: Style,
    /// Code block line style.
    pub code_block: Style,
    /// Emphasis (italic) style.
    pub emphasis: Style,
    /// Strong (bold) style.
    pub strong: Style,
    /// List marker (bullet/number) style.
    pub list_marker: Style,
    /// Link text style.
    pub link: Style,
    /// Blockquote style.
    pub blockquote: Style,
    /// Normal text style.
    pub text: Style,
    /// Strikethrough style.
    pub strikethrough: Style,
}

impl MarkdownStyles {
    /// Create styles from a theme.
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            h1: Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
            h2: Style::default()
                .fg(theme.text)
                .add_modifier(Modifier::BOLD),
            h3: Style::default()
                .fg(theme.subtext)
                .add_modifier(Modifier::BOLD),
            code: Style::default()
                .fg(theme.secondary)
                .bg(theme.surface),
            code_block: Style::default()
                .fg(theme.secondary)
                .bg(theme.surface),
            emphasis: Style::default()
                .add_modifier(Modifier::ITALIC),
            strong: Style::default()
                .add_modifier(Modifier::BOLD),
            list_marker: Style::default()
                .fg(theme.muted),
            link: Style::default()
                .fg(theme.info)
                .add_modifier(Modifier::UNDERLINED),
            blockquote: Style::default()
                .fg(theme.subtext)
                .add_modifier(Modifier::ITALIC),
            text: Style::default()
                .fg(theme.text),
            strikethrough: Style::default()
                .add_modifier(Modifier::CROSSED_OUT),
        }
    }
}

impl Default for MarkdownStyles {
    fn default() -> Self {
        Self::from_theme(&Theme::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_theme() {
        let theme = Theme::default();
        let styles = MarkdownStyles::from_theme(&theme);

        // Verify h1 has bold modifier
        assert!(styles.h1.add_modifier.contains(Modifier::BOLD));
        // Verify emphasis has italic
        assert!(styles.emphasis.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn test_default() {
        let styles = MarkdownStyles::default();
        assert!(styles.strong.add_modifier.contains(Modifier::BOLD));
    }
}
