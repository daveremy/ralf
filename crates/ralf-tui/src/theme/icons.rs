//! Icon sets for Nerd Fonts, Unicode, and ASCII fallback.
//!
//! See `TUI_STYLE_GUIDE.md` for the complete icon reference table.

/// Icon mode configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconMode {
    /// Nerd Font icons (default, richest experience).
    #[default]
    Nerd,
    /// Standard Unicode symbols (wide compatibility).
    Unicode,
    /// ASCII-only fallback (maximum compatibility, also used with `NO_COLOR`).
    Ascii,
}

/// Icon set based on configured mode.
#[derive(Debug, Clone)]
pub struct IconSet {
    mode: IconMode,
}

impl Default for IconSet {
    fn default() -> Self {
        Self::new(IconMode::default())
    }
}

impl IconSet {
    /// Create a new icon set with the specified mode.
    pub fn new(mode: IconMode) -> Self {
        Self { mode }
    }

    /// Get the current icon mode.
    pub fn mode(&self) -> IconMode {
        self.mode
    }

    // === Status Icons ===

    pub fn running(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰄬",
            IconMode::Unicode => "●",
            IconMode::Ascii => "[*]",
        }
    }

    pub fn stopped(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰅖",
            IconMode::Unicode => "○",
            IconMode::Ascii => "[ ]",
        }
    }

    pub fn in_progress(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰦖",
            IconMode::Unicode => "◐",
            IconMode::Ascii => "[~]",
        }
    }

    pub fn paused(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰏤",
            IconMode::Unicode => "◑",
            IconMode::Ascii => "[=]",
        }
    }

    // === Result Icons ===

    pub fn success(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰄬",
            IconMode::Unicode => "✓",
            IconMode::Ascii => "[x]",
        }
    }

    pub fn error(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰅖",
            IconMode::Unicode => "✗",
            IconMode::Ascii => "[X]",
        }
    }

    pub fn warning(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰀦",
            IconMode::Unicode => "⚠",
            IconMode::Ascii => "[!]",
        }
    }

    pub fn info(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰋼",
            IconMode::Unicode => "ℹ",
            IconMode::Ascii => "[i]",
        }
    }

    // === Navigation Icons ===

    pub fn collapsed(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰅂",
            IconMode::Unicode => "▸",
            IconMode::Ascii => ">",
        }
    }

    pub fn expanded(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰅀",
            IconMode::Unicode => "▾",
            IconMode::Ascii => "v",
        }
    }

    pub fn arrow_right(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰁔",
            IconMode::Unicode => "→",
            IconMode::Ascii => "->",
        }
    }

    pub fn arrow_left(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰁍",
            IconMode::Unicode => "←",
            IconMode::Ascii => "<-",
        }
    }

    // === Timeline Event Icons ===

    pub fn event_spec(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰭻",
            IconMode::Unicode => "💬",
            IconMode::Ascii => "[S]",
        }
    }

    pub fn event_run(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󱐋",
            IconMode::Unicode => "⚡",
            IconMode::Ascii => "[R]",
        }
    }

    pub fn event_review(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰈈",
            IconMode::Unicode => "👁",
            IconMode::Ascii => "[V]",
        }
    }

    pub fn event_system(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰒓",
            IconMode::Unicode => "⚙",
            IconMode::Ascii => "[.]", // Changed from [*] to avoid collision with Running
        }
    }

    // === Git/File Icons ===

    pub fn file_added(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰐕",
            IconMode::Unicode | IconMode::Ascii => "+",
        }
    }

    pub fn file_modified(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰦒",
            IconMode::Unicode | IconMode::Ascii => "~",
        }
    }

    pub fn file_deleted(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰍴",
            IconMode::Unicode | IconMode::Ascii => "-",
        }
    }

    pub fn git_branch(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰘬",
            IconMode::Unicode => "⎇",
            IconMode::Ascii => "@",
        }
    }

    pub fn git_commit(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰜘",
            IconMode::Unicode => "•",
            IconMode::Ascii => "o",
        }
    }

    // === Model Icons ===

    pub fn model_claude(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰚩",
            IconMode::Unicode => "🤖",
            IconMode::Ascii => "[C]",
        }
    }

    pub fn model_gemini(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󱗻",
            IconMode::Unicode => "💎",
            IconMode::Ascii => "[G]",
        }
    }

    pub fn model_codex(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰘦",
            IconMode::Unicode => "⌘",
            IconMode::Ascii => "[X]",
        }
    }

    // === Misc Icons ===

    pub fn help(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰋖",
            IconMode::Unicode | IconMode::Ascii => "?",
        }
    }

    pub fn settings(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰒓",
            IconMode::Unicode => "⚙",
            IconMode::Ascii => "*",
        }
    }

    pub fn folder(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰉋",
            IconMode::Unicode => "📁",
            IconMode::Ascii => "/",
        }
    }

    pub fn file(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰈔",
            IconMode::Unicode => "📄",
            IconMode::Ascii => "-",
        }
    }

    // === Spinner Frames (for animation) ===

    pub fn spinner_frames(&self) -> &'static [&'static str] {
        match self.mode {
            IconMode::Nerd => &["󰪞", "󰪟", "󰪠", "󰪡", "󰪢", "󰪣"],
            IconMode::Unicode => &["◐", "◓", "◑", "◒"],
            IconMode::Ascii => &["|", "/", "-", "\\"],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_nerd() {
        let icons = IconSet::default();
        assert_eq!(icons.mode(), IconMode::Nerd);
    }

    #[test]
    fn test_nerd_icons() {
        let icons = IconSet::new(IconMode::Nerd);
        assert_eq!(icons.success(), "󰄬");
        assert_eq!(icons.error(), "󰅖");
    }

    #[test]
    fn test_unicode_icons() {
        let icons = IconSet::new(IconMode::Unicode);
        assert_eq!(icons.success(), "✓");
        assert_eq!(icons.error(), "✗");
    }

    #[test]
    fn test_ascii_icons() {
        let icons = IconSet::new(IconMode::Ascii);
        assert_eq!(icons.success(), "[x]");
        assert_eq!(icons.error(), "[X]");
    }

    #[test]
    fn test_no_ascii_symbol_collisions() {
        let icons = IconSet::new(IconMode::Ascii);
        // Running and System should not collide
        assert_ne!(icons.running(), icons.event_system());
        // Running is [*], System is [.]
        assert_eq!(icons.running(), "[*]");
        assert_eq!(icons.event_system(), "[.]");
    }

    #[test]
    fn test_spinner_frames_count() {
        let nerd = IconSet::new(IconMode::Nerd);
        let unicode = IconSet::new(IconMode::Unicode);
        let ascii = IconSet::new(IconMode::Ascii);

        assert_eq!(nerd.spinner_frames().len(), 6);
        assert_eq!(unicode.spinner_frames().len(), 4);
        assert_eq!(ascii.spinner_frames().len(), 4);
    }
}
