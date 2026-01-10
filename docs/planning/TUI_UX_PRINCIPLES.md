# ralf TUI UX Principles

## Overview

ralf uses an opinionated, phase-driven workflow for multi-model autonomous coding. These principles guide the TUI design to support this workflow while keeping the experience simple and prescriptive.

---

## Core Layout Model

```
┌─────────────────────────────────────────────────────────────────┐
│ [Status Bar: Phase | Thread Title | Model | Key Metric]   [?:help]
├─────────────────────────────────┬───────────────────────────────┤
│                                 │                               │
│   TIMELINE PANE                 │   CONTEXT PANE                │
│   (Persistent event stream)     │   (Adapts to current phase)   │
│                                 │                               │
│   ▸ [Spec] User: I want to...   │   Drafting → Spec editor      │
│   ▸ [Spec] claude: Here's...    │   Running → Output + Criteria │
│   ▾ [Run] Iteration 1           │   Review → Diff viewer        │
│     ├─ claude: attempted...     │   Commit → Message editor     │
│     ├─ ✓ tests passed           │                               │
│     └─ ✗ lint: 3 warnings       │                               │
│   ▸ [Run] Iteration 2 (gemini)  │                               │
│   ▸ [Review] Approved           │                               │
│                                 │                               │
├─────────────────────────────────┴───────────────────────────────┤
│ > Type your message...                           [/:commands]
└─────────────────────────────────────────────────────────────────┘
```

**Key Layout Features:**
- Timeline entries are **collapsible** (▸/▾) to manage noise
- Timeline entries are **typed** ([Spec], [Run], [Review]) for filtering
- Timeline entries show **model attribution** (which model did this)
- Context pane adapts to phase; Timeline stays consistent
- Slash commands (`/help`) for discoverability across all phases
- Input-first design: typing always goes to input area

---

## The 13 Principles

### 1. Timeline is Primary

The timeline pane is the continuous record of work. It persists across all phases as a **typed event stream**:
- **[Spec]** - Chat messages during spec creation
- **[Run]** - Iteration logs, model outputs, verifier results
- **[Review]** - Comments, approval decisions
- **[System]** - Phase transitions, warnings, errors

Events are collapsible: an entire iteration can fold to one line. Events are filterable: "show decisions only" or "show errors only."

**Rationale:** The timeline IS documentation, but documentation needs structure. Typed, collapsible events prevent noise while preserving the full story.

### 2. Context Adapts, Layout Stays

One consistent two-pane layout throughout the workflow. Left = timeline (the record). Right = context (current work artifact). The context pane adapts to the phase while overall layout remains stable.

**Context Pane by Phase:**
| Phase | Context Shows |
|-------|---------------|
| Drafting, Assessing, Finalized | Spec editor |
| Preflight, PreflightFailed | Check results + actions |
| Configuring | Run configuration form |
| Running, Verifying | Output + criteria checklist |
| Paused, Stuck | Status + decision options |
| Implemented | Summary + next actions |
| PendingReview, Approved | Diff viewer |
| ReadyToCommit, Done | Commit message + summary |

**Rationale:** Spatial consistency reduces cognitive load. Users learn one layout.

### 3. Phase is Visible, Not Overwhelming

Status bar shows current phase + one key metric. Keep it minimal:
```
● Running (iter 3) │ "Add auth" │ gemini │ 2/5 criteria
```

Additional detail lives in the timeline or is accessible via expand/help. Phase transitions are logged to timeline.

**Rationale:** The state machine has 17 phases. Users need awareness without complexity.

### 4. Decisions Happen Inline

Human checkpoints (PreflightFailed, Stuck, PendingReview) present options in the context pane with clear action keys:

```
┌─ Stuck after 5 iterations ─────────────────────┐
│ Best: 2/4 criteria │ Models tried: all 3       │
│                                                │
│ [1] Revise spec   [2] Reconfigure              │
│ [3] Manual assist [4] Abandon                  │
└────────────────────────────────────────────────┘
```

