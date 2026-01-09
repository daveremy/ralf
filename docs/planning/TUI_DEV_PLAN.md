# TUI Development Plan

## Overview

This document outlines the development plan for ralf's terminal user interface. The TUI supports ralf's opinionated, phase-driven workflow for multi-model autonomous coding.

**Design References:**
- [TUI_UX_PRINCIPLES.md](TUI_UX_PRINCIPLES.md) - UX decisions, layout specifications, interaction patterns
- [TUI_STYLE_GUIDE.md](TUI_STYLE_GUIDE.md) - Visual design: colors, typography, icons, spacing

**Approach:** Clean slate implementation based on UX principles and style guide. Existing TUI code (Milestones 2-4) may be referenced for patterns but will not be migrated.

---

## CLI-First Model Architecture

**ralf depends on CLI AI coding tools being installed on the system.** This is a deliberate design choice:

- **No API key management** - ralf leverages existing CLI tools (claude, codex, gemini) that handle their own authentication
- **User's existing setup** - if you can run `claude` in your terminal, ralf can use it
- **Reduced complexity** - no need to configure API keys, endpoints, or credentials in ralf
- **Security** - API keys stay in their respective CLI tool configs, not duplicated

**Tradeoff:** This approach requires users to have CLI tools pre-configured, which may increase onboarding friction for users new to these tools. However, for the target audience (developers already using AI coding assistants), this is typically a non-issue.

### Supported CLI Tools

| Tool | Command | Auth Method | Test Command |
|------|---------|-------------|--------------|
| Claude Code | `claude` | Anthropic CLI auth | `claude --version` |
| OpenAI Codex | `codex` | OpenAI CLI auth | `codex --version` |
| Gemini CLI | `gemini` | Google Cloud auth | `gemini --version` |

### Model Discovery & Status

The TUI must clearly communicate model availability:

1. **Startup probe** - On launch, probe each CLI to verify:
   - Command exists on PATH (`which {tool}`)
   - Version is compatible (`{tool} --version`)
   - Auth is configured (probe with timeout, no interactive prompts)
   - Model responds within timeout (default: 10s)

   **Probe sequence:**
   - Run probes in parallel for faster startup
   - Use 10-second timeout per model (configurable)
   - Show progressive status updates ("Checking claude... ●")
   - Background re-probe if model recovers from cooldown

2. **Status display** - Users need to see at-a-glance:
   - Which models are available (●)
   - Which are in cooldown/rate-limited (◐)
   - Which need attention (○) - with reason on hover/expand

3. **Error categorization** - Distinguish failure modes with specific messages:
   - **Not installed:** "codex not found. Install: https://..."
   - **Auth required:** "claude needs auth. Run: `claude auth login`"
   - **Auth expired:** "gemini auth expired. Run: `gcloud auth login`"
   - **Timeout:** "claude not responding (10s timeout)"
   - **Rate limited:** "gemini rate-limited, cooldown 60s remaining"
   - **Network error:** "Cannot reach API (check network)"

4. **Guidance** - When models are unavailable, show actionable instructions with specific commands to run

### Model Selection Strategy

When multiple models are available, ralf uses this selection logic:

1. **Round-robin by default** - Distribute work across available models
2. **Skip cooling models** - Don't select models in cooldown
3. **User preference** - Config can specify preferred model order
4. **Fallback hierarchy:**
   ```
   1. Try user's preferred model (if set and available)
   2. If rate-limited, try next available model
   3. If all rate-limited, wait for shortest cooldown
   4. If all unavailable, enter limited mode (spec editing only)
   ```

### Offline/Limited Mode

When no models are available:
- **Allow spec editing** - Users can still draft and refine specs
- **Block run actions** - "Run" button disabled with explanation
- **Show recovery path** - "0 models available. Run `ralf doctor` for help."
- **Background retry** - Periodically re-probe (every 60s) for recovery

### Model Management in UI

**Status Bar (condensed):**
```
claude ● │ gemini ◐ │ codex ○    (●=ready, ◐=cooldown, ○=unavailable)
```

