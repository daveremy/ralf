//! Border sets for Unicode and ASCII fallback.
//!
//! Supports `NO_COLOR` environment variable by using ASCII borders.

use ratatui::symbols::border;

use super::icons::IconMode;

/// Border set based on icon mode (supports `NO_COLOR`/ASCII fallback).
#[derive(Debug, Clone)]
pub struct BorderSet {
    mode: IconMode,
}

impl Default for BorderSet {
    fn default() -> Self {
        Self::new(IconMode::default())
    }
}

impl BorderSet {
    /// Create a new border set with the specified mode.
    pub fn new(mode: IconMode) -> Self {
        Self { mode }
    }

    /// Get the current mode.
    pub fn mode(&self) -> IconMode {
        self.mode
    }

    /// Normal (unfocused) borders - rounded for Unicode, plain for ASCII.
    pub fn normal(&self) -> border::Set {
        match self.mode {
            IconMode::Nerd | IconMode::Unicode => border::ROUNDED,
            IconMode::Ascii => border::PLAIN, // +--+ ASCII corners
        }
    }

    /// Focused borders - thick for Unicode, double for ASCII.
    pub fn focused(&self) -> border::Set {
        match self.mode {
            IconMode::Nerd | IconMode::Unicode => border::THICK,
            IconMode::Ascii => border::DOUBLE, // Best ASCII emphasis
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_nerd_mode() {
        let borders = BorderSet::default();
        assert_eq!(borders.mode(), IconMode::Nerd);
    }

    #[test]
    fn test_unicode_borders() {
        let borders = BorderSet::new(IconMode::Unicode);
        let normal = borders.normal();
        let focused = borders.focused();

        // Rounded corners for normal
        assert_eq!(normal.top_left, "╭");
        // Thick corners for focused
        assert_eq!(focused.top_left, "┏");
    }

    #[test]
    fn test_ascii_borders() {
        let borders = BorderSet::new(IconMode::Ascii);
        let normal = borders.normal();
        let focused = borders.focused();

        // Plain corners for normal
        assert_eq!(normal.top_left, "┌");
        // Double corners for focused
        assert_eq!(focused.top_left, "╔");
    }

    #[test]
    fn test_nerd_same_as_unicode_borders() {
        let nerd = BorderSet::new(IconMode::Nerd);
        let unicode = BorderSet::new(IconMode::Unicode);

        assert_eq!(nerd.normal().top_left, unicode.normal().top_left);
        assert_eq!(nerd.focused().top_left, unicode.focused().top_left);
    }
}
