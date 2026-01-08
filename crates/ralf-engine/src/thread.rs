//! Thread state model for ralf workflows.
//!
//! A Thread represents a single work item (feature, fix, improvement) that
//! progresses through well-defined phases from initial idea to merged code.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Error returned when a state transition is invalid.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum TransitionError {
    /// The transition is not allowed by the state machine.
    #[error("Cannot transition from {from} to {to}: {reason}")]
    InvalidTransition {
        from: String,
        to: String,
        reason: String,
    },

    /// Cannot transition from a terminal state.
    #[error("Cannot transition from terminal state: {0}")]
    FromTerminalState(String),
}

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
        self.phase.display_name()
    }

    /// Check if transition to target phase is valid per state machine.
    ///
    /// Returns `Ok(())` if the transition is valid, or an error explaining why not.
    /// This validates the phase kind (discriminant), not internal data consistency.
    pub fn can_transition_to(&self, target: &ThreadPhase) -> Result<(), TransitionError> {
        // Cannot transition from terminal states
        if self.is_terminal() {
            return Err(TransitionError::FromTerminalState(
                self.phase.display_name().to_string(),
            ));
        }

        // Check if transition is valid based on current phase
        let valid = self.phase.valid_transitions();
        let target_kind = target.kind();

        if valid.contains(&target_kind) {
            Ok(())
        } else {
            Err(TransitionError::InvalidTransition {
                from: self.phase.display_name().to_string(),
                to: target.display_name().to_string(),
                reason: format!(
                    "Valid transitions from {} are: {:?}",
                    self.phase.display_name(),
                    valid
                ),
            })
        }
    }

    /// Execute transition: validates, updates phase, updates timestamp.
    ///
    /// Returns error if transition is invalid, leaving state unchanged.
    pub fn transition_to(&mut self, target: ThreadPhase) -> Result<(), TransitionError> {
        self.can_transition_to(&target)?;
        self.phase = target;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Get all valid next phases from current phase.
    ///
    /// Always includes `Abandoned` for non-terminal states.
    /// Returns empty vec for terminal states.
    /// Returned phases have sensible default data (caller populates actual data).
    pub fn available_transitions(&self) -> Vec<ThreadPhase> {
        if self.is_terminal() {
            return vec![];
        }

        self.phase
            .valid_transitions()
            .into_iter()
            .map(PhaseKind::to_phase_with_defaults)
            .collect()
    }

    /// Check if transitioning to target requires workspace reset.
    ///
    /// Returns true for backward transitions that discard implementation work:
    /// - `Stuck → Drafting`
    /// - `PendingReview → Drafting`
    pub fn requires_workspace_reset(&self, target: &ThreadPhase) -> bool {
        matches!(
            (&self.phase, target),
            (ThreadPhase::Stuck { .. } | ThreadPhase::PendingReview, ThreadPhase::Drafting)
        )
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

/// Phase kind for comparing discriminants without data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhaseKind {
    Drafting,
    Assessing,
    Finalized,
    Preflight,
    PreflightFailed,
    Configuring,
    Running,
    Paused,
    Verifying,
    Stuck,
    Implemented,
    Polishing,
    PendingReview,
    Approved,
    ReadyToCommit,
    Done,
    Abandoned,
}

impl std::fmt::Display for PhaseKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl PhaseKind {
    /// Convert to a `ThreadPhase` with sensible default data.
    pub fn to_phase_with_defaults(self) -> ThreadPhase {
        match self {
            PhaseKind::Drafting => ThreadPhase::Drafting,
            PhaseKind::Assessing => ThreadPhase::Assessing,
            PhaseKind::Finalized => ThreadPhase::Finalized,
            PhaseKind::Preflight => ThreadPhase::Preflight,
            PhaseKind::PreflightFailed => ThreadPhase::PreflightFailed {
                reason: String::new(),
            },
            PhaseKind::Configuring => ThreadPhase::Configuring,
            PhaseKind::Running => ThreadPhase::Running { iteration: 1 },
            PhaseKind::Paused => ThreadPhase::Paused { iteration: 1 },
            PhaseKind::Verifying => ThreadPhase::Verifying { iteration: 1 },
            PhaseKind::Stuck => ThreadPhase::Stuck {
                diagnosis: StuckDiagnosis {
                    iterations_attempted: 0,
                    models_tried: vec![],
                    best_criteria_passed: 0,
                    total_criteria: 0,
                    last_error: None,
                },
            },
            PhaseKind::Implemented => ThreadPhase::Implemented,
            PhaseKind::Polishing => ThreadPhase::Polishing,
            PhaseKind::PendingReview => ThreadPhase::PendingReview,
            PhaseKind::Approved => ThreadPhase::Approved,
            PhaseKind::ReadyToCommit => ThreadPhase::ReadyToCommit,
            PhaseKind::Done => ThreadPhase::Done {
                commit_sha: String::new(),
            },
            PhaseKind::Abandoned => ThreadPhase::Abandoned {
                reason: String::new(),
            },
        }
    }
}

impl ThreadPhase {
    /// Get the phase kind (discriminant) for comparison.
    pub fn kind(&self) -> PhaseKind {
        match self {
            ThreadPhase::Drafting => PhaseKind::Drafting,
            ThreadPhase::Assessing => PhaseKind::Assessing,
            ThreadPhase::Finalized => PhaseKind::Finalized,
            ThreadPhase::Preflight => PhaseKind::Preflight,
            ThreadPhase::PreflightFailed { .. } => PhaseKind::PreflightFailed,
            ThreadPhase::Configuring => PhaseKind::Configuring,
            ThreadPhase::Running { .. } => PhaseKind::Running,
            ThreadPhase::Paused { .. } => PhaseKind::Paused,
            ThreadPhase::Verifying { .. } => PhaseKind::Verifying,
            ThreadPhase::Stuck { .. } => PhaseKind::Stuck,
            ThreadPhase::Implemented => PhaseKind::Implemented,
            ThreadPhase::Polishing => PhaseKind::Polishing,
            ThreadPhase::PendingReview => PhaseKind::PendingReview,
            ThreadPhase::Approved => PhaseKind::Approved,
            ThreadPhase::ReadyToCommit => PhaseKind::ReadyToCommit,
            ThreadPhase::Done { .. } => PhaseKind::Done,
            ThreadPhase::Abandoned { .. } => PhaseKind::Abandoned,
        }
    }

    /// Get a human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
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

    /// Get valid transitions from this phase per the state machine.
    ///
    /// All non-terminal phases can transition to Abandoned.
    #[allow(clippy::enum_glob_use)] // Glob import improves readability here
    pub fn valid_transitions(&self) -> Vec<PhaseKind> {
        use PhaseKind::*;

        match self {
            // Phase 1: Spec Creation
            ThreadPhase::Drafting => vec![Assessing, Finalized, Abandoned],
            ThreadPhase::Assessing => vec![Drafting, Finalized, Abandoned],
            ThreadPhase::Finalized => vec![Drafting, Preflight, Abandoned],

            // Phase 2: Implementation
            ThreadPhase::Preflight => vec![Configuring, PreflightFailed, Abandoned],
            ThreadPhase::PreflightFailed { .. } => vec![Preflight, Drafting, Abandoned],
            ThreadPhase::Configuring => vec![Running, Abandoned],
            ThreadPhase::Running { .. } => vec![Verifying, Paused, Stuck, Abandoned],
            ThreadPhase::Paused { .. } => vec![Running, Configuring, Abandoned],
            ThreadPhase::Verifying { .. } => vec![Running, Stuck, Implemented, Abandoned],
            ThreadPhase::Stuck { .. } => vec![Configuring, Running, Drafting, Abandoned],
            ThreadPhase::Implemented => vec![Polishing, PendingReview, Abandoned],

            // Phase 3: Polish
            ThreadPhase::Polishing => vec![Implemented, Abandoned],

            // Phase 4: Review
            ThreadPhase::PendingReview => vec![Approved, Running, Drafting, Abandoned],
            ThreadPhase::Approved => vec![ReadyToCommit, Abandoned],

            // Phase 5: Complete
            ThreadPhase::ReadyToCommit => vec![Done, Abandoned],

            // Terminal states have no valid transitions
            ThreadPhase::Done { .. } | ThreadPhase::Abandoned { .. } => vec![],
        }
    }
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

    // ==========================================
    // F2: State Transition Tests
    // ==========================================

    #[test]
    fn test_can_transition_from_drafting() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Drafting;

        // Valid transitions
        assert!(thread.can_transition_to(&ThreadPhase::Assessing).is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Finalized).is_ok());
        assert!(thread
            .can_transition_to(&ThreadPhase::Abandoned {
                reason: "x".to_string()
            })
            .is_ok());

        // Invalid transitions
        assert!(thread.can_transition_to(&ThreadPhase::Running { iteration: 1 }).is_err());
        assert!(thread.can_transition_to(&ThreadPhase::Implemented).is_err());
    }

    #[test]
    fn test_can_transition_from_assessing() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Assessing;

        assert!(thread.can_transition_to(&ThreadPhase::Drafting).is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Finalized).is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Running { iteration: 1 }).is_err());
    }

    #[test]
    fn test_can_transition_from_finalized() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Finalized;

        assert!(thread.can_transition_to(&ThreadPhase::Drafting).is_ok()); // reopen
        assert!(thread.can_transition_to(&ThreadPhase::Preflight).is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Running { iteration: 1 }).is_err());
    }

    #[test]
    fn test_can_transition_from_preflight() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Preflight;

        assert!(thread.can_transition_to(&ThreadPhase::Configuring).is_ok());
        assert!(thread
            .can_transition_to(&ThreadPhase::PreflightFailed {
                reason: "x".to_string()
            })
            .is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Running { iteration: 1 }).is_err());
    }

    #[test]
    fn test_can_transition_from_preflight_failed() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::PreflightFailed {
            reason: "dirty".to_string(),
        };

        assert!(thread.can_transition_to(&ThreadPhase::Preflight).is_ok()); // retry
        assert!(thread.can_transition_to(&ThreadPhase::Drafting).is_ok()); // fix spec
        assert!(thread.can_transition_to(&ThreadPhase::Running { iteration: 1 }).is_err());
    }

    #[test]
    fn test_can_transition_from_configuring() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Configuring;

        assert!(thread.can_transition_to(&ThreadPhase::Running { iteration: 1 }).is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Drafting).is_err());
    }

    #[test]
    fn test_can_transition_from_running() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Running { iteration: 3 };

        assert!(thread.can_transition_to(&ThreadPhase::Verifying { iteration: 3 }).is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Paused { iteration: 3 }).is_ok());
        assert!(thread
            .can_transition_to(&ThreadPhase::Stuck {
                diagnosis: StuckDiagnosis {
                    iterations_attempted: 3,
                    models_tried: vec![],
                    best_criteria_passed: 0,
                    total_criteria: 1,
                    last_error: None,
                }
            })
            .is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Drafting).is_err());
    }

    #[test]
    fn test_can_transition_from_paused() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Paused { iteration: 2 };

        assert!(thread.can_transition_to(&ThreadPhase::Running { iteration: 2 }).is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Configuring).is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Drafting).is_err());
    }

    #[test]
    fn test_can_transition_from_verifying() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Verifying { iteration: 2 };

        assert!(thread.can_transition_to(&ThreadPhase::Running { iteration: 3 }).is_ok());
        assert!(thread
            .can_transition_to(&ThreadPhase::Stuck {
                diagnosis: StuckDiagnosis {
                    iterations_attempted: 2,
                    models_tried: vec![],
                    best_criteria_passed: 0,
                    total_criteria: 1,
                    last_error: None,
                }
            })
            .is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Implemented).is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Drafting).is_err());
    }

    #[test]
    fn test_can_transition_from_stuck() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Stuck {
            diagnosis: StuckDiagnosis {
                iterations_attempted: 5,
                models_tried: vec!["claude".to_string()],
                best_criteria_passed: 2,
                total_criteria: 5,
                last_error: None,
            },
        };

        assert!(thread.can_transition_to(&ThreadPhase::Configuring).is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Running { iteration: 1 }).is_ok()); // manual assist
        assert!(thread.can_transition_to(&ThreadPhase::Drafting).is_ok()); // spec was wrong
        assert!(thread.can_transition_to(&ThreadPhase::Implemented).is_err());
    }

    #[test]
    fn test_can_transition_from_implemented() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Implemented;

        assert!(thread.can_transition_to(&ThreadPhase::Polishing).is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::PendingReview).is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Drafting).is_err());
    }

    #[test]
    fn test_can_transition_from_polishing() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Polishing;

        assert!(thread.can_transition_to(&ThreadPhase::Implemented).is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::PendingReview).is_err());
    }

    #[test]
    fn test_can_transition_from_pending_review() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::PendingReview;

        assert!(thread.can_transition_to(&ThreadPhase::Approved).is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Running { iteration: 1 }).is_ok()); // impl bugs
        assert!(thread.can_transition_to(&ThreadPhase::Drafting).is_ok()); // spec was wrong
        assert!(thread.can_transition_to(&ThreadPhase::Implemented).is_err());
    }

    #[test]
    fn test_can_transition_from_approved() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Approved;

        assert!(thread.can_transition_to(&ThreadPhase::ReadyToCommit).is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Drafting).is_err());
    }

    #[test]
    fn test_can_transition_from_ready_to_commit() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::ReadyToCommit;

        assert!(thread
            .can_transition_to(&ThreadPhase::Done {
                commit_sha: "abc".to_string()
            })
            .is_ok());
        assert!(thread.can_transition_to(&ThreadPhase::Drafting).is_err());
    }

    #[test]
    fn test_cannot_transition_from_terminal() {
        let mut thread = Thread::new("Test");

        // From Done
        thread.phase = ThreadPhase::Done {
            commit_sha: "abc".to_string(),
        };
        let err = thread.can_transition_to(&ThreadPhase::Drafting).unwrap_err();
        assert!(matches!(err, TransitionError::FromTerminalState(_)));

        // From Abandoned
        thread.phase = ThreadPhase::Abandoned {
            reason: "test".to_string(),
        };
        let err = thread.can_transition_to(&ThreadPhase::Drafting).unwrap_err();
        assert!(matches!(err, TransitionError::FromTerminalState(_)));
    }

    #[test]
    fn test_abandon_from_any_non_terminal() {
        let phases = vec![
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
        ];

        for phase in phases {
            let mut thread = Thread::new("Test");
            thread.phase = phase.clone();
            assert!(
                thread
                    .can_transition_to(&ThreadPhase::Abandoned {
                        reason: "test".to_string()
                    })
                    .is_ok(),
                "Should be able to abandon from {:?}",
                phase
            );
        }
    }

    #[test]
    fn test_transition_to_updates_state() {
        let mut thread = Thread::new("Test");
        let original_updated_at = thread.updated_at;

        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));

        thread.transition_to(ThreadPhase::Assessing).unwrap();
        assert_eq!(thread.phase, ThreadPhase::Assessing);
        assert!(thread.updated_at > original_updated_at);
    }

    #[test]
    fn test_transition_to_fails_on_invalid() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Drafting;
        let original_phase = thread.phase.clone();

        let result = thread.transition_to(ThreadPhase::Implemented);
        assert!(result.is_err());
        assert_eq!(thread.phase, original_phase); // State unchanged
    }

    #[test]
    fn test_available_transitions_from_drafting() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Drafting;

        let available = thread.available_transitions();
        let kinds: Vec<PhaseKind> = available.iter().map(|p| p.kind()).collect();

        assert!(kinds.contains(&PhaseKind::Assessing));
        assert!(kinds.contains(&PhaseKind::Finalized));
        assert!(kinds.contains(&PhaseKind::Abandoned));
        assert!(!kinds.contains(&PhaseKind::Running));
    }

    #[test]
    fn test_available_transitions_from_terminal() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Done {
            commit_sha: "abc".to_string(),
        };

        let available = thread.available_transitions();
        assert!(available.is_empty());
    }

    #[test]
    fn test_available_transitions_includes_abandoned() {
        // All non-terminal states should include Abandoned
        let phases = vec![
            ThreadPhase::Drafting,
            ThreadPhase::Running { iteration: 1 },
            ThreadPhase::Implemented,
            ThreadPhase::PendingReview,
        ];

        for phase in phases {
            let mut thread = Thread::new("Test");
            thread.phase = phase;
            let available = thread.available_transitions();
            let kinds: Vec<PhaseKind> = available.iter().map(|p| p.kind()).collect();
            assert!(
                kinds.contains(&PhaseKind::Abandoned),
                "Abandoned should be available from {:?}",
                thread.phase
            );
        }
    }

    #[test]
    fn test_requires_workspace_reset_stuck_to_drafting() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Stuck {
            diagnosis: StuckDiagnosis {
                iterations_attempted: 5,
                models_tried: vec![],
                best_criteria_passed: 0,
                total_criteria: 1,
                last_error: None,
            },
        };

        assert!(thread.requires_workspace_reset(&ThreadPhase::Drafting));
        assert!(!thread.requires_workspace_reset(&ThreadPhase::Configuring));
    }

    #[test]
    fn test_requires_workspace_reset_pending_review_to_drafting() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::PendingReview;

        assert!(thread.requires_workspace_reset(&ThreadPhase::Drafting));
        assert!(!thread.requires_workspace_reset(&ThreadPhase::Running { iteration: 1 }));
        assert!(!thread.requires_workspace_reset(&ThreadPhase::Approved));
    }

    #[test]
    fn test_requires_workspace_reset_preflight_failed_to_drafting() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::PreflightFailed {
            reason: "dirty".to_string(),
        };

        // PreflightFailed -> Drafting does NOT require reset (no implementation yet)
        assert!(!thread.requires_workspace_reset(&ThreadPhase::Drafting));
    }

    #[test]
    fn test_requires_workspace_reset_forward_transitions() {
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Drafting;

        assert!(!thread.requires_workspace_reset(&ThreadPhase::Finalized));
        assert!(!thread.requires_workspace_reset(&ThreadPhase::Assessing));
    }

    #[test]
    fn test_transition_error_display() {
        let err = TransitionError::InvalidTransition {
            from: "Drafting".to_string(),
            to: "Implemented".to_string(),
            reason: "not allowed".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Drafting"));
        assert!(msg.contains("Implemented"));

        let err = TransitionError::FromTerminalState("Done".to_string());
        let msg = err.to_string();
        assert!(msg.contains("terminal"));
        assert!(msg.contains("Done"));
    }

    #[test]
    fn test_phase_kind_to_phase_with_defaults() {
        // Verify all PhaseKind variants can be converted to ThreadPhase
        let kinds = vec![
            PhaseKind::Drafting,
            PhaseKind::Assessing,
            PhaseKind::Finalized,
            PhaseKind::Preflight,
            PhaseKind::PreflightFailed,
            PhaseKind::Configuring,
            PhaseKind::Running,
            PhaseKind::Paused,
            PhaseKind::Verifying,
            PhaseKind::Stuck,
            PhaseKind::Implemented,
            PhaseKind::Polishing,
            PhaseKind::PendingReview,
            PhaseKind::Approved,
            PhaseKind::ReadyToCommit,
            PhaseKind::Done,
            PhaseKind::Abandoned,
        ];

        for kind in kinds {
            let phase = kind.to_phase_with_defaults();
            assert_eq!(phase.kind(), kind, "Round-trip failed for {:?}", kind);
        }
    }

    #[test]
    fn test_full_workflow_happy_path() {
        let mut thread = Thread::new("Feature X");

        // Spec creation
        assert!(thread.transition_to(ThreadPhase::Finalized).is_ok());

        // Implementation
        assert!(thread.transition_to(ThreadPhase::Preflight).is_ok());
        assert!(thread.transition_to(ThreadPhase::Configuring).is_ok());
        assert!(thread.transition_to(ThreadPhase::Running { iteration: 1 }).is_ok());
        assert!(thread.transition_to(ThreadPhase::Verifying { iteration: 1 }).is_ok());
        assert!(thread.transition_to(ThreadPhase::Implemented).is_ok());

        // Review
        assert!(thread.transition_to(ThreadPhase::PendingReview).is_ok());
        assert!(thread.transition_to(ThreadPhase::Approved).is_ok());

        // Complete
        assert!(thread.transition_to(ThreadPhase::ReadyToCommit).is_ok());
        assert!(thread
            .transition_to(ThreadPhase::Done {
                commit_sha: "abc123".to_string()
            })
            .is_ok());

        assert!(thread.is_terminal());
    }

    #[test]
    fn test_workflow_with_stuck_recovery() {
        let mut thread = Thread::new("Feature Y");

        thread.transition_to(ThreadPhase::Finalized).unwrap();
        thread.transition_to(ThreadPhase::Preflight).unwrap();
        thread.transition_to(ThreadPhase::Configuring).unwrap();
        thread
            .transition_to(ThreadPhase::Running { iteration: 1 })
            .unwrap();

        // Get stuck
        thread
            .transition_to(ThreadPhase::Stuck {
                diagnosis: StuckDiagnosis {
                    iterations_attempted: 5,
                    models_tried: vec!["claude".to_string()],
                    best_criteria_passed: 2,
                    total_criteria: 5,
                    last_error: Some("Tests failed".to_string()),
                },
            })
            .unwrap();

        // Reconfigure and retry
        thread.transition_to(ThreadPhase::Configuring).unwrap();
        thread
            .transition_to(ThreadPhase::Running { iteration: 1 })
            .unwrap();
        thread
            .transition_to(ThreadPhase::Verifying { iteration: 1 })
            .unwrap();
        thread.transition_to(ThreadPhase::Implemented).unwrap();

        assert_eq!(thread.phase, ThreadPhase::Implemented);
    }
}
