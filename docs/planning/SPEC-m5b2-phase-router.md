# SPEC-m5b2: Phase Router & Dynamic Status

## Promise

Wire up the TUI shell to thread state so that:
1. The status bar displays live thread information (phase, title, iteration metric)
2. Footer hints change based on the current phase AND focused pane
3. The context pane routes to the appropriate view based on phase

**Note:** Model indicators already exist in the status bar (via `models: &[ModelStatus]`). This milestone does not change model display.

This milestone connects the static shell (M5-A) and timeline (M5-B.1) to thread display state, making the UI dynamic and phase-aware. Note: Uses mock `ThreadDisplay` for testing; real engine connection comes in a later milestone.

## Background

The TUI currently displays placeholder content in the status bar and a static Models panel in the context pane. To support the full workflow, the UI must adapt to the current thread phase:

- **Drafting**: Show spec editor, hints for chat input
- **Running**: Show run output, hints for pause/cancel
- **Stuck**: Show decision prompt, hints for options (including Diagnose)
- etc.

See `docs/state-machine.md` for full ThreadPhase definitions.

## Deliverables

### 1. Thread Display State

**File:** `crates/ralf-tui/src/thread_state.rs`

Create a UI-friendly wrapper that extracts display information from engine's `Thread`:

```rust
// Note: Import from ralf_engine::thread::RunConfig, NOT ralf_engine::RunConfig
// (the latter is for runner config, this is thread-specific run config)
use ralf_engine::thread::{PhaseKind, ThreadPhase, Thread, StuckDiagnosis, RunConfig};

/// Thread state extracted for UI display.
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
    /// Max iterations configured (from run_config or default).
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
            ThreadPhase::PreflightFailed { reason } => (None, Some(reason.clone())),
            ThreadPhase::Abandoned { reason } => (None, Some(reason.clone())),
            ThreadPhase::Stuck { diagnosis } => (None, Some(Self::format_diagnosis(diagnosis))),
            _ => (None, None),
        };

        // Use run_config max_iterations if set, otherwise engine default
        let max_iterations = thread.run_config
            .as_ref()
            .map(|c| c.max_iterations)
            .unwrap_or(RunConfig::default().max_iterations);

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

    /// Format StuckDiagnosis for display.
    fn format_diagnosis(d: &StuckDiagnosis) -> String {
        format!(
            "{}/{} criteria ({} iterations)",
            d.best_criteria_passed, d.total_criteria, d.iterations_attempted
        )
    }
}
```

### 2. Phase Router Component

**File:** `crates/ralf-tui/src/context/router.rs`

Route `PhaseKind` to context view:

```rust
use ralf_engine::thread::PhaseKind;

/// Terminal state kind for completion view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Done,
    Abandoned,
}

/// Context view variants based on phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextView {
    /// No thread loaded - show Models panel.
    NoThread,
    /// Spec editing (Drafting, Assessing, Finalized).
    SpecEditor,
    /// Preflight check results (Preflight, PreflightFailed).
    PreflightResults,
    /// Run configuration (Configuring).
    RunConfig,
    /// Run output streaming (Running, Verifying).
    RunOutput,
    /// Decision prompt (Paused, Stuck).
    DecisionPrompt,
    /// Implementation summary (Implemented, Polishing).
    Summary,
    /// Diff viewer (PendingReview, Approved).
    DiffViewer,
    /// Commit view (ReadyToCommit).
    CommitView,
    /// Completion summary (Done or Abandoned).
    CompletionSummary(CompletionKind),
}

impl ContextView {
    /// Route phase to appropriate view.
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
}
```

### 3. Dynamic Status Bar

**File:** Update `crates/ralf-tui/src/widgets/status_bar.rs`

Status bar content driven by thread state. Uses existing `StatusBarContent` shape (no changes needed to struct):

```rust
// Existing struct - no changes required:
pub struct StatusBarContent {
    pub phase: String,           // e.g., "Drafting", "Running", "No Thread"
    pub title: String,           // Thread title or placeholder
    pub file: Option<String>,    // File:line (populated by heartbeat in M5-C)
    pub metric: Option<String>,  // e.g., "2/5" for iterations
    pub hint: Option<String>,    // Next action hint (plain text, widget prepends "→ ")
}
// Note: The hint field contains plain text like "Describe your task".
// The StatusBar widget adds the "→ " prefix during rendering.
```

