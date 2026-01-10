//! Text wrapping utilities for ratatui Lines.
//!
//! Provides functions to wrap styled text to fit within a given width.

use ratatui::text::{Line, Span};

/// Wrap a plain text string to the specified width.
/// Returns a vector of wrapped lines.
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    textwrap::wrap(text, width)
        .into_iter()
        .map(std::borrow::Cow::into_owned)
        .collect()
}

/// Wrap a vector of Lines to fit within the specified width.
/// Each line that exceeds the width will be split into multiple lines.
/// Styling is preserved across wrapped lines.
pub fn wrap_lines(lines: Vec<Line<'static>>, width: usize) -> Vec<Line<'static>> {
    if width == 0 {
        return lines;
    }

    let mut result = Vec::new();

    for line in lines {
        let wrapped = wrap_line(line, width);
        result.extend(wrapped);
    }

    result
}

/// Wrap a single Line to fit within the specified width.
/// Returns one or more Lines with preserved styling.
fn wrap_line(line: Line<'static>, width: usize) -> Vec<Line<'static>> {
    // Calculate the total visible width of the line
    let total_width: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();

    if total_width <= width {
        return vec![line];
    }

    // Need to wrap - collect all characters with their styles
    let mut chars_with_styles: Vec<(char, ratatui::style::Style)> = Vec::new();
    for span in &line.spans {
        for ch in span.content.chars() {
            chars_with_styles.push((ch, span.style));
        }
    }

    // Build the plain text for wrapping
    let plain_text: String = chars_with_styles.iter().map(|(ch, _)| ch).collect();

    // Use textwrap to determine wrap points
    let wrapped_strings: Vec<String> = textwrap::wrap(&plain_text, width)
        .into_iter()
        .map(std::borrow::Cow::into_owned)
        .collect();

    // Now rebuild Lines with proper styling
    let mut result = Vec::new();
    let mut char_idx = 0;

    for wrapped_str in wrapped_strings {
        let mut spans = Vec::new();
        let mut current_style = None;
        let mut current_text = String::new();

        // Skip leading whitespace that textwrap might have trimmed
        while char_idx < chars_with_styles.len() {
            let (ch, _) = chars_with_styles[char_idx];
            if !wrapped_str.starts_with(ch) && ch.is_whitespace() {
                char_idx += 1;
            } else {
                break;
            }
        }

        for expected_char in wrapped_str.chars() {
            if char_idx >= chars_with_styles.len() {
                // Shouldn't happen, but handle gracefully
                current_text.push(expected_char);
                continue;
            }

            let (ch, style) = chars_with_styles[char_idx];
            char_idx += 1;

            // Handle style changes
            match current_style {
                None => {
                    current_style = Some(style);
                    current_text.push(ch);
                }
                Some(s) if s == style => {
                    current_text.push(ch);
                }
                Some(s) => {
                    // Style changed - flush current span
                    if !current_text.is_empty() {
                        spans.push(Span::styled(std::mem::take(&mut current_text), s));
                    }
                    current_style = Some(style);
                    current_text.push(ch);
                }
            }
        }

        // Flush remaining text
        if !current_text.is_empty() {
            if let Some(s) = current_style {
                spans.push(Span::styled(current_text, s));
            } else {
                spans.push(Span::raw(current_text));
            }
        }

        if !spans.is_empty() {
            result.push(Line::from(spans));
        }
    }

    if result.is_empty() {
        result.push(Line::from(""));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Style};

    #[test]
    fn test_wrap_text_short() {
        let lines = wrap_text("Hello", 10);
        assert_eq!(lines, vec!["Hello"]);
    }

    #[test]
    fn test_wrap_text_long() {
        let lines = wrap_text("Hello world this is a long line", 10);
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(line.chars().count() <= 10);
        }
    }

    #[test]
    fn test_wrap_line_short() {
        let line = Line::from("Short");
        let wrapped = wrap_line(line, 20);
        assert_eq!(wrapped.len(), 1);
    }

    #[test]
    fn test_wrap_line_preserves_style() {
        let line = Line::from(vec![
            Span::styled("Hello ", Style::default().fg(Color::Red)),
            Span::styled("world", Style::default().fg(Color::Blue)),
        ]);
        let wrapped = wrap_line(line, 100);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0].spans.len(), 2);
    }

    #[test]
    fn test_wrap_lines_multiple() {
        let lines = vec![
            Line::from("Short line"),
            Line::from("This is a very long line that should definitely be wrapped to fit"),
        ];
        let wrapped = wrap_lines(lines, 20);
        assert!(wrapped.len() > 2);
    }

    #[test]
    fn test_wrap_text_unicode() {
        // Test with emoji and unicode characters
        let text = "Hello 🎉 world 你好 this is a test with émojis and ünïcödé";
        let lines = wrap_text(text, 15);
        assert!(lines.len() > 1);
        // Verify all content is preserved (no panics, no lost chars)
        let rejoined: String = lines.join(" ");
        // textwrap may normalize whitespace, so just check key parts exist
        assert!(rejoined.contains("🎉"));
        assert!(rejoined.contains("你好"));
        assert!(rejoined.contains("émojis"));
    }

    #[test]
    fn test_wrap_line_unicode_with_style() {
        // Test styled line with unicode - this is the risky case
        let line = Line::from(vec![
            Span::styled("Hello 🎉 ", Style::default().fg(Color::Red)),
            Span::styled("你好世界", Style::default().fg(Color::Blue)),
        ]);
        let wrapped = wrap_line(line, 10);
        // Should not panic and should produce output
        assert!(!wrapped.is_empty());
        // Verify emoji and Chinese chars are somewhere in output
        let all_text: String = wrapped
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(all_text.contains("🎉"));
        assert!(all_text.contains("你好"));
    }

    #[test]
    fn test_wrap_line_emoji_sequence() {
        // Test with emoji that have modifiers (skin tones, ZWJ sequences)
        let line = Line::from(vec![
            Span::styled("Family: 👨‍👩‍👧‍👦 ", Style::default().fg(Color::Green)),
            Span::styled("Wave: 👋🏽", Style::default().fg(Color::Yellow)),
        ]);
        let wrapped = wrap_line(line, 20);
        assert!(!wrapped.is_empty());
        // Just verify no panic - complex emoji handling is tricky
    }
}
