# ralf Roadmap

> **ralf**: An opinionated multi-model AI development tool for thread-centric workflows.

This document is the single source of truth for project status and direction.

---

## Project Status

### Completed

**Milestone 0: Repo Bootstrap**
- Cargo workspace with `ralf` binary and `ralf-engine`/`ralf-tui` crates
- CI: formatting, linting, tests
- Core documentation

**Milestone 1: Engine Core**
- Config/state storage under `.ralf/`
- Model discovery (claude, codex, gemini on PATH)
- Loop runner with rate-limit detection, cooldowns, verifiers
- Changelog generation
- Promise tag completion policy

**Milestone 2-4: Initial TUI**
- TUI shell with ratatui + crossterm
- Welcome/Setup screens
- Spec Studio (chat + finalize)
- Run Dashboard (basic)
- *Note: Being replaced by new TUI architecture*

**Foundation Features (F1-F5)**
- F1: Thread state model (`ThreadPhase` enum, 17 phases)
- F2: State transitions with validation
- F3: Git safety layer (baseline capture, branch management)
- F4: Thread persistence (atomic writes, spec revisions)
- F5: Preflight checks (7 validations before run)

---

## Current: TUI Rebuild (M5)

The TUI is being rebuilt from scratch with a unified two-pane architecture. See [planning/TUI_DEV_PLAN.md](planning/TUI_DEV_PLAN.md) for details.

### M5-A: Core Shell
**Theme:** Structure you can see

- Two-pane layout (Timeline | Context)
- Status bar, footer hints
- Focus management, screen modes
- Color scheme, responsive sizing

**Spec:** `planning/SPEC-m5a-tui-shell.md`

### M5-B: Timeline & Context
**Theme:** Real content, real data

- Timeline event system (typed, collapsible, filterable)
- All 8 phase-specific context views
- Dynamic status bar and footer

**Spec:** `planning/SPEC-m5b-timeline-context.md`

### M5-C: Activity & Polish
**Theme:** Feel alive, self-teaching

- Activity visibility (file indicator, heartbeat, toasts)
- Command palette
- Self-teaching UI elements
- Accessibility (NO_COLOR support)

**Spec:** `planning/SPEC-m5c-activity-polish.md`

---

## Next: Core Features

### Criteria Verification
AI-powered verification of completion criteria after promise tag detected.
- Verifier model checks each criterion against repo state
- Structured PASS/FAIL results
- Run continues if criteria fail

### Review Rounds (Coordinator/Collaborator Model)
Multi-model spec review using the Coordinator + Collaborator pattern.
- User works with coordinator AI to draft spec
- On `/accept`, spec sent to collaborator models for review
- Collaborators provide independent feedback
- Coordinator synthesizes feedback and makes recommendations
- User decides: incorporate feedback (`/edit`) or approve (`/approve`)

See [Workflow UX](planning/WORKFLOW_UX.md) for detailed flow.

### Workflow Status (`/status`)
Visual workflow progress indicator.
- Show current phase in overall workflow
- Display available commands for current state
- Phase-aware footer hints with next action
- Future: slide-up status pane for persistent context

### Stuck/Paused Handling
Robust UX for interrupted or failed runs.
- Stuck state with diagnosis and options
- Paused state with resume capability
- Workspace reset for backward transitions

---

## Future: Production Release (M6)

### Installation
- Cross-platform installers
- `curl | bash` with pinned versions
- SHA256 checksums

### Polish
- Troubleshooting docs for real failure modes
- First-run experience optimization
- Performance tuning

---

## Architecture References

- [State Machine](state-machine.md) - ThreadPhase definitions and transitions
- [Workflow UX](planning/WORKFLOW_UX.md) - User experience flow, Coordinator/Collaborator model
- [TUI UX Principles](planning/TUI_UX_PRINCIPLES.md) - Design decisions
- [TUI Dev Plan](planning/TUI_DEV_PLAN.md) - Implementation phases

---

## Version Targets

### v0.1 (MVP)
- [ ] TUI rebuild complete (M5-A/B/C)
- [ ] Full workflow: Draft → Run → Review → Commit
- [ ] Single-thread enforcement
- [ ] Thread persistence and resume

### v0.2
- [ ] Criteria verification
- [ ] Review rounds
- [ ] Quick mode (auto-advance to checkpoints)

### v1.0
- [ ] Production installers
- [ ] Multi-thread support
- [ ] Polished first-run experience