**New helper methods to create from ThreadDisplay:**

```rust
impl StatusBarContent {
    /// Create status bar content from thread display state.
    pub fn from_thread(thread: Option<&ThreadDisplay>) -> Self {
        match thread {
            None => Self {
                phase: "No Thread".into(),
                title: "Select a thread to start".into(),
                file: None,
                metric: None,
                hint: None,
            },
            Some(t) => {
                let metric = t.iteration.map(|i| format!("{}/{}", i, t.max_iterations));
                let hint = Some(Self::next_action_hint(t.phase_kind));
                Self {
                    phase: t.phase_display.clone(),  // Uses display_name() string
                    title: t.title.clone(),
                    file: None,
                    metric,
                    hint,
                }
            }
        }
    }

    /// Get next action hint for a phase (from Section 6 table).
    pub fn next_action_hint(phase: PhaseKind) -> String {
        match phase {
            PhaseKind::Drafting => "Describe your task",
            PhaseKind::Assessing => "Review AI feedback",
            PhaseKind::Finalized => "Press [r] to run",
            PhaseKind::Preflight => "Checking prerequisites...",
            PhaseKind::PreflightFailed => "Fix issues to continue",
            PhaseKind::Configuring => "Configure and start",
            PhaseKind::Running => "Loop in progress...",
            PhaseKind::Verifying => "Checking criteria...",
            PhaseKind::Paused => "Resume or reconfigure",
            PhaseKind::Stuck => "Choose next action",
            PhaseKind::Implemented => "Review changes",
            PhaseKind::Polishing => "Add docs/tests",
            PhaseKind::PendingReview => "Review the diff",
            PhaseKind::Approved => "Ready to commit",
            PhaseKind::ReadyToCommit => "Commit when ready",
            PhaseKind::Done => "Complete!",
            PhaseKind::Abandoned => "Thread abandoned",
        }.into()
    }
}
```

**Note:** Model indicators remain via existing `models: &[ModelStatus]` parameter in StatusBar widget.

**Overflow handling:** When phase+title+models+metric+hint exceed terminal width, the existing status bar clips content (right side truncated). This behavior is acceptable - no priority-based elision is implemented in this milestone.

### 4. Phase-Aware Footer Hints

**File:** Update `crates/ralf-tui/src/widgets/footer_hints.rs`

Footer hints depend on BOTH phase AND focused pane.

**Note:** `FooterHints<'a>` is a render widget; `hints_for_state()` is a standalone function.

**Export:** Update `crates/ralf-tui/src/widgets/mod.rs` to re-export:
```rust
pub use footer_hints::{FooterHints, KeyHint, hints_for_state};
```

This allows `layout::shell` to call `hints_for_state()` from `crate::widgets::hints_for_state`.

