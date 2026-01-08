# F1: Thread State Model

## Promise

Define the core data structures that represent a ralf thread and its lifecycle phases. These types will be the foundation for all thread-related functionality.

## Deliverables

**File:** `crates/ralf-engine/src/thread.rs` (new)

### Types to Define

1. **`Thread`** - The main struct representing a work item
2. **`ThreadPhase`** - Enum of all possible states from the state machine
3. **`ThreadMode`** - Quick vs Methodical mode
4. **`StuckDiagnosis`** - Details when a thread gets stuck
5. **`GitBaseline`** - Captured git state for workspace reset
6. **`RunConfig`** - Configuration for implementation runs (stub for F1)

### Thread Struct

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
    pub mode: ThreadMode,
    pub run_config: Option<RunConfig>,

    // Git baseline (captured at Preflight)
    pub baseline: Option<GitBaseline>,
}
```

### ThreadPhase Enum

Must represent ALL states from `docs/state-machine.md`:

```rust
pub enum ThreadPhase {
    // Phase 1: Spec Creation
    Drafting,
    Assessing,
    Finalized,

    // Phase 2: Implementation
    Preflight,
    PreflightFailed { reason: String },
    Configuring,
    Running { iteration: u32 },
    Paused { iteration: u32 },
    Verifying { iteration: u32 },  // iteration for UI display and restart context
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
```

### Supporting Types

```rust
pub struct GitBaseline {
    pub branch: String,
    pub commit_sha: String,
    pub captured_at: DateTime<Utc>,
}

pub enum ThreadMode {
    Quick,
    Methodical,
}

pub struct StuckDiagnosis {
    pub iterations_attempted: u32,
    pub models_tried: Vec<String>,
    pub best_criteria_passed: u32,
    pub total_criteria: u32,
    pub last_error: Option<String>,
}

/// Configuration for a run (stub for F1, expanded in later milestones)
pub struct RunConfig {
    pub max_iterations: u32,
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
```

### Required Methods

```rust
impl Thread {
    /// Create a new thread with a title
    pub fn new(title: impl Into<String>) -> Self;

    /// Get the phase category (1-5) for UI grouping
    pub fn phase_category(&self) -> u8;

    /// Check if thread is in a terminal state
    pub fn is_terminal(&self) -> bool;

    /// Human-readable phase name for status display
    pub fn phase_display_name(&self) -> &'static str;
}

impl Default for ThreadPhase {
    fn default() -> Self {
        ThreadPhase::Drafting
    }
}
```

## Acceptance Criteria

- [ ] All phases from state-machine.md are represented in `ThreadPhase`
- [ ] No duplication between phase enum variants and Thread struct fields
- [ ] Thread struct has all fields needed for workflow (id, title, timestamps, phase, pointers, config, baseline)
- [ ] Structs derive `Serialize, Deserialize` for JSON persistence
- [ ] Structs derive `Debug, Clone` for debugging and flexibility
- [ ] `Thread::new()` creates thread in Drafting phase with UUID
- [ ] `Thread::new()` defaults mode to `Methodical`
- [ ] `Thread::is_terminal()` returns true for Done and Abandoned
- [ ] `Thread::phase_display_name()` returns human-readable names
- [ ] Round-trip JSON serialization/deserialization works for all types
- [ ] `Thread::phase_category()` returns 1-5 for each phase group
- [ ] `cargo build -p ralf-engine` succeeds
- [ ] `cargo clippy -p ralf-engine` has no warnings
- [ ] At least 5 unit tests covering: new(), is_terminal(), phase_category(), phase_display_name(), JSON round-trip
- [ ] Module is exported from `ralf-engine/src/lib.rs`

## Non-Goals (for F1)

- State transitions (F2)
- Persistence/serialization to disk (F4)
- Git operations (F3)
- Preflight checks (F5)

## Dependencies

- `chrono` for DateTime (already in Cargo.toml)
- `serde` for Serialize/Deserialize (already in Cargo.toml)
- `uuid` for generating thread IDs (may need to add)
