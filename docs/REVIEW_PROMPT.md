# Review Request: ralf Development Plan

## Context

ralf is an AI-assisted development tool that orchestrates multi-model autonomous loops for implementing features and fixes. We've designed a state machine-based workflow and development plan.

## What We'd Like Reviewed

Please review the following documents and provide feedback on:

### 1. State Machine Design (`docs/state-machine.md`)
- Are the states and transitions complete?
- Are there edge cases or failure modes we haven't considered?
- Is the "Stuck" state handling sufficient?
- Are the human checkpoints in the right places?

### 2. Development Plan (`docs/DEVELOPMENT_PLAN.md`)
- Is the phased approach logical?
- Are there dependencies we've missed?
- Is the "Foundation first" strategy correct?
- Are the acceptance criteria clear and testable?
- Are there simpler approaches we should consider?

### 3. Architecture
- Is the shift from screen-centric to thread-centric the right call?
- Is the data model (Thread struct, file structure) appropriate?
- Are there scalability concerns?

### 4. General
- What are the biggest risks to this plan?
- What would you prioritize differently?
- Are there similar tools/patterns we should learn from?
- What's missing that we haven't thought of?

## Specific Questions

1. **Backward transitions**: We allow going from Review back to Drafting if the spec was wrong. Is this too disruptive? Should there be an intermediate "revise spec" state?

2. **Multi-thread complexity**: Is supporting multiple threads worth the complexity for v1, or should we enforce single-thread until the core flow is solid?

3. **Quick mode**: Is it premature to design for quick mode now, or should it be considered from the start to avoid painting ourselves into a corner?

4. **Assessment phase**: Is AI-reviewing-the-spec valuable enough to include, or is it gold-plating?

5. **Git integration**: Should ralf manage git branches, or stay out of git workflow entirely and let users handle branching?

## Files to Review

- `docs/state-machine.md` - State transition diagram and reference
- `docs/DEVELOPMENT_PLAN.md` - Comprehensive development plan
- `crates/ralf-engine/src/` - Current engine implementation
- `crates/ralf-tui/src/` - Current TUI implementation
