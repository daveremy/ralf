//! Slash command system for ralf TUI.
//!
//! Commands are invoked by typing `/` followed by the command name.
//! For example: `/help`, `/quit`, `/split`.

mod parse;

pub use parse::{parse_command, Command, CommandInfo, COMMANDS};

use ralf_engine::thread::PhaseKind;

/// Check if input starts with '/' (is a command).
pub fn is_command(input: &str) -> bool {
    input.trim().starts_with('/') && !input.trim().starts_with("//")
}

/// Check if input is an escaped slash (starts with //).
pub fn is_escaped_slash(input: &str) -> bool {
    input.trim().starts_with("//")
}

/// Unescape a slash command (// -> /).
pub fn unescape_slash(input: &str) -> String {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix("//") {
        format!("/{rest}")
    } else {
        input.to_string()
    }
}

/// Get command completions for autocomplete.
///
/// Returns commands that match the partial input, filtered by current phase.
pub fn get_completions(partial: &str, phase: Option<PhaseKind>) -> Vec<&'static CommandInfo> {
    let partial = partial.trim().to_lowercase();
    let partial = partial.strip_prefix('/').unwrap_or(&partial);

    COMMANDS
        .iter()
        .filter(|cmd| {
            // Match by name or aliases
            let matches = cmd.name.starts_with(partial)
                || cmd.aliases.iter().any(|a| a.starts_with(partial));

            if !matches {
                return false;
            }

            // Filter by phase availability
            if cmd.phase_specific {
                matches!(
                    (phase, cmd.name),
                    (Some(PhaseKind::PendingReview), "approve" | "reject")
                        | (Some(PhaseKind::Running), "pause" | "cancel")
                        | (Some(PhaseKind::Paused), "resume" | "cancel")
                        | (Some(PhaseKind::Drafting), "finalize" | "assess")
                )
            } else {
                true
            }
        })
        .collect()
}

/// Result of executing a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult {
    /// Command was handled successfully.
    Handled,
    /// Command requires an action from the shell.
    Action(CommandAction),
    /// Command is not available in current context.
    NotAvailable(String),
    /// Command failed with an error.
    Error(String),
}

/// Actions that commands can trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandAction {
    /// Quit the application.
    Quit,
    /// Show help overlay.
    ShowHelp,
    /// Set screen mode.
    SetScreenMode(crate::layout::ScreenMode),
    /// Refresh models.
    RefreshModels,
    /// Clear conversation.
    ClearConversation,
    /// Show message (toast).
    ShowMessage(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_command() {
        assert!(is_command("/help"));
        assert!(is_command("/quit"));
        assert!(is_command("  /help  "));
        assert!(!is_command("hello"));
        assert!(!is_command("//escaped"));
        assert!(!is_command(""));
    }

    #[test]
    fn test_is_escaped_slash() {
        assert!(is_escaped_slash("//etc/config"));
        assert!(is_escaped_slash("  //foo"));
        assert!(!is_escaped_slash("/help"));
        assert!(!is_escaped_slash("hello"));
    }

    #[test]
    fn test_unescape_slash() {
        assert_eq!(unescape_slash("//etc/config"), "/etc/config");
        assert_eq!(unescape_slash("//"), "/");
        assert_eq!(unescape_slash("/help"), "/help");
    }

    #[test]
    fn test_get_completions() {
        let completions = get_completions("/h", None);
        assert!(completions.iter().any(|c| c.name == "help"));

        let completions = get_completions("/q", None);
        assert!(completions.iter().any(|c| c.name == "quit"));

        let completions = get_completions("", None);
        assert!(!completions.is_empty());
    }

    #[test]
    fn test_phase_specific_completions() {
        // Approve should only show in PendingReview
        let completions = get_completions("/app", Some(PhaseKind::PendingReview));
        assert!(completions.iter().any(|c| c.name == "approve"));

        let completions = get_completions("/app", Some(PhaseKind::Drafting));
        assert!(!completions.iter().any(|c| c.name == "approve"));
    }
}
