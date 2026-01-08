//! Thread state model for ralf workflows.
//!
//! A Thread represents a single work item (feature, fix, improvement) that
//! progresses through well-defined phases from initial idea to merged code.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A ralf workflow thread representing a single work item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    /// Unique thread identifier (UUID).
    pub id: String,

    /// Human-readable title for the thread.
    pub title: String,

    /// When the thread was created.
    pub created_at: DateTime<Utc>,

    /// When the thread was last updated.
    pub updated_at: DateTime<Utc>,

    /// Current phase in the workflow (SINGLE SOURCE OF TRUTH).
    pub phase: ThreadPhase,

    /// Current spec revision number (1-indexed).
    pub current_spec_revision: u32,

    /// ID of the current/latest run, if any.
    pub current_run_id: Option<String>,

    /// Workflow mode (Quick or Methodical).
    pub mode: ThreadMode,

    /// Configuration for implementation runs.
    pub run_config: Option<RunConfig>,

    /// Git baseline captured at Preflight for workspace reset.
    pub baseline: Option<GitBaseline>,
}

impl Thread {
    /// Create a new thread with the given title.
    ///
    /// The thread starts in `Drafting` phase with `Methodical` mode.
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.into(),
            created_at: now,
            updated_at: now,
            phase: ThreadPhase::default(),
            current_spec_revision: 1,
            current_run_id: None,
            mode: ThreadMode::default(),
            run_config: None,
            baseline: None,
        }
    }

    /// Check if the thread is in a terminal state (Done or Abandoned).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.phase,
            ThreadPhase::Done { .. } | ThreadPhase::Abandoned { .. }
        )
    }

    /// Get the phase category (1-5) for UI grouping.
    ///
    /// - 1: Spec Creation (Drafting, Assessing, Finalized)
    /// - 2: Implementation (Preflight, `PreflightFailed`, Configuring, Running, Paused, Verifying, Stuck, Implemented)
    /// - 3: Polish (Polishing)
    /// - 4: Review (`PendingReview`, Approved)
    /// - 5: Complete (`ReadyToCommit`, Done, Abandoned)
    pub fn phase_category(&self) -> u8 {
        match &self.phase {
            // Phase 1: Spec Creation
            ThreadPhase::Drafting | ThreadPhase::Assessing | ThreadPhase::Finalized => 1,

            // Phase 2: Implementation
            ThreadPhase::Preflight
            | ThreadPhase::PreflightFailed { .. }
            | ThreadPhase::Configuring
            | ThreadPhase::Running { .. }
            | ThreadPhase::Paused { .. }
            | ThreadPhase::Verifying { .. }
            | ThreadPhase::Stuck { .. }
            | ThreadPhase::Implemented => 2,

            // Phase 3: Polish
            ThreadPhase::Polishing => 3,

            // Phase 4: Review
            ThreadPhase::PendingReview | ThreadPhase::Approved => 4,

            // Phase 5: Complete
            ThreadPhase::ReadyToCommit
            | ThreadPhase::Done { .. }
            | ThreadPhase::Abandoned { .. } => 5,
        }
    }

    /// Get a human-readable display name for the current phase.
    pub fn phase_display_name(&self) -> &'static str {
        match &self.phase {
            ThreadPhase::Drafting => "Drafting",
            ThreadPhase::Assessing => "Assessing",
            ThreadPhase::Finalized => "Finalized",
            ThreadPhase::Preflight => "Preflight",
            ThreadPhase::PreflightFailed { .. } => "Preflight Failed",
            ThreadPhase::Configuring => "Configuring",
            ThreadPhase::Running { .. } => "Running",
            ThreadPhase::Paused { .. } => "Paused",
            ThreadPhase::Verifying { .. } => "Verifying",
            ThreadPhase::Stuck { .. } => "Stuck",
            ThreadPhase::Implemented => "Implemented",
            ThreadPhase::Polishing => "Polishing",
            ThreadPhase::PendingReview => "Pending Review",
            ThreadPhase::Approved => "Approved",
            ThreadPhase::ReadyToCommit => "Ready to Commit",
            ThreadPhase::Done { .. } => "Done",
            ThreadPhase::Abandoned { .. } => "Abandoned",
        }
    }
}

