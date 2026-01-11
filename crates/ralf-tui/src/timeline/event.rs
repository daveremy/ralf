//! Timeline event types.
//!
//! Events represent all activity in a thread's history:
//! - Spec events: user input, spec changes
//! - Run events: model invocations, file changes
//! - Review events: verification results
//! - System events: model status, errors

use chrono::{DateTime, Local, Utc};

/// Maximum lines to show for expanded content.
pub const MAX_EXPANDED_LINES: usize = 10;

/// Lines per collapsed event (timestamp+badge line, content preview line).
pub const COLLAPSED_HEIGHT: usize = 2;

/// A timeline event representing thread activity.
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    /// Unique event ID (sequential).
    pub id: u64,
    /// When the event occurred (UTC).
    pub timestamp: DateTime<Utc>,
    /// Event type and content.
    pub kind: EventKind,
    /// Whether the event is collapsed (for multi-line content).
    pub collapsed: bool,
}

impl TimelineEvent {
    /// Create a new event with the given kind.
    pub fn new(id: u64, kind: EventKind) -> Self {
        let collapsed = kind.default_collapsed();
        Self {
            id,
            timestamp: Utc::now(),
            kind,
            collapsed,
        }
    }

    /// Create an event with a specific timestamp.
    pub fn with_timestamp(id: u64, timestamp: DateTime<Utc>, kind: EventKind) -> Self {
        let collapsed = kind.default_collapsed();
        Self {
            id,
            timestamp,
            kind,
            collapsed,
        }
    }

    /// Get the timestamp formatted for display (HH:MM in local time).
    pub fn time_str(&self) -> String {
        let local: DateTime<Local> = self.timestamp.into();
        local.format("%H:%M").to_string()
    }

    /// Get the badge text for this event (legacy, for tests).
    pub fn badge(&self) -> &'static str {
        match &self.kind {
            EventKind::Spec(_) => "SPEC",
            EventKind::Run(_) => "RUN",
            EventKind::Review(_) => "REVIEW",
            EventKind::System(_) => "SYS",
        }
    }

    /// Get the speaker symbol for compact display.
    ///
    /// Returns:
    /// - `›` for user messages
    /// - `●` for coordinator AI (Spec, Run)
    /// - `◦` for collaborator AI (Review)
    /// - `!` for system messages
    pub fn speaker_symbol(&self) -> &'static str {
        match &self.kind {
            EventKind::Spec(e) if e.is_user => "\u{203a}", // ›
            EventKind::Spec(_) | EventKind::Run(_) => "\u{25cf}", // ●
            EventKind::Review(_) => "\u{25cb}",            // ◦
            EventKind::System(_) => "!",
        }
    }

    /// Get the speaker symbol for ASCII mode.
    pub fn speaker_symbol_ascii(&self) -> &'static str {
        match &self.kind {
            EventKind::Spec(e) if e.is_user => ">",
            EventKind::Spec(_) | EventKind::Run(_) => "*",
            EventKind::Review(_) => "o",
            EventKind::System(_) => "!",
        }
    }

    /// Check if this event is from the user.
    pub fn is_user(&self) -> bool {
        matches!(&self.kind, EventKind::Spec(e) if e.is_user)
    }

    /// Get the model name for attribution (AI events only).
    pub fn model_attribution(&self) -> Option<String> {
        match &self.kind {
            EventKind::Spec(e) => e.model.clone(),
            EventKind::Run(e) => Some(format!("{} #{}", e.model, e.iteration)),
            EventKind::Review(e) => e.model.clone(),
            EventKind::System(_) => None,
        }
    }

    /// Get the attribution text (model name, "User", etc.).
    pub fn attribution(&self) -> String {
        match &self.kind {
            EventKind::Spec(e) => {
                if e.is_user {
                    "User".to_string()
                } else if let Some(ref model) = e.model {
                    model.clone()
                } else {
                    "System".to_string()
                }
            }
            EventKind::Run(e) => format!("{} #{}", e.model, e.iteration),
            EventKind::Review(_) | EventKind::System(_) => String::new(),
        }
    }

    /// Get the first line of content for display.
    pub fn summary(&self) -> String {
        match &self.kind {
            EventKind::Spec(e) => first_line(&e.content),
            EventKind::Run(e) => {
                if let Some(ref file) = e.file {
                    file.clone()
                } else {
                    first_line(&e.content)
                }
            }
            EventKind::Review(e) => {
                let icon = match e.result {
                    ReviewResult::Passed => "\u{2713}", // ✓
                    ReviewResult::Failed => "\u{2717}", // ✗
                    ReviewResult::Skipped => "-",
                };
                format!("{} {}", icon, e.criterion)
            }
            EventKind::System(e) => first_line(&e.message),
        }
    }

    /// Get all content lines (for expanded view).
    pub fn content_lines(&self) -> Vec<&str> {
        match &self.kind {
            EventKind::Spec(e) => e.content.lines().collect(),
            EventKind::Run(e) => e.content.lines().collect(),
            EventKind::Review(e) => {
                if let Some(ref details) = e.details {
                    details.lines().collect()
                } else {
                    vec![]
                }
            }
            EventKind::System(e) => e.message.lines().collect(),
        }
    }

    /// Check if this event is collapsible (has multiple lines).
    pub fn is_collapsible(&self) -> bool {
        self.content_lines().len() > 1
    }

    /// Get the display height of this event in lines.
    ///
    /// Accounts for collapsed/expanded state and content truncation.
    pub fn display_height(&self) -> usize {
        if self.collapsed || !self.is_collapsible() {
            // Header line + summary line
            COLLAPSED_HEIGHT
        } else {
            // Header line + expanded content lines + optional "[+N more]" line
            let content_lines = self.content_lines().len();
            let display_lines = content_lines.min(MAX_EXPANDED_LINES);
            let has_more = content_lines > MAX_EXPANDED_LINES;
            1 + display_lines + usize::from(has_more)
        }
    }

    /// Get the model name if this is a Run event.
    pub fn model(&self) -> Option<&str> {
        match &self.kind {
            EventKind::Run(e) => Some(&e.model),
            _ => None,
        }
    }

    /// Get the full content for copying to clipboard.
    ///
    /// Returns the complete, untruncated content of this event.
    pub fn copyable_content(&self) -> String {
        match &self.kind {
            EventKind::Spec(e) => e.content.clone(),
            EventKind::Run(e) => e.content.clone(),
            EventKind::Review(e) => {
                let result_str = match e.result {
                    ReviewResult::Passed => "PASSED",
                    ReviewResult::Failed => "FAILED",
                    ReviewResult::Skipped => "SKIPPED",
                };
                if let Some(ref details) = e.details {
                    format!("{}: {} - {}\n{}", result_str, e.criterion, result_str, details)
                } else {
                    format!("{}: {}", result_str, e.criterion)
                }
            }
            EventKind::System(e) => e.message.clone(),
        }
    }
}

