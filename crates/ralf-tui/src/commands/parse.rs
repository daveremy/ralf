//! Command parser and registry for slash commands.

/// A parsed slash command from user input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    // Global commands
    /// Show help overlay
    Help,
    /// Exit the application
    Quit,
    /// Switch to split view mode
    Split,
    /// Switch to timeline focus mode
    Focus,
    /// Switch to canvas/context focus mode
    Canvas,
    /// Refresh model status
    Refresh,
    /// Clear conversation
    Clear,
    /// Search timeline (future)
    Search(Option<String>),
    /// Switch active model
    Model(Option<String>),
    /// Copy last response to clipboard
    Copy,
    /// Open in $EDITOR
    Editor,

    // Phase-specific commands (stubs for now)
    /// Approve pending changes (`PendingReview` phase)
    Approve,
    /// Reject pending changes with optional feedback (`PendingReview` phase)
    Reject(Option<String>),
    /// Pause running operation (Running phase)
    Pause,
    /// Resume paused operation (Paused phase)
    Resume,
    /// Cancel current operation (Running/Paused phases)
    Cancel,
    /// Finalize the spec (Drafting phase)
    Finalize,
    /// Request AI assessment (Drafting phase)
    Assess,

    /// Unknown command
    Unknown(String),
}

/// Command metadata for help display and autocomplete.
#[derive(Debug, Clone)]
pub struct CommandInfo {
    /// Primary command name (without the /)
    pub name: &'static str,
    /// Alternative names for the command
    pub aliases: &'static [&'static str],
    /// Brief description for help
    pub description: &'static str,
    /// Keyboard shortcut if available
    pub keybinding: Option<&'static str>,
    /// Whether this command is only available in specific phases
    pub phase_specific: bool,
}

/// Static registry of all available commands.
pub static COMMANDS: &[CommandInfo] = &[
    // Global commands
    CommandInfo {
        name: "help",
        aliases: &["?"],
        description: "Show available commands",
        keybinding: Some("F1"),
        phase_specific: false,
    },
    CommandInfo {
        name: "quit",
        aliases: &["q"],
        description: "Exit ralf",
        keybinding: None,
        phase_specific: false,
    },
    CommandInfo {
        name: "exit",
        aliases: &[],
        description: "Exit ralf",
        keybinding: None,
        phase_specific: false,
    },
    CommandInfo {
        name: "split",
        aliases: &["1"],
        description: "Split view mode",
        keybinding: Some("Alt+1"),
        phase_specific: false,
    },
    CommandInfo {
        name: "focus",
        aliases: &["2"],
        description: "Focus conversation mode",
        keybinding: Some("Alt+2"),
        phase_specific: false,
    },
    CommandInfo {
        name: "canvas",
        aliases: &["3"],
        description: "Focus canvas mode",
        keybinding: Some("Alt+3"),
        phase_specific: false,
    },
    CommandInfo {
        name: "refresh",
        aliases: &[],
        description: "Refresh model status",
        keybinding: Some("Ctrl+R"),
        phase_specific: false,
    },
    CommandInfo {
        name: "clear",
        aliases: &[],
        description: "Clear conversation",
        keybinding: Some("Ctrl+L"),
        phase_specific: false,
    },
    CommandInfo {
        name: "search",
        aliases: &["find"],
        description: "Search timeline",
        keybinding: Some("Ctrl+F"),
        phase_specific: false,
    },
    CommandInfo {
        name: "model",
        aliases: &[],
        description: "Switch active model",
        keybinding: None,
        phase_specific: false,
    },
    CommandInfo {
        name: "copy",
        aliases: &[],
        description: "Copy last response to clipboard",
        keybinding: None,
        phase_specific: false,
    },
    CommandInfo {
        name: "editor",
        aliases: &[],
        description: "Open in $EDITOR",
        keybinding: None,
        phase_specific: false,
    },
    // Phase-specific commands
    CommandInfo {
        name: "approve",
        aliases: &["a"],
        description: "Approve pending changes",
        keybinding: None,
        phase_specific: true,
    },
    CommandInfo {
        name: "reject",
        aliases: &["r"],
        description: "Reject with optional feedback",
        keybinding: None,
        phase_specific: true,
    },
    CommandInfo {
        name: "pause",
        aliases: &[],
        description: "Pause running operation",
        keybinding: None,
        phase_specific: true,
    },
    CommandInfo {
        name: "resume",
        aliases: &[],
        description: "Resume paused operation",
        keybinding: None,
        phase_specific: true,
    },
    CommandInfo {
        name: "cancel",
        aliases: &[],
        description: "Cancel current operation",
        keybinding: None,
        phase_specific: true,
    },
    CommandInfo {
        name: "finalize",
        aliases: &[],
        description: "Finalize the spec",
        keybinding: None,
        phase_specific: true,
    },
    CommandInfo {
        name: "assess",
        aliases: &[],
        description: "Request AI assessment",
        keybinding: None,
        phase_specific: true,
    },
];

