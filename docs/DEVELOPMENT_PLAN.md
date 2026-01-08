# ralf Development Plan

> **ralf**: An opinionated multi-model AI development tool inspired by Ralph Wiggum.
> "Me fail English? That's unpossible!" - Ralph Wiggum

## Vision

ralf is a thread-centric AI development assistant that guides developers through a structured workflow for implementing features, fixes, and improvements. Each "thread" represents a single work item that progresses through well-defined phases from initial idea to merged code.

### Core Principles

1. **Thread as Workflow**: Each feature/fix is a thread with explicit state, not just a conversation
2. **Human Checkpoints**: AI automates the middle, humans approve the gates (NEVER bypassed)
3. **Multi-Model**: Leverage different AI models for different strengths
4. **Recoverable**: Can resume, backtrack, and diagnose failures (requires git safety)
5. **Two Speeds**: Methodical for learning/complex work, quick for confident/simple changes

## Architecture Overview

### Current State (Screen-Centric)

```
┌─────────────────────────────────────┐
│            TUI App                  │
│  ┌─────────┐ ┌─────────┐ ┌───────┐  │
│  │Settings │ │SpecStudio│ │Status │  │
│  └─────────┘ └─────────┘ └───────┘  │
│       (screens are independent)     │
└─────────────────────────────────────┘
```

**Problems:**
- No persistent workflow state
- Can't resume sessions
- No concept of "where am I in the process?"
- Screens don't know about each other
- No git safety for backward transitions

### Target State (Thread-Centric)

```
┌─────────────────────────────────────┐
│            TUI App                  │
│         ┌───────────┐               │
│         │  Thread   │               │
│         │  Manager  │               │
│         └─────┬─────┘               │
│               │                     │
│         ┌─────▼─────┐               │
│         │  Active   │               │
│         │  Thread   │               │
│         └─────┬─────┘               │
│               │                     │
│         ┌─────▼─────┐               │
│         │  Phase    │               │
│         │  Router   │               │
│         └─────┬─────┘               │
│               │                     │
│    ┌──────────┼──────────┐          │
│    ▼          ▼          ▼          │
│ ┌──────┐ ┌────────┐ ┌────────┐      │
│ │Draft │ │  Run   │ │ Review │ ...  │
│ │Screen│ │ Screen │ │ Screen │      │
│ └──────┘ └────────┘ └────────┘      │
└─────────────────────────────────────┘
```

**Benefits:**
- Thread knows its phase, drives the UI
- Persistent state survives restarts
- Clear "where am I?" at all times
- Natural forward/backward navigation
- Git safety enables safe rollbacks

## State Machine

See [state-machine.md](./state-machine.md) for the complete state diagram.

### Phases Summary

| Phase | States | Description |
|-------|--------|-------------|
| **1. Spec Creation** | Drafting → Assessing → Finalized | Define what to build |
| **2. Implementation** | Preflight → Configuring → Running → Verifying → (Stuck/Implemented) | AI builds it |
| **3. Polish** | Polishing (optional, loops back to Implemented) | Add docs/tests |
| **4. Review** | PendingReview → Approved | Human verifies |
| **5. Complete** | ReadyToCommit → Done | Ship it |

### Human Checkpoints (Never Bypassed)

Four transitions **require** human approval:

1. **Finalize Spec**: "Yes, build exactly this"
2. **Handle Stuck**: "What to do: revise, reconfigure, assist, or abandon"
3. **Approve Review**: "Yes, this implementation is correct"
4. **Commit**: "Yes, merge this to the codebase"

> **Quick mode** auto-advances TO these checkpoints but never PAST them.

## Data Model

### Thread Structure (Minimal, Phase is Truth)

```rust
pub struct Thread {
    // Identity
    pub id: String,                    // UUID
    pub title: String,                 // Human-readable name
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Current State (SINGLE SOURCE OF TRUTH)
    pub phase: ThreadPhase,

    // Pointers (NOT duplicated in phase enum)
    pub current_spec_revision: u32,
    pub current_run_id: Option<String>,

    // Configuration
    pub mode: ThreadMode,              // Quick or Methodical
    pub run_config: Option<RunConfig>,

    // Git baseline (captured at Preflight)
    pub baseline: Option<GitBaseline>,
}

pub struct GitBaseline {
    pub branch: String,
    pub commit_sha: String,
    pub captured_at: DateTime<Utc>,
}

pub enum ThreadMode {
    Quick,
    Methodical,
}

pub enum ThreadPhase {
    // Phase 1: Spec Creation
    Drafting,
    Assessing,           // feedback stored in separate file
    Finalized,

    // Phase 2: Implementation
    Preflight,
    PreflightFailed { reason: String },
    Configuring,
    Running { iteration: u32 },
    Paused { iteration: u32 },
    Verifying,
    Stuck { diagnosis: StuckDiagnosis },
    Implemented,

    // Phase 3: Polish
    Polishing,

    // Phase 4: Review
    PendingReview,
    Approved,

    // Phase 5: Complete
    ReadyToCommit,
    Done { commit_sha: String },

    // Terminal
    Abandoned { reason: String },
}

pub struct StuckDiagnosis {
    pub iterations_attempted: u32,
    pub models_tried: Vec<String>,
    pub best_criteria_passed: u32,
    pub total_criteria: u32,
    pub last_error: Option<String>,
}
```