/// Event type and content.
#[derive(Debug, Clone)]
pub enum EventKind {
    /// Spec-related events (user input, spec changes).
    Spec(SpecEvent),
    /// Run-related events (model invocations, iterations).
    Run(RunEvent),
    /// Review-related events (verification, approval).
    Review(ReviewEvent),
    /// System events (model status, errors).
    System(SystemEvent),
}

impl EventKind {
    /// Whether this event type should be collapsed by default.
    fn default_collapsed(&self) -> bool {
        matches!(self, Self::Run(_))
    }
}

/// Spec-related event.
#[derive(Debug, Clone)]
pub struct SpecEvent {
    /// User message or spec update.
    pub content: String,
    /// Whether this is user input vs system-generated.
    pub is_user: bool,
    /// Model name for assistant responses.
    pub model: Option<String>,
}

impl SpecEvent {
    /// Create a user spec event.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_user: true,
            model: None,
        }
    }

    /// Create an assistant (AI) spec event with model attribution.
    pub fn assistant(content: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_user: false,
            model: Some(model.into()),
        }
    }

    /// Create a system spec event.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_user: false,
            model: None,
        }
    }
}

/// Run-related event.
#[derive(Debug, Clone)]
pub struct RunEvent {
    /// Which model produced this.
    pub model: String,
    /// Iteration number (1-based).
    pub iteration: u32,
    /// Event content (file change, command output, etc.).
    pub content: String,
    /// Optional file path if this is a file change.
    pub file: Option<String>,
}

impl RunEvent {
    /// Create a run event.
    pub fn new(model: impl Into<String>, iteration: u32, content: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            iteration,
            content: content.into(),
            file: None,
        }
    }

    /// Create a file change event.
    pub fn file_change(
        model: impl Into<String>,
        iteration: u32,
        file: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            model: model.into(),
            iteration,
            content: content.into(),
            file: Some(file.into()),
        }
    }
}

/// Review-related event.
#[derive(Debug, Clone)]
pub struct ReviewEvent {
    /// Criterion being verified.
    pub criterion: String,
    /// Verification result.
    pub result: ReviewResult,
    /// Optional details.
    pub details: Option<String>,
    /// Model that performed the review.
    pub model: Option<String>,
}

impl ReviewEvent {
    /// Create a review event.
    pub fn new(criterion: impl Into<String>, result: ReviewResult) -> Self {
        Self {
            criterion: criterion.into(),
            result,
            details: None,
            model: None,
        }
    }

