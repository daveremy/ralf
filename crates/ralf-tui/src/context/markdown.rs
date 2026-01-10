//! Simple markdown parser for spec preview.
//!
//! Parses markdown into blocks for styled rendering.
//! Supports headers, code blocks, lists, checkboxes, and inline code.

/// A parsed markdown block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownBlock {
    /// Header with level (1-6) and text.
    Header { level: u8, text: String },
    /// Fenced code block with optional language hint.
    CodeBlock { language: Option<String>, code: String },
    /// Unordered list item with indent level and text.
    ListItem { indent: usize, text: String },
    /// Numbered list item with indent level, number, and text.
    NumberedItem { indent: usize, number: u32, text: String },
    /// Checkbox item (checked or unchecked) with text.
    Checkbox { checked: bool, text: String },
    /// Regular paragraph text.
    Paragraph(String),
    /// Empty line (for spacing).
    Empty,
}

/// Parse markdown text into blocks.
pub fn parse_markdown(input: &str) -> Vec<MarkdownBlock> {
    let mut blocks = Vec::new();
    let mut lines = input.lines().peekable();

    while let Some(line) = lines.next() {
        // Empty line
        if line.trim().is_empty() {
            blocks.push(MarkdownBlock::Empty);
            continue;
        }

        // Header (# ## ### etc.)
        if let Some(header) = parse_header(line) {
            blocks.push(header);
            continue;
        }

        // Code fence start
        if line.trim_start().starts_with("```") {
            let indent = line.len() - line.trim_start().len();
            let fence_content = line.trim_start().trim_start_matches('`');
            let language = if fence_content.is_empty() {
                None
            } else {
                Some(fence_content.to_string())
            };

            // Collect code until closing fence
            let mut code_lines = Vec::new();
            for code_line in lines.by_ref() {
                if code_line.trim_start().starts_with("```") {
                    break;
                }
                // Remove common indent from code lines
                let trimmed = if code_line.len() > indent {
                    &code_line[indent.min(code_line.len() - code_line.trim_start().len())..]
                } else {
                    code_line
                };
                code_lines.push(trimmed.to_string());
            }

            blocks.push(MarkdownBlock::CodeBlock {
                language,
                code: code_lines.join("\n"),
            });
            continue;
        }

        // Checkbox (- [ ] or - [x])
        if let Some(checkbox) = parse_checkbox(line) {
            blocks.push(checkbox);
            continue;
        }

        // Unordered list item (- or *)
        if let Some(list_item) = parse_list_item(line) {
            blocks.push(list_item);
            continue;
        }

        // Numbered list item (1. 2. etc.)
        if let Some(numbered) = parse_numbered_item(line) {
            blocks.push(numbered);
            continue;
        }

        // Regular paragraph
        blocks.push(MarkdownBlock::Paragraph(line.to_string()));
    }

    blocks
}

/// Parse a header line (# ## ### etc.).
fn parse_header(line: &str) -> Option<MarkdownBlock> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }

    // Count consecutive # characters
    let level = trimmed.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }

    let rest = &trimmed[level..];

    // Must have space after #s (or be empty for just "###")
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }

    #[allow(clippy::cast_possible_truncation)]
    Some(MarkdownBlock::Header {
        level: level as u8,
        text: rest.trim().to_string(),
    })
}

/// Parse a checkbox line (- [ ] or - [x]).
fn parse_checkbox(line: &str) -> Option<MarkdownBlock> {
    let trimmed = line.trim_start();

    // Check for unchecked: - [ ]
    if let Some(text) = trimmed.strip_prefix("- [ ] ") {
        return Some(MarkdownBlock::Checkbox {
            checked: false,
            text: text.to_string(),
        });
    }

    // Check for checked: - [x] or - [X]
    if let Some(text) = trimmed.strip_prefix("- [x] ").or_else(|| trimmed.strip_prefix("- [X] ")) {
        return Some(MarkdownBlock::Checkbox {
            checked: true,
            text: text.to_string(),
        });
    }

    None
}

/// Parse an unordered list item (- or *).
fn parse_list_item(line: &str) -> Option<MarkdownBlock> {
    let indent = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();

    // Check for - or * followed by space
    if (trimmed.starts_with("- ") || trimmed.starts_with("* "))
        && !trimmed.starts_with("- [ ")  // Not a checkbox
    {
        return Some(MarkdownBlock::ListItem {
            indent: indent / 2, // Normalize to levels
            text: trimmed[2..].to_string(),
        });
    }

    None
}

/// Parse a numbered list item (1. 2. etc.).
fn parse_numbered_item(line: &str) -> Option<MarkdownBlock> {
    let indent = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();

    // Find the number
    let mut num_end = 0;
    for (i, c) in trimmed.char_indices() {
        if c.is_ascii_digit() {
            num_end = i + 1;
        } else {
            break;
        }
    }

    if num_end == 0 {
        return None;
    }

    // Must be followed by ". "
    let rest = &trimmed[num_end..];
    if !rest.starts_with(". ") {
        return None;
    }

    let number: u32 = trimmed[..num_end].parse().ok()?;

    Some(MarkdownBlock::NumberedItem {
        indent: indent / 2,
        number,
        text: rest[2..].to_string(),
    })
}

/// Segments of inline text with styling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InlineSegment {
    /// Plain text.
    Text(String),
    /// Inline code (`code`).
    Code(String),
}