/// Parse a slash command from user input.
///
/// Returns `None` if the input is not a valid command format.
/// Returns `Command::Unknown` if the command is not recognized.
///
/// # Examples
///
/// ```
/// use ralf_tui::commands::{parse_command, Command};
///
/// assert!(matches!(parse_command("/help"), Some(Command::Help)));
/// assert!(matches!(parse_command("/q"), Some(Command::Quit)));
/// assert!(matches!(parse_command("/model gpt-4"), Some(Command::Model(Some(_)))));
/// ```
pub fn parse_command(input: &str) -> Option<Command> {
    let input = input.trim();

    // Must start with /
    if !input.starts_with('/') {
        return None;
    }

    // Skip the leading /
    let content = &input[1..];

    // Split into command and arguments
    let (cmd_str, args) = match content.find(char::is_whitespace) {
        Some(idx) => {
            let (c, a) = content.split_at(idx);
            (c.to_lowercase(), Some(a.trim().to_string()))
        }
        None => (content.to_lowercase(), None),
    };

    // Map command string to Command enum
    Some(match cmd_str.as_str() {
        // Help
        "help" | "?" => Command::Help,

        // Quit
        "quit" | "q" | "exit" => Command::Quit,

        // Screen modes
        "split" | "1" => Command::Split,
        "focus" | "2" => Command::Focus,
        "canvas" | "3" => Command::Canvas,

        // Actions
        "refresh" => Command::Refresh,
        "clear" => Command::Clear,
        "search" | "find" => Command::Search(args),
        "model" => Command::Model(args),
        "copy" => Command::Copy,
        "editor" => Command::Editor,

        // Phase-specific
        "approve" | "a" => Command::Approve,
        "reject" | "r" => Command::Reject(args),
        "pause" => Command::Pause,
        "resume" => Command::Resume,
        "cancel" => Command::Cancel,
        "finalize" => Command::Finalize,
        "assess" => Command::Assess,

        // Unknown
        other => Command::Unknown(other.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_help_commands() {
        assert!(matches!(parse_command("/help"), Some(Command::Help)));
        assert!(matches!(parse_command("/?"), Some(Command::Help)));
        assert!(matches!(parse_command("/HELP"), Some(Command::Help)));
        assert!(matches!(parse_command("  /help  "), Some(Command::Help)));
    }

    #[test]
    fn test_parse_quit_commands() {
        assert!(matches!(parse_command("/quit"), Some(Command::Quit)));
        assert!(matches!(parse_command("/q"), Some(Command::Quit)));
        assert!(matches!(parse_command("/exit"), Some(Command::Quit)));
    }

    #[test]
    fn test_parse_screen_mode_commands() {
        assert!(matches!(parse_command("/split"), Some(Command::Split)));
        assert!(matches!(parse_command("/1"), Some(Command::Split)));
        assert!(matches!(parse_command("/focus"), Some(Command::Focus)));
        assert!(matches!(parse_command("/2"), Some(Command::Focus)));
        assert!(matches!(parse_command("/canvas"), Some(Command::Canvas)));
        assert!(matches!(parse_command("/3"), Some(Command::Canvas)));
    }

    #[test]
    fn test_parse_action_commands() {
        assert!(matches!(parse_command("/refresh"), Some(Command::Refresh)));
        assert!(matches!(parse_command("/clear"), Some(Command::Clear)));
        assert!(matches!(parse_command("/copy"), Some(Command::Copy)));
        assert!(matches!(parse_command("/editor"), Some(Command::Editor)));
    }

    #[test]
    fn test_parse_commands_with_args() {
        match parse_command("/search foo bar") {
            Some(Command::Search(Some(s))) => assert_eq!(s, "foo bar"),
            other => panic!("Expected Search with args, got {:?}", other),
        }

        match parse_command("/model gpt-4") {
            Some(Command::Model(Some(s))) => assert_eq!(s, "gpt-4"),
            other => panic!("Expected Model with args, got {:?}", other),
        }

        match parse_command("/reject This needs more work") {
            Some(Command::Reject(Some(s))) => assert_eq!(s, "This needs more work"),
            other => panic!("Expected Reject with args, got {:?}", other),
        }

        // Commands without args
        match parse_command("/search") {
            Some(Command::Search(None)) => {}
            other => panic!("Expected Search without args, got {:?}", other),
        }

        match parse_command("/model") {
            Some(Command::Model(None)) => {}
            other => panic!("Expected Model without args, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_phase_specific_commands() {
        assert!(matches!(parse_command("/approve"), Some(Command::Approve)));
        assert!(matches!(parse_command("/a"), Some(Command::Approve)));
        assert!(matches!(parse_command("/pause"), Some(Command::Pause)));
        assert!(matches!(parse_command("/resume"), Some(Command::Resume)));
        assert!(matches!(parse_command("/cancel"), Some(Command::Cancel)));
        assert!(matches!(parse_command("/finalize"), Some(Command::Finalize)));
        assert!(matches!(parse_command("/assess"), Some(Command::Assess)));
    }

    #[test]
    fn test_parse_unknown_command() {
        match parse_command("/foobar") {
            Some(Command::Unknown(s)) => assert_eq!(s, "foobar"),
            other => panic!("Expected Unknown, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_non_command() {
        assert!(parse_command("hello").is_none());
        assert!(parse_command("").is_none());
        assert!(parse_command("   ").is_none());
    }

    #[test]
    fn test_commands_registry() {
        // Verify registry has expected entries
        assert!(COMMANDS.iter().any(|c| c.name == "help"));
        assert!(COMMANDS.iter().any(|c| c.name == "quit"));
        assert!(COMMANDS.iter().any(|c| c.name == "exit"));
        assert!(COMMANDS.iter().any(|c| c.name == "approve" && c.phase_specific));

        // Verify aliases are set up
        let quit_cmd = COMMANDS.iter().find(|c| c.name == "quit").unwrap();
        assert!(quit_cmd.aliases.contains(&"q"));
    }
}
