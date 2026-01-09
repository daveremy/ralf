//! Phase router for context pane views.
//!
//! Routes [`PhaseKind`] to the appropriate [`ContextView`] for rendering
//! in the context pane.

use ralf_engine::thread::PhaseKind;

/// Terminal state kind for completion view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    /// Thread completed successfully.
    Done,
    /// Thread was abandoned.
    Abandoned,
}

/// Context view variants based on phase.
///
/// Each variant represents a different view that can be shown in the
/// context pane based on the current thread phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextView {
    /// No thread loaded - show Models panel.
    NoThread,
    /// Spec editing (Drafting, Assessing, Finalized).
    SpecEditor,
    /// Preflight check results (Preflight, `PreflightFailed`).
    PreflightResults,
    /// Run configuration (Configuring).
    RunConfig,
    /// Run output streaming (Running, Verifying).
    RunOutput,
    /// Decision prompt (Paused, Stuck).
    DecisionPrompt,
    /// Implementation summary (Implemented, Polishing).
    Summary,
    /// Diff viewer (`PendingReview`, Approved).
    DiffViewer,
    /// Commit view (`ReadyToCommit`).
    CommitView,
    /// Completion summary (Done or Abandoned).
    CompletionSummary(CompletionKind),
}

impl ContextView {
    /// Route phase to appropriate view.
    #[must_use]
    pub fn from_phase(phase: Option<PhaseKind>) -> Self {
        match phase {
            None => Self::NoThread,
            Some(PhaseKind::Drafting | PhaseKind::Assessing | PhaseKind::Finalized) => {
                Self::SpecEditor
            }
            Some(PhaseKind::Preflight | PhaseKind::PreflightFailed) => Self::PreflightResults,
            Some(PhaseKind::Configuring) => Self::RunConfig,
            Some(PhaseKind::Running | PhaseKind::Verifying) => Self::RunOutput,
            Some(PhaseKind::Paused | PhaseKind::Stuck) => Self::DecisionPrompt,
            Some(PhaseKind::Implemented | PhaseKind::Polishing) => Self::Summary,
            Some(PhaseKind::PendingReview | PhaseKind::Approved) => Self::DiffViewer,
            Some(PhaseKind::ReadyToCommit) => Self::CommitView,
            Some(PhaseKind::Done) => Self::CompletionSummary(CompletionKind::Done),
            Some(PhaseKind::Abandoned) => Self::CompletionSummary(CompletionKind::Abandoned),
        }
    }

    /// Get the title for this view (for pane title bar).
    #[must_use]
    pub fn title(&self) -> &'static str {
        match self {
            Self::NoThread => " Models ",
            Self::SpecEditor => " Spec ",
            Self::PreflightResults => " Preflight ",
            Self::RunConfig => " Configure ",
            Self::RunOutput => " Output ",
            Self::DecisionPrompt => " Decision ",
            Self::Summary => " Summary ",
            Self::DiffViewer => " Diff ",
            Self::CommitView => " Commit ",
            Self::CompletionSummary(_) => " Complete ",
        }
    }

    /// Get placeholder text for this view (used until real views are implemented).
    #[must_use]
    pub fn placeholder_text(&self) -> &'static str {
        match self {
            Self::NoThread => "", // ModelsPanel handles this
            Self::SpecEditor => "Spec Editor\n\n(Implementation in M5-B.3)",
            Self::PreflightResults => "Preflight Results\n\n(Implementation in M5-B.4)",
            Self::RunConfig => "Run Configuration\n\n(Implementation in M5-B.4)",
            Self::RunOutput => "Run Output\n\n(Implementation in M5-B.3)",
            Self::DecisionPrompt => "Decision Required\n\n(Implementation in M5-B.4)",
            Self::Summary => "Implementation Summary\n\n(Implementation in M5-B.3)",
            Self::DiffViewer => "Diff Viewer\n\n(Implementation in M5-B.4)",
            Self::CommitView => "Commit View\n\n(Implementation in M5-B.4)",
            Self::CompletionSummary(CompletionKind::Done) => {
                "Complete!\n\n[n] New Thread  [o] Open Thread"
            }
            Self::CompletionSummary(CompletionKind::Abandoned) => {
                "Thread Abandoned\n\n[n] New Thread  [o] Open Thread"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_thread_routes_to_no_thread() {
        assert_eq!(ContextView::from_phase(None), ContextView::NoThread);
    }

    #[test]
    fn test_drafting_phases_route_to_spec_editor() {
        assert_eq!(
            ContextView::from_phase(Some(PhaseKind::Drafting)),
            ContextView::SpecEditor
        );
        assert_eq!(
            ContextView::from_phase(Some(PhaseKind::Assessing)),
            ContextView::SpecEditor
        );
        assert_eq!(
            ContextView::from_phase(Some(PhaseKind::Finalized)),
            ContextView::SpecEditor
        );
    }

    #[test]
    fn test_preflight_phases_route_to_preflight_results() {
        assert_eq!(
            ContextView::from_phase(Some(PhaseKind::Preflight)),
            ContextView::PreflightResults
        );
        assert_eq!(
            ContextView::from_phase(Some(PhaseKind::PreflightFailed)),
            ContextView::PreflightResults
        );
    }

    #[test]
    fn test_running_phases_route_to_run_output() {
        assert_eq!(
            ContextView::from_phase(Some(PhaseKind::Running)),
            ContextView::RunOutput
        );
        assert_eq!(
            ContextView::from_phase(Some(PhaseKind::Verifying)),
            ContextView::RunOutput
        );
    }

    #[test]
    fn test_decision_phases_route_to_decision_prompt() {
        assert_eq!(
            ContextView::from_phase(Some(PhaseKind::Paused)),
            ContextView::DecisionPrompt
        );
        assert_eq!(
            ContextView::from_phase(Some(PhaseKind::Stuck)),
            ContextView::DecisionPrompt
        );
    }

    #[test]
    fn test_terminal_phases_route_to_completion() {
        assert_eq!(
            ContextView::from_phase(Some(PhaseKind::Done)),
            ContextView::CompletionSummary(CompletionKind::Done)
        );
        assert_eq!(
            ContextView::from_phase(Some(PhaseKind::Abandoned)),
            ContextView::CompletionSummary(CompletionKind::Abandoned)
        );
    }

    #[test]
    fn test_all_17_phases_covered() {
        use PhaseKind::*;
        let all_phases = [
            Drafting,
            Assessing,
            Finalized,
            Preflight,
            PreflightFailed,
            Configuring,
            Running,
            Verifying,
            Paused,
            Stuck,
            Implemented,
            Polishing,
            PendingReview,
            Approved,
            ReadyToCommit,
            Done,
            Abandoned,
        ];

        // Ensure all phases produce a valid view (not NoThread)
        for phase in all_phases {
            let view = ContextView::from_phase(Some(phase));
            assert_ne!(view, ContextView::NoThread);
        }
    }

    #[test]
    fn test_view_titles() {
        assert_eq!(ContextView::NoThread.title(), " Models ");
        assert_eq!(ContextView::SpecEditor.title(), " Spec ");
        assert_eq!(
            ContextView::CompletionSummary(CompletionKind::Done).title(),
            " Complete "
        );
    }
}
