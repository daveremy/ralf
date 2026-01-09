//! Footer hints widget for keybinding display.
//!
//! Uses input-first model format: `[/] Commands │ [Esc] Clear/Quit │ [Tab] Focus`
//!
//! The [`hints_for_state`] function generates phase-aware hints that change
//! based on the current thread phase and focused pane.

use ralf_engine::thread::PhaseKind;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::layout::{FocusedPane, ScreenMode};
use crate::theme::Theme;

/// A single keybinding hint.
#[derive(Debug, Clone)]
pub struct KeyHint {
    /// The key or key combination (e.g., "Tab", "Ctrl+Q").
    pub key: String,
    /// The action description (e.g., "Focus", "Quit").
    pub action: String,
}

impl KeyHint {
    /// Create a new key hint.
    pub fn new(key: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            action: action.into(),
        }
    }
}

/// Footer hints widget.
pub struct FooterHints<'a> {
    hints: &'a [KeyHint],
    theme: &'a Theme,
}

impl<'a> FooterHints<'a> {
    /// Create a new footer hints widget.
    pub fn new(hints: &'a [KeyHint], theme: &'a Theme) -> Self {
        Self { hints, theme }
    }

    /// Get the default hints for the shell (input-first model).
    ///
    /// All typing goes to input; commands via `/cmd` or modifier keys.
    pub fn default_hints() -> Vec<KeyHint> {
        vec![
            KeyHint::new("/", "Commands"),
            KeyHint::new("Esc", "Clear/Quit"),
            KeyHint::new("Tab", "Focus"),
            KeyHint::new("F1", "Help"),
        ]
    }
}

impl Widget for FooterHints<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut spans = Vec::new();

        for (i, hint) in self.hints.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" │ ", Style::default().fg(self.theme.muted)));
            }

            // Key in brackets
            spans.push(Span::styled("[", Style::default().fg(self.theme.muted)));
            spans.push(Span::styled(&hint.key, Style::default().fg(self.theme.primary)));
            spans.push(Span::styled("] ", Style::default().fg(self.theme.muted)));

            // Action
            spans.push(Span::styled(&hint.action, Style::default().fg(self.theme.subtext)));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line).style(Style::default().bg(self.theme.surface));
        paragraph.render(area, buf);
    }
}

/// Get hints for the current state.
///
/// Hints depend on:
/// - `phase`: Current thread phase (None = no thread)
/// - `screen_mode`: Current screen mode
/// - `focused`: Which pane has focus (used in Split mode)
/// - `show_models_panel`: Whether models panel is showing (enables 'r' refresh)
///
/// In `TimelineFocus` mode, effective focus is Timeline.
/// In `ContextFocus` mode, effective focus is Context.
/// In `Split` mode, use the `focused` parameter.
#[must_use]
pub fn hints_for_state(
    phase: Option<PhaseKind>,
    screen_mode: ScreenMode,
    focused: FocusedPane,
    show_models_panel: bool,
) -> Vec<KeyHint> {
    // Derive effective focus from screen mode
    let effective_focus = match screen_mode {
        ScreenMode::TimelineFocus => FocusedPane::Timeline,
        ScreenMode::ContextFocus => FocusedPane::Context,
        ScreenMode::Split => focused,
    };

    hints_for_focus(phase, effective_focus, show_models_panel)
}

/// Internal helper for generating hints based on effective focus.
///
/// Uses input-first model: character keys go to input, commands via `/` or modifiers.
fn hints_for_focus(
    phase: Option<PhaseKind>,
    focused: FocusedPane,
    show_models_panel: bool,
) -> Vec<KeyHint> {
    let mut hints = Vec::new();

    // Pane-specific hints first (using modifier keys for input-first model)
    match focused {
        FocusedPane::Timeline => {
            hints.push(KeyHint::new("Alt+j/k", "Navigate"));
            hints.push(KeyHint::new("Enter", "Send/Toggle"));
            hints.push(KeyHint::new("Ctrl+C", "Copy"));
        }
        FocusedPane::Context => {
            // Context hints depend on phase (use slash commands)
            hints.extend(context_hints_for_phase(phase));
        }
    }

    // Common hints (input-first model)
    hints.push(KeyHint::new("/", "Commands"));
    hints.push(KeyHint::new("Esc", "Clear/Quit"));
    hints.push(KeyHint::new("Tab", "Focus"));
    if show_models_panel && phase.is_none() {
        // Only show refresh when no thread and models panel visible
        hints.push(KeyHint::new("Ctrl+R", "Refresh"));
    }
    hints.push(KeyHint::new("F1", "Help"));

    hints
}