/// All possible phases a thread can be in.
///
/// This enum is the single source of truth for thread state.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum ThreadPhase {
    // Phase 1: Spec Creation
    /// Interactive drafting of the spec.
    #[default]
    Drafting,
    /// AI reviewing the spec for clarity/completeness.
    Assessing,
    /// Spec is locked and ready for implementation.
    Finalized,

    // Phase 2: Implementation
    /// Checking prerequisites before running.
    Preflight,
    /// Prerequisites check failed.
    PreflightFailed { reason: String },
    /// Setting up models, iterations, verifiers.
    Configuring,
    /// Autonomous implementation loop executing.
    Running { iteration: u32 },
    /// User interrupted the loop.
    Paused { iteration: u32 },
    /// Checking completion criteria.
    Verifying { iteration: u32 },
    /// Loop couldn't complete the task.
    Stuck { diagnosis: StuckDiagnosis },
    /// All criteria verified as passing.
    Implemented,

    // Phase 3: Polish
    /// Adding docs, tests, cleanup (optional).
    Polishing,

    // Phase 4: Review
    /// Changes ready for human inspection.
    PendingReview,
    /// Human confirmed changes are correct.
    Approved,

    // Phase 5: Complete
    /// Ready to commit/merge.
    ReadyToCommit,
    /// Committed and complete.
    Done { commit_sha: String },

    // Terminal
    /// Thread was abandoned.
    Abandoned { reason: String },
}

/// Workflow mode determining how much automation to use.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ThreadMode {
    /// Faster mode with defaults, fewer stops, but same human checkpoints.
    Quick,
    /// Step-by-step mode with more human oversight.
    #[default]
    Methodical,
}

/// Diagnosis information when a thread gets stuck.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StuckDiagnosis {
    /// Number of iterations attempted.
    pub iterations_attempted: u32,
    /// Models that were tried.
    pub models_tried: Vec<String>,
    /// Best number of criteria that passed in any iteration.
    pub best_criteria_passed: u32,
    /// Total number of criteria to pass.
    pub total_criteria: u32,
    /// Last error message, if any.
    pub last_error: Option<String>,
}

/// Captured git state for workspace reset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitBaseline {
    /// Branch name at capture time.
    pub branch: String,
    /// Commit SHA at capture time.
    pub commit_sha: String,
    /// When the baseline was captured.
    pub captured_at: DateTime<Utc>,
}

