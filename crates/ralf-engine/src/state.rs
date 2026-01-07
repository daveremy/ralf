//! State management for ralf engine.
//!
//! This module handles run state persistence and cooldown tracking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Current run state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunState {
    /// Unique run identifier.
    pub run_id: Option<String>,

    /// Current iteration number (1-indexed).
    pub iteration: u32,

    /// Current status.
    pub status: RunStatus,

    /// Last model used (for round-robin).
    pub last_model_index: usize,

    /// When the run started (Unix timestamp).
    pub started_at: Option<u64>,

    /// When the run ended (Unix timestamp).
    pub ended_at: Option<u64>,
}

/// Run status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// No active run.
    #[default]
    Idle,
    /// Run is in progress.
    Running,
    /// Run completed successfully.
    Completed,
    /// Run was cancelled.
    Cancelled,
    /// Run failed with error.
    Failed,
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl RunState {
    /// Load state from a file.
    pub fn load(path: &Path) -> Result<Self, StateError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path).map_err(StateError::Io)?;
        serde_json::from_str(&content).map_err(StateError::Parse)
    }

    /// Save state to a file.
    pub fn save(&self, path: &Path) -> Result<(), StateError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(StateError::Io)?;
        }
        let content = serde_json::to_string_pretty(self).map_err(StateError::Serialize)?;
        std::fs::write(path, content).map_err(StateError::Io)
    }

    /// Start a new run.
    pub fn start_run(&mut self) -> String {
        let run_id = generate_run_id();
        self.run_id = Some(run_id.clone());
        self.iteration = 0;
        self.status = RunStatus::Running;
        self.last_model_index = 0;
        self.started_at = Some(current_timestamp());
        self.ended_at = None;
        run_id
    }

    /// Increment iteration counter.
    pub fn next_iteration(&mut self) {
        self.iteration += 1;
    }

    /// Mark run as completed.
    pub fn complete(&mut self) {
        self.status = RunStatus::Completed;
        self.ended_at = Some(current_timestamp());
    }

    /// Mark run as cancelled.
    pub fn cancel(&mut self) {
        self.status = RunStatus::Cancelled;
        self.ended_at = Some(current_timestamp());
    }

    /// Mark run as failed.
    pub fn fail(&mut self) {
        self.status = RunStatus::Failed;
        self.ended_at = Some(current_timestamp());
    }

    /// Check if a run is active.
    pub fn is_running(&self) -> bool {
        self.status == RunStatus::Running
    }
}

/// Cooldown tracking for models.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Cooldowns {
    /// Per-model cooldown entries.
    #[serde(flatten)]
    pub entries: HashMap<String, CooldownEntry>,
}

/// A single cooldown entry for a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CooldownEntry {
    /// Unix timestamp when cooldown expires.
    pub cooldown_until: u64,

    /// Reason for the cooldown.
    pub reason: String,

    /// When the cooldown was observed.
    pub observed_at: u64,
}

impl Cooldowns {
    /// Load cooldowns from a file.
    pub fn load(path: &Path) -> Result<Self, StateError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path).map_err(StateError::Io)?;
        let entries: HashMap<String, CooldownEntry> =
            serde_json::from_str(&content).map_err(StateError::Parse)?;
        Ok(Self { entries })
    }

    /// Save cooldowns to a file.
    pub fn save(&self, path: &Path) -> Result<(), StateError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(StateError::Io)?;
        }
        let content = serde_json::to_string_pretty(&self.entries).map_err(StateError::Serialize)?;
        std::fs::write(path, content).map_err(StateError::Io)
    }

    /// Check if a model is in cooldown.
    pub fn is_cooling(&self, model: &str) -> bool {
        self.entries
            .get(model)
            .is_some_and(|entry| entry.cooldown_until > current_timestamp())
    }

    /// Get remaining cooldown time in seconds for a model.
    pub fn remaining_seconds(&self, model: &str) -> Option<u64> {
        self.entries.get(model).and_then(|entry| {
            let now = current_timestamp();
            if entry.cooldown_until > now {
                Some(entry.cooldown_until - now)
            } else {
                None
            }
        })
    }

    /// Set cooldown for a model.
    pub fn set_cooldown(&mut self, model: &str, duration_seconds: u64, reason: &str) {
        let now = current_timestamp();
        self.entries.insert(
            model.to_string(),
            CooldownEntry {
                cooldown_until: now + duration_seconds,
                reason: reason.to_string(),
                observed_at: now,
            },
        );
    }

    /// Clear expired cooldowns.
    pub fn clear_expired(&mut self) {
        let now = current_timestamp();
        self.entries.retain(|_, entry| entry.cooldown_until > now);
    }

    /// Get the earliest cooldown expiry time.
    pub fn earliest_expiry(&self) -> Option<u64> {
        let now = current_timestamp();
        self.entries
            .values()
            .filter(|e| e.cooldown_until > now)
            .map(|e| e.cooldown_until)
            .min()
    }

    /// Get all models currently in cooldown.
    pub fn cooling_models(&self) -> Vec<&str> {
        let now = current_timestamp();
        self.entries
            .iter()
            .filter(|(_, entry)| entry.cooldown_until > now)
            .map(|(name, _)| name.as_str())
            .collect()
    }
}

/// Generate a unique run ID.
fn generate_run_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{timestamp:x}")
}

/// Get current Unix timestamp.
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Errors that can occur when working with state.
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Parse error.
    #[error("Parse error: {0}")]
    Parse(#[source] serde_json::Error),

    /// Serialize error.
    #[error("Serialize error: {0}")]
    Serialize(#[source] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_state_lifecycle() {
        let mut state = RunState::default();
        assert_eq!(state.status, RunStatus::Idle);
        assert!(!state.is_running());

        let run_id = state.start_run();
        assert!(!run_id.is_empty());
        assert!(state.is_running());
        assert_eq!(state.iteration, 0);

        state.next_iteration();
        assert_eq!(state.iteration, 1);

        state.complete();
        assert_eq!(state.status, RunStatus::Completed);
        assert!(!state.is_running());
    }

    #[test]
    fn test_cooldowns() {
        let mut cooldowns = Cooldowns::default();
        assert!(!cooldowns.is_cooling("claude"));

        cooldowns.set_cooldown("claude", 60, "rate limit");
        assert!(cooldowns.is_cooling("claude"));
        assert!(!cooldowns.is_cooling("codex"));

        let remaining = cooldowns.remaining_seconds("claude");
        assert!(remaining.is_some());
        assert!(remaining.unwrap() <= 60);

        let cooling = cooldowns.cooling_models();
        assert_eq!(cooling, vec!["claude"]);
    }

    #[test]
    fn test_cooldowns_serialization() {
        let mut cooldowns = Cooldowns::default();
        cooldowns.set_cooldown("claude", 60, "rate limit");

        let json = serde_json::to_string(&cooldowns.entries).unwrap();
        assert!(json.contains("claude"));
        assert!(json.contains("rate limit"));
    }
}
