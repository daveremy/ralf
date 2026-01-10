//! Footer status bar widget.
//!
//! Minimal status bar format: `Split │ Timeline │ Drafting            [Tab] focus │ [?] help`
//!
//! Components:
//! - Screen mode (Split/Timeline/Canvas)
//! - Focused pane name (Timeline/Canvas/Input)
//! - Thread phase (if any)
//! - Minimal hints (Tab for focus, ? for help)

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

/// Footer status bar widget.
///
/// Shows: `Mode │ Focus │ Phase            [Tab] focus │ [?] help`
pub struct FooterHints<'a> {
    hints: &'a [KeyHint],
    theme: &'a Theme,
    screen_mode: Option<ScreenMode>,
    focused_pane: Option<FocusedPane>,
    phase: Option<PhaseKind>,
}

impl<'a> FooterHints<'a> {
    /// Create a new footer hints widget.
    pub fn new(hints: &'a [KeyHint], theme: &'a Theme) -> Self {
        Self {
            hints,
            theme,
            screen_mode: None,
            focused_pane: None,
            phase: None,
        }
    }

    /// Set screen mode to display.
    #[must_use]
    pub fn screen_mode(mut self, mode: ScreenMode) -> Self {
        self.screen_mode = Some(mode);
        self
    }

    /// Set focused pane to display.
    #[must_use]
    pub fn focused_pane(mut self, pane: FocusedPane) -> Self {
        self.focused_pane = Some(pane);
        self
    }

    /// Set phase to display.
    #[must_use]
    pub fn phase(mut self, phase: Option<PhaseKind>) -> Self {
        self.phase = phase;
        self
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

    /// Get minimal hints for the status bar.
    pub fn minimal_hints() -> Vec<KeyHint> {
        vec![
            KeyHint::new("Tab", "focus"),
            KeyHint::new("?", "help"),
        ]
    }

    /// Get pane-specific hints based on focused pane.
    ///
    /// `keyboard_enhanced` indicates whether the terminal supports Kitty keyboard protocol,
    /// which enables Ctrl+Enter for newlines. Falls back to Ctrl+J otherwise.
    pub fn pane_hints(
        focused: FocusedPane,
        show_models_panel: bool,
        keyboard_enhanced: bool,
    ) -> Vec<KeyHint> {
        match focused {
            FocusedPane::Timeline => vec![
                KeyHint::new("j/k", "scroll"),
                KeyHint::new("y", "copy"),
                KeyHint::new("\\", "canvas"),
                KeyHint::new("Tab", "focus"),
            ],
            FocusedPane::Context => {
                if show_models_panel {
                    vec![
                        KeyHint::new("r", "refresh"),
                        KeyHint::new("\\", "canvas"),
                        KeyHint::new("Tab", "focus"),
                    ]
                } else {
                    vec![
                        KeyHint::new("j/k", "scroll"),
                        KeyHint::new("y", "copy"),
                        KeyHint::new("\\", "canvas"),
                        KeyHint::new("Tab", "focus"),
                    ]
                }
            }
            FocusedPane::Input => {
                // Show Ctrl+Enter if terminal supports it, otherwise Ctrl+J
                let newline_hint = if keyboard_enhanced {
                    KeyHint::new("Ctrl+Enter", "newline")
                } else {
                    KeyHint::new("Ctrl+J", "newline")
                };
                vec![
                    KeyHint::new("Enter", "send"),
                    newline_hint,
                    KeyHint::new("/", "commands"),
                    KeyHint::new("Tab", "focus"),
                ]
            }
        }
    }
}

impl Widget for FooterHints<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut left_spans = Vec::new();
        let mut right_spans = Vec::new();

        // Left side: Mode │ Focus │ Phase
        if let Some(mode) = self.screen_mode {
            let mode_str = match mode {
                ScreenMode::Split => "Split",
                ScreenMode::TimelineFocus => "Timeline",
                ScreenMode::ContextFocus => "Canvas",
            };
            left_spans.push(Span::styled(mode_str, Style::default().fg(self.theme.subtext)));
        }

        if let Some(pane) = self.focused_pane {
            if !left_spans.is_empty() {
                left_spans.push(Span::styled(" │ ", Style::default().fg(self.theme.muted)));
            }
            let pane_str = match pane {
                FocusedPane::Timeline => "Timeline",
                FocusedPane::Context => "Canvas",
                FocusedPane::Input => "Input",
            };
            left_spans.push(Span::styled(pane_str, Style::default().fg(self.theme.primary)));
        }

        if let Some(phase) = self.phase {
            left_spans.push(Span::styled(" │ ", Style::default().fg(self.theme.muted)));
            let phase_str = format!("{phase:?}");
            left_spans.push(Span::styled(phase_str, Style::default().fg(self.theme.subtext)));
        }

        // Right side: hints (rendered right-aligned)
        for (i, hint) in self.hints.iter().enumerate() {
            if i > 0 {
                right_spans.push(Span::styled(" │ ", Style::default().fg(self.theme.muted)));
            }

            // Key in brackets
            right_spans.push(Span::styled("[", Style::default().fg(self.theme.muted)));
            right_spans.push(Span::styled(&hint.key, Style::default().fg(self.theme.primary)));
            right_spans.push(Span::styled("] ", Style::default().fg(self.theme.muted)));

            // Action
            right_spans.push(Span::styled(&hint.action, Style::default().fg(self.theme.subtext)));
        }

        // Calculate widths for alignment
        let left_width: usize = left_spans.iter().map(|s| s.content.len()).sum();
        let right_width: usize = right_spans.iter().map(|s| s.content.len()).sum();
        let total_width = area.width as usize;

        // Add padding between left and right
        let padding = total_width.saturating_sub(left_width + right_width);
        if padding > 0 {
            left_spans.push(Span::raw(" ".repeat(padding)));
        }

        // Combine left and right spans
        left_spans.extend(right_spans);

        let line = Line::from(left_spans);
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
            hints.push(KeyHint::new("j/k", "Navigate"));
            hints.push(KeyHint::new("Enter", "Toggle"));
            hints.push(KeyHint::new("y", "Copy"));
        }
        FocusedPane::Context => {
            // Context hints depend on what's showing
            if show_models_panel {
                hints.push(KeyHint::new("r", "Refresh"));
            } else {
                hints.extend(context_hints_for_phase(phase));
            }
        }
        FocusedPane::Input => {
            hints.push(KeyHint::new("Enter", "Send"));
            hints.push(KeyHint::new("Ctrl+J", "Newline"));
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

        // Should have refresh hint when models panel showing
        assert!(hints.iter().any(|h| h.key == "r" && h.action == "Refresh"));
        // Common hints
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
            FocusedPane::Timeline, // Timeline focused
            false,
        );

        // Timeline hints should appear (no modifier needed when Timeline focused)
        assert!(hints.iter().any(|h| h.key == "j/k" && h.action == "Navigate"));
        assert!(hints.iter().any(|h| h.key == "Enter" && h.action == "Toggle"));
        assert!(hints.iter().any(|h| h.key == "y" && h.action == "Copy"));
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
        assert!(hints.iter().any(|h| h.key == "j/k" && h.action == "Navigate"));

        // Context focused in split mode (with phase, not models panel)
        let hints = hints_for_state(
            Some(PhaseKind::Drafting),
            ScreenMode::Split,
            FocusedPane::Context,
            false,
        );
        assert!(hints.iter().any(|h| h.key == "Enter" && h.action == "Send"));

        // Input focused in split mode
        let hints = hints_for_state(
            Some(PhaseKind::Drafting),
            ScreenMode::Split,
            FocusedPane::Input,
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
