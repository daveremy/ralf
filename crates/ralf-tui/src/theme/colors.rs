//! Catppuccin Mocha color palette for the TUI.
//!
//! See `TUI_STYLE_GUIDE.md` for design rationale.

use ratatui::style::Color;

/// Theme color palette.
#[derive(Debug, Clone)]
pub struct Theme {
    // Backgrounds
    pub base: Color,
    pub surface: Color,
    pub overlay: Color,

    // Foregrounds
    pub text: Color,
    pub subtext: Color,
    pub muted: Color,

    // Accents
    pub primary: Color,
    pub secondary: Color,

    // Semantic
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Model attribution
    pub claude: Color,
    pub gemini: Color,
    pub codex: Color,

    // Borders
    pub border: Color,
    pub border_focused: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::mocha()
    }
}

impl Theme {
    /// Catppuccin Mocha theme (default dark theme).
    pub fn mocha() -> Self {
        Self {
            // Backgrounds
            base: Color::Rgb(30, 30, 46),       // #1e1e2e
            surface: Color::Rgb(49, 50, 68),    // #313244
            overlay: Color::Rgb(69, 71, 90),    // #45475a

            // Foregrounds
            text: Color::Rgb(205, 214, 244),    // #cdd6f4
            subtext: Color::Rgb(166, 173, 200), // #a6adc8
            muted: Color::Rgb(108, 112, 134),   // #6c7086

            // Accents
            primary: Color::Rgb(180, 190, 254),   // #b4befe (lavender)
            secondary: Color::Rgb(148, 226, 213), // #94e2d5 (teal)

            // Semantic
            success: Color::Rgb(166, 227, 161), // #a6e3a1 (green)
            warning: Color::Rgb(249, 226, 175), // #f9e2af (yellow)
            error: Color::Rgb(243, 139, 168),   // #f38ba8 (red)
            info: Color::Rgb(137, 180, 250),    // #89b4fa (blue)

            // Model attribution
            claude: Color::Rgb(250, 179, 135), // #fab387 (peach)
            gemini: Color::Rgb(137, 180, 250), // #89b4fa (blue)
            codex: Color::Rgb(166, 227, 161),  // #a6e3a1 (green)

            // Borders
            border: Color::Rgb(69, 71, 90),       // #45475a
            border_focused: Color::Rgb(180, 190, 254), // #b4befe (lavender)
        }
    }

    /// Catppuccin Latte theme (light theme).
    pub fn latte() -> Self {
        Self {
            // Backgrounds (inverted for light theme)
            base: Color::Rgb(239, 241, 245),    // #eff1f5
            surface: Color::Rgb(230, 233, 239), // #e6e9ef
            overlay: Color::Rgb(220, 224, 232), // #dce0e8

            // Foregrounds
            text: Color::Rgb(76, 79, 105),      // #4c4f69
            subtext: Color::Rgb(92, 95, 119),   // #5c5f77
            muted: Color::Rgb(140, 143, 161),   // #8c8fa1

            // Accents
            primary: Color::Rgb(114, 135, 253),   // #7287fd (lavender)
            secondary: Color::Rgb(23, 146, 153),  // #179299 (teal)

            // Semantic
            success: Color::Rgb(64, 160, 43),   // #40a02b (green)
            warning: Color::Rgb(223, 142, 29),  // #df8e1d (yellow)
            error: Color::Rgb(210, 15, 57),     // #d20f39 (red)
            info: Color::Rgb(30, 102, 245),     // #1e66f5 (blue)

            // Model attribution
            claude: Color::Rgb(254, 100, 11),  // #fe640b (peach)
            gemini: Color::Rgb(30, 102, 245),  // #1e66f5 (blue)
            codex: Color::Rgb(64, 160, 43),    // #40a02b (green)

            // Borders
            border: Color::Rgb(188, 192, 204),      // #bcc0cc
            border_focused: Color::Rgb(114, 135, 253), // #7287fd (lavender)
        }
    }

    /// High contrast theme for accessibility.
    pub fn high_contrast() -> Self {
        Self {
            // Maximum contrast backgrounds
            base: Color::Black,
            surface: Color::Rgb(20, 20, 20),
            overlay: Color::Rgb(40, 40, 40),

            // Maximum contrast foregrounds
            text: Color::White,
            subtext: Color::Rgb(200, 200, 200),
            muted: Color::Rgb(150, 150, 150),

            // Bright accents
            primary: Color::Cyan,
            secondary: Color::Magenta,

            // Semantic (bright versions)
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Blue,

            // Model attribution
            claude: Color::LightRed,
            gemini: Color::LightBlue,
            codex: Color::LightGreen,

            // Borders
            border: Color::White,
            border_focused: Color::Cyan,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mocha_theme_creates() {
        let theme = Theme::mocha();
        assert!(matches!(theme.base, Color::Rgb(30, 30, 46)));
    }

    #[test]
    fn test_latte_theme_creates() {
        let theme = Theme::latte();
        assert!(matches!(theme.base, Color::Rgb(239, 241, 245)));
    }

    #[test]
    fn test_high_contrast_theme_creates() {
        let theme = Theme::high_contrast();
        assert!(matches!(theme.base, Color::Black));
    }

    #[test]
    fn test_default_is_mocha() {
        let default = Theme::default();
        let mocha = Theme::mocha();
        assert!(matches!(default.base, Color::Rgb(30, 30, 46)));
        assert!(matches!(mocha.base, Color::Rgb(30, 30, 46)));
    }
}