```rust
use ralf_engine::thread::PhaseKind;
use crate::layout::{FocusedPane, ScreenMode};
// KeyHint is in the same file - no import needed

/// Get hints for the current state.
///
/// Hints depend on:
/// - `phase`: Current thread phase (None = no thread)
/// - `screen_mode`: Current screen mode
/// - `focused`: Which pane has focus (used in Split mode)
/// - `show_models_panel`: Whether models panel is showing (enables 'r' refresh)
///
/// In `TimelineFocus` mode, effective focus is Timeline.
/// In `ContextFocus` mode, effective focus is Context.
/// In `Split` mode, use the `focused` parameter.
pub fn hints_for_state(
    phase: Option<PhaseKind>,
    screen_mode: ScreenMode,
    focused: FocusedPane,
    show_models_panel: bool,
) -> Vec<KeyHint> {
    // Derive effective focus from screen mode
    let effective_focus = match screen_mode {
        ScreenMode::TimelineFocus => FocusedPane::Timeline,
        ScreenMode::ContextFocus => FocusedPane::Context,
        ScreenMode::Split => focused,
    };

    hints_for_focus(phase, effective_focus, show_models_panel)
}

/// Internal helper for generating hints based on effective focus.
fn hints_for_focus(
    phase: Option<PhaseKind>,
    focused: FocusedPane,
    show_models_panel: bool,
) -> Vec<KeyHint> {
    let mut hints = Vec::new();

    // Pane-specific hints first
    match focused {
        FocusedPane::Timeline => {
            hints.push(KeyHint::new("j/k", "Navigate"));
            hints.push(KeyHint::new("Enter", "Toggle"));
            hints.push(KeyHint::new("y", "Copy"));
        }
        FocusedPane::Context => {
            // Context hints depend on phase
            hints.extend(context_hints_for_phase(phase));
        }
    }

    // Common hints
    hints.push(KeyHint::new("Tab", "Focus"));
    hints.push(KeyHint::new("1/2/3", "Modes"));
    if show_models_panel && phase.is_none() {
        // Only show refresh when no thread and models panel visible
        hints.push(KeyHint::new("r", "Refresh"));
    }
    hints.push(KeyHint::new("?", "Help"));
    hints.push(KeyHint::new("q", "Quit"));

    hints
}

/// Get context-pane hints for a phase.
fn context_hints_for_phase(phase: Option<PhaseKind>) -> Vec<KeyHint> {
    match phase {
        None => vec![
            KeyHint::new("n", "New Thread"),
            KeyHint::new("o", "Open Thread"),
        ],
        Some(PhaseKind::Drafting | PhaseKind::Assessing) => vec![
            KeyHint::new("Enter", "Send"),
            KeyHint::new("Ctrl+F", "Finalize"),
        ],
        Some(PhaseKind::Finalized) => vec![
            KeyHint::new("r", "Run"),
            KeyHint::new("e", "Edit Spec"),
        ],
        Some(PhaseKind::Preflight) => vec![],  // Auto-progresses
        Some(PhaseKind::PreflightFailed) => vec![
            KeyHint::new("r", "Retry"),
            KeyHint::new("e", "Edit Spec"),
        ],
        Some(PhaseKind::Configuring) => vec![
            KeyHint::new("Enter", "Start"),
            KeyHint::new("m", "Models"),
        ],
        // Note: Running can pause, Verifying cannot (different transitions)
        Some(PhaseKind::Running) => vec![
            KeyHint::new("p", "Pause"),
        ],
        Some(PhaseKind::Verifying) => vec![
            // Verifying can't transition to Paused; wait for completion
        ],
        Some(PhaseKind::Paused) => vec![
            KeyHint::new("r", "Resume"),
            KeyHint::new("c", "Reconfigure"),
            KeyHint::new("a", "Abandon"),
        ],
        Some(PhaseKind::Stuck) => vec![
            KeyHint::new("r", "Revise Spec"),
            KeyHint::new("c", "Reconfigure"),
            KeyHint::new("m", "Manual Assist"),
            KeyHint::new("d", "Diagnose"),
            KeyHint::new("a", "Abandon"),
        ],
        Some(PhaseKind::Implemented) => vec![
            KeyHint::new("Enter", "Review"),
            KeyHint::new("p", "Polish"),
        ],
        Some(PhaseKind::Polishing) => vec![
            KeyHint::new("Enter", "Finish"),  // Returns to Implemented state
        ],
        Some(PhaseKind::PendingReview) => vec![
            KeyHint::new("a", "Approve"),
            KeyHint::new("j/k", "Navigate Diff"),
            KeyHint::new("r", "Request Changes"),
        ],
        Some(PhaseKind::Approved) => vec![
            KeyHint::new("Enter", "Ready"),  // Transitions to ReadyToCommit
        ],
        Some(PhaseKind::ReadyToCommit) => vec![
            KeyHint::new("c", "Commit"),
            KeyHint::new("e", "Edit Message"),
        ],
        Some(PhaseKind::Done) => vec![
            KeyHint::new("n", "New Thread"),
            KeyHint::new("o", "Open Thread"),
        ],
        Some(PhaseKind::Abandoned) => vec![
            KeyHint::new("n", "New Thread"),
            KeyHint::new("o", "Open Thread"),
        ],
    }
}
```

### 5. Footer Hints Reference Table

| Phase | Context-Focused Hints |
|-------|----------------------|
| (None) | n: New Thread, o: Open Thread |
| Drafting | Enter: Send, Ctrl+F: Finalize |
| Assessing | Enter: Send, Ctrl+F: Finalize |
| Finalized | r: Run, e: Edit Spec |
| Preflight | (none - auto-progresses) |
| PreflightFailed | r: Retry, e: Edit Spec |
| Configuring | Enter: Start, m: Models |
| Running | p: Pause |
| Verifying | (none - wait for completion) |
| Paused | r: Resume, c: Reconfigure, a: Abandon |
| Stuck | r: Revise, c: Reconfigure, m: Manual, d: Diagnose, a: Abandon |
| Implemented | Enter: Review, p: Polish |
| Polishing | Enter: Finish |
| PendingReview | a: Approve, j/k: Navigate, r: Request Changes |
| Approved | Enter: Ready |
| ReadyToCommit | c: Commit, e: Edit Message |
| Done | n: New Thread, o: Open Thread |
| Abandoned | n: New Thread, o: Open Thread |

