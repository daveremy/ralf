# Review Request Template

## Context

ralf is an AI-assisted development tool that orchestrates multi-model autonomous loops for implementing features and fixes. It uses a thread-centric, phase-driven workflow with 17 states.

## Files to Review

When requesting spec or implementation reviews, include relevant files:

**Architecture:**
- `docs/state-machine.md` - State transition diagram and phase definitions
- `docs/ROADMAP.md` - Project status and roadmap

**TUI Design:**
- `docs/planning/TUI_UX_PRINCIPLES.md` - UX design decisions
- `docs/planning/TUI_DEV_PLAN.md` - TUI implementation phases

**Specs:**
- `docs/planning/SPEC-*.md` - Individual feature specs

**Code:**
- `crates/ralf-engine/src/` - Engine implementation
- `crates/ralf-tui/src/` - TUI implementation

## Spec Review Prompt

```
Review this spec for:
1. Completeness - are all requirements captured?
2. Correctness - does the design make sense?
3. Consistency - does it fit the existing architecture?
4. Testability - can acceptance criteria be verified?
5. Edge cases - what's missing?

Provide a PASSED or FAILED verdict with specific issues.
```

## Implementation Review Prompt

```
Review this implementation against the spec. Check for:
1. Correctness - does it implement the spec correctly?
2. Safety - any security or memory safety issues?
3. Error handling - are all error cases covered?
4. Test coverage - are tests comprehensive?
5. Acceptance criteria - are all criteria met?

Provide a PASSED or FAILED verdict with specific issues.
```

## UX/Design Review Prompt

```
Review this UX design for:
1. Usability - is the workflow intuitive?
2. Consistency - do patterns repeat predictably?
3. Learnability - can users discover features without docs?
4. Accessibility - are there barriers for any users?
5. Completeness - are all states and transitions covered?

Provide specific feedback and suggestions.
```
