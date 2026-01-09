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
///
/// The UI has two main panes: Timeline (left) and Context (right).
/// The Context pane shows either the Models panel or Context content
/// depending on state, but focus is simply left vs right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedPane {
    /// Timeline pane (left) has focus.
    #[default]
    Timeline,
    /// Context pane (right) has focus - shows Models or Context content.
    Context,
}

impl FocusedPane {
    /// Toggle focus between the two panes.
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