/// Get context-pane hints for a phase.
///
/// Uses slash commands for phase-specific actions in the input-first model.
fn context_hints_for_phase(phase: Option<PhaseKind>) -> Vec<KeyHint> {
    match phase {
        // Terminal states and no thread share the same hints
        None | Some(PhaseKind::Done | PhaseKind::Abandoned) => {
            vec![KeyHint::new("Enter", "Send message")]
        }
        Some(PhaseKind::Drafting | PhaseKind::Assessing) => vec![
            KeyHint::new("Enter", "Send"),
            KeyHint::new("/finalize", "Finalize"),
        ],
        Some(PhaseKind::Finalized) => {
            vec![KeyHint::new("Enter", "Run")]
        }
        Some(PhaseKind::Preflight) => vec![], // Auto-progresses
        Some(PhaseKind::PreflightFailed) => {
            vec![KeyHint::new("Enter", "Retry")]
        }
        Some(PhaseKind::Configuring) => {
            vec![KeyHint::new("Enter", "Start")]
        }
        // Note: Running can pause, Verifying cannot (different transitions)
        Some(PhaseKind::Running) => vec![KeyHint::new("/pause", "Pause")],
        Some(PhaseKind::Verifying) => {
            // Verifying can't transition to Paused; wait for completion
            vec![]
        }
        Some(PhaseKind::Paused) => vec![
            KeyHint::new("/resume", "Resume"),
            KeyHint::new("/cancel", "Cancel"),
        ],
        Some(PhaseKind::Stuck) => {
            vec![KeyHint::new("Enter", "Provide input")]
        }
        Some(PhaseKind::Implemented) => {
            vec![KeyHint::new("Enter", "Review")]
        }
        Some(PhaseKind::Polishing) => {
            vec![KeyHint::new("Enter", "Finish")]
        }
        Some(PhaseKind::PendingReview) => vec![
            KeyHint::new("/approve", "Approve"),
            KeyHint::new("/reject", "Reject"),
        ],
        Some(PhaseKind::Approved) => {
            vec![KeyHint::new("Enter", "Ready")]
        }
        Some(PhaseKind::ReadyToCommit) => {
            vec![KeyHint::new("Enter", "Commit")]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_hint_creation() {
        let hint = KeyHint::new("Tab", "Focus");
        assert_eq!(hint.key, "Tab");
        assert_eq!(hint.action, "Focus");
    }

    #[test]
    fn test_default_hints() {
        let hints = FooterHints::default_hints();
        assert_eq!(hints.len(), 4);

        // Input-first model hints
        assert!(hints.iter().any(|h| h.key == "/" && h.action == "Commands"));
        assert!(hints
            .iter()
            .any(|h| h.key == "Esc" && h.action == "Clear/Quit"));
        assert!(hints.iter().any(|h| h.key == "F1" && h.action == "Help"));
    }

    #[test]
    fn test_hints_for_state_no_thread_context_focus() {
        let hints = hints_for_state(None, ScreenMode::ContextFocus, FocusedPane::Context, true);

        // Should have send message hint
        assert!(hints
            .iter()
            .any(|h| h.key == "Enter" && h.action == "Send message"));
        // Should have refresh when models panel showing and no thread
        assert!(hints
            .iter()
            .any(|h| h.key == "Ctrl+R" && h.action == "Refresh"));
        // Common hints (input-first model)
        assert!(hints.iter().any(|h| h.key == "/" && h.action == "Commands"));
        assert!(hints
            .iter()
            .any(|h| h.key == "Esc" && h.action == "Clear/Quit"));
        assert!(hints.iter().any(|h| h.key == "Tab" && h.action == "Focus"));
        assert!(hints.iter().any(|h| h.key == "F1" && h.action == "Help"));
    }

    #[test]
    fn test_hints_for_state_timeline_focus() {
        let hints = hints_for_state(
            Some(PhaseKind::Running),
            ScreenMode::TimelineFocus,
            FocusedPane::Context, // Ignored in TimelineFocus mode
            false,
        );

        // Timeline hints should appear (with modifier keys)
        assert!(hints
            .iter()
            .any(|h| h.key == "Alt+j/k" && h.action == "Navigate"));
        assert!(hints
            .iter()
            .any(|h| h.key == "Enter" && h.action == "Send/Toggle"));
        assert!(hints.iter().any(|h| h.key == "Ctrl+C" && h.action == "Copy"));
    }

    #[test]
    fn test_hints_for_state_split_mode_respects_focused() {
        // Timeline focused in split mode
        let hints = hints_for_state(
            Some(PhaseKind::Drafting),
            ScreenMode::Split,
            FocusedPane::Timeline,
            false,
        );
        assert!(hints
            .iter()
            .any(|h| h.key == "Alt+j/k" && h.action == "Navigate"));

        // Context focused in split mode
        let hints = hints_for_state(
            Some(PhaseKind::Drafting),
            ScreenMode::Split,
            FocusedPane::Context,
            false,
        );
        assert!(hints.iter().any(|h| h.key == "Enter" && h.action == "Send"));
    }

    #[test]
    fn test_hints_for_state_running_phase() {
        let hints = hints_for_state(
            Some(PhaseKind::Running),
            ScreenMode::ContextFocus,
            FocusedPane::Context,
            false,
        );

        // Running phase should show /pause command
        assert!(hints
            .iter()
            .any(|h| h.key == "/pause" && h.action == "Pause"));
    }

    #[test]
    fn test_hints_for_state_paused_phase() {
        let hints = hints_for_state(
            Some(PhaseKind::Paused),
            ScreenMode::ContextFocus,
            FocusedPane::Context,
            false,
        );

        // Paused phase should show slash commands
        assert!(hints
            .iter()
            .any(|h| h.key == "/resume" && h.action == "Resume"));
        assert!(hints
            .iter()
            .any(|h| h.key == "/cancel" && h.action == "Cancel"));
    }

    #[test]
    fn test_hints_for_state_stuck_phase() {
        let hints = hints_for_state(
            Some(PhaseKind::Stuck),
            ScreenMode::ContextFocus,
            FocusedPane::Context,
            false,
        );

        // Stuck phase should show enter to provide input
        assert!(hints
            .iter()
            .any(|h| h.key == "Enter" && h.action == "Provide input"));
    }

    #[test]
    fn test_hints_for_state_pending_review() {
        let hints = hints_for_state(
            Some(PhaseKind::PendingReview),
            ScreenMode::ContextFocus,
            FocusedPane::Context,
            false,
        );

        // Pending review should show slash commands
        assert!(hints
            .iter()
            .any(|h| h.key == "/approve" && h.action == "Approve"));
        assert!(hints
            .iter()
            .any(|h| h.key == "/reject" && h.action == "Reject"));
    }

    #[test]
    fn test_hints_no_refresh_when_thread_active() {
        let hints = hints_for_state(
            Some(PhaseKind::Drafting),
            ScreenMode::ContextFocus,
            FocusedPane::Context,
            true, // models panel showing
        );

        // Should NOT have refresh when thread is active
        assert!(!hints
            .iter()
            .any(|h| h.key == "Ctrl+R" && h.action == "Refresh"));
    }

    #[test]
    fn test_verifying_phase_no_pause() {
        let hints = hints_for_state(
            Some(PhaseKind::Verifying),
            ScreenMode::ContextFocus,
            FocusedPane::Context,
            false,
        );

        // Verifying cannot pause (unlike Running)
        assert!(!hints
            .iter()
            .any(|h| h.key == "/pause" && h.action == "Pause"));
    }
}