### File Structure

```
.ralf/
├── config.json                 # Global configuration
├── threads/
│   ├── active.json            # { "thread_id": "abc123" }
│   ├── abc123/
│   │   ├── thread.json        # Thread state (small, atomic writes)
│   │   ├── baseline.json      # Git baseline captured at Preflight
│   │   ├── spec/
│   │   │   ├── v1.md          # First draft
│   │   │   └── v2.md          # Finalized version
│   │   ├── runs/
│   │   │   ├── run-001/
│   │   │   │   ├── run.json   # Run metadata
│   │   │   │   └── output.log # Model outputs (append-only)
│   │   │   └── run-002/
│   │   │       └── ...
│   │   ├── assessment.md      # AI feedback (if requested)
│   │   └── transcript.jsonl   # Conversation (append-only, line-delimited)
│   └── def456/
│       └── ...
└── archive/                    # Completed/abandoned threads
    └── ...
```

> **Key insight**: `thread.json` stays small. Large artifacts (transcripts, outputs) in separate append-only files.

## Development Phases

### Foundation (F1-F6) - REQUIRED FIRST

Everything else depends on threads with state and git safety.

#### F1: Thread State Model
**File:** `ralf-engine/src/thread.rs`

- Define `Thread` struct (minimal, as shown above)
- Define `ThreadPhase` enum with all states
- Define `StuckDiagnosis`, `GitBaseline`, etc.

**Acceptance Criteria:**
- [ ] All phases from state machine represented
- [ ] No duplication between phase enum and struct fields
- [ ] Serializable to JSON with schema version
- [ ] Unit tests for struct creation

#### F2: State Transitions
**File:** `ralf-engine/src/thread.rs`

```rust
impl Thread {
    /// Check if transition is valid per state machine
    pub fn can_transition_to(&self, target: &ThreadPhase) -> Result<(), TransitionError>;

    /// Execute transition (updates state, timestamps)
    pub fn transition_to(&mut self, target: ThreadPhase) -> Result<(), TransitionError>;

    /// Get valid next phases from current phase
    pub fn available_transitions(&self) -> Vec<ThreadPhase>;

    /// Check if this transition requires workspace reset
    pub fn requires_workspace_reset(&self, target: &ThreadPhase) -> bool;
}
```

**Acceptance Criteria:**
- [ ] All valid transitions from state machine work
- [ ] Invalid transitions return clear errors
- [ ] Backward transitions identified for workspace reset
- [ ] Abandon works from any non-terminal state
- [ ] Unit tests cover all transition paths

#### F3: Git Safety Layer
**File:** `ralf-engine/src/git.rs` (new)

```rust
pub struct GitSafety {
    repo_path: PathBuf,
}

impl GitSafety {
    /// Check if working tree is clean
    pub fn is_clean(&self) -> Result<bool, GitError>;

    /// Capture baseline (branch + commit SHA)
    pub fn capture_baseline(&self) -> Result<GitBaseline, GitError>;

    /// Create thread branch: ralf/<thread-id>
    pub fn create_thread_branch(&self, thread_id: &str) -> Result<(), GitError>;

    /// Reset to baseline (with user confirmation in TUI)
    pub fn reset_to_baseline(&self, baseline: &GitBaseline) -> Result<(), GitError>;

    /// Get diff from baseline to current state
    pub fn diff_from_baseline(&self, baseline: &GitBaseline) -> Result<String, GitError>;
}
```

**Acceptance Criteria:**
- [ ] Can detect clean/dirty working tree
- [ ] Can capture and restore baseline
- [ ] Branch creation works
- [ ] Diff generation works
- [ ] Graceful degradation if not a git repo

#### F4: Thread Persistence
**File:** `ralf-engine/src/thread.rs`