**Common hints (always shown):** Tab: Focus, 1/2/3: Modes, ?: Help, q: Quit

**Timeline-focused hints (override context):** j/k: Navigate, Enter: Toggle, y: Copy

### 6. Next Action Guidance

| Phase | Next Action |
|-------|-------------|
| Drafting | "Describe your task" |
| Assessing | "Review AI feedback" |
| Finalized | "Press [r] to run" |
| Preflight | "Checking prerequisites..." |
| PreflightFailed | "Fix issues to continue" |
| Configuring | "Configure and start" |
| Running | "Loop in progress..." |
| Verifying | "Checking criteria..." |
| Paused | "Resume or reconfigure" |
| Stuck | "Choose next action" |
| Implemented | "Review changes" |
| Polishing | "Add docs/tests" |
| PendingReview | "Review the diff" |
| Approved | "Ready to commit" |
| ReadyToCommit | "Commit when ready" |
| Done | "Complete!" |
| Abandoned | "Thread abandoned" |

### 7. No-Thread Behavior

When no thread is loaded:
- **Context pane**: Shows Models panel (existing behavior via `show_models_panel = true`)
- **Status bar**: Shows placeholder ("No Thread" or similar)
- **Footer hints**: n: New Thread, o: Open Thread, r: Refresh (if models visible), Tab, ?, q

**Existing field behavior:**
- The existing `ShellApp.show_models_panel` field is **kept** for this milestone
- It is **set** when `current_thread` changes: `show_models_panel = current_thread.is_none()`
- This preserves the existing `render_shell(..., show_models_panel, ...)` signature
- The refresh gating `self.show_models_panel && self.probe_complete` remains unchanged

```rust
// In ShellApp when loading/unloading a thread:
fn set_thread(&mut self, thread: Option<ThreadDisplay>) {
    self.current_thread = thread;
    self.show_models_panel = self.current_thread.is_none();
}
```

This keeps the existing field/param while ensuring consistency with `current_thread`.

**Note:** `ContextView::NoThread` signals that the existing ModelsPanel should be rendered, NOT a placeholder text. The layout code checks for `ContextView::NoThread` and renders the ModelsPanel widget instead of a placeholder.

### 8. Context Pane Placeholder Views

For this milestone, context views (except NoThread) render placeholder content:

```rust
/// Render context view - called from layout code with Frame.
/// Matches existing render_context_pane signature in shell.rs.
fn render_context_view(
    frame: &mut Frame<'_>,
    view: ContextView,
    area: Rect,
    focused: bool,
    theme: &Theme,
    borders: &BorderSet,
    models: &[ModelStatus],
    ascii_mode: bool,
) {
    match view {
        ContextView::NoThread => {
            // Render existing ModelsPanel widget with all styling options
            let widget = ModelsPanel::new(models, theme)
                .ascii_mode(ascii_mode)
                .focused(focused);
            frame.render_widget(widget, area);
        }
        _ => render_placeholder(frame, view, area, focused, theme, borders),
    }
}

fn render_placeholder(
    frame: &mut Frame<'_>,
    view: ContextView,
    area: Rect,
    focused: bool,
    theme: &Theme,
    borders: &BorderSet,
) {
    let text = match view {
        ContextView::NoThread => unreachable!(), // Handled above
        ContextView::SpecEditor => "Spec Editor\n\n(Implementation in M5-B.3)",
        ContextView::PreflightResults => "Preflight Results\n\n(Implementation in M5-B.4)",
        ContextView::RunConfig => "Run Configuration\n\n(Implementation in M5-B.4)",
        ContextView::RunOutput => "Run Output\n\n(Implementation in M5-B.3)",
        ContextView::DecisionPrompt => "Decision Required\n\n(Implementation in M5-B.4)",
        ContextView::Summary => "Implementation Summary\n\n(Implementation in M5-B.3)",
        ContextView::DiffViewer => "Diff Viewer\n\n(Implementation in M5-B.4)",
        ContextView::CommitView => "Commit View\n\n(Implementation in M5-B.4)",
        ContextView::CompletionSummary(CompletionKind::Done) => {
            "Complete!\n\n[n] New Thread  [o] Open Thread"
        }
        ContextView::CompletionSummary(CompletionKind::Abandoned) => {
            "Thread Abandoned\n\n[n] New Thread  [o] Open Thread"
        }
    };
    // Use Pane widget for consistent borders/focus styling
    // Note: view_title() returns &'static str, avoiding temporary borrow issues
    let title = view_title(view);
    let pane = Pane::new(theme, borders)
        .title(title)  // &'static str from view_title()
        .focused(focused)
        .content(text);
    frame.render_widget(pane, area);
}

fn view_title(view: ContextView) -> &'static str {
    // Returns &'static str - no temporary lifetime issues
    match view {
        ContextView::NoThread => " Models ",
        ContextView::SpecEditor => " Spec ",
        ContextView::PreflightResults => " Preflight ",
        ContextView::RunConfig => " Configure ",
        ContextView::RunOutput => " Output ",
        ContextView::DecisionPrompt => " Decision ",
        ContextView::Summary => " Summary ",
        ContextView::DiffViewer => " Diff ",
        ContextView::CommitView => " Commit ",
        ContextView::CompletionSummary(_) => " Complete ",
    }
}
```

