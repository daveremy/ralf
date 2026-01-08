//! Chat module for Spec Studio conversations.
//!
//! This module provides types and functions for managing multi-turn
//! conversations with AI models, including thread persistence.

use crate::config::ModelConfig;
use crate::runner::RunnerError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;
use uuid::Uuid;

/// Role in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message (instructions to the model).
    System,
    /// User message.
    User,
    /// Assistant (model) response.
    Assistant,
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role of the message author.
    pub role: Role,
    /// Message content.
    pub content: String,
    /// Model name (for assistant messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Timestamp of the message.
    pub timestamp: DateTime<Utc>,
}

impl ChatMessage {
    /// Create a new user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            model: None,
            timestamp: Utc::now(),
        }
    }

    /// Create a new assistant message.
    pub fn assistant(content: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            model: Some(model.into()),
            timestamp: Utc::now(),
        }
    }

    /// Create a new system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            model: None,
            timestamp: Utc::now(),
        }
    }
}

/// Context for a chat invocation.
#[derive(Debug, Clone)]
pub struct ChatContext {
    /// Conversation history.
    pub messages: Vec<ChatMessage>,
    /// Current draft content.
    pub draft: String,
}

impl ChatContext {
    /// Create a new empty chat context.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            draft: String::new(),
        }
    }

    /// Build the prompt to send to the model.
    pub fn build_prompt(&self) -> String {
        use std::fmt::Write;

        let mut prompt = String::new();

        // System instructions
        prompt.push_str(SPEC_STUDIO_SYSTEM_PROMPT);
        prompt.push_str("\n\n");

        // Current draft
        if !self.draft.is_empty() {
            prompt.push_str("Current draft:\n---\n");
            prompt.push_str(&self.draft);
            prompt.push_str("\n---\n\n");
        }

        // Conversation history
        prompt.push_str("Conversation:\n");
        for msg in &self.messages {
            match msg.role {
                Role::System => {
                    let _ = write!(prompt, "[System]: {}\n\n", msg.content);
                }
                Role::User => {
                    let _ = write!(prompt, "User: {}\n\n", msg.content);
                }
                Role::Assistant => {
                    let model = msg.model.as_deref().unwrap_or("assistant");
                    let _ = write!(prompt, "{}: {}\n\n", model, msg.content);
                }
            }
        }

        prompt.push_str("\nRespond to the user's last message. If appropriate, suggest updates to the draft specification.\n");

        prompt
    }

    /// Add a user message.
    pub fn add_user_message(&mut self, content: impl Into<String>) {
        self.messages.push(ChatMessage::user(content));
    }

    /// Add an assistant message.
    pub fn add_assistant_message(&mut self, content: impl Into<String>, model: impl Into<String>) {
        self.messages.push(ChatMessage::assistant(content, model));
    }

    /// Get the last N messages (for context windowing).
    pub fn last_messages(&self, n: usize) -> Vec<&ChatMessage> {
        self.messages.iter().rev().take(n).rev().collect()
    }
}

impl Default for ChatContext {
    fn default() -> Self {
        Self::new()
    }
}

/// System prompt for Spec Studio conversations.
const SPEC_STUDIO_SYSTEM_PROMPT: &str = r#"You are a specification assistant helping the user create a clear task specification for an autonomous coding agent (like Claude Code, Codex, or Gemini CLI).

## Your Role
Help the user define WHAT they want built, not HOW to build it. The coding agent will figure out the implementation.

## Workflow
1. **Understand**: Ask 1-2 clarifying questions if the task is unclear
2. **Structure**: Help organize requirements into clear sections
3. **Refine**: Suggest concrete acceptance criteria the agent can verify
4. **Finalize**: When ready, output a complete specification starting with a markdown heading

## Specification Format
When you produce a draft specification, format it as:
```
# [Task Title]

## Goal
[One sentence describing the objective]

## Requirements
- [Specific, testable requirements as bullet points]

## Completion Criteria
- [ ] [Checkable items - each must be independently verifiable]

## Notes
[Any constraints, preferences, or context]

<promise>COMPLETE</promise>
```

The `<promise>COMPLETE</promise>` tag signals the spec is ready for the autonomous agent.