Modals reserved for truly destructive confirmations: quit with unsaved work, abandon thread, hard reset.

**Rationale:** Inline decisions keep context visible. Modals only when "are you SURE?" is warranted.

### 5. Single Thread Focus

One thread active at a time. Thread picker (Ctrl+T) for resume/switch. Warn on repo divergence with clear options.

**Thread States:**
- **Active** - Currently being worked on
- **Suspended** - Paused/crashed, resumable
- **Completed** - Done, archived for reference
- **Abandoned** - Stopped, archived

**Rationale:** Multi-thread adds complexity. Most work is single-threaded. Archive completed threads as documentation.

### 6. Model Flexibility with Attribution

Every timeline event shows which model produced it. Easy model switching via Ctrl+M (not Tab - that's for focus). Show active model in status bar.

```
▸ [Run] Iteration 2
  └─ gemini: Fixed the lint warnings by...
```

**Rationale:** "Which model did this?" is a core debugging question in multi-model systems.

### 7. Transparency and Control

Users should understand and control what's happening:
- Show what context will be sent to the model (spec excerpt, file list)
- Show queue status (waiting for rate limit, retrying)
- Cancellable operations where possible
- Manual override path always available

**Rationale:** Trust requires transparency. "Magic" that can't be inspected breeds frustration.

### 8. Prescriptive but Escapable

Default workflow follows the state machine:
```
Drafting → Assessing → Finalized → Preflight → Configuring →
Running → Verifying → Implemented → PendingReview →
Approved → ReadyToCommit → Done
```

Skip gates exist but require explicit action and are logged to timeline. The UI prevents skipping into invalid states (enforced by state machine).

**Rationale:** Beginners need guidance; experts need escape hatches. Default = best practice.

### 9. Errors are Actionable

When something fails, show:
1. **What** went wrong (clear message)
2. **Where** the evidence is (link to log line, file:line, test output)
3. **What to do** (specific next actions)

```
✗ Preflight: Git Working Tree
  Working tree has uncommitted changes.
  → View: git status (press 'v')
  → Fix: Commit or stash, then [r]etry
```

**Rationale:** Errors are learning opportunities. Link diagnosis to evidence.

### 10. Progressive Disclosure

Show what's needed for current phase. Advanced options behind Ctrl+P command palette or help overlay (?). The UI reveals complexity as users need it.

**Rationale:** Simple by default, powerful when needed.

### 11. Work is Always Resumable

Thread state persists to disk. Crash recovery is seamless. Paused phase exists for intentional interruption. Timeline captures enough context to understand where you left off.

**Rationale:** Never lose work. Trust is built by reliability.

### 12. Navigability Across Artifacts

From any timeline event, jump to related artifacts:
- From error → relevant log/output
- From iteration → diff at that point
- From review comment → specific file/line
- From any phase → spec (read-only if finalized)

Keyboard navigation: Ctrl+G for "go to", or click/select in timeline.

**Rationale:** The story and the artifacts must be connected.

### 13. Self-Teaching UI

The interface should teach users without documentation. Every screen answers two questions: "What am I looking at?" and "What do I do next?"

**Techniques:**

1. **"Next Action" in status bar** - Not just state, but guidance:
   ```
   ● Drafting │ "Add auth" │ claude │ → Describe feature, Ctrl+F to finalize
   ```

2. **Empty states that teach** - When nothing's there, explain what goes there:
   ```
   Start by describing what you want to build.
   Be specific about the outcome, not the implementation.
   ```

3. **Phase transitions explain themselves** - Timeline narrates:
   ```
   ▸ [System] Entering Preflight
     Checking: git state, spec validity, models...
   ```

4. **Decisions include context** - Not just options, but why:
   ```
   ✗ Git Working Tree
     You have uncommitted changes. ralf needs a clean slate.
     [s] Stash (recommended)  [c] Commit first  [i] Ignore (risky)
   ```

5. **Input bar prompts change per phase**:
   ```
   Drafting:     │ Describe what you want to build...        │
   Running:      │ Running... Ctrl+C to pause, ? for help    │
   PendingReview:│ Review the diff. [a]pprove [r]eject       │
   ```

6. **First-run onboarding** - Gentle intro for new users explaining the workflow in 3 bullet points, then get out of the way.

**Rationale:** If users need to read docs to use the tool, the UI has failed. The interface itself is the teacher.

---

## Screen Modes

The two-pane layout supports **focus modes** for different terminal sizes and tasks:

| Mode | Description | Shortcut |
|------|-------------|----------|
| **Split** (default) | Timeline + Context side by side | Ctrl+1 |
| **Timeline Focus** | Timeline full width, context hidden | Ctrl+2 |
| **Context Focus** | Context full width, timeline hidden | Ctrl+3 |
| **Swap** | Swap pane positions | Ctrl+\\ |

**Focused Pane Indicator:** The active pane has a highlighted border (bright color) while the inactive pane has a dim border. This makes it immediately clear which pane will receive keyboard input. Tab cycles focus between panes.

**Rationale:** Two-pane breaks on narrow terminals. Diffs need full width. Focus modes adapt.

---

## Input Model & Command System

### Input-First Philosophy

The conversation pane is the primary interaction point. All typing goes directly to the input area—no mode switching, no reserved keys that block text entry. This matches the mental model of chat-based AI assistants like Claude Code.

**Core principle:** When users see a cursor, they can type anything.

### Slash Commands

Actions are invoked via slash commands, discoverable through `/help`:

```
┌─ Commands ──────────────────────────────────────┐
│ /help, /?      Show this help               [F1]│
│ /quit, /q      Exit ralf                   [Esc]│
│ /split, /1     Split view mode          [Ctrl+1]│
│ /focus, /2     Focus conversation       [Ctrl+2]│
│ /canvas, /3    Focus canvas             [Ctrl+3]│
│ /refresh       Refresh model status     [Ctrl+R]│
│ /clear         Clear conversation       [Ctrl+L]│
│ /model [name]  Switch active model              │
│ /copy          Copy last response               │
│ /editor        Open in $EDITOR                  │
│                                                 │
│ Phase commands (current: Reviewing):            │
│ /approve       Approve the spec                 │
│ /reject        Reject with feedback             │
└─────────────────────────────────────────────────┘
```

### Layered Command Access

Commands have multiple access paths for different user needs:

| Action | Slash Command | Keybinding | User Type |
|--------|---------------|------------|-----------|
| Quit | `/quit`, `/q` | `Escape` | New users learn slash, power users use key |
| Help | `/help`, `/?` | `F1` | Discoverable + muscle memory |
| Screen modes | `/split`, `/focus`, `/canvas` | `Ctrl+1/2/3` | Both paths work |
| Refresh | `/refresh` | `Ctrl+R` | Both paths work |
| Clear | `/clear` | `Ctrl+L` | Both paths work |
| Switch pane | — | `Tab` | Keybinding only (too frequent) |
| Timeline scroll | — | `PageUp/Down`, `Alt+j/k` | Keybinding only |
| Submit | — | `Enter` | Keybinding only |
| Newline | — | `Shift+Enter` | Keybinding only |
| Model switch | `/model [name]` | — | Slash only (needs argument) |
| Phase: approve | `/approve` | — | Slash only (deliberate action) |

**Rationale:**
- New users discover via `/help`, use readable slash commands
- Power users graduate to keybindings for speed
- Everyone can choose their preferred interaction style
- Slash commands are self-documenting; keybindings are fast

### Command Discoverability

1. **Typing `/` shows autocomplete** - Popup menu filters as you type
2. **`/help` is context-aware** - Shows phase-specific commands at top
3. **Footer hints** - Show most relevant commands for current state
4. **Focus trap escape** - Pressing `/` from any pane jumps to input

### Phase-Specific Commands

Commands adapt to the current phase:

| Phase | Available Commands |
|-------|-------------------|
| Drafting | `/finalize`, `/assess` |
| Running | `/pause`, `/cancel` |
| Paused | `/resume`, `/cancel` |
| Stuck | `/revise`, `/reconfigure`, `/abandon` |
| PendingReview | `/approve`, `/reject` |
| ReadyToCommit | `/commit`, `/amend` |

**Rationale:** Discoverability without clutter. Users see only what's relevant.

---

## Keyboard Reference

### Navigation & Focus

| Key | Action |
|-----|--------|
| `Tab` | Cycle focus between panes |
| `Shift+Tab` | Cycle focus (reverse) |
| `PageUp/PageDown` | Scroll timeline |
| `Alt+j/k` | Scroll timeline (vim-style) |
| `Ctrl+G` | Go to / navigate |

### Screen Modes

| Key | Action | Slash Equivalent |
|-----|--------|------------------|
| `Ctrl+1` | Split view | `/split`, `/1` |
| `Ctrl+2` | Focus conversation | `/focus`, `/2` |
| `Ctrl+3` | Focus canvas | `/canvas`, `/3` |

### Input

| Key | Action |
|-----|--------|
| `Enter` | Submit message |
| `Shift+Enter` | Insert newline |
| `/` | Start slash command |
| `Escape` | Clear input / Cancel / Quit (cascade) |

### Global Actions

| Key | Action | Slash Equivalent |
|-----|--------|------------------|
| `F1` | Show help | `/help`, `/?` |
| `Ctrl+R` | Refresh models | `/refresh` |
| `Ctrl+L` | Clear conversation | `/clear` |
| `Ctrl+C` | Pause (graceful stop) | `/pause` |
| `Ctrl+D` | Cancel (abort immediately) | `/cancel` |

**Escape Cascade:** Pressing `Escape` performs context-dependent action:
1. If input has text → clear input
2. If operation in progress → cancel operation
3. Otherwise → quit ralf

**Pause vs Cancel:** `Ctrl+C` is a graceful pause that lets the current operation finish and saves state. `Ctrl+D` is an immediate abort that discards in-progress work.

Phase-specific actions are available via slash commands (see `/help`).

---

## Footer Keybinding Hints

The bottom of the screen displays context-sensitive hints:

```
Drafting:      Tab:Switch  Enter:Send  /:Commands  Esc:Quit
Running:       Tab:Switch  Ctrl+C:Pause  /:Commands  Esc:Cancel
PendingReview: Tab:Switch  /approve  /reject  /:Commands
```

**Guidelines:**
- Show 3-5 most relevant actions for current phase
- Use compact formatting: `Key:Action` or `/command`
- Prioritize left-to-right by frequency of use
- Always include `/:Commands` to hint at slash command system
- Update immediately on phase transition
- Show phase-specific slash commands when relevant

**Rationale:** Reduces cognitive load by showing only relevant actions. Slash command hints teach discoverability.

---

## Color Guidelines

Use color consistently and sparingly to convey meaning:

| Color | Meaning | Usage |
|-------|---------|-------|
| **Green** | Success, passed | ✓ checkmarks, passed verifiers, approved |
| **Red** | Failure, error | ✗ marks, failed checks, rejected |
| **Yellow** | Warning, attention | Warnings, rate limits, paused state |
| **Blue** | Information, active | Current phase, focused pane border |
| **Model colors** | Model attribution | Claude=Peach, Gemini=Blue, Codex=Green (see Style Guide) |
| **Dim/Gray** | Secondary info | Timestamps, metadata, inactive elements |

**Principles:**
- Color should reinforce, not replace, meaning (accessibility)
- Use bold/bright variants for emphasis, dim for de-emphasis
- Inactive/unfocused elements use dimmer colors
- Status indicators (●) use color to show state at a glance
- Support `NO_COLOR` environment variable for colorless mode

**Phase Status Indicators:**
```
● Drafting    (blue - active)
● Running     (yellow - working)
● Verifying   (yellow - working)
● Implemented (green - success)
● Stuck       (red - needs attention)
● Done        (green - complete)
```

---

## Live Status Indicators

Real-time feedback keeps users informed during long operations:

**Rate Limit Cooldowns:**
```
⏱ claude: Rate limited (47s remaining)
  Next: gemini (ready)
```
Show countdown timer for rate-limited models. Indicate which model will be tried next.

**Iteration Progress:**
```
● Running (iter 3/10) │ "Add auth" │ gemini │ ◐ Building...
```
Use spinner characters (◐◑◒◓) for active operations. Show iteration count and limit.

**Verifier Progress:**
```
Verifying: [██████░░░░] 3/5 checks
  ✓ Build    ✓ Lint    ◐ Tests    ○ Types    ○ Custom
```
Progress bar with individual verifier status. Shows what's running vs. pending.

**Model Queue Status:**
```
Queue: claude (running) → gemini → o1
       ↑ current        next in 2m if needed
```
Show model rotation order and timing.

**Rationale:** Long autonomous runs can feel like black boxes. Live status builds trust and reduces "is it stuck?" anxiety.

---

## Codebase Activity Visibility

During autonomous runs, users need two things:
1. **Feeling of activity** - The subjective sense that the system is alive and working
2. **Awareness of changes** - Understanding what parts of the codebase are being touched

These are distinct needs requiring different UI techniques at different attention levels.

### Layered Attention Model

| Level | Technique | Purpose | Screen Cost |
|-------|-----------|---------|-------------|
| **Peripheral** | Status bar file:line | Ambient "movement" signal | None (reuses status bar) |
| **Glanceable** | Activity heartbeat row | Quick scan of what's touched | 1 row |
| **Focused** | Transient diff toasts | IDE-like flash, actual content | Temporary overlay |
| **On-demand** | Context pane file list | Full detail when desired | Context pane |

Activity should be **felt at the periphery without demanding attention**, but detail should be available when you look.

### Technique 1: Streaming File Indicator

The status bar shows the current file being touched, updating in real-time:
```
● Running (iter 2) │ gemini │ src/auth/middleware.rs:47 ◐
                              ↑ updates as model works
```
This creates a sense of the model "moving through" the codebase. Low cognitive load, high activity signal.

### Technique 2: Activity Heartbeat Row

A single row between status bar and panes showing recent file activity:
```
━━ auth.rs +12 ━━ lib.rs ~2 ━━ middleware.rs +34 ━━━━━━━━━━━━━━━━━━━━
   ↑ newest                    older → → →                    fades out
```
Like a stock ticker for code changes. Scrolls left as new files are touched. Provides ambient awareness without demanding focus.

**Format:** `filename +added ~modified -deleted` with newest on left.

### Technique 3: Transient Diff Toasts

Brief overlays that appear when files are saved, then fade (2-3 seconds):
```
                              ┌─ src/auth.rs ─────────┐
  [Timeline]    [Context]     │ + pub fn login() {    │
                              │ +     validate()?;    │
                              │ + }                   │
                              └───────────────────────┘
                                    fades after 2-3s
```
Gives the IDE "diff flash" feeling without permanent screen cost. The diff appears in a corner, lingers briefly, then dissolves.

**Guidelines:**
- Show max 5-7 lines of diff (most significant changes)
- Position in corner that doesn't obscure active work
- Fade gracefully (don't pop out abruptly)
- Queue multiple changes if they happen rapidly
- User can dismiss early with any key

### Technique 4: File Tree with Activity Pulse

During Running, a compact file tree can show recently-changed files with brightness indicating recency:
```
src/
  auth/
    middleware.rs ●   ← bright (just now)
    tokens.rs     ◐   ← dimming (30s ago)
  lib.rs          ○   ← dim (1m ago)
tests/
  auth_test.rs    ●   ← bright (just now)
```
Uses brightness/intensity rather than color to show temporal recency. Files fade from bright → dim → unmarked over ~60 seconds.

### Technique 5: Diff Waterfall Mode

During active model work, the timeline can temporarily transform into a streaming diff view:
```
│ + fn authenticate(token: &str) -> Result<User> {
│ +     let claims = decode(token)?;
│ +     Ok(User::from(claims))
│ + }
│
│ ~ use jsonwebtoken::{decode, Validation};
│
│                              ↓ auto-scrolls as changes stream
```
When the iteration completes, the waterfall collapses back into a normal timeline event:
```
▸ [Run] Iteration 2 (gemini)
  └─ Files: +3 ~1 -0 (expand for diff)
```

Toggle with a keybinding (e.g., `d` for diff mode during Running).

### Recommended Default Configuration

For the initial implementation, start with:

1. **Always on:** Status bar file indicator (Technique 1)
2. **Default on:** Activity heartbeat row (Technique 2)
3. **Default on:** Transient diff toasts (Technique 3)
4. **On-demand:** File tree pulse in context pane
5. **Toggle:** Diff waterfall mode (`d` key)

Users can disable toasts or heartbeat if they find them distracting.

### Implementation Notes

**Transient overlays in TUI:**
- Use a layered rendering approach (base UI + overlay layer)
- Track overlay lifetime with timestamps
- Fade effect: reduce intensity over last 500ms
- Position calculation: avoid active cursor/input areas

**Heartbeat row scrolling:**
- New entries push from left
- Entries older than 60s fade out
- Maximum ~10 entries visible (depends on terminal width)
- Clicking/selecting an entry could show full diff

**File indicator updates:**
- Parse model output for file paths
- Update on file open/write operations
- Show "..." or spinner when waiting for model response

**Rationale:** Autonomous coding can feel like a black box. These techniques make the system's activity tangible without overwhelming the user. The layered approach lets users choose their awareness level—peripheral glances or focused attention.

---

## Open Questions

1. **Polishing Phase**: Is this needed, or does "Implemented → PendingReview" suffice? Polishing implies manual editing which TUI can't fully support.

2. **Context Sent to Models**: What parts of the timeline get fed back to models? Full history wastes tokens. Need clear boundary between "displayed" and "sent to model."

3. **External Tool Integration**: "Open in VS Code" from Stuck/Polishing phases? (Future consideration.)

*Note: "Live Diff During Run" was previously an open question, now addressed in the "Codebase Activity Visibility" section.*

*Note: "Assessment Phase" was resolved - yes, keep it. AI spec review catches issues early before expensive run iterations. The Drafting→Assessing→Finalized flow is valuable.*

---

## Design Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Left pane | Timeline (event stream) | Structure > raw conversation |
| Timeline entries | Typed, collapsible | Manage noise, enable filtering |
| Model switching | Ctrl+M (not Tab) | Tab is for focus traversal |
| Human checkpoints | Inline in context | Modals only for destructive |
| Phase names | Match state machine | Consistency, learnability |
| Navigation | Cross-linked | Any event → its artifacts |
| Small terminals | Focus modes | Graceful degradation |
| Guidance | Inline "next action" hints | No docs needed |
| Pane focus | Highlighted border | Clear visual indicator (lazygit pattern) |
| Pause vs Cancel | Ctrl+C / Ctrl+D | Graceful vs immediate (user choice) |
| Footer hints | Context-sensitive | Only relevant actions shown |
| Color usage | Semantic, accessible | Meaning > decoration, NO_COLOR support |
| Live status | Spinners, countdowns | Transparency during long operations |
| Codebase activity | Layered attention model | Peripheral → glanceable → focused → on-demand |
| Diff visibility | Transient toasts | IDE-like flash without permanent screen cost |
| Input model | Input-first, no reserved keys | Cursor visible = can type anything (Claude Code pattern) |
| Actions | Slash commands + keybindings | Discoverable for new users, fast for power users |
| Screen mode keys | Ctrl+1/2/3 (not plain 1/2/3) | Plain keys go to input, modifiers for actions |
| Quit | Escape cascade | Clear → Cancel → Quit progression |
| Model switching | `/model` command | Ctrl+M removed; slash command takes argument |

---

## Future Considerations

- **Autonomy modes** - Configurable levels of human oversight:
  - **Supervised**: Pause at every checkpoint for approval
  - **Semi-autonomous** (default): Pause only on failures or ambiguity
  - **Autonomous**: Run to completion, review at end
  - Mode visible in status bar, switchable via Ctrl+A
- **File tree / project browser** - Browse codebase context during spec/polish
- **External tool hooks** - Open in editor, run shell command
- **Cost/latency dashboard** - Per-model metrics, rate limit status
- **Git worktree support** - Parallel threads via worktrees

*Note: "Live diff during run" moved from future consideration to implemented design in "Codebase Activity Visibility" section.*

---

## Appendix: Sources and Inspirations

This document draws from established patterns in developer tools and AI assistants. Key sources consulted:

### Terminal UI Tools

- **[lazygit](https://github.com/jesseduffield/lazygit)** - Git TUI
  - Panel focus with highlighted borders
  - Footer keybinding hints that change per context
  - Inline confirmation for destructive actions
  - Status bar with current operation

- **[k9s](https://github.com/derailed/k9s)** - Kubernetes TUI
  - Command palette pattern (`:` prefix)
  - Crumb trail showing navigation context
  - Filter/search within views
  - Color-coded status indicators

- **[tig](https://github.com/jonas/tig)** - Git text-mode interface
  - Pager-style navigation
  - View stacking and switching
  - Keybinding reference in status line

### AI Coding Assistants

- **[Cursor](https://cursor.sh/)** - AI-powered IDE
  - Accept/reject flow for AI suggestions
  - Inline diff presentation
  - Model selection in UI
  - Progressive disclosure of AI capabilities

- **[Aider](https://aider.chat/)** - AI pair programming CLI
  - Chat-based spec refinement
  - Git integration patterns
  - Model flexibility with clear attribution
  - Commit message generation flow

- **[Claude Code](https://claude.ai/code)** - Anthropic CLI
  - **Input-first design** - all typing goes to input, no mode switching
  - **Slash commands** - `/help`, `/clear`, `/compact` for actions
  - Phase-based operation (plan → execute)
  - Inline approval for actions
  - Transparent context display

### Design Guidelines

- **[Command Line Interface Guidelines](https://clig.dev/)** - General CLI best practices
  - Human-readable output by default
  - Progressive disclosure
  - Consistent error formatting
  - Help text conventions

- **[12 Factor CLI Apps](https://medium.com/@jdxcode/12-factor-cli-apps-dd3c227a0e46)** - Modern CLI principles
  - Prefer flags to prompts
  - Respect `NO_COLOR`
  - Show progress for long operations

### Accessibility

- **[WCAG 2.1](https://www.w3.org/WAI/WCAG21/quickref/)** - Web Content Accessibility Guidelines
  - Color should not be sole indicator of meaning
  - Sufficient contrast ratios
  - Keyboard navigability

- **[NO_COLOR](https://no-color.org/)** - Convention for disabling color
  - Respect user preference for colorless output

### Inspiration Summary

| Pattern | Source | Our Application |
|---------|--------|-----------------|
| Focus indicator | lazygit, k9s | Highlighted pane border |
| Footer hints | lazygit | Phase-specific hints |
| Slash commands | Claude Code | `/help`, `/quit`, phase commands |
| Input-first | Claude Code | All typing goes to input |
| Layered access | Common pattern | Slash commands + keybindings |
| Inline approval | Cursor, Aider | Human checkpoints in context |
| Model attribution | Aider | Every event shows model |
| Progress indicators | CLI Guidelines | Spinners, countdown timers |
| Two-pane layout | lazygit, tig | Timeline + Context |
| Collapsible entries | k9s tree view | Fold iterations to one line |
