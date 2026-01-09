//! Phase-aware placeholder text for the input area.

use ralf_engine::thread::PhaseKind;

/// Get placeholder text for the input area based on current phase.
///
/// The placeholder guides users on what they can do in the current state.
#[must_use]
pub fn input_placeholder(phase: Option<PhaseKind>) -> &'static str {
    match phase {
        None => "Start typing to create a thread...",
        Some(PhaseKind::Drafting) => "Describe your task...",
        Some(PhaseKind::Assessing) => "Refine your specification...",
        Some(PhaseKind::Finalized) => "Type to edit, or press [r] to run...",
        Some(PhaseKind::Preflight) => "Waiting for preflight checks...",
        Some(PhaseKind::PreflightFailed) => "Fix issues or type to retry...",
        Some(PhaseKind::Configuring) => "Confirm settings...",
        Some(PhaseKind::Running | PhaseKind::Verifying) => "Type to cancel or direct...",
        Some(PhaseKind::Paused | PhaseKind::Stuck) => "Provide direction...",
        Some(PhaseKind::Implemented) => "Continue to review...",
        Some(PhaseKind::Polishing) => "Add docs, tests, or continue...",
        Some(PhaseKind::PendingReview) => "Comment or approve...",
        Some(PhaseKind::Approved) => "Proceed to commit...",
        Some(PhaseKind::ReadyToCommit) => "Edit commit message...",
        Some(PhaseKind::Done | PhaseKind::Abandoned) => "Start a new thread...",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_thread_placeholder() {
        assert_eq!(
            input_placeholder(None),
            "Start typing to create a thread..."
        );
    }

    #[test]
    fn test_drafting_placeholder() {
        assert_eq!(
            input_placeholder(Some(PhaseKind::Drafting)),
            "Describe your task..."
        );
    }

    #[test]
    fn test_finalized_placeholder() {
        assert_eq!(
            input_placeholder(Some(PhaseKind::Finalized)),
            "Type to edit, or press [r] to run..."
        );
    }

    #[test]
    fn test_running_placeholder() {
        assert_eq!(
            input_placeholder(Some(PhaseKind::Running)),
            "Type to cancel or direct..."
        );
        assert_eq!(
            input_placeholder(Some(PhaseKind::Verifying)),
            "Type to cancel or direct..."
        );
    }

    #[test]
    fn test_terminal_phases_placeholder() {
        assert_eq!(
            input_placeholder(Some(PhaseKind::Done)),
            "Start a new thread..."
        );
        assert_eq!(
            input_placeholder(Some(PhaseKind::Abandoned)),
            "Start a new thread..."
        );
    }

    #[test]
    fn test_all_phases_have_placeholder() {
        // Ensure every phase has a non-empty placeholder
        let phases = [
            PhaseKind::Drafting,
            PhaseKind::Assessing,
            PhaseKind::Finalized,
            PhaseKind::Preflight,
            PhaseKind::PreflightFailed,
            PhaseKind::Configuring,
            PhaseKind::Running,
            PhaseKind::Verifying,
            PhaseKind::Paused,
            PhaseKind::Stuck,
            PhaseKind::Implemented,
            PhaseKind::Polishing,
            PhaseKind::PendingReview,
            PhaseKind::Approved,
            PhaseKind::ReadyToCommit,
            PhaseKind::Done,
            PhaseKind::Abandoned,
        ];

        for phase in phases {
            let placeholder = input_placeholder(Some(phase));
            assert!(!placeholder.is_empty(), "Phase {:?} has empty placeholder", phase);
        }
    }
}