## Non-Goals

- **Actual view implementations**: SpecEditor, RunOutput, etc. are placeholders (M5-B.3/B.4)
- **Thread persistence**: Loading/saving threads (engine concern)
- **Keyboard action handlers**: Display hints for future keybindings, but handlers are implemented in later milestones. Keys that don't have handlers yet will simply do nothing when pressed.
- **Real thread data from engine**: Use mock `ThreadDisplay` for testing routing

**Note on hints**: Hints show what actions *will be* available for each phase once handlers are implemented. This lets us validate the UI flow before wiring up actual functionality.

## Acceptance Criteria

### Status Bar
- [ ] Status bar displays phase badge when thread is loaded
- [ ] Status bar displays thread title
- [ ] Status bar displays iteration metric (when Running/Paused/Verifying)
- [ ] Status bar displays next action hint
- [ ] Status bar shows "No Thread" when no thread loaded
- [ ] Model indicators remain visible (not replaced)

### Footer Hints
- [ ] Footer hints change based on current phase (see table above)
- [ ] Footer hints change based on focused pane (Timeline vs Context)
- [ ] Common hints (Tab, 1/2/3, ?, q) always present
- [ ] Refresh hint only shown when no thread and models panel visible
- [ ] All 17 phases have defined hints

### Phase Router
- [ ] Context pane routes to correct view based on `PhaseKind`
- [ ] All 17 phases map to appropriate views
- [ ] NoThread view shown when no thread loaded
- [ ] Placeholder text identifies which view is active

### Integration
- [ ] Uses `PhaseKind` from `ralf_engine::thread` (no duplicate enum)
- [ ] `ThreadDisplay::from_thread()` correctly extracts iteration/failure info
- [ ] Can cycle through phases with mock data to verify routing
- [ ] All existing tests pass
- [ ] New unit tests for `ContextView::from_phase()`
- [ ] New unit tests for `hints_for_state()`
- [ ] New unit tests for `StatusBarContent::next_action_hint()`

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_context_view_from_phase() {
    // Test all 17 PhaseKind values map correctly
    assert_eq!(ContextView::from_phase(None), ContextView::NoThread);
    assert_eq!(ContextView::from_phase(Some(PhaseKind::Drafting)), ContextView::SpecEditor);
    assert_eq!(
        ContextView::from_phase(Some(PhaseKind::Done)),
        ContextView::CompletionSummary(CompletionKind::Done)
    );
    assert_eq!(
        ContextView::from_phase(Some(PhaseKind::Abandoned)),
        ContextView::CompletionSummary(CompletionKind::Abandoned)
    );
    // ... all phases
}

