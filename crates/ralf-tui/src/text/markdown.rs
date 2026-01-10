//! Markdown rendering using pulldown-cmark.
//!
//! Provides [`render_markdown`] to convert markdown text to styled ratatui Lines.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::theme::Theme;

use super::styles::MarkdownStyles;

/// Render markdown text to styled ratatui Lines.
///
/// # Arguments
/// * `input` - The markdown text to render
/// * `_width` - Available width for wrapping (not yet used, reserved for future)
/// * `theme` - Theme for styling
///
/// # Returns
/// A vector of styled Lines ready for rendering.
pub fn render_markdown(input: &str, _width: usize, theme: &Theme) -> Vec<Line<'static>> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(input, options);
    let styles = MarkdownStyles::from_theme(theme);

    let mut renderer = MarkdownRenderer::new(styles);
    renderer.run(parser);
    renderer.lines
}

/// Internal renderer that processes pulldown-cmark events.
struct MarkdownRenderer {
    /// Accumulated output lines.
    lines: Vec<Line<'static>>,
    /// Style configuration.
    styles: MarkdownStyles,
    /// Stack of active styles for nested formatting.
    style_stack: Vec<Style>,
    /// Current line being built.
    current_spans: Vec<Span<'static>>,
    /// Current indentation level (for nested lists).
    indent_level: usize,
    /// Whether we're inside a code block.
    in_code_block: bool,
    /// Whether we're inside a blockquote.
    in_blockquote: bool,
    /// Pending list marker to prepend to next text.
    pending_list_marker: Option<String>,
    /// Task list checkbox state (Some(checked) if in task item).
    task_checkbox: Option<bool>,
}

impl MarkdownRenderer {
    fn new(styles: MarkdownStyles) -> Self {
        Self {
            lines: Vec::new(),
            styles,
            style_stack: Vec::new(),
            current_spans: Vec::new(),
            indent_level: 0,
            in_code_block: false,
            in_blockquote: false,
            pending_list_marker: None,
            task_checkbox: None,
        }
    }

