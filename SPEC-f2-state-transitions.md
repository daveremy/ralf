# F2: State Transitions

## Promise

Implement state transition logic for Thread, enforcing the state machine rules from `docs/state-machine.md`. This enables safe navigation through the workflow with clear validation and error handling.

## Deliverables

**File:** `crates/ralf-engine/src/thread.rs` (extend existing)

### New Types

```rust
/// Error returned when a transition is invalid.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TransitionError {
    #[error("Cannot transition from {from} to {to}: {reason}")]
    InvalidTransition {
        from: String,
        to: String,
        reason: String,
    },

    #[error("Cannot transition from terminal state {0}")]
    FromTerminalState(String),
}
```

### New Methods on Thread

```rust
impl Thread {
    /// Check if transition to target phase is valid per state machine.
    /// Returns Ok(()) if valid, Err with reason if not.
    pub fn can_transition_to(&self, target: &ThreadPhase) -> Result<(), TransitionError>;

    /// Execute transition: validates, updates phase, updates timestamp.
    /// Returns error if transition is invalid.
    pub fn transition_to(&mut self, target: ThreadPhase) -> Result<(), TransitionError>;

    /// Get all valid next phases from current phase.
    /// Always includes Abandoned for non-terminal states.
    pub fn available_transitions(&self) -> Vec<ThreadPhase>;

    /// Check if transitioning to target requires workspace reset.
    /// True for backward transitions that discard implementation work.
    pub fn requires_workspace_reset(&self, target: &ThreadPhase) -> bool;
}
```

## Valid Transitions (from state-machine.md)

### Phase 1: Spec Creation
| From | To | Trigger |
|------|-----|---------|
| Drafting | Assessing | request review |
| Drafting | Finalized | human approves (skip assess) |
| Assessing | Drafting | revise needed |
| Assessing | Finalized | human approves |
| Finalized | Drafting | reopen spec |
| Finalized | Preflight | begin implementation |

### Phase 2: Implementation
| From | To | Trigger |
|------|-----|---------|
| Preflight | Configuring | checks pass |
| Preflight | PreflightFailed | checks fail |
| PreflightFailed | Preflight | retry |
| PreflightFailed | Drafting | fix spec (BACKWARD) |
| Configuring | Running | start loop |
| Running | Verifying | model claims done |
| Running | Paused | user interrupt |
| Running | Stuck | max iterations |
| Paused | Running | resume |
| Paused | Configuring | reconfigure |
| Verifying | Running | criteria failed, iterations remain |
| Verifying | Stuck | criteria failed, no iterations |
| Verifying | Implemented | all criteria pass |
| Stuck | Configuring | reconfigure |
| Stuck | Running | manual assist |
| Stuck | Drafting | spec was wrong (BACKWARD) |

### Phase 3: Polish
| From | To | Trigger |
|------|-----|---------|
| Implemented | Polishing | add docs/tests |
| Polishing | Implemented | polish done |
| Implemented | PendingReview | ready for review |

### Phase 4: Review
| From | To | Trigger |
|------|-----|---------|
| PendingReview | Approved | human approves |
| PendingReview | Running | impl bugs (BACKWARD) |
| PendingReview | Drafting | spec was wrong (BACKWARD) |

### Phase 5: Complete
| From | To | Trigger |
|------|-----|---------|
| Approved | ReadyToCommit | prepare commit |
| ReadyToCommit | Done | human commits |

### Abandon
| From | To |
|------|-----|
| Any non-terminal | Abandoned |

## Backward Transitions

Per `docs/state-machine.md`, backward transitions have different workspace actions:

**Require workspace reset (discard implementation):**
- `Stuck → Drafting` - Reset to baseline
- `PendingReview → Drafting` - Reset to baseline

**No workspace action:**
- `PreflightFailed → Drafting` - No reset (no implementation yet)
- `PendingReview → Running` - Continue fixing (keep changes)

## Acceptance Criteria

- [ ] `can_transition_to()` returns Ok for all valid transitions in tables above
- [ ] `can_transition_to()` returns Err with clear reason for invalid transitions
- [ ] `can_transition_to()` returns `FromTerminalState` error from Done/Abandoned
- [ ] `transition_to()` updates phase and `updated_at` timestamp on success
- [ ] `transition_to()` returns error and leaves state unchanged on failure
- [ ] `available_transitions()` returns correct list for each phase
- [ ] `available_transitions()` always includes Abandoned for non-terminal states
- [ ] `available_transitions()` returns empty vec for terminal states
- [ ] `requires_workspace_reset()` returns true for `Stuck → Drafting` and `PendingReview → Drafting`
- [ ] `requires_workspace_reset()` returns false for `PreflightFailed → Drafting` and `PendingReview → Running`
- [ ] `requires_workspace_reset()` returns false for forward transitions
- [ ] `TransitionError` implements Display with helpful messages
- [ ] Unit tests cover all valid transitions (minimum 30 test cases)
- [ ] Unit tests verify invalid transitions are rejected
- [ ] `cargo build -p ralf-engine` succeeds
- [ ] `cargo clippy -p ralf-engine` has no warnings
- [ ] `cargo test -p ralf-engine` passes

## Non-Goals (for F2)

- Actually performing workspace reset (F3: Git Safety)
- Persistence of transitions (F4: Thread Persistence)
- UI integration (F6: Thread-Aware TUI)

## Implementation Notes

- Use a match-based approach for `can_transition_to` to make the state machine explicit
- Consider grouping transitions by source phase for readability
- The `Running`, `Paused`, `Verifying` phases carry `iteration` data that should be preserved or updated appropriately during transitions
- `available_transitions()` returns phase variants with sensible defaults:
  - For phases needing iteration data, use 0 or current iteration as appropriate
  - For phases needing reason/diagnosis, use empty defaults
  - Caller is responsible for populating actual data before calling `transition_to()`
- `can_transition_to()` validates phase kind (discriminant), not internal data consistency
  - Data validation (e.g., iteration not regressing) is caller's responsibility