## Criteria Guidelines
IMPORTANT: Each completion criterion will be verified by an AI after the task runs. Write criteria that are:

- **Concrete and observable**: "File `src/utils.rs` exists" not "code is organized"
- **Independently checkable**: Each criterion can be verified on its own
- **Based on artifacts**: Files exist, contain specific content, tests pass, etc.

Good examples:
- [ ] File `hello.txt` exists with content "Hello, World!"
- [ ] Function `calculate_total` is exported from `src/lib.rs`
- [ ] All tests in `tests/` pass
- [ ] No TypeScript errors when running `tsc --noEmit`

Bad examples (too vague):
- [ ] Code works correctly
- [ ] Implementation is clean
- [ ] User experience is good

## Guidelines
- Be concise - agents work better with focused specs
- Prefer concrete over vague ("add a button that does X" vs "improve the UI")
- Include file paths if the user mentions them
- The user can finalize whenever the draft looks good"#;

/// Result of a chat invocation.
#[derive(Debug, Clone)]
pub struct ChatResult {
    /// Model that generated the response.
    pub model: String,
    /// Response content.
    pub content: String,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Whether the model suggested draft updates.
    pub has_draft_update: bool,
}

/// Invoke a model for a chat turn.
pub async fn invoke_chat(
    model: &ModelConfig,
    context: &ChatContext,
    timeout_secs: u64,
) -> Result<ChatResult, RunnerError> {
    let start = std::time::Instant::now();
    let prompt = context.build_prompt();

    // Build command - handle model-specific invocation patterns
    let mut cmd = Command::new(&model.command_argv[0]);

    // Gemini CLI uses positional argument for prompt, not stdin
    let uses_stdin = if model.name == "gemini" {
        // For gemini: gemini "prompt text"
        cmd.arg(&prompt);
        false
    } else {
        // For other models: pass command args, then write to stdin
        for arg in &model.command_argv[1..] {
            cmd.arg(arg);
        }
        true
    };

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = cmd.spawn().map_err(RunnerError::Spawn)?;

    // Write prompt to stdin if needed
    if uses_stdin {
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(prompt.as_bytes())
                .await
                .map_err(RunnerError::Io)?;
            drop(stdin);
        }
    }

    // Wait with timeout
    let timeout_duration = Duration::from_secs(timeout_secs);
    let result = timeout(timeout_duration, child.wait_with_output()).await;

    #[allow(clippy::cast_possible_truncation)]
    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            // Use stdout if available, otherwise stderr (some CLIs output to stderr)
            let response = if stdout.trim().is_empty() {
                stderr
            } else {
                stdout
            };

            Ok(ChatResult {
                model: model.name.clone(),
                content: response,
                duration_ms,
                has_draft_update: false, // Could be detected with heuristics later
            })
        }
        Ok(Err(e)) => Err(RunnerError::Io(e)),
        Err(_) => Err(RunnerError::Timeout(model.name.clone())),
    }
}

/// A conversation thread with persistence.
#[derive(Debug, Clone)]
pub struct Thread {
    /// Unique thread ID.
    pub id: String,
    /// Thread title (derived from first message or user-provided).
    pub title: String,
    /// Messages in the thread.
    pub messages: Vec<ChatMessage>,
    /// Current draft content.
    pub draft: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp.
    pub updated_at: DateTime<Utc>,
}

