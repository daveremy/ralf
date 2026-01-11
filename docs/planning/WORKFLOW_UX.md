# ralf Workflow UX Design

> This document describes the user experience of ralf's workflow phases, focusing on how users understand where they are and how to progress.

## Core Concepts

### The Coordinator + Collaborator Model

Every phase in ralf follows a **Coordinator + Collaborator** pattern:

- **Coordinator AI**: Your primary partner throughout the workflow
  - Facilitates the conversation
  - Synthesizes input from collaborators
  - Makes recommendations
  - Guides you through decisions

- **Collaborator AI(s)**: Specialized consultants brought in for specific tasks
  - Review specs for gaps/issues
  - Provide alternative perspectives
  - Run verification checks
  - The user never talks directly to collaborators - they talk to the coordinator

**Example in Spec Phase:**
```
User ◄──► Coordinator (claude)
              │
              ├──► Collaborator (gemini) ──► Review feedback
              └──► Collaborator (codex)  ──► Review feedback
              │
              └──► Synthesizes feedback for user
```

This pattern applies across all phases, creating a consistent mental model.

---

## Phase 1: Spec Creation

### User Journey

```
┌─────────────────────────────────────────────────────────────────────┐
│                        SPEC PHASE                                    │
│                                                                      │
│   ● DRAFTING ────► ○ ASSESSING ────► ○ FINALIZED                    │
│        ↑                                                             │
│   you are here                                                       │
│                                                                      │
│   /accept → request review    /finalize → skip review                │
└─────────────────────────────────────────────────────────────────────┘
```

### States

| State | What's Happening | User Actions |
|-------|------------------|--------------|
| **Drafting** | User and coordinator AI iterate on the spec | Chat freely, then `/accept` or `/finalize` |
| **Assessing** | Collaborator AIs review the spec | Review feedback, then `/approve` or `/edit` |
| **Finalized** | Spec is locked, ready for implementation | `/run` to start, `/edit` to reopen |

### Drafting → Assessing (`/accept`)

When user types `/accept`:
1. Coordinator submits spec to configured collaborator models
2. Status changes to `[Assessing]` / `[Under Review]`
3. Collaborators review asynchronously
4. Coordinator synthesizes feedback
5. User sees review results in canvas

**Canvas during Assessing:**
```
┌─ Spec ─────────────────────────────────────────────────┐
│ [Under Review]                                         │
│                                                        │
│ ## My Feature Spec                                     │
│ ...spec content...                                     │
│                                                        │
├─ Reviews ──────────────────────────────────────────────┤
│ ● gemini: "Consider adding error handling for..."      │
│ ● codex: "The acceptance criteria could be more..."    │
│                                                        │
│ claude (coordinator): "Both reviewers raise valid      │
│ points about error handling. I recommend addressing    │
│ this before finalizing."                               │
│                                                        │
├─ Actions ──────────────────────────────────────────────┤
│ /edit - Incorporate feedback (back to drafting)        │
│ /approve - Finalize as-is                              │
└────────────────────────────────────────────────────────┘
```

### Assessing → Finalized (`/approve`)

When user types `/approve`:
1. Spec is locked
2. Status changes to `[Ready]`
3. Canvas shows "Ready for implementation"
4. User can `/run` to begin

### Skipping Assessment (`/finalize`)

From Drafting, user can type `/finalize` to skip review and go directly to Finalized. Useful for:
- Small, well-understood changes
- Time-sensitive fixes
- When user is confident in the spec

---

## Phase 2: Implementation

### User Journey

```
┌─────────────────────────────────────────────────────────────────────┐
│                     IMPLEMENTATION PHASE                             │
│                                                                      │
│   ○ PREFLIGHT ──► ○ CONFIGURING ──► ● RUNNING ──► ○ VERIFYING       │
│                                          ↑                           │
│                                     you are here                     │
│                                                                      │
│   /pause → interrupt    /cancel → abort                              │
└─────────────────────────────────────────────────────────────────────┘
```

### States

| State | What's Happening | User Actions |
|-------|------------------|--------------|
| **Preflight** | Checking prerequisites | Wait, or fix issues if failed |
| **Configuring** | Setting up models, iterations | Confirm or adjust settings |
| **Running** | Autonomous implementation loop | Watch, `/pause` to interrupt |
| **Verifying** | Checking completion criteria | Wait for results |
| **Paused** | User interrupted | `/resume`, `/configure`, or `/abandon` |
| **Stuck** | Can't complete automatically | `/edit` spec, `/configure`, `/assist`, or `/abandon` |
| **Implemented** | All criteria pass | Continue to review |

