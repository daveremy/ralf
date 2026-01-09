//! ralf-engine: Headless engine for multi-model autonomous loops
//!
//! This crate provides the core orchestration logic for ralf, including:
//! - Configuration and state management
//! - Model adapters for CLI process execution
//! - Rate-limit detection and cooldown management
//! - Verification runners
//! - Changelog generation
//! - Chat/conversation management for Spec Studio

pub mod changelog;
pub mod chat;
pub mod config;
pub mod discovery;
pub mod git;
pub mod persistence;
pub mod preflight;
pub mod runner;
pub mod state;
pub mod thread;

// Re-export commonly used types
pub use changelog::{write_changelog_entry, ChangelogEntry, ChangelogError, IterationStatus};
pub use chat::{
    draft_has_promise, extract_draft_promise, extract_spec_from_response, invoke_chat,
    save_draft_snapshot, ChatContext, ChatError, ChatMessage, ChatResult, Role, Thread,
};
pub use config::{Config, ConfigError, ModelConfig, ModelSelection, VerifierConfig};
pub use discovery::{
    discover_model, discover_models, probe_model, DiscoveryResult, ModelInfo, ProbeResult,
};
pub use git::{GitError, GitSafety};
pub use persistence::{PersistenceError, ThreadStore, ThreadSummary};
pub use preflight::{run_preflight, PreflightCheck, PreflightResult};
pub use runner::{
    check_promise, extract_promise, get_git_info, hash_prompt, invoke_model, run_verifier,
    select_model, start_run, GitInfo, InvocationResult, RunConfig, RunEvent, RunHandle,
    RunnerError, VerifierResult,
};
pub use state::{Cooldowns, RunState, RunStatus, StateError};

/// Returns the engine version.
pub fn engine_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Parse completion criteria from a PROMPT.md string.
///
/// Looks for sections named "Requirements", "Completion Criteria", "Criteria",
/// or "Acceptance Criteria" and extracts bullet points from them.
#[allow(clippy::manual_map)]
pub fn parse_criteria(prompt: &str) -> Vec<String> {
    let mut criteria = Vec::new();
    let mut in_criteria_section = false;

    for line in prompt.lines() {
        let trimmed = line.trim();

        // Count header level (number of leading #)
        let header_level = trimmed.chars().take_while(|c| *c == '#').count();

        // Check for level-2 headers (## Section)
        if header_level == 2 {
            let header = trimmed.trim_start_matches('#').trim().to_lowercase();
            in_criteria_section = header.contains("requirement")
                || header.contains("criteria")
                || header.contains("acceptance")
                || header.contains("completion")
                || header.contains("verification");
            continue;
        }

        // If we hit a level-1 header (# Title), end the criteria section
        if header_level == 1 && in_criteria_section {
            in_criteria_section = false;
            continue;
        }

        // Level-3+ headers (### Subsection) are allowed within criteria sections

        // Extract bullet points in criteria section
        if in_criteria_section {
            // Match various bullet formats: -, *, [ ], [x], •
            let content = if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
                Some(rest)
            } else if let Some(rest) = trimmed.strip_prefix("- [x] ") {
                Some(rest)
            } else if let Some(rest) = trimmed.strip_prefix("- ") {
                Some(rest)
            } else if let Some(rest) = trimmed.strip_prefix("* ") {
                Some(rest)
            } else if let Some(rest) = trimmed.strip_prefix("• ") {
                Some(rest)
            } else {
                None
            };

            if let Some(text) = content {
                if !text.is_empty() {
                    criteria.push(text.to_string());
                }
            }
        }
    }

    criteria
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_version() {
        let version = engine_version();
        assert!(!version.is_empty());
        assert!(version.starts_with("0."));
    }

    #[test]
    fn test_parse_criteria() {
        let prompt = r#"
# Test Task

Do something cool.

## Requirements
- Create a file called `hello.txt`
- File should contain "Hello, World!"

## Instructions

Follow these steps...
"#;
        let criteria = parse_criteria(prompt);
        assert_eq!(criteria.len(), 2);
        assert_eq!(criteria[0], "Create a file called `hello.txt`");
        assert_eq!(criteria[1], "File should contain \"Hello, World!\"");
    }

    #[test]
    fn test_parse_criteria_checkbox_format() {
        let prompt = r#"
## Completion Criteria
- [ ] First thing
- [x] Already done
- [ ] Third thing
"#;
        let criteria = parse_criteria(prompt);
        assert_eq!(criteria.len(), 3);
        assert_eq!(criteria[0], "First thing");
        assert_eq!(criteria[1], "Already done");
        assert_eq!(criteria[2], "Third thing");
    }

    #[test]
    fn test_parse_criteria_empty() {
        let prompt = "# Just a title\n\nSome content without criteria.";
        let criteria = parse_criteria(prompt);
        assert!(criteria.is_empty());
    }

    #[test]
    fn test_parse_criteria_with_nested_headers() {
        let prompt = r#"
# Test Task

## Requirements

- First requirement

### Subsection

- Second requirement
- Third requirement

## Instructions

- Not a requirement
"#;
        let criteria = parse_criteria(prompt);
        assert_eq!(criteria.len(), 3);
        assert_eq!(criteria[0], "First requirement");
        assert_eq!(criteria[1], "Second requirement");
        assert_eq!(criteria[2], "Third requirement");
    }
}