**Settings Context View (full panel):**
- Model list with status indicators and error details
- Probe/refresh action (`r` key)
- Cooldown timers (when rate-limited)
- Enable/disable toggles (persistent, saved to config)
- Link to CLI setup instructions per model

**Enable/Disable semantics:**
- Disabled models are skipped during model selection
- Setting persists to `.ralf/config.json`
- Useful for temporarily excluding a problematic model
- Re-enabling triggers immediate re-probe

**Timeline Events:**
- Model status changes appear as system events
- "gemini rate-limited, cooling down 60s"
- "claude recovered, ready"

### CLI Version Management

Users should know if their CLI tools are up to date:

**Version checking:**
- On startup (or periodically), check installed version vs latest available
- Show indicator in Settings panel: "claude v1.2.3 (update available: v1.3.0)"
- Timeline event for significant updates: "New claude version available with improved context"

**Update guidance:**
| Tool | Update Command | Notes |
|------|---------------|-------|
| `claude` | `claude update` or reinstall | Check Anthropic docs for canonical method |
| `codex` | `npm update -g @openai/codex` | Assuming npm install |
| `gemini` | `gcloud components update` | Part of gcloud SDK |

**Considerations:**
- Don't auto-update (could break user's setup)
- Cache version check results (don't hit network on every launch)
- Graceful fallback if version check fails (network down, API unavailable)
- Consider minimum version requirements for ralf compatibility

**Version check approaches (needs investigation):**
- GitHub releases API for each tool
- `{tool} --version` output parsing
- Package manager queries (npm, homebrew, etc.)

### Future Considerations

As the engine evolves, the Models panel may show:
- Token usage per model
- Cost estimates (if available from CLIs)
- Success/failure rates
- Average response times

### Rate Limit Strategies (Future - Needs Investigation)

Each CLI tool may have different mechanisms for querying rate limit status proactively (rather than just detecting failures):

| CLI | Potential Approach | Notes | Status |
|-----|-------------------|-------|--------|
| `claude` | `claude usage` or API headers | Check if Anthropic CLI exposes usage | Unverified |
| `codex` | OpenAI usage API | May need API key (conflicts with CLI-first) | Unverified |
| `gemini` | `gcloud` quota APIs | Project-level quotas, not API rate limits | Unverified |

> **Note:** The approaches above are speculative and need investigation. As of writing, it's unclear whether these CLI tools expose proactive rate limit/usage queries. The reactive approach (detect limits on failure) is the reliable fallback.

**Benefits of proactive rate limit awareness (if achievable):**
- Show remaining quota before hitting limits
- Smarter model selection (prefer models with headroom)
- Warn users before exhausting quota
- Estimate "runs remaining" based on typical token usage

**Implementation considerations:**
- Cache results (don't query on every iteration)
- Graceful fallback if query not supported (use reactive detection)
- Per-model strategy abstraction in engine
- "Runs remaining" estimates are nice-to-have, not core

---

## Architecture

### Component Hierarchy

```
App
├── StatusBar
│   └── [phase] │ [title] │ [model] │ [file:line] │ [metric] │ [next action]
├── HeartbeatRow (default: enabled, togglable via config)
│   └── ━━ file.rs +12 ━━ other.rs ~3 ━━━━━━━━━━━━━━━━━━━━━━
├── MainArea
│   ├── TimelinePane (left, persistent)
│   │   └── TimelineEvent[]
│   │       └── Typed (Spec|Run|Review|System), collapsible, filterable
│   └── ContextPane (right, phase-adaptive)
│       └── PhaseView (routed by ThreadPhase)
├── FooterHints
│   └── [key] Action │ [key] Action │ ... │ [?] Help │ [Ctrl+Q] Quit
└── OverlayLayer
    └── Toasts, CommandPalette, Modals (z-ordered)
```

### Screen Regions

```
┌─────────────────────────────────────────────────────────────────────┐
│ StatusBar                                                           │
├─────────────────────────────────────────────────────────────────────┤
│ HeartbeatRow (optional)                                             │
├─────────────────────────────────┬───────────────────────────────────┤
│                                 │                                   │
│   TimelinePane                  │   ContextPane                     │
│   (40% width default)           │   (60% width default)             │
│                                 │                                   │
├─────────────────────────────────┴───────────────────────────────────┤
│ FooterHints                                                         │
└─────────────────────────────────────────────────────────────────────┘

        ┌─────────────────────┐
        │ OverlayLayer        │  ← Floats above, for toasts/palettes
        │ (when active)       │
        └─────────────────────┘
```

### Data Flow

```
ThreadStore ──→ Thread ──→ App State ──→ UI Components
                  │
                  ├── phase ──→ StatusBar, ContextPane router
                  ├── events ──→ TimelinePane
                  ├── spec ──→ SpecEditor (context)
                  └── run_state ──→ RunOutput (context)

User Input ──→ Event Loop ──→ Action ──→ State Update ──→ Re-render
```

---

## Phase Views

The ContextPane renders different views based on `ThreadPhase`:

| Phase(s) | Context View | Description |
|----------|--------------|-------------|
| Drafting, Assessing, Finalized | SpecEditor | Chat input + spec preview |
| Preflight, PreflightFailed | PreflightResults | Check list with pass/fail + actions |
| Configuring | RunConfig | Model selection, iteration limit, verifiers |
| Running, Verifying | RunOutput | Streaming output + criteria checklist |
| Paused, Stuck | DecisionPrompt | Options with numbered keys |
| Implemented | Summary | What was done + next actions |
| PendingReview, Approved | DiffViewer | File-by-file diff with navigation |
| ReadyToCommit, Done | CommitView | Commit message editor + summary |

---

## Development Phases

### Phase 1: Core Shell (M5-A)
**Theme:** Structure you can see

Build the application skeleton with all regions, layout management, focus handling, and screen modes. Content is placeholder/hardcoded.

**Spec:** `SPEC-m5a-tui-shell.md`

**Deliverables:**
- App shell with 5 regions
- Two-pane layout with configurable split
- Status bar (static)
- Footer hints (static)
- Focus management (Tab, borders)
- Screen modes (Ctrl+1/2/3, Ctrl+\)
- Color scheme
- Responsive sizing
- Headless test infrastructure

**Exit Criteria:** Can launch TUI, see layout, switch focus between panes, change screen modes.

---

### Phase 2: Timeline & Context (M5-B)
**Theme:** Real content, real data

Connect the shell to real thread data. Build the timeline event system and all phase-specific context views.

This phase is broken into subphases for incremental delivery:

#### M5-B.1: Timeline Foundation
**Spec:** `SPEC-m5b1-timeline-foundation.md`

Build the timeline event system that forms the backbone of the TUI.

**Deliverables:**
- `TimelineEvent` data model (4 types: Spec, Run, Review, System)
- Timeline pane with scrolling and keyboard navigation (j/k, Up/Down)
- Event rendering with timestamps, badges, and model attribution
- Selection highlighting
- Basic collapsible events (▸/▾ with Enter)

**Exit Criteria:** Timeline pane shows events, can scroll and select, events display with proper formatting.

#### M5-B.2: Phase Router & Dynamic Status
**Spec:** `SPEC-m5b2-phase-router.md`

Wire up the shell to thread state for dynamic content.

**Deliverables:**
- Phase router component (ThreadPhase → Context View)
- Status bar driven by thread state (phase, title, current model)
- Footer hints that change per phase
- "Next action" guidance in status bar

**Exit Criteria:** Status bar and footer update based on thread phase, context pane routes to appropriate view.

#### M5-B.3: Core Context Views
**Spec:** `SPEC-m5b3-core-views.md`

Build the most frequently used context views.

**Deliverables:**
- **SpecEditor** - Chat input with spec preview (Drafting, Assessing, Finalized phases)
- **RunOutput** - Streaming model output with criteria checklist (Running, Verifying phases)
- **Summary** - What was done + next actions (Implemented phase)

**Exit Criteria:** Can draft a spec, see run output, and view summary after completion.

#### M5-B.4: Advanced Context Views
**Spec:** `SPEC-m5b4-advanced-views.md`

Build remaining context views for full workflow support.

**Deliverables:**
- **PreflightResults** - Check list with pass/fail + actions (Preflight, PreflightFailed)
- **RunConfig** - Model selection, iteration limit, verifiers (Configuring)
- **DecisionPrompt** - Options with numbered keys (Paused, Stuck)
- **DiffViewer** - File-by-file diff with navigation (PendingReview, Approved)
- **CommitView** - Commit message editor + summary (ReadyToCommit, Done)
- Timeline filtering by event type

**Exit Criteria:** Can walk through entire workflow (Draft → Run → Review → Commit) with appropriate views at each phase.

---

### Phase 3: Activity & Polish (M5-C)
**Theme:** Feel alive, self-teaching

Add the activity visibility features that make autonomous runs tangible. Polish the UI for learnability and accessibility.

**Spec:** `SPEC-m5c-activity-polish.md`

**Deliverables:**

*Activity Visibility:*
- Status bar file indicator (streaming updates)
- Heartbeat row (activity ticker)
- Overlay rendering system
- Transient diff toasts with fade
- Diff waterfall mode (d key)

*Polish:*
- Command palette (Ctrl+P)
- Self-teaching empty states
- First-run onboarding
- Help overlay (?)
- NO_COLOR support
- Full keyboard navigation audit
- Error state handling

**Exit Criteria:** Run an autonomous loop and feel the activity. New user can learn the UI without docs.

---

## Dependencies

```
M5-A (Shell)
  ├── M5-A.1 (Model Probing) ✓
  │
  ▼
M5-B (Timeline & Context)
  ├── M5-B.1 (Timeline Foundation)
  ├── M5-B.2 (Phase Router & Dynamic Status)
  ├── M5-B.3 (Core Context Views)
  └── M5-B.4 (Advanced Context Views)
  │
  ▼
M5-C (Activity & Polish)
```

Each major phase builds on the previous. No parallel development between major phases.

**Within M5-B**, subphases should be completed sequentially:
- M5-B.1 → M5-B.2: Phase router needs timeline events to display
- M5-B.2 → M5-B.3/B.4: Context views need router infrastructure
- M5-B.3 and M5-B.4 could potentially overlap once router is ready

**Within M5-C**, activity features and polish features can be developed independently.

---

## Technical Decisions

### Framework
- **ratatui** for TUI rendering (already in use)
- **crossterm** for terminal backend (already in use)

### State Management
- Single `App` struct owns all UI state
- Thread data accessed via `ThreadStore` (read) and engine APIs (write)
- Event loop pattern from existing code

### Input Handling
- **Keyboard-first** - All functionality accessible via keyboard
- **Mouse support** - Enabled when terminal supports it (crossterm handles detection)
  - Scroll wheel for scrolling panes
  - Click to select items
  - Double-click for toggle actions (collapse/expand)
  - Graceful degradation when mouse unavailable
- **Vi-style navigation** - j/k for up/down, g/G for jump to start/end

### Testing Strategy
- **Headless mode** for automated testing (render to buffer, assert content)
- **Snapshot tests** for complex views
- **Unit tests** for individual components
- **Integration tests** for user flows

### File Structure
```
crates/ralf-tui/src/
├── app.rs              # App struct, main loop
├── event.rs            # Event handling
├── lib.rs              # Public API
├── headless.rs         # Test infrastructure
│
├── layout/
│   ├── mod.rs
│   ├── shell.rs        # Main shell layout (status bar, panes, footer)
│   └── screen_modes.rs # Focus modes
│
├── widgets/
│   ├── mod.rs
│   ├── status_bar.rs
│   ├── footer_hints.rs
│   ├── heartbeat_row.rs
│   └── text_input.rs   # From existing code
│
├── timeline/
│   ├── mod.rs
│   ├── event_model.rs  # TimelineEvent enum
│   ├── timeline_pane.rs
│   └── event_widget.rs # Single event rendering
│
├── context/
│   ├── mod.rs
│   ├── router.rs       # Phase → View routing
│   ├── spec_editor.rs
│   ├── preflight_results.rs
│   ├── run_config.rs
│   ├── run_output.rs
│   ├── decision_prompt.rs
│   ├── summary.rs
│   ├── diff_viewer.rs
│   └── commit_view.rs
│
├── overlay/
│   ├── mod.rs
│   ├── toast.rs
│   ├── command_palette.rs
│   └── modal.rs
│
└── theme/
    ├── mod.rs
    └── colors.rs       # Color definitions per UX principles
```

---

## Reference Materials

- [TUI_UX_PRINCIPLES.md](TUI_UX_PRINCIPLES.md) - UX decisions and interaction patterns
- [TUI_STYLE_GUIDE.md](TUI_STYLE_GUIDE.md) - Visual design: colors, icons, spacing
- [state-machine.md](../state-machine.md) - ThreadPhase definitions
- Existing code in `crates/ralf-tui/src/` - Patterns to reference

---

## Resolved Questions

1. **Thread selection:** Thread picker (Ctrl+T) is an overlay modal that appears over the current view. Lists recent threads with status indicators. Can create new thread from picker.

2. **No-thread view:** When no thread is loaded (app start or after closing last thread), show a welcome/thread picker screen with:
   - Recent threads list
   - "New Thread" option
   - Quick keyboard: `n` for new, numbers for recent threads

3. **Recovery flow:** When TUI is closed during Running phase:
   - Thread state persists to disk (already in state machine design)
   - On relaunch, show thread in its saved state (Running/Stuck/etc.)
   - User can resume, abort, or revise spec
   - No automatic resumption - user must explicitly continue

## Open Questions

1. **Settings integration:** Settings screen from M4 - incorporate into command palette or keep separate?

2. **Model output streaming:** How do we get real-time model output into RunOutput view? Engine API changes needed?

3. **File change detection:** How do we detect file changes for activity indicators? Watch filesystem or parse model output?

4. **Model variant selection:** Each CLI tool supports multiple model variants with different capabilities and costs:
   - Claude: Opus (complex reasoning), Sonnet (balanced), Haiku (fast/cheap)
   - OpenAI: GPT-4o, GPT-4, etc.
   - Gemini: Ultra, Pro, Flash

   How should ralf handle variant selection?
   - **Option A:** User configures per-phase preferences (Opus for review, Sonnet for coding, Haiku for verification)
   - **Option B:** ralf auto-selects based on task complexity (needs heuristics)
   - **Option C:** Defer to CLI defaults, let users configure their CLI tools directly
   - **Option D:** Expose in Settings panel as simple preference per CLI tool

   This affects cost, speed, and quality. Needs user research to understand preferences.

---

## Changelog

| Date | Change |
|------|--------|
| 2025-01-08 | Initial plan created |
| 2025-01-08 | Added TUI_STYLE_GUIDE.md reference |
| 2025-01-08 | Added HeartbeatRow default (enabled), resolved thread picker and recovery flow questions |
| 2025-01-08 | Added CLI-First Model Architecture section documenting dependency on CLI tools and model management UI design |
| 2025-01-08 | Expanded model architecture based on Gemini/Codex review: added error categorization, model selection strategy, offline mode, probe sequence details, enable/disable semantics, and marked rate limit APIs as needing investigation |
| 2025-01-08 | Added CLI Version Management section for tracking tool updates and providing upgrade guidance |
| 2025-01-08 | Added Open Question #4: Model variant selection (opus/sonnet/haiku etc.) |
| 2026-01-08 | Broke M5-B into subphases: B.1 Timeline Foundation, B.2 Phase Router, B.3 Core Views, B.4 Advanced Views |
| 2026-01-08 | Added Input Handling section: keyboard-first with mouse support, vi-style navigation |