/// Configuration for implementation runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunConfig {
    /// Maximum number of iterations before giving up.
    pub max_iterations: u32,
    /// Models to use (in order of preference).
    pub models: Vec<String>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            max_iterations: 5,
            models: vec!["claude-sonnet".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_new() {
        let thread = Thread::new("Test feature");

        // Verify UUID format (8-4-4-4-12 hex digits)
        assert_eq!(thread.id.len(), 36);
        assert!(thread.id.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));

        // Verify defaults
        assert_eq!(thread.title, "Test feature");
        assert_eq!(thread.phase, ThreadPhase::Drafting);
        assert_eq!(thread.mode, ThreadMode::Methodical);
        assert_eq!(thread.current_spec_revision, 1);
        assert!(thread.current_run_id.is_none());
        assert!(thread.run_config.is_none());
        assert!(thread.baseline.is_none());
    }

    #[test]
    fn test_is_terminal() {
        let mut thread = Thread::new("Test");

        // Non-terminal states
        thread.phase = ThreadPhase::Drafting;
        assert!(!thread.is_terminal());

        thread.phase = ThreadPhase::Running { iteration: 1 };
        assert!(!thread.is_terminal());

        thread.phase = ThreadPhase::Stuck {
            diagnosis: StuckDiagnosis {
                iterations_attempted: 5,
                models_tried: vec!["claude".to_string()],
                best_criteria_passed: 2,
                total_criteria: 5,
                last_error: None,
            },
        };
        assert!(!thread.is_terminal());

        // Terminal states
        thread.phase = ThreadPhase::Done {
            commit_sha: "abc123".to_string(),
        };
        assert!(thread.is_terminal());

        thread.phase = ThreadPhase::Abandoned {
            reason: "Changed mind".to_string(),
        };
        assert!(thread.is_terminal());
    }

    #[test]
    fn test_phase_category() {
        let mut thread = Thread::new("Test");

        // Phase 1: Spec Creation
        thread.phase = ThreadPhase::Drafting;
        assert_eq!(thread.phase_category(), 1);
        thread.phase = ThreadPhase::Assessing;
        assert_eq!(thread.phase_category(), 1);
        thread.phase = ThreadPhase::Finalized;
        assert_eq!(thread.phase_category(), 1);

        // Phase 2: Implementation
        thread.phase = ThreadPhase::Preflight;
        assert_eq!(thread.phase_category(), 2);
        thread.phase = ThreadPhase::Running { iteration: 1 };
        assert_eq!(thread.phase_category(), 2);
        thread.phase = ThreadPhase::Implemented;
        assert_eq!(thread.phase_category(), 2);

        // Phase 3: Polish
        thread.phase = ThreadPhase::Polishing;
        assert_eq!(thread.phase_category(), 3);

        // Phase 4: Review
        thread.phase = ThreadPhase::PendingReview;
        assert_eq!(thread.phase_category(), 4);
        thread.phase = ThreadPhase::Approved;
        assert_eq!(thread.phase_category(), 4);

        // Phase 5: Complete
        thread.phase = ThreadPhase::ReadyToCommit;
        assert_eq!(thread.phase_category(), 5);
        thread.phase = ThreadPhase::Done {
            commit_sha: "abc".to_string(),
        };
        assert_eq!(thread.phase_category(), 5);
        thread.phase = ThreadPhase::Abandoned {
            reason: "test".to_string(),
        };
        assert_eq!(thread.phase_category(), 5);
    }

    #[test]
    fn test_phase_display_name() {
        let mut thread = Thread::new("Test");

        thread.phase = ThreadPhase::Drafting;
        assert_eq!(thread.phase_display_name(), "Drafting");

        thread.phase = ThreadPhase::Running { iteration: 3 };
        assert_eq!(thread.phase_display_name(), "Running");

        thread.phase = ThreadPhase::PreflightFailed {
            reason: "dirty".to_string(),
        };
        assert_eq!(thread.phase_display_name(), "Preflight Failed");

        thread.phase = ThreadPhase::PendingReview;
        assert_eq!(thread.phase_display_name(), "Pending Review");

        // Verify all phases return non-empty strings
        let all_phases = vec![
            ThreadPhase::Drafting,
            ThreadPhase::Assessing,
            ThreadPhase::Finalized,
            ThreadPhase::Preflight,
            ThreadPhase::PreflightFailed {
                reason: "x".to_string(),
            },
            ThreadPhase::Configuring,
            ThreadPhase::Running { iteration: 1 },
            ThreadPhase::Paused { iteration: 1 },
            ThreadPhase::Verifying { iteration: 1 },
            ThreadPhase::Stuck {
                diagnosis: StuckDiagnosis {
                    iterations_attempted: 1,
                    models_tried: vec![],
                    best_criteria_passed: 0,
                    total_criteria: 1,
                    last_error: None,
                },
            },
            ThreadPhase::Implemented,
            ThreadPhase::Polishing,
            ThreadPhase::PendingReview,
            ThreadPhase::Approved,
            ThreadPhase::ReadyToCommit,
            ThreadPhase::Done {
                commit_sha: "x".to_string(),
            },
            ThreadPhase::Abandoned {
                reason: "x".to_string(),
            },
        ];

        for phase in all_phases {
            thread.phase = phase;
            assert!(!thread.phase_display_name().is_empty());
        }
    }

    #[test]
    fn test_json_round_trip() {
        // Test Thread round-trip
        let thread = Thread::new("JSON test");
        let json = serde_json::to_string(&thread).expect("serialize thread");
        let restored: Thread = serde_json::from_str(&json).expect("deserialize thread");
        assert_eq!(thread.id, restored.id);
        assert_eq!(thread.title, restored.title);
        assert_eq!(thread.phase, restored.phase);
        assert_eq!(thread.mode, restored.mode);

        // Test Thread with complex phase
        let mut thread_stuck = Thread::new("Stuck test");
        thread_stuck.phase = ThreadPhase::Stuck {
            diagnosis: StuckDiagnosis {
                iterations_attempted: 5,
                models_tried: vec!["claude-sonnet".to_string(), "gpt-4".to_string()],
                best_criteria_passed: 3,
                total_criteria: 5,
                last_error: Some("Build failed".to_string()),
            },
        };
        let json = serde_json::to_string(&thread_stuck).expect("serialize stuck");
        let restored: Thread = serde_json::from_str(&json).expect("deserialize stuck");
        assert_eq!(thread_stuck.phase, restored.phase);

        // Test StuckDiagnosis round-trip
        let diagnosis = StuckDiagnosis {
            iterations_attempted: 3,
            models_tried: vec!["a".to_string(), "b".to_string()],
            best_criteria_passed: 2,
            total_criteria: 4,
            last_error: Some("error".to_string()),
        };
        let json = serde_json::to_string(&diagnosis).expect("serialize diagnosis");
        let restored: StuckDiagnosis = serde_json::from_str(&json).expect("deserialize diagnosis");
        assert_eq!(diagnosis, restored);

        // Test GitBaseline round-trip
        let baseline = GitBaseline {
            branch: "main".to_string(),
            commit_sha: "abc123def456".to_string(),
            captured_at: Utc::now(),
        };
        let json = serde_json::to_string(&baseline).expect("serialize baseline");
        let restored: GitBaseline = serde_json::from_str(&json).expect("deserialize baseline");
        assert_eq!(baseline.branch, restored.branch);
        assert_eq!(baseline.commit_sha, restored.commit_sha);

        // Test RunConfig round-trip
        let config = RunConfig {
            max_iterations: 10,
            models: vec!["model1".to_string(), "model2".to_string()],
        };
        let json = serde_json::to_string(&config).expect("serialize config");
        let restored: RunConfig = serde_json::from_str(&json).expect("deserialize config");
        assert_eq!(config, restored);
    }
}