#[test]
fn test_hints_for_state() {
    // Timeline focused (Split mode): should show j/k, Enter, y
    let hints = hints_for_state(
        Some(PhaseKind::Running),
        ScreenMode::Split,
        FocusedPane::Timeline,
        false
    );
    assert!(hints.iter().any(|h| h.key == "j/k"));

    // Context focused in Running (Split mode): should show Pause
    let hints = hints_for_state(
        Some(PhaseKind::Running),
        ScreenMode::Split,
        FocusedPane::Context,
        false
    );
    assert!(hints.iter().any(|h| h.key == "p"));

    // TimelineFocus mode: always shows timeline hints
    let hints = hints_for_state(
        Some(PhaseKind::Running),
        ScreenMode::TimelineFocus,
        FocusedPane::Context, // Ignored in focus mode
        false
    );
    assert!(hints.iter().any(|h| h.key == "j/k"));
}

#[test]
fn test_next_action_hint() {
    assert_eq!(StatusBarContent::next_action_hint(PhaseKind::Drafting), "Describe your task");
    assert_eq!(StatusBarContent::next_action_hint(PhaseKind::Running), "Loop in progress...");
    assert_eq!(StatusBarContent::next_action_hint(PhaseKind::Done), "Complete!");
}
```

### Existing Snapshot Tests

**IMPORTANT:** Existing shell layout snapshot tests in `crates/ralf-tui/src/lib.rs` (around line 541) call `layout::render_shell()`. These will need updates when the render_shell signature is extended.

**Signature change:** The existing `render_shell()` signature is **extended** (not replaced):
```rust
// Current signature (kept):
pub fn render_shell(..., show_models_panel: bool, ...) -> ...

// Extended to add thread parameter:
pub fn render_shell(..., show_models_panel: bool, thread: Option<&ThreadDisplay>, ...) -> ...
```

The `show_models_panel` parameter is preserved (as noted in Section 7). The `thread` parameter is **added** to enable dynamic status/hints.

Update checklist:
- [ ] Add `thread: Option<&ThreadDisplay>` parameter to `render_shell()`
- [ ] Update all existing snapshot test calls to pass `None` for thread (preserves existing behavior)
- [ ] Re-run `cargo insta review` to approve new snapshots

### New Unit Tests
Add these alongside existing tests in the respective modules:

```rust
// In crates/ralf-tui/src/thread_state.rs
#[test]
fn test_thread_display_from_running() {
    // Use Thread::new() and mutate - don't use struct literal (has many required fields)
    let mut thread = Thread::new("Test Feature");
    thread.phase = ThreadPhase::Running { iteration: 2 };
    // RunConfig requires both max_iterations AND models - use default and modify
    let mut config = RunConfig::default();
    config.max_iterations = 5;
    thread.run_config = Some(config);

    let display = ThreadDisplay::from_thread(&thread);
    assert_eq!(display.iteration, Some(2));
    assert_eq!(display.max_iterations, 5);
    assert_eq!(display.phase_display, "Running");
}

#[test]
fn test_thread_display_from_stuck() {
    let mut thread = Thread::new("Stuck Feature");
    thread.phase = ThreadPhase::Stuck {
        diagnosis: StuckDiagnosis {
            iterations_attempted: 5,
            models_tried: vec!["claude-sonnet".into()],  // Required field
            last_error: Some("Tests fail".into()),
            best_criteria_passed: 2,
            total_criteria: 3,
        },
    };

    let display = ThreadDisplay::from_thread(&thread);
    assert!(display.failure_reason.is_some());
    assert!(display.failure_reason.unwrap().contains("2/3"));
}

// In crates/ralf-tui/src/context/router.rs
#[test]
fn test_context_view_all_phases() {
    use PhaseKind::*;
    // Test each phase maps correctly
    assert_eq!(ContextView::from_phase(None), ContextView::NoThread);
    assert_eq!(ContextView::from_phase(Some(Drafting)), ContextView::SpecEditor);
    assert_eq!(ContextView::from_phase(Some(Running)), ContextView::RunOutput);
    assert_eq!(
        ContextView::from_phase(Some(Done)),
        ContextView::CompletionSummary(CompletionKind::Done)
    );
    // ... test all 17 phases
}
```

### Visual Snapshot Tests
Follow existing pattern in `lib.rs` - extend the existing test helpers to accept thread state:

```rust
// Update the existing test helper to accept thread state:
fn render_shell_to_string_with_thread(
    models: &[ModelStatus],
    timeline: &TimelineState,
    thread: Option<&ThreadDisplay>,  // New parameter
    // ... other existing params
) -> String {
    // ... render using Backend::TestBackend ...
}