    fn run<'a>(&mut self, parser: impl Iterator<Item = Event<'a>>) {
        for event in parser {
            self.handle_event(event);
        }
        // Flush any remaining content
        self.flush_line();
    }

    #[allow(clippy::too_many_lines)]
    fn handle_event(&mut self, event: Event<'_>) {
        match event {
            // Headings
            Event::Start(Tag::Heading { level, .. }) => {
                self.flush_line();
                let style = self.heading_style(level);
                self.style_stack.push(style);
                // Text content will be styled by the style_stack
            }
            Event::End(TagEnd::Heading(_)) => {
                self.flush_line();
                self.style_stack.pop();
            }

            // Emphasis (italic)
            Event::Start(Tag::Emphasis) => {
                self.style_stack.push(self.styles.emphasis);
            }

            // Strong (bold)
            Event::Start(Tag::Strong) => {
                self.style_stack.push(self.styles.strong);
            }

            // Strikethrough
            Event::Start(Tag::Strikethrough) => {
                self.style_stack.push(self.styles.strikethrough);
            }

            // End inline formatting - all pop the style stack
            Event::End(TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link) => {
                self.style_stack.pop();
            }

            // Code blocks
            Event::Start(Tag::CodeBlock(_)) => {
                self.flush_line();
                self.in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                self.flush_line();
                self.in_code_block = false;
            }

            // Lists
            Event::Start(Tag::List(_)) => {
                self.flush_line();
                self.indent_level += 1;
            }
            Event::End(TagEnd::List(_)) => {
                self.indent_level = self.indent_level.saturating_sub(1);
            }

            // List items
            Event::Start(Tag::Item) => {
                self.flush_line();
                let indent = "  ".repeat(self.indent_level.saturating_sub(1));
                self.pending_list_marker = Some(format!("{indent}• "));
            }
            Event::End(TagEnd::Item) => {
                self.flush_line();
                self.task_checkbox = None;
            }

            // Task list markers
            Event::TaskListMarker(checked) => {
                self.task_checkbox = Some(checked);
            }

            // Blockquotes
            Event::Start(Tag::BlockQuote) => {
                self.flush_line();
                self.in_blockquote = true;
            }
            Event::End(TagEnd::BlockQuote) => {
                self.flush_line();
                self.in_blockquote = false;
            }

            // Links
            Event::Start(Tag::Link { .. }) => {
                self.style_stack.push(self.styles.link);
            }

            // Paragraphs
            Event::End(TagEnd::Paragraph) => {
                self.flush_line();
                // Add blank line after paragraph
                self.lines.push(Line::from(""));
            }

            // Text content
            Event::Text(text) => {
                self.add_text(&text);
            }

            // Inline code
            Event::Code(code) => {
                let style = self.styles.code;
                self.current_spans.push(Span::styled(
                    format!("`{code}`"),
                    style,
                ));
            }

            // Line breaks
            Event::SoftBreak => {
                // Soft break = space
                self.add_text(" ");
            }
            Event::HardBreak => {
                self.flush_line();
            }

            // Events we don't handle specially (ignore)
            Event::Start(
                Tag::Paragraph
                | Tag::Image { .. }
                | Tag::Table(_)
                | Tag::TableHead
                | Tag::TableRow
                | Tag::TableCell
                | Tag::FootnoteDefinition(_)
                | Tag::MetadataBlock(_)
                | Tag::HtmlBlock,
            )
            | Event::End(
                TagEnd::Image
                | TagEnd::Table
                | TagEnd::TableHead
                | TagEnd::TableRow
                | TagEnd::TableCell
                | TagEnd::FootnoteDefinition
                | TagEnd::MetadataBlock(_)
                | TagEnd::HtmlBlock,
            )
            | Event::Html(_)
            | Event::InlineHtml(_)
            | Event::FootnoteReference(_)
            | Event::Rule => {}
        }
    }

    fn add_text(&mut self, text: &str) {
        if self.in_code_block {
            // In code block, render each line with code styling
            for line in text.lines() {
                let indent = "  ".repeat(self.indent_level.saturating_sub(1));
                self.current_spans.push(Span::styled(
                    format!("{indent}  {line}"),
                    self.styles.code_block,
                ));
                self.flush_line();
            }
            return;
        }

        // Handle list marker if pending
        if let Some(marker) = self.pending_list_marker.take() {
            // Add list marker
            self.current_spans.push(Span::styled(
                marker,
                self.styles.list_marker,
            ));
            // Add task checkbox if present
            if let Some(checked) = self.task_checkbox.take() {
                let checkbox = if checked { "[x] " } else { "[ ] " };
                self.current_spans.push(Span::styled(
                    checkbox,
                    self.styles.list_marker,
                ));
            }
        }

        // Blockquote prefix
        if self.in_blockquote && self.current_spans.is_empty() {
            self.current_spans.push(Span::styled(
                "> ".to_string(),
                self.styles.blockquote,
            ));
        }

        // Compute current style from stack
        let style = self.current_style();
        self.current_spans.push(Span::styled(text.to_string(), style));
    }

    fn current_style(&self) -> Style {
        // Combine all styles in the stack
        let mut style = self.styles.text;
        for s in &self.style_stack {
            style = style.patch(*s);
        }
        style
    }

    fn heading_style(&self, level: HeadingLevel) -> Style {
        match level {
            HeadingLevel::H1 => self.styles.h1,
            HeadingLevel::H2 => self.styles.h2,
            _ => self.styles.h3,
        }
    }

    fn flush_line(&mut self) {
        if !self.current_spans.is_empty() {
            let spans = std::mem::take(&mut self.current_spans);
            self.lines.push(Line::from(spans));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_theme() -> Theme {
        Theme::default()
    }

    #[test]
    fn test_render_simple_text() {
        let lines = render_markdown("Hello, world!", 80, &test_theme());
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_heading() {
        let lines = render_markdown("# Title", 80, &test_theme());
        assert!(!lines.is_empty());
        // First non-empty line should contain the title
        let first = &lines[0];
        let text: String = first.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("Title"));
    }

    #[test]
    fn test_render_bold() {
        let lines = render_markdown("**bold text**", 80, &test_theme());
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_italic() {
        let lines = render_markdown("*italic text*", 80, &test_theme());
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_inline_code() {
        let lines = render_markdown("Use `code` here", 80, &test_theme());
        assert!(!lines.is_empty());
        let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("`code`"));
    }

    #[test]
    fn test_render_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let lines = render_markdown(md, 80, &test_theme());
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_list() {
        let md = "- Item 1\n- Item 2";
        let lines = render_markdown(md, 80, &test_theme());
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_render_checkbox() {
        let md = "- [ ] Unchecked\n- [x] Checked";
        let lines = render_markdown(md, 80, &test_theme());
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_render_blockquote() {
        let md = "> This is a quote";
        let lines = render_markdown(md, 80, &test_theme());
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_nested_formatting() {
        let md = "**bold and *italic* text**";
        let lines = render_markdown(md, 80, &test_theme());
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_empty() {
        let lines = render_markdown("", 80, &test_theme());
        assert!(lines.is_empty());
    }

    #[test]
    fn test_render_multiple_paragraphs() {
        let md = "First paragraph.\n\nSecond paragraph.";
        let lines = render_markdown(md, 80, &test_theme());
        // Should have content and blank lines
        assert!(lines.len() >= 3);
    }
}
