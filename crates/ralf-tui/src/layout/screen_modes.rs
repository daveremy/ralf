//! Screen display modes for the TUI.
//!
//! - Split: Timeline (40%) | Context (60%)
//! - `TimelineFocus`: Timeline (100%)
//! - `ContextFocus`: Context (100%)

/// Screen display modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScreenMode {
    /// Split view: Timeline (40%) | Context (60%).
    #[default]
    Split,
    /// Timeline only (100% width).
    TimelineFocus,
    /// Context only (100% width).
    ContextFocus,
}

/// Which pane has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedPane {
    /// Timeline pane has focus.
    #[default]
    Timeline,
    /// Context pane has focus.
    Context,
}

impl FocusedPane {
    /// Toggle focus to the other pane.
    #[must_use]
    pub fn toggle(self) -> Self {
        match self {
            Self::Timeline => Self::Context,
            Self::Context => Self::Timeline,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_screen_mode() {
        assert_eq!(ScreenMode::default(), ScreenMode::Split);
    }

    #[test]
    fn test_default_focused_pane() {
        assert_eq!(FocusedPane::default(), FocusedPane::Timeline);
    }

    #[test]
    fn test_focus_toggle() {
        let focus = FocusedPane::Timeline;
        assert_eq!(focus.toggle(), FocusedPane::Context);
        assert_eq!(focus.toggle().toggle(), FocusedPane::Timeline);
    }
}