```rust
impl Thread {
    /// Save with atomic write pattern
    pub fn save(&self, base_path: &Path) -> Result<(), ThreadError>;

    /// Load with schema migration
    pub fn load(base_path: &Path, id: &str) -> Result<Thread, ThreadError>;

    /// List all threads with summary
    pub fn list(base_path: &Path) -> Result<Vec<ThreadSummary>, ThreadError>;

    /// Get/set active thread
    pub fn get_active(base_path: &Path) -> Result<Option<String>, ThreadError>;
    pub fn set_active(base_path: &Path, id: &str) -> Result<(), ThreadError>;
}
```

**Acceptance Criteria:**
- [ ] Atomic writes (write tmp, fsync, rename)
- [ ] Schema versioning in JSON
- [ ] Thread survives save/load round-trip
- [ ] Spec revisions saved as separate files
- [ ] Active thread tracking works
- [ ] Integration tests with temp directories

#### F5: Preflight Checks
**File:** `ralf-engine/src/preflight.rs` (new)

```rust
pub struct PreflightResult {
    pub passed: bool,
    pub checks: Vec<PreflightCheck>,
}

pub struct PreflightCheck {
    pub name: String,
    pub passed: bool,
    pub message: String,
}

pub fn run_preflight(thread: &Thread, config: &Config) -> PreflightResult {
    // 1. Git state clean (or on managed branch)
    // 2. Baseline can be captured
    // 3. Spec has promise tag
    // 4. Criteria parseable
    // 5. Required models available
    // 6. No other thread is Running
}
```

**Acceptance Criteria:**
- [ ] All checks implemented
- [ ] Clear error messages for each failure
- [ ] Returns structured result for UI display
- [ ] Single-thread enforcement works

#### F6: Thread-Aware TUI
**Files:** `ralf-tui/src/app.rs`, `ralf-tui/src/lib.rs`

- On startup: Load active thread (or show thread list/create new)
- Route to screen based on `thread.phase`
- Update thread on phase transitions
- Save thread on significant changes
- Show phase indicator in status bar

**Acceptance Criteria:**
- [ ] TUI loads existing thread on startup
- [ ] Correct screen shown for each phase
- [ ] Thread state persists across TUI restart
- [ ] Phase indicator visible in UI
- [ ] Transitions trigger appropriate saves

---

### Stuck State (High Value)

Loops will get stuck. Good UX here is critical.

#### S1: Stuck State View
**Files:** `ralf-tui/src/screens/stuck.rs` (new)

Display:
- How many iterations were attempted
- Which models were tried
- Best verification result achieved (X/Y criteria)
- Which criteria passed/failed
- Diff from baseline

Actions:
- [R] Revise spec → **prompt for workspace reset**, then Drafting
- [C] Reconfigure → Configuring (keep changes)
- [M] Manual assist → user edits, then Running
- [A] Abandon → Abandoned
- [D] Diagnose → show detailed logs (stays in Stuck)

**Acceptance Criteria:**
- [ ] Clear display of what was attempted
- [ ] All actions work and transition correctly
- [ ] Workspace reset prompts and works
- [ ] Diagnosis view shows useful information

---

### Paused State (User Control)

User can interrupt without losing progress.

#### P1: Paused State Handling
**Files:** `ralf-tui/src/lib.rs`, `ralf-tui/src/screens/paused.rs` (new)

- Ctrl+C during Running → Paused
- Display current progress
- Options: Resume, Reconfigure, Abandon