/// Parse inline segments from text (for inline code detection).
pub fn parse_inline(text: &str) -> Vec<InlineSegment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_code = false;

    for c in text.chars() {
        if c == '`' {
            if in_code {
                // End of code span
                segments.push(InlineSegment::Code(std::mem::take(&mut current)));
                in_code = false;
            } else {
                // Start of code span
                if !current.is_empty() {
                    segments.push(InlineSegment::Text(std::mem::take(&mut current)));
                }
                in_code = true;
            }
        } else {
            current.push(c);
        }
    }

    // Handle remaining content
    if !current.is_empty() {
        if in_code {
            // Unclosed code span - treat as text with backtick
            segments.push(InlineSegment::Text(format!("`{current}")));
        } else {
            segments.push(InlineSegment::Text(current));
        }
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_header() {
        let blocks = parse_markdown("# Title");
        assert_eq!(blocks, vec![MarkdownBlock::Header { level: 1, text: "Title".into() }]);

        let blocks = parse_markdown("### Sub-heading");
        assert_eq!(blocks, vec![MarkdownBlock::Header { level: 3, text: "Sub-heading".into() }]);
    }

    #[test]
    fn test_parse_code_block() {
        let input = "```rust\nfn main() {}\n```";
        let blocks = parse_markdown(input);
        assert_eq!(blocks, vec![MarkdownBlock::CodeBlock {
            language: Some("rust".into()),
            code: "fn main() {}".into(),
        }]);
    }

    #[test]
    fn test_parse_code_block_no_language() {
        let input = "```\nsome code\n```";
        let blocks = parse_markdown(input);
        assert_eq!(blocks, vec![MarkdownBlock::CodeBlock {
            language: None,
            code: "some code".into(),
        }]);
    }

    #[test]
    fn test_parse_list_items() {
        let input = "- First\n- Second";
        let blocks = parse_markdown(input);
        assert_eq!(blocks, vec![
            MarkdownBlock::ListItem { indent: 0, text: "First".into() },
            MarkdownBlock::ListItem { indent: 0, text: "Second".into() },
        ]);
    }

    #[test]
    fn test_parse_nested_list() {
        let input = "- Outer\n  - Inner";
        let blocks = parse_markdown(input);
        assert_eq!(blocks, vec![
            MarkdownBlock::ListItem { indent: 0, text: "Outer".into() },
            MarkdownBlock::ListItem { indent: 1, text: "Inner".into() },
        ]);
    }

    #[test]
    fn test_parse_numbered_list() {
        let input = "1. First\n2. Second";
        let blocks = parse_markdown(input);
        assert_eq!(blocks, vec![
            MarkdownBlock::NumberedItem { indent: 0, number: 1, text: "First".into() },
            MarkdownBlock::NumberedItem { indent: 0, number: 2, text: "Second".into() },
        ]);
    }

    #[test]
    fn test_parse_checkbox() {
        let input = "- [ ] Unchecked\n- [x] Checked";
        let blocks = parse_markdown(input);
        assert_eq!(blocks, vec![
            MarkdownBlock::Checkbox { checked: false, text: "Unchecked".into() },
            MarkdownBlock::Checkbox { checked: true, text: "Checked".into() },
        ]);
    }

    #[test]
    fn test_parse_paragraph() {
        let blocks = parse_markdown("Just some text.");
        assert_eq!(blocks, vec![MarkdownBlock::Paragraph("Just some text.".into())]);
    }

    #[test]
    fn test_parse_empty_lines() {
        let blocks = parse_markdown("Line 1\n\nLine 2");
        assert_eq!(blocks, vec![
            MarkdownBlock::Paragraph("Line 1".into()),
            MarkdownBlock::Empty,
            MarkdownBlock::Paragraph("Line 2".into()),
        ]);
    }

    #[test]
    fn test_parse_inline_code() {
        let segments = parse_inline("Use `foo` here");
        assert_eq!(segments, vec![
            InlineSegment::Text("Use ".into()),
            InlineSegment::Code("foo".into()),
            InlineSegment::Text(" here".into()),
        ]);
    }

    #[test]
    fn test_parse_inline_unclosed_code() {
        let segments = parse_inline("Unclosed `code");
        assert_eq!(segments, vec![
            InlineSegment::Text("Unclosed ".into()),
            InlineSegment::Text("`code".into()),
        ]);
    }

    #[test]
    fn test_unclosed_code_fence() {
        // Unclosed fence should consume until end
        let input = "```rust\nfn main() {}";
        let blocks = parse_markdown(input);
        assert_eq!(blocks, vec![MarkdownBlock::CodeBlock {
            language: Some("rust".into()),
            code: "fn main() {}".into(),
        }]);
    }

    #[test]
    fn test_mixed_content() {
        let input = "# Title\n\nSome text.\n\n- Item 1\n- Item 2\n\n```\ncode\n```";
        let blocks = parse_markdown(input);
        assert_eq!(blocks.len(), 8);
        assert!(matches!(blocks[0], MarkdownBlock::Header { level: 1, .. }));
        assert!(matches!(blocks[1], MarkdownBlock::Empty));
        assert!(matches!(blocks[2], MarkdownBlock::Paragraph(_)));
        assert!(matches!(blocks[3], MarkdownBlock::Empty));
        assert!(matches!(blocks[4], MarkdownBlock::ListItem { .. }));
        assert!(matches!(blocks[5], MarkdownBlock::ListItem { .. }));
        assert!(matches!(blocks[6], MarkdownBlock::Empty));
        assert!(matches!(blocks[7], MarkdownBlock::CodeBlock { .. }));
    }
}