    /// Create a review event with model attribution.
    pub fn with_model(
        criterion: impl Into<String>,
        result: ReviewResult,
        model: impl Into<String>,
    ) -> Self {
        Self {
            criterion: criterion.into(),
            result,
            details: None,
            model: Some(model.into()),
        }
    }

    /// Create a review event with details.
    pub fn with_details(
        criterion: impl Into<String>,
        result: ReviewResult,
        details: impl Into<String>,
    ) -> Self {
        Self {
            criterion: criterion.into(),
            result,
            details: Some(details.into()),
            model: None,
        }
    }

    /// Create a review event with model and details.
    pub fn with_model_and_details(
        criterion: impl Into<String>,
        result: ReviewResult,
        model: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            criterion: criterion.into(),
            result,
            details: Some(details.into()),
            model: Some(model.into()),
        }
    }
}

/// Verification result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewResult {
    Passed,
    Failed,
    Skipped,
}

/// System event.
#[derive(Debug, Clone)]
pub struct SystemEvent {
    /// System message (model ready, error, etc.).
    pub message: String,
    /// Severity level.
    pub level: SystemLevel,
}

impl SystemEvent {
    /// Create an info-level system event.
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: SystemLevel::Info,
        }
    }

    /// Create a warning-level system event.
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: SystemLevel::Warning,
        }
    }

    /// Create an error-level system event.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: SystemLevel::Error,
        }
    }
}

/// System event severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemLevel {
    Info,
    Warning,
    Error,
}

/// Get the first line of a string.
fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_event_user() {
        let event = TimelineEvent::new(1, EventKind::Spec(SpecEvent::user("Add login feature")));
        assert_eq!(event.badge(), "SPEC");
        assert_eq!(event.attribution(), "User");
        assert_eq!(event.summary(), "Add login feature");
    }

    #[test]
    fn test_spec_event_assistant() {
        let event = TimelineEvent::new(
            1,
            EventKind::Spec(SpecEvent::assistant("Here's a draft spec...", "claude")),
        );
        assert_eq!(event.badge(), "SPEC");
        assert_eq!(event.attribution(), "claude");
        assert_eq!(event.summary(), "Here's a draft spec...");
    }

    #[test]
    fn test_run_event() {
        let event = TimelineEvent::new(
            2,
            EventKind::Run(RunEvent::new("claude", 1, "Running tests...")),
        );
        assert_eq!(event.badge(), "RUN");
        assert_eq!(event.attribution(), "claude #1");
        assert_eq!(event.summary(), "Running tests...");
        assert!(event.collapsed); // Run events default to collapsed
    }

    #[test]
    fn test_run_event_file_change() {
        let event = TimelineEvent::new(
            3,
            EventKind::Run(RunEvent::file_change(
                "gemini",
                2,
                "src/auth.rs +47",
                "+ pub fn login() {\n+     // impl\n+ }",
            )),
        );
        assert_eq!(event.summary(), "src/auth.rs +47");
        assert_eq!(event.model(), Some("gemini"));
    }

    #[test]
    fn test_review_event_passed() {
        let event = TimelineEvent::new(
            4,
            EventKind::Review(ReviewEvent::new("Tests pass", ReviewResult::Passed)),
        );
        assert_eq!(event.badge(), "REVIEW");
        assert!(event.summary().contains('\u{2713}')); // ✓
        assert!(event.summary().contains("Tests pass"));
    }

    #[test]
    fn test_review_event_failed() {
        let event = TimelineEvent::new(
            5,
            EventKind::Review(ReviewEvent::new("Lint clean", ReviewResult::Failed)),
        );
        assert!(event.summary().contains('\u{2717}')); // ✗
    }

    #[test]
    fn test_system_event() {
        let event = TimelineEvent::new(6, EventKind::System(SystemEvent::info("claude ready")));
        assert_eq!(event.badge(), "SYS");
        assert_eq!(event.summary(), "claude ready");
    }

    #[test]
    fn test_collapsible() {
        let single_line = TimelineEvent::new(
            1,
            EventKind::Spec(SpecEvent::user("Single line")),
        );
        assert!(!single_line.is_collapsible());

        let multi_line = TimelineEvent::new(
            2,
            EventKind::Spec(SpecEvent::user("Line 1\nLine 2\nLine 3")),
        );
        assert!(multi_line.is_collapsible());
        assert_eq!(multi_line.content_lines().len(), 3);
    }

    #[test]
    fn test_time_str_format() {
        let event = TimelineEvent::new(1, EventKind::System(SystemEvent::info("test")));
        let time_str = event.time_str();
        // Should be HH:MM format
        assert_eq!(time_str.len(), 5);
        assert!(time_str.contains(':'));
    }
}
