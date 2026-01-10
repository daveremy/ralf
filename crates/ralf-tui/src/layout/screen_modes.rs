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
/// The UI has three focus targets:
/// - Timeline pane (left) - Event list navigation
/// - Context/Canvas pane (right) - Context-specific interaction
/// - Input area (bottom) - Text entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedPane {
    /// Timeline pane (left) has focus.
    #[default]
    Timeline,
    /// Context/Canvas pane (right) has focus - shows Models or Context content.
    Context,
    /// Input area (bottom) has focus - text entry.
    Input,
}

impl FocusedPane {
    /// Cycle focus to the next pane.
    ///
    /// In Split mode: Timeline → Context → Input → Timeline...
    /// In single-pane modes, use `cycle_for_mode` instead.
    #[must_use]
    pub fn cycle_next(self) -> Self {
        match self {
            Self::Timeline => Self::Context,
            Self::Context => Self::Input,
            Self::Input => Self::Timeline,
        }
    }

    /// Cycle focus to the previous pane.
    #[must_use]
    pub fn cycle_prev(self) -> Self {
        match self {
            Self::Timeline => Self::Input,
            Self::Context => Self::Timeline,
            Self::Input => Self::Context,
        }
    }

    /// Cycle focus for a specific screen mode.
    ///
    /// - Split: Timeline → Context → Input → Timeline
    /// - `TimelineFocus`: Timeline → Input → Timeline (skip Context)
    /// - `ContextFocus`: Context → Input → Context (skip Timeline)
    #[must_use]
    pub fn cycle_for_mode(self, mode: ScreenMode) -> Self {
        match mode {
            ScreenMode::Split => self.cycle_next(),
            ScreenMode::TimelineFocus => match self {
                Self::Timeline => Self::Input,
                Self::Input | Self::Context => Self::Timeline,
            },
            ScreenMode::ContextFocus => match self {
                Self::Context => Self::Input,
                Self::Input | Self::Timeline => Self::Context,
            },
        }
    }

    /// Toggle focus between the two panes (legacy, for backwards compatibility).
    /// Prefer `cycle_next` or `cycle_for_mode` for three-way focus.
    #[must_use]
    pub fn toggle(self) -> Self {
        match self {
            Self::Timeline => Self::Context,
            Self::Context | Self::Input => Self::Timeline,
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
    fn test_focus_cycle_next() {
        // Three-way cycle: Timeline → Context → Input → Timeline
        let focus = FocusedPane::Timeline;
        assert_eq!(focus.cycle_next(), FocusedPane::Context);
        assert_eq!(focus.cycle_next().cycle_next(), FocusedPane::Input);
        assert_eq!(
            focus.cycle_next().cycle_next().cycle_next(),
            FocusedPane::Timeline
        );
    }

    #[test]
    fn test_focus_cycle_prev() {
        // Reverse cycle: Timeline → Input → Context → Timeline
        let focus = FocusedPane::Timeline;
        assert_eq!(focus.cycle_prev(), FocusedPane::Input);
        assert_eq!(focus.cycle_prev().cycle_prev(), FocusedPane::Context);
        assert_eq!(
            focus.cycle_prev().cycle_prev().cycle_prev(),
            FocusedPane::Timeline
        );
    }

    #[test]
    fn test_focus_cycle_for_split_mode() {
        // Split mode: Timeline → Context → Input → Timeline
        let mode = ScreenMode::Split;
        let focus = FocusedPane::Timeline;
        assert_eq!(focus.cycle_for_mode(mode), FocusedPane::Context);
        assert_eq!(
            focus.cycle_for_mode(mode).cycle_for_mode(mode),
            FocusedPane::Input
        );
        assert_eq!(
            focus
                .cycle_for_mode(mode)
                .cycle_for_mode(mode)
                .cycle_for_mode(mode),
            FocusedPane::Timeline
        );
    }

    #[test]
    fn test_focus_cycle_for_timeline_focus_mode() {
        // TimelineFocus mode: Timeline → Input → Timeline (skip Context)
        let mode = ScreenMode::TimelineFocus;
        let focus = FocusedPane::Timeline;
        assert_eq!(focus.cycle_for_mode(mode), FocusedPane::Input);
        assert_eq!(
            focus.cycle_for_mode(mode).cycle_for_mode(mode),
            FocusedPane::Timeline
        );

        // Context should jump to Timeline in this mode
        assert_eq!(
            FocusedPane::Context.cycle_for_mode(mode),
            FocusedPane::Timeline
        );
    }

    #[test]
    fn test_focus_cycle_for_context_focus_mode() {
        // ContextFocus mode: Context → Input → Context (skip Timeline)
        let mode = ScreenMode::ContextFocus;
        let focus = FocusedPane::Context;
        assert_eq!(focus.cycle_for_mode(mode), FocusedPane::Input);
        assert_eq!(
            focus.cycle_for_mode(mode).cycle_for_mode(mode),
            FocusedPane::Context
        );

        // Timeline should jump to Context in this mode
        assert_eq!(
            FocusedPane::Timeline.cycle_for_mode(mode),
            FocusedPane::Context
        );
    }

    #[test]
    fn test_focus_toggle_legacy() {
        // Legacy toggle for backwards compatibility
        let focus = FocusedPane::Timeline;
        assert_eq!(focus.toggle(), FocusedPane::Context);
        assert_eq!(focus.toggle().toggle(), FocusedPane::Timeline);
        // Input toggles to Timeline
        assert_eq!(FocusedPane::Input.toggle(), FocusedPane::Timeline);
    }
}