### The Implementation Loop

```
                    ┌─────────────────┐
                    │    RUNNING      │
                    │                 │
                    │  Coordinator AI │
                    │  writes code    │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │   VERIFYING     │
                    │                 │
                    │  Check criteria │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              ▼              ▼              ▼
         ┌────────┐    ┌──────────┐   ┌─────────┐
         │ PASS   │    │  FAIL    │   │  STUCK  │
         │        │    │          │   │         │
         │ Done!  │    │ Try again│   │ Need    │
         │        │    │          │   │ help    │
         └────────┘    └──────────┘   └─────────┘
```

---

## Phase 3: Review

### User Journey

```
┌─────────────────────────────────────────────────────────────────────┐
│                        REVIEW PHASE                                  │
│                                                                      │
│   ○ IMPLEMENTED ──► ● PENDING REVIEW ──► ○ APPROVED ──► ○ COMMITTED │
│                           ↑                                          │
│                      you are here                                    │
│                                                                      │
│   /approve → accept changes    /reject → back to implementation      │
└─────────────────────────────────────────────────────────────────────┘
```

### States

| State | What's Happening | User Actions |
|-------|------------------|--------------|
| **PendingReview** | Changes ready for inspection | `/approve` or `/reject` |
| **Approved** | User confirmed changes | Continue to commit |
| **ReadyToCommit** | Preparing commit | `/commit` to finalize |
| **Done** | Complete! | Start new thread |

---

## The `/status` Command

Show the user where they are in the overall workflow.

### Basic View
```
╭─────────────────────────────────────────────────────────────────────╮
│                          WORKFLOW STATUS                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  SPEC           ● Drafting → ○ Assessing → ○ Finalized              │
│                 ↑ you are here                                       │
│                                                                      │
│  IMPLEMENTATION ○ Preflight → ○ Running → ○ Verifying               │
│                                                                      │
│  REVIEW         ○ Pending → ○ Approved → ○ Committed                │
│                                                                      │
├─────────────────────────────────────────────────────────────────────┤
│  Next actions:                                                       │
│  • /accept   Submit spec for AI review                              │
│  • /finalize Skip review, finalize now                              │
│  • /help     Show all commands                                       │
╰─────────────────────────────────────────────────────────────────────╯
```

### Future: Slide-Up Status Pane

A persistent (toggleable) pane at the bottom showing workflow context:
- Current phase with visual progress
- Available actions
- Recent events summary
- Coordinator status

Toggle with `/status` or a keybinding.

---

## Command Reference by Phase

### Spec Phase

| Command | From State | To State | Description |
|---------|------------|----------|-------------|
| `/accept` | Drafting | Assessing | Submit for AI review |
| `/finalize` | Drafting | Finalized | Skip review, finalize now |
| `/edit` | Assessing/Finalized | Drafting | Return to drafting |
| `/approve` | Assessing | Finalized | Accept spec after review |

### Implementation Phase

| Command | From State | To State | Description |
|---------|------------|----------|-------------|
| `/run` | Finalized | Preflight | Start implementation |
| `/pause` | Running | Paused | Interrupt the loop |
| `/resume` | Paused | Running | Continue the loop |
| `/configure` | Paused/Stuck | Configuring | Change settings |
| `/assist` | Stuck | Running | Continue after manual help |
| `/abandon` | Any | Abandoned | Give up on this thread |

### Review Phase

| Command | From State | To State | Description |
|---------|------------|----------|-------------|
| `/approve` | PendingReview | Approved | Accept the changes |
| `/reject` | PendingReview | Running | Back to implementation |
| `/commit` | ReadyToCommit | Done | Commit the changes |

---

## Visual Indicators

### Status Bar
```
Drafting │ "my feature" │ claude ● gemini ● codex ●   → /accept when ready
```

The status bar shows:
- Current phase
- Thread title (truncated)
- Model status
- **Next action hint** (key addition)

### Spec Pane Badge
```
┌─ Spec ─────────────────────────────────────────────────┐
│ [Drafting] /accept when ready                          │
```

