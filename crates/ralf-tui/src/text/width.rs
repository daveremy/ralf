//! Text width and truncation utilities.
//!
//! Provides unicode-aware text width calculation and safe truncation.

use unicode_width::UnicodeWidthStr;

/// Get the visual width of a string in terminal cells.
///
/// Accounts for wide characters (CJK, emoji) that take 2 cells.
pub fn visual_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Truncate a string to fit within a maximum visual width.
///
/// Returns the truncated string with "..." appended if truncation occurred.
/// This is unicode-safe and respects character boundaries.
pub fn truncate_to_width(s: &str, max_width: usize) -> String {
    let current_width = visual_width(s);
    if current_width <= max_width {
        return s.to_string();
    }

    // Need to truncate - account for "..." suffix (3 chars)
    let target_width = max_width.saturating_sub(3);
    if target_width == 0 {
        return "...".to_string();
    }

    let mut result = String::new();
    let mut width = 0;

    for ch in s.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width > target_width {
            break;
        }
        result.push(ch);
        width += ch_width;
    }

    result.push_str("...");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visual_width_ascii() {
        assert_eq!(visual_width("hello"), 5);
        assert_eq!(visual_width(""), 0);
    }

    #[test]
    fn test_visual_width_wide_chars() {
        // CJK characters are 2 cells wide
        assert_eq!(visual_width("你好"), 4);
        assert_eq!(visual_width("hello你好"), 9);
    }

    #[test]
    fn test_visual_width_emoji() {
        // Basic emoji - width varies by terminal, but generally 2
        let width = visual_width("🎉");
        assert!(width >= 1 && width <= 2);
    }

    #[test]
    fn test_truncate_no_truncation_needed() {
        assert_eq!(truncate_to_width("hello", 10), "hello");
        assert_eq!(truncate_to_width("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_ascii() {
        assert_eq!(truncate_to_width("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_unicode_safe() {
        // Should not panic on unicode
        let result = truncate_to_width("你好世界", 5);
        assert!(result.ends_with("..."));
        // Should have at least one Chinese char before ...
        assert!(result.chars().next().unwrap() == '你');
    }

    #[test]
    fn test_truncate_very_short() {
        assert_eq!(truncate_to_width("hello", 3), "...");
        assert_eq!(truncate_to_width("hello", 0), "...");
    }

    #[test]
    fn test_truncate_emoji_boundary() {
        // Should not split emoji
        let result = truncate_to_width("🎉🎊🎁 celebration", 10);
        assert!(!result.is_empty());
        // Should end with ...
        assert!(result.ends_with("..."));
    }
}
