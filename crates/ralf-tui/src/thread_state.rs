//! Thread display state for the TUI.
//!
//! This module provides [`ThreadDisplay`], a UI-friendly wrapper that extracts
//! display information from the engine's [`Thread`].

use ralf_engine::thread::{PhaseKind, RunConfig, StuckDiagnosis, Thread, ThreadPhase};

/// Thread state extracted for UI display.
///
/// This struct contains only the information needed for rendering,
/// avoiding tight coupling between the UI and engine internals.
#[derive(Debug, Clone)]
pub struct ThreadDisplay {
    /// Thread ID.
    pub id: String,
    /// Thread title.
    pub title: String,
    /// Phase kind (for routing/hints).
    pub phase_kind: PhaseKind,
    /// Human-readable phase name (e.g., "Preflight Failed").
    pub phase_display: String,
    /// Current iteration (if Running/Paused/Verifying).
    pub iteration: Option<u32>,
    /// Max iterations configured (from `run_config` or default).
    pub max_iterations: u32,
    /// Failure/status reason (if PreflightFailed/Abandoned/Stuck).
    pub failure_reason: Option<String>,
}

impl ThreadDisplay {
    /// Extract display state from an engine Thread.
    pub fn from_thread(thread: &Thread) -> Self {
        let phase_kind = thread.phase.kind();
        let phase_display = thread.phase.display_name().to_string();
        let (iteration, failure_reason) = match &thread.phase {
            ThreadPhase::Running { iteration }
            | ThreadPhase::Paused { iteration }
            | ThreadPhase::Verifying { iteration } => (Some(*iteration), None),
            ThreadPhase::PreflightFailed { reason } | ThreadPhase::Abandoned { reason } => {
                (None, Some(reason.clone()))
            }
            ThreadPhase::Stuck { diagnosis } => (None, Some(Self::format_diagnosis(diagnosis))),
            _ => (None, None),
        };

        // Use run_config max_iterations if set, otherwise engine default
        let max_iterations = thread
            .run_config
            .as_ref()
            .map_or(RunConfig::default().max_iterations, |c| c.max_iterations);

        Self {
            id: thread.id.clone(),
            title: thread.title.clone(),
            phase_kind,
            phase_display,
            iteration,
            max_iterations,
            failure_reason,
        }
    }

    /// Format `StuckDiagnosis` for display.
    fn format_diagnosis(d: &StuckDiagnosis) -> String {
        format!(
            "{}/{} criteria ({} iterations)",
            d.best_criteria_passed, d.total_criteria, d.iterations_attempted
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_display_from_drafting() {
        let thread = Thread::new("Test Feature");
        let display = ThreadDisplay::from_thread(&thread);

        assert_eq!(display.title, "Test Feature");
        assert_eq!(display.phase_kind, PhaseKind::Drafting);
        assert_eq!(display.phase_display, "Drafting");
        assert_eq!(display.iteration, None);
        assert_eq!(display.max_iterations, 5); // Default
        assert_eq!(display.failure_reason, None);
    }

    #[test]
    fn test_thread_display_from_running() {
        let mut thread = Thread::new("Test Feature");
        thread.phase = ThreadPhase::Running { iteration: 2 };
        let mut config = RunConfig::default();
        config.max_iterations = 10;
        thread.run_config = Some(config);

        let display = ThreadDisplay::from_thread(&thread);

        assert_eq!(display.iteration, Some(2));
        assert_eq!(display.max_iterations, 10);
        assert_eq!(display.phase_display, "Running");
    }

    #[test]
    fn test_thread_display_from_stuck() {
        let mut thread = Thread::new("Stuck Feature");
        thread.phase = ThreadPhase::Stuck {
            diagnosis: StuckDiagnosis {
                iterations_attempted: 5,
                models_tried: vec!["claude-sonnet".into()],
                last_error: Some("Tests fail".into()),
                best_criteria_passed: 2,
                total_criteria: 3,
            },
        };

        let display = ThreadDisplay::from_thread(&thread);

        assert!(display.failure_reason.is_some());
        let reason = display.failure_reason.unwrap();
        assert!(reason.contains("2/3"));
        assert!(reason.contains("5 iterations"));
    }

    #[test]
    fn test_thread_display_from_abandoned() {
        let mut thread = Thread::new("Abandoned Feature");
        thread.phase = ThreadPhase::Abandoned {
            reason: "User cancelled".into(),
        };

        let display = ThreadDisplay::from_thread(&thread);

        assert_eq!(display.phase_kind, PhaseKind::Abandoned);
        assert_eq!(display.failure_reason, Some("User cancelled".into()));
    }
}