The badge includes a subtle hint about what to do next.

### Footer (Phase-Aware)
```
Split │ Timeline │ Drafting                [/accept] review │ [Tab] focus
```

Footer hints change based on phase to show the most relevant next actions.

---

## Implementation Plan

### Phase 1: Visual Guidance (Now)
1. Add phase-aware hints to footer
2. Add "next action" hint to status bar
3. Add guidance text to spec pane badge
4. Update existing commands to trigger phase transitions

### Phase 2: `/status` Command
1. Implement `/status` command
2. Show workflow diagram with current position
3. Show available commands for current phase
4. Consider overlay vs inline display

### Phase 3: Multi-Model Assessment
1. Configure collaborator models for assessment
2. Send spec to collaborators on `/accept`
3. Display review feedback in canvas
4. Coordinator synthesizes feedback

### Phase 4: Slide-Up Status Pane
1. Toggleable workflow status pane
2. Persistent view of progress
3. Quick action buttons
4. Activity summary

---

## Model Role Assignment

### Per-Phase Configuration

Each **major phase** has its own coordinator + collaborator configuration:

| Major Phase | Coordinator Does | Collaborators Do |
|-------------|------------------|------------------|
| **Spec** | Conversation, draft spec | Review spec for completeness |
| **Implementation** | Write code, iterate | Review implementation |
| **Finalization** | Polish, commit prep | Final review |

### Models Panel Design

The Models panel (Context pane when no thread) shows both status and role assignment:

```
┌─ Models ─────────────────────────────────────────────────┐
│ Status                                                    │
│   ● claude  ready                                         │
│   ● gemini  ready                                         │
│   ○ codex   rate limited (2m)                             │
│                                                           │
│ Role Configuration                         j/k Enter      │
│ ┌───────────────────────────────────────────────────────┐ │
│ │ ▸ Spec           claude      + gemini                 │ │
│ │   Implementation codex       + claude, gemini         │ │
│ │   Finalization   claude      + gemini, codex          │ │
│ └───────────────────────────────────────────────────────┘ │
│                                                           │
│ Coordinator: Runs the conversation                        │
│ Collaborators: Provide reviews/feedback                   │
└───────────────────────────────────────────────────────────┘
```

**Interaction:**
- `j/k` to select phase row
- `Enter` to open configuration for that phase
- Configuration shows:
  - Coordinator dropdown (single model)
  - Collaborator checkboxes (multi-select)
  - Only shows "ready" models as selectable
- Click support for mouse users

### Dynamic Switching

Users may want to temporarily change the coordinator without updating config:

**Options:**
- `/model <name>` - Switch coordinator for current major phase (session only)
- `Ctrl+M` - Cycle through ready models for current phase
- Status bar shows current model, indicates if overridden

**Status bar with override:**
```
● Drafting │ "My Feature" │ gemini* ●     → /accept when ready
                           ↑ asterisk indicates temporary override
```

### Persistence

- Role configuration saved to `~/.ralf/config.toml` or `.ralf/config.json`
- Dynamic overrides are session-only (not persisted)
- Default: claude as coordinator, all others as collaborators

### System Prompts

Each role needs appropriate system prompts:

**Coordinator (Spec Phase):**
- Help user articulate requirements
- Ask clarifying questions
- Structure spec with Promise, Deliverables, Acceptance Criteria
- Guide through iterations

**Collaborator (Spec Review):**
- Review spec for completeness and correctness
- Check testability of acceptance criteria
- Identify missing edge cases
- Provide PASSED/FAILED verdict with specific issues

**Coordinator (Implementation Phase):**
- Execute code changes according to spec
- Track progress against acceptance criteria
- Report status and issues

**Collaborator (Code Review):**
- Review implementation against spec
- Check code quality, safety, test coverage
- Provide PASSED/FAILED verdict

---

## Open Questions

1. **How many collaborators?** Should assessment always use all available models, or should user configure which to use?
   - **Answer:** User configures per-phase in Models panel.

2. **Assessment timeout?** What if a collaborator doesn't respond? Proceed without their feedback?

3. **Review persistence?** Should review feedback be saved with the thread for later reference?

4. **Quick mode integration?** In quick mode, should `/accept` auto-approve if reviews pass?

5. **Model variant selection?** Should users configure variants (opus/sonnet/haiku) per role, or use CLI defaults?
