//! Theme and styling definitions for the ralf TUI.

use ratatui::style::{Color, Modifier, Style};

/// Color palette for the TUI.
pub struct Palette;

impl Palette {
    // Base colors
    pub const BG: Color = Color::Rgb(30, 30, 40);
    pub const FG: Color = Color::Rgb(220, 220, 230);
    pub const DIM: Color = Color::Rgb(140, 140, 160);

    // Accent colors
    pub const ACCENT: Color = Color::Rgb(130, 170, 255);
    pub const ACCENT_DIM: Color = Color::Rgb(80, 100, 160);

    // Status bar colors (high contrast)
    pub const STATUS_BG: Color = Color::Rgb(45, 45, 60);
    pub const STATUS_KEY_BG: Color = Color::Rgb(70, 90, 140);

    // Status colors
    pub const SUCCESS: Color = Color::Rgb(130, 220, 130);
    pub const WARNING: Color = Color::Rgb(240, 200, 100);
    pub const ERROR: Color = Color::Rgb(240, 100, 100);

    // Border colors
    pub const BORDER: Color = Color::Rgb(80, 80, 100);
    pub const BORDER_ACTIVE: Color = Color::Rgb(130, 170, 255);
}

/// Status indicator symbols (with ASCII fallbacks).
pub struct Symbols;

impl Symbols {
    pub const CHECK: &'static str = "[ok]";
    pub const WARN: &'static str = "[!]";
    pub const ERROR: &'static str = "[x]";
    pub const PENDING: &'static str = "[ ]";
    pub const SPINNER: [&'static str; 4] = ["|", "/", "-", "\\"];
}

/// Common styles used throughout the TUI.
pub struct Styles;

impl Styles {
    /// Default text style.
    pub fn default() -> Style {
        Style::default().fg(Palette::FG).bg(Palette::BG)
    }

    /// Dimmed text for secondary information.
    pub fn dim() -> Style {
        Style::default().fg(Palette::DIM).bg(Palette::BG)
    }

    /// Highlighted/selected item.
    pub fn highlight() -> Style {
        Style::default()
            .fg(Palette::ACCENT)
            .bg(Palette::BG)
            .add_modifier(Modifier::BOLD)
    }

    /// Active/focused element.
    pub fn active() -> Style {
        Style::default().fg(Palette::ACCENT).bg(Palette::BG)
    }

    /// Success status.
    pub fn success() -> Style {
        Style::default().fg(Palette::SUCCESS).bg(Palette::BG)
    }

    /// Warning status.
    pub fn warning() -> Style {
        Style::default().fg(Palette::WARNING).bg(Palette::BG)
    }

    /// Error status.
    pub fn error() -> Style {
        Style::default().fg(Palette::ERROR).bg(Palette::BG)
    }

    /// Title style.
    pub fn title() -> Style {
        Style::default()
            .fg(Palette::ACCENT)
            .add_modifier(Modifier::BOLD)
    }

    /// Key hint style (for status bar) - bright on dark for visibility.
    pub fn key_hint() -> Style {
        Style::default()
            .fg(Palette::FG)
            .bg(Palette::STATUS_KEY_BG)
            .add_modifier(Modifier::BOLD)
    }

    /// Key hint label style - readable on status bar background.
    pub fn key_label() -> Style {
        Style::default().fg(Palette::FG).bg(Palette::STATUS_BG)
    }

    /// Status bar background style.
    pub fn status_bar() -> Style {
        Style::default().fg(Palette::FG).bg(Palette::STATUS_BG)
    }

    /// Border style for inactive elements.
    pub fn border() -> Style {
        Style::default().fg(Palette::BORDER)
    }

    /// Border style for active/focused elements.
    pub fn border_active() -> Style {
        Style::default().fg(Palette::BORDER_ACTIVE)
    }
}

/// Progress bar rendering.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
pub fn progress_bar(progress: f32, width: usize) -> String {
    let filled = ((progress * width as f32).round() as usize).min(width);
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "=".repeat(filled), " ".repeat(empty))
}

/// Format a status indicator with the appropriate symbol.
pub fn status_indicator(status: Status) -> (&'static str, Style) {
    match status {
        Status::Ready => (Symbols::CHECK, Styles::success()),
        Status::Warning => (Symbols::WARN, Styles::warning()),
        Status::Error => (Symbols::ERROR, Styles::error()),
        Status::Pending => (Symbols::PENDING, Styles::dim()),
    }
}

/// Status types for indicators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ready,
    Warning,
    Error,
    Pending,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar() {
        assert_eq!(progress_bar(0.0, 10), "[          ]");
        assert_eq!(progress_bar(0.5, 10), "[=====     ]");
        assert_eq!(progress_bar(1.0, 10), "[==========]");
    }

    #[test]
    fn test_status_indicator() {
        let (sym, _) = status_indicator(Status::Ready);
        assert_eq!(sym, "[ok]");
    }
}