**Acceptance Criteria:**
- [ ] Ctrl+C gracefully pauses (doesn't kill)
- [ ] Progress preserved
- [ ] Can resume from exact point
- [ ] Can reconfigure and restart

---

### Review Phase (Core Loop Completion)

#### R1: Diff View
**Files:** `ralf-tui/src/screens/review.rs` (new), uses `ralf-engine/src/git.rs`

- Generate diff from baseline to current state
- Display in scrollable, syntax-highlighted view
- Show summary (files changed, lines +/-)

**Acceptance Criteria:**
- [ ] Diff generated correctly from baseline
- [ ] Syntax highlighting works
- [ ] Scrollable in TUI

#### R2: Review Screen
**File:** `ralf-tui/src/screens/review.rs`

Display:
- Diff summary and full diff
- Verification results
- Test output (if any)

Actions:
- [A] Approve → Approved
- [R] Reject (spec issue) → **prompt for reset**, Drafting
- [B] Reject (impl bug) → Running (continue fixing)

**Acceptance Criteria:**
- [ ] All information visible
- [ ] Approve transitions correctly
- [ ] Reject with reset prompts and works
- [ ] Clear UX for the decision

---

### Completion Phase (End-to-End)

#### C1: Commit Preparation
**Files:** `ralf-tui/src/screens/commit.rs` (new)

- Generate commit message from spec + changes
- Show what will be committed
- Allow human to edit message
- Execute commit on approval

**Acceptance Criteria:**
- [ ] Reasonable commit message generated
- [ ] Human can edit before committing
- [ ] Commit executes on thread branch
- [ ] Thread transitions to Done with SHA

---

### Multi-Thread Support (V1: Single Running)

#### M1: Thread List View
**File:** `ralf-tui/src/screens/thread_list.rs` (new)

- Show all threads with phase indicators
- Allow selection/switching
- Create new thread
- **Block** starting a run if another thread is Running

#### M2: Thread Switching
- Save current thread
- Load selected thread
- UI updates to new thread's phase

**Acceptance Criteria:**
- [ ] Thread list shows all threads
- [ ] Can switch between threads
- [ ] Running thread blocks other runs (not just warns)
- [ ] Clear indicator of which thread is active

---

### Nice-to-Have Features (Post-V1)

#### Assessment Integration (Optional)
- AI reviews spec before finalize
- Shows feedback on clarity, testability, scope
- User can revise or proceed anyway
- **Not a core phase** - just a lint tool from Drafting

#### Polish Screen (Optional)
- Prompt to add docs/tests after Implemented
- AI can help generate these
- Skip button to proceed directly to review

#### Quick Mode Preset
- `--quick` flag or toggle in TUI
- Same states, fewer stops
- Auto-advances to checkpoints (never past them)
- Shows "Recommend Approve" if tests pass

## Implementation Sequence

```
Week 1-2: Foundation + Git Safety
├── F1: Thread State Model
├── F2: State Transitions
├── F3: Git Safety Layer        ← Critical for backward transitions
├── F4: Thread Persistence
├── F5: Preflight Checks
└── F6: Thread-Aware TUI

Week 3: User Control + Stuck
├── S1: Stuck State View
└── P1: Paused State Handling

Week 4: Review + Completion
├── R1: Diff View
├── R2: Review Screen
└── C1: Commit Preparation

──── DOGFOODABLE ────

Week 5+: Multi-Thread + Polish
├── M1-M2: Thread List + Switching
├── Assessment (optional)
├── Polish Screen (optional)
└── Quick Mode Preset
```

## Success Criteria

### Dogfoodable (MVP)
- [ ] Can create a thread with a spec
- [ ] Can finalize spec (human checkpoint)
- [ ] Preflight validates before running
- [ ] Git baseline captured, can restore
- [ ] Can run implementation loop
- [ ] Can pause/resume loop
- [ ] Stuck state handled gracefully with workspace reset option
- [ ] Can review changes with diff view (human checkpoint)
- [ ] Can commit (human checkpoint)
- [ ] Thread survives TUI restart
- [ ] Clear indication of current phase at all times

### Complete Product
- [ ] All MVP criteria
- [ ] Multiple threads supported (single Running enforced)
- [ ] Assessment helps improve specs (optional)
- [ ] Polish phase for docs/tests (optional)
- [ ] Quick mode for simple changes
- [ ] Thread history/archive viewable

## Key Decisions Made

Based on external reviews (Gemini, Codex):

1. **Git safety is foundational** - Implemented in F3 before anything that depends on backward transitions

2. **Polish before review, not after** - Any code changes after approval would invalidate the approval

3. **Quick mode = same states, fewer stops** - Never bypasses human checkpoints, just auto-advances to them

4. **Single running thread for V1** - Block (not warn) multiple concurrent runs

5. **Minimal Thread struct** - Phase enum is source of truth, large artifacts in separate files

6. **Assessment is optional** - Nice-to-have lint tool, not core workflow

7. **Preflight is required** - Can't enter Running without validating prerequisites

8. **Paused state exists** - User can interrupt without losing progress

## Appendix: Existing Code to Refactor

### `ralf-engine/src/chat.rs`
- Current `Thread` struct → rename to `Conversation` or deprecate
- `ChatMessage` → keep, referenced from Thread via transcript file

### `ralf-tui/src/app.rs`
- `App.thread` → becomes `App.active_thread: Option<Thread>`
- `App.screen` → derived from `active_thread.phase` where possible
- Add phase indicator to status bar

### `ralf-engine/src/runner.rs`
- `start_run` → takes Thread, updates Thread phase
- "Max iterations" → transitions to Stuck, not Completed
- Add support for Paused state

### `ralf-engine/src/state.rs`
- `RunState` → may be absorbed into Thread or kept for run-specific state

## References

- [State Machine Diagram](./state-machine.md)
- [External Reviews](./REVIEWS.md)
- [README](../README.md)