#[test]
fn test_shell_with_thread_running() {
    let thread = ThreadDisplay {
        id: "test-001".into(),
        title: "Test Feature".into(),
        phase_kind: PhaseKind::Running,
        phase_display: "Running".into(),
        iteration: Some(2),
        max_iterations: 5,
        failure_reason: None,
    };
    // Extend existing helper - don't create new one
    let result = render_shell_to_string_with_thread(&models, &timeline, Some(&thread), ...);
    insta::assert_snapshot!("shell_running_phase", result);
}
```

### Manual Testing
Use hardcoded mock `ThreadDisplay` in `ShellApp` to verify routing visually during development. No debug command needed - just modify mock data and rebuild.

## Dependencies

- M5-A (Shell) - Completed
- M5-B.1 (Timeline Foundation) - Completed
- `ralf_engine::thread::PhaseKind` - Existing

## File Changes

| File | Change |
|------|--------|
| `crates/ralf-tui/src/thread_state.rs` | New - ThreadDisplay extraction |
| `crates/ralf-tui/src/context/mod.rs` | New - Context module |
| `crates/ralf-tui/src/context/router.rs` | New - Phase router |
| `crates/ralf-tui/src/widgets/status_bar.rs` | Update - Add phase/title/metric |
| `crates/ralf-tui/src/widgets/footer_hints.rs` | Update - Add `hints_for_state()` function |
| `crates/ralf-tui/src/layout/shell.rs` | Update - Wire up routing |
| `crates/ralf-tui/src/shell.rs` | Update - Add thread state |
| `crates/ralf-tui/src/lib.rs` | Update - Export new modules |

## Resolved Questions

1. **Thread loading**: Use mock `ThreadDisplay` for testing; real loading comes from engine in later milestone.
2. **Models panel**: Shows when no thread loaded (`show_models_panel = true`). When thread loads, context pane shows phase-appropriate view.

## Integration Story

This milestone creates the **display infrastructure** for thread state but does NOT connect to real engine data:

1. **ShellApp gets new field**: `current_thread: Option<ThreadDisplay>`
2. **Rendering uses this field**: Status bar, footer hints, and context routing all read from `current_thread`
3. **Field is populated with mock data** for testing: hardcoded `ThreadDisplay` with specific phase for visual verification
4. **`show_models_panel` logic**: When `current_thread.is_some()`, set `show_models_panel = false` so context pane routes to phase view instead of ModelsPanel
5. **Real engine connection** comes in later milestone (M5-C or beyond) when we have thread persistence and loading

**What drives ThreadDisplay in this milestone:**
- For unit tests: construct `ThreadDisplay` directly with test data
- For manual visual testing: hardcode a mock in `ShellApp::new()` or toggle via compile-time flag
- For production: field remains `None` until later milestone adds real loading

## Keybinding Notes

**Context-dependent keys:**
- `r` is "Refresh models" when no thread; phase-specific `r` (Run, Resume, Retry) when thread loaded
- `a` is "Abandon" in Paused/Stuck phases; "Approve" in PendingReview phase
- These don't conflict because phase determines which action is active

**Esc behavior:**
- Currently `Esc` quits the app (same as `q`) - this milestone does not change this
- Future milestones may make `Esc` context-sensitive when implementing action handlers

**State machine constraints:**
- Running can transition to Paused (`p` key in TUI)
- Verifying cannot transition to Paused (must wait for completion → Stuck/Implemented)
- Hints show *actionable UI options* for each phase, not a 1:1 mapping to state transitions
- Example: `m: Models` is a UI action to open config, not a state transition
- The `Abandoned` transition is globally valid but only shown in specific phases where abandoning is a primary action (Paused, Stuck)

**TUI vs CLI pause behavior:**
- `docs/state-machine.md` references "Ctrl+C" for pause (CLI context)
- In the TUI, we use `p` key instead because:
  - `Ctrl+C` is often intercepted by terminal emulators or triggers OS signals
  - `Ctrl+C` is already used for "Copy" when timeline-focused (vim convention)
  - `p` follows the pattern of other single-key actions in the TUI
- The state transition is the same (Running → Paused); only the trigger differs

**Abandon availability:**
- `a: Abandon` is shown in Paused and Stuck phases where it's a primary decision
- In PendingReview, `a: Approve` takes precedence; abandon is available via menu (future)