impl Thread {
    /// Create a new thread.
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title: "New Specification".into(),
            messages: Vec::new(),
            draft: String::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a thread with a specific ID (for loading).
    pub fn with_id(id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            title: "New Specification".into(),
            messages: Vec::new(),
            draft: String::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a message to the thread.
    pub fn add_message(&mut self, message: ChatMessage) {
        // Update title from first user message
        if self.messages.is_empty() && message.role == Role::User {
            self.title = message.content.chars().take(50).collect();
            if message.content.len() > 50 {
                self.title.push_str("...");
            }
        }
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    /// Convert to chat context for model invocation.
    pub fn to_context(&self) -> ChatContext {
        ChatContext {
            messages: self.messages.clone(),
            draft: self.draft.clone(),
        }
    }

    /// Save thread to a JSONL file.
    pub fn save(&self, spec_dir: &Path) -> Result<(), ChatError> {
        use std::io::Write;

        let threads_dir = spec_dir.join("threads");
        std::fs::create_dir_all(&threads_dir).map_err(ChatError::Io)?;

        let path = threads_dir.join(format!("{}.jsonl", self.id));
        let mut file = std::fs::File::create(&path).map_err(ChatError::Io)?;

        // Write metadata as first line
        let metadata = ThreadMetadata {
            id: self.id.clone(),
            title: self.title.clone(),
            draft: self.draft.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        };
        let meta_json = serde_json::to_string(&metadata).map_err(ChatError::Serialize)?;
        writeln!(file, "{meta_json}").map_err(ChatError::Io)?;

        // Write each message
        for msg in &self.messages {
            let json = serde_json::to_string(msg).map_err(ChatError::Serialize)?;
            writeln!(file, "{json}").map_err(ChatError::Io)?;
        }

        Ok(())
    }

    /// Load thread from a JSONL file.
    pub fn load(spec_dir: &Path, thread_id: &str) -> Result<Self, ChatError> {
        let path = spec_dir.join("threads").join(format!("{thread_id}.jsonl"));
        let content = std::fs::read_to_string(&path).map_err(ChatError::Io)?;

        let mut lines = content.lines();

        // First line is metadata
        let meta_line = lines.next().ok_or(ChatError::EmptyThread)?;
        let metadata: ThreadMetadata = serde_json::from_str(meta_line).map_err(ChatError::Parse)?;

        // Rest are messages
        let mut messages = Vec::new();
        for line in lines {
            if !line.trim().is_empty() {
                let msg: ChatMessage = serde_json::from_str(line).map_err(ChatError::Parse)?;
                messages.push(msg);
            }
        }

        Ok(Self {
            id: metadata.id,
            title: metadata.title,
            messages,
            draft: metadata.draft,
            created_at: metadata.created_at,
            updated_at: metadata.updated_at,
        })
    }

    /// List all thread IDs in the spec directory.
    pub fn list_threads(spec_dir: &Path) -> Result<Vec<String>, ChatError> {
        let threads_dir = spec_dir.join("threads");
        if !threads_dir.exists() {
            return Ok(Vec::new());
        }

        let mut ids = Vec::new();
        for entry in std::fs::read_dir(&threads_dir).map_err(ChatError::Io)? {
            let entry = entry.map_err(ChatError::Io)?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "jsonl") {
                if let Some(stem) = path.file_stem() {
                    ids.push(stem.to_string_lossy().to_string());
                }
            }
        }

        Ok(ids)
    }
}

impl Default for Thread {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread metadata (stored as first line of JSONL).
#[derive(Debug, Serialize, Deserialize)]
struct ThreadMetadata {
    id: String,
    title: String,
    draft: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Save a draft snapshot.
pub fn save_draft_snapshot(spec_dir: &Path, draft: &str) -> Result<String, ChatError> {
    let drafts_dir = spec_dir.join("drafts");
    std::fs::create_dir_all(&drafts_dir).map_err(ChatError::Io)?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("{timestamp}.md");
    let path = drafts_dir.join(&filename);

    std::fs::write(&path, draft).map_err(ChatError::Io)?;
    Ok(filename)
}

/// Check if a draft contains a promise tag.
pub fn draft_has_promise(draft: &str) -> bool {
    draft.contains("<promise>") && draft.contains("</promise>")
}

/// Extract the promise value from a draft.
pub fn extract_draft_promise(draft: &str) -> Option<String> {
    let re = regex::Regex::new(r"<promise>([^<]+)</promise>").ok()?;
    re.captures(draft)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Extract the spec/draft portion from an assistant response.
///
/// Looks for content between `---` markers that contains markdown spec structure,
/// or falls back to extracting the markdown portion starting with a `#` header.
pub fn extract_spec_from_response(response: &str) -> Option<String> {
    // First, try to find content between --- markers
    let parts: Vec<&str> = response.split("---").collect();

    if parts.len() >= 3 {
        // Content between first and second --- markers
        let between = parts[1].trim();
        // Check if it looks like a spec (has headers)
        if between.contains("# ") || between.starts_with('#') {
            return Some(between.to_string());
        }
    }

    // Fallback: find the first markdown header and extract from there
    // Look for a line starting with "# " (title) and extract until we hit
    // another "---" or conversational text
    let lines: Vec<&str> = response.lines().collect();
    let mut in_spec = false;
    let mut spec_lines = Vec::new();

    for line in lines {
        let trimmed = line.trim();

        // Start capturing when we see a markdown title
        if !in_spec && trimmed.starts_with("# ") {
            in_spec = true;
        }

        if in_spec {
            // Stop if we hit end markers
            if trimmed == "---" {
                // Check if we already have content
                if !spec_lines.is_empty() {
                    break;
                }
                // Otherwise skip this marker (it's the opening one)
                continue;
            }

            spec_lines.push(line);
        }
    }

    if spec_lines.is_empty() {
        return None;
    }

    // Trim trailing empty lines
    while spec_lines.last().is_some_and(|l| l.trim().is_empty()) {
        spec_lines.pop();
    }

    Some(spec_lines.join("\n"))
}

/// Errors that can occur in chat operations.
#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization error.
    #[error("Serialization error: {0}")]
    Serialize(#[source] serde_json::Error),

    /// JSON parse error.
    #[error("Parse error: {0}")]
    Parse(#[source] serde_json::Error),

    /// Empty thread file.
    #[error("Thread file is empty")]
    EmptyThread,

    /// Thread not found.
    #[error("Thread not found: {0}")]
    NotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_creation() {
        let user_msg = ChatMessage::user("Hello");
        assert_eq!(user_msg.role, Role::User);
        assert_eq!(user_msg.content, "Hello");
        assert!(user_msg.model.is_none());

        let assistant_msg = ChatMessage::assistant("Hi there!", "claude");
        assert_eq!(assistant_msg.role, Role::Assistant);
        assert_eq!(assistant_msg.model, Some("claude".into()));
    }

    #[test]
    fn test_chat_context_build_prompt() {
        let mut ctx = ChatContext::new();
        ctx.draft = "# Task\nBuild something".into();
        ctx.add_user_message("I want to build a CLI tool");

        let prompt = ctx.build_prompt();
        assert!(prompt.contains("Current draft:"));
        assert!(prompt.contains("# Task"));
        assert!(prompt.contains("User: I want to build a CLI tool"));
    }

    #[test]
    fn test_thread_title_from_first_message() {
        let mut thread = Thread::new();
        thread.add_message(ChatMessage::user("Build a markdown to HTML converter"));

        assert!(thread.title.starts_with("Build a markdown"));
    }

    #[test]
    fn test_draft_has_promise() {
        assert!(draft_has_promise(
            "Some text <promise>COMPLETE</promise> more text"
        ));
        assert!(!draft_has_promise("No promise here"));
        assert!(!draft_has_promise("<promise>incomplete"));
    }

    #[test]
    fn test_extract_draft_promise() {
        assert_eq!(
            extract_draft_promise("Text <promise>DONE</promise> more"),
            Some("DONE".into())
        );
        assert_eq!(extract_draft_promise("No promise"), None);
    }

    #[test]
    fn test_extract_spec_from_response() {
        // Test with --- delimited spec
        let response = r#"Here's a draft:

---

# My Tool

## Goal
Build something cool.

## Completion Criteria
- [ ] Works

---

What do you think?"#;

        let spec = extract_spec_from_response(response).unwrap();
        assert!(spec.starts_with("# My Tool"));
        assert!(spec.contains("## Goal"));
        assert!(spec.contains("## Completion Criteria"));
        assert!(!spec.contains("Here's a draft")); // No conversational text
        assert!(!spec.contains("What do you think")); // No trailing text

        // Test with spec starting with # but no delimiters
        let response2 = "Let me help!\n\n# Simple Spec\n\n## Goal\nDo stuff.\n\nLet me know!";
        let spec2 = extract_spec_from_response(response2).unwrap();
        assert!(spec2.starts_with("# Simple Spec"));

        // Test with no spec
        let response3 = "Just a regular message without any spec.";
        assert!(extract_spec_from_response(response3).is_none());
    }
}
