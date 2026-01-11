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

### Core Principle: Conversation + Artifact

The TUI is built around two complementary panes:

1. **Conversation Pane (Left)**: The persistent interaction layer
   - Timeline of all events (Spec, Run, Review, System)
   - Input area at bottom for user interaction
   - Always present, always the "conversation" with the system
   - Feels like a chat interface

2. **Artifact Pane (Right)**: The phase-specific output layer
   - Shows what's being produced (spec, run output, diff, etc.)
   - Adapts based on current phase
   - Can have contextual actions when focused
   - Display-focused, not input-focused

**Key insight**: Input lives on the LEFT (part of the conversation), not the right. The right pane shows the "product" being built. This creates a chat-first feel where everything flows through the timeline.

### Timeline as Source of Truth

All actions result in timeline events:
- User types a message → SpecEvent appears in timeline
- User presses `r` to refresh models → SystemEvent appears
- AI responds → SpecEvent with model attribution
- Run completes iteration → RunEvent appears

The timeline IS the conversation history, persisting across all phases.

### Component Hierarchy

```
App
├── StatusBar
│   └── [phase] │ [title] │ [model] │ [file:line] │ [metric] │ [next action]
├── HeartbeatRow (default: enabled, togglable via config)
│   └── ━━ file.rs +12 ━━ other.rs ~3 ━━━━━━━━━━━━━━━━━━━━━━
├── MainArea
│   ├── ConversationPane (left, persistent, interactive)
│   │   ├── TimelineEvents[] (scrollable history)
│   │   │   └── Typed (Spec|Run|Review|System), collapsible
│   │   └── InputArea (phase-aware, always present)
│   └── ArtifactPane (right, phase-adaptive, contextual actions)
│       └── ArtifactView (routed by ThreadPhase)
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
├─────────────────────────────┬───────────────────────────────────────┤
│   CONVERSATION              │   ARTIFACT                            │
│   ┌─────────────────────┐   │   ┌─────────────────────────────────┐ │
│   │                     │   │   │                                 │ │
│   │  Timeline Events    │   │   │  Phase-specific view:           │ │
│   │  (scrollable)       │   │   │  - SpecPreview                  │ │
│   │                     │   │   │  - RunOutput                    │ │
│   │                     │   │   │  - DiffViewer                   │ │
│   │                     │   │   │  - etc.                         │ │
│   ├─────────────────────┤   │   │                                 │ │
│   │ > Input area        │   │   │  [contextual actions when       │ │
│   │   (phase-aware)     │   │   │   focused: r, a, j/k, etc.]     │ │
│   └─────────────────────┘   │   └─────────────────────────────────┘ │
├─────────────────────────────┴───────────────────────────────────────┤
│ FooterHints (change based on phase AND focused pane)                │
└─────────────────────────────────────────────────────────────────────┘

        ┌─────────────────────┐
        │ OverlayLayer        │  ← Floats above, for toasts/palettes
        │ (when active)       │
        └─────────────────────┘
```

### Focus Model

- **Tab** cycles focus between Conversation (left) and Artifact (right)
- **When Conversation is focused**: Typing goes to input area
- **When Artifact is focused**: Keybinds go to artifact view actions
- **Footer hints** update to show available actions for the focused pane

### Data Flow

```
User Input ──→ Event Loop ──→ Action ──→ State Update ──→ Re-render
                                │
                                ├── Text input ──→ ChatState ──→ AI invocation
                                ├── Keybind ──→ Artifact action ──→ State change
                                └── Both ──→ TimelineEvent ──→ Persistent record

ThreadStore ──→ Thread ──→ App State ──→ UI Components
                  │
                  ├── phase ──→ StatusBar, ArtifactPane router
                  ├── events ──→ ConversationPane timeline
                  ├── spec ──→ SpecPreview (artifact)
                  └── run_state ──→ RunOutput (artifact)
```

---

## Artifact Views & Actions

The ArtifactPane renders different views based on `ThreadPhase`. Each view can define contextual actions available when focused.

| Phase(s) | Artifact View | Description | Focused Actions |
|----------|---------------|-------------|-----------------|
| No Thread | ModelsPanel | Model status | `r` Refresh, `a` Auth |
| Drafting, Assessing | SpecPreview | Live spec from chat | `y` Copy |
| Finalized | SpecPreview | Ready to run | `e` Edit (revert) |
| Preflight, PreflightFailed | PreflightResults | Check list | `r` Retry |
| Configuring | RunConfig | Model, iterations | Form navigation |
| Running, Verifying | RunOutput | Streaming + criteria | `c` Cancel |
| Paused, Stuck | DecisionPrompt | Options | `1-4` Choose |
| Implemented | Summary | What was done | `d` View diff |
| Polishing | PolishChecklist | Docs/tests checklist | `j/k` Nav, `Enter` Toggle |
| PendingReview, Approved | DiffViewer | File-by-file diff | `a` Approve, `j/k` Nav |
| ReadyToCommit, Done | CommitView | Commit message | `c` Commit |

**All actions result in timeline events** - the timeline is the unified record of everything that happened.

## Input Purpose by Phase

The input area (left pane) is always present but its purpose changes:

| Phase(s) | Input Purpose |
|----------|---------------|
| No Thread | "Start typing to create a thread..." |
| Drafting, Assessing | Chat with AI to develop spec |
| Finalized | Type to edit (reverts to Drafting), or commands |
| Preflight | Wait / type to cancel |
| Configuring | Confirm settings |
| Running, Verifying | Type to cancel or direct next iteration |
| Paused, Stuck | Provide direction, choose action |
| Implemented | Feedback, continue to review |
| Polishing | Direct what to add (docs, tests), continue |
| PendingReview | Comments, approve/reject |
| ReadyToCommit, Done | Edit commit message, finalize |

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
- Mouse support (scroll, click-to-select, double-click-to-toggle)
- Copy to clipboard (`y` or `Ctrl+C` to copy selected event content)

**Exit Criteria:** Timeline pane shows events, can scroll and select, events display with proper formatting. Can copy event content to clipboard.

#### M5-B.2: Phase Router & Dynamic Status
**Spec:** `SPEC-m5b2-phase-router.md`

Wire up the shell to thread state for dynamic content.

**Deliverables:**
- Phase router component (ThreadPhase → Context View)
- Status bar driven by thread state (phase, title, current model)
- Footer hints that change per phase
- "Next action" guidance in status bar

**Exit Criteria:** Status bar and footer update based on thread phase, context pane routes to appropriate view.

#### M5-B.3: Conversation & Spec Flow

Build the conversation layer (input in timeline) and spec artifact view. This delivers the full Drafting → Finalized flow with the new Conversation + Artifact architecture.

##### M5-B.3a: Timeline Input ✓
**Spec:** `SPEC-m5b3a-timeline-input.md`
**Status:** Complete (2026-01-09)

Add the input area to the conversation pane (left), making the timeline interactive.

**Deliverables:**
- ✓ Input widget at bottom of timeline pane (ConversationPane)
- ✓ Focus management (Tab between conversation/artifact)
- ✓ Text input handling (Enter to send, Shift+Enter for newline)
- ✓ Phase-aware placeholder text
- Input history deferred to M5-B.3a'

**Exit Criteria:** ✓ Can type in the input area, input is visually part of the timeline, focus switches between panes.

##### M5-B.3a': Slash Command Infrastructure ✓
**Spec:** `SPEC-slash-commands.md`
**Status:** Complete (2026-01-09)

Implement the slash command system for TUI actions. All actions available via `/command` syntax.

**Background:**
The initial M5-B.3a implementation revealed a UX conflict: reserved keys (q=quit, 1/2/3=modes) blocked free typing. After analysis, we adopted slash commands as the primary action mechanism.

**Deliverables:**
- ✓ Command parser and registry
- ✓ Global commands: `/help`, `/quit`, `/exit`, `/split`, `/focus`, `/canvas`, `/refresh`, `/clear`, `/copy`
- ✓ Autocomplete popup when `/` typed
- ✓ Focus trap: `/` from any pane jumps to input
- ✓ `F1` for help overlay
- ✓ Phase-specific command stubs: `/approve`, `/reject`, `/pause`, `/resume`, `/cancel`, `/finalize`, `/assess`
- ✓ Click-to-focus for panes
- ✓ Esc only clears input (use `/quit` to exit)
- ✓ Help overlay with keyboard shortcuts

**Exit Criteria:** ✓ Can use `/help` to see commands, slash commands execute actions, typing is never blocked.

##### M5-B.3a'': Focus Model & Layout Rework
**Spec:** `SPEC-m5b3a-focus-layout.md`
**Status:** Complete (2026-01-09)

Rework the focus model and layout based on manual testing feedback. This addresses UX issues discovered in M5-B.3a' testing.

**Background:**
Manual testing of M5-B.3a' revealed several UX issues:
1. Can't type in canvas mode (input hidden)
2. Timeline navigation confusing (Alt+j/k awkward, when does it apply?)
3. Footer hints cluttered and not like Claude Code
4. No way to click on a pane to focus it
5. Need explicit focus states with pane-specific keybindings

**Design Decisions:**
- **Full-width input bar:** Input always visible at bottom, spans both panes
- **Status bar at bottom:** Replace footer hints with simple status bar (like Claude Code)
- **Explicit focus model:** Tab cycles focus, each pane has its own keybindings
- **Click-to-focus:** Mouse click on pane switches focus
- **Pane-specific keybindings:**
  - Timeline focused: `j/k` navigate, `y` copy, `Enter` toggle
  - Canvas focused: TBD context-specific
  - Input focused: typing, `Enter` submit

**Proposed Layout:**
```
┌─────────────────────────────────────────────────────┐
│ Status Bar (phase, title, model)                    │
├────────────────────────┬────────────────────────────┤
│ Timeline/Conversation  │ Canvas/Context             │
│                        │                            │
├────────────────────────┴────────────────────────────┤
│ > Input area (full width, always visible)           │
├─────────────────────────────────────────────────────┤
│ Split │ Timeline │ No thread       (minimal status) │
└─────────────────────────────────────────────────────┘
```

**Deliverables:**
- ✓ Full-width input bar (visible in all modes)
- ✓ Minimal status bar at bottom (mode, focus, phase, pane-specific hints)
- ✓ Three-way focus model with Tab cycling (Timeline → Canvas → Input)
- ✓ Click-to-focus for panes
- ✓ Pane-specific keybindings (j/k, y in timeline; r in canvas; typing in input)
- ✓ Context-aware status (mode, focus, thread state)
- ✓ Toast notification when y pressed with no selection

**Exit Criteria:** ✓ Can navigate timeline with j/k when focused, input always accessible, clean minimal UI like Claude Code.

**Future: Kitty Keyboard Protocol**
Consider adding [Kitty keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) support for enhanced keybinding detection. Benefits:
- Command+1/2/3 on Mac (currently only Alt+1/2/3 works cross-platform)
- Better modifier key detection (distinguish Ctrl+i from Tab, etc.)
- Key release events for hold-to-preview features

Requires `PushKeyboardEnhancementFlags` with `DISAMBIGUATE_ESCAPE_CODES`. Supported in modern terminals: iTerm2, WezTerm, Kitty, Alacritty. Should gracefully fall back when unsupported.

##### M5-B.3b: Chat Integration ✓
**Spec:** `SPEC-m5b3b-chat-integration.md`
**Status:** Complete (2026-01-10)

Wire the input to the chat system, creating SpecEvents from user/AI messages.

**Deliverables:**
- ✓ ChatState management (thread, input, loading, error)
- ✓ Async AI invocation via tokio + mpsc channels
- ✓ User messages → SpecEvent in timeline
- ✓ AI responses → SpecEvent with model attribution
- ✓ Thread creation on first message
- ✓ Thread.draft feedback loop (extracted spec fed to AI)
- Thread persistence deferred to M5-B.5
- **Live model status updates:**
  - ✓ Update status on API success → "Ready"
  - ✓ Update status on rate limit → "Rate Limited" + cooldown timer
  - ✓ Update status on auth failure → "Auth Required"
  - ✓ Update status on error/timeout → "Unavailable"
  - ✓ Status bar shows model status indicators
  - ✓ Manual refresh via `Ctrl+R` (when models panel focused)
  - Model status caching deferred to M5-B.4

**Exit Criteria:** ✓ Can chat with AI, messages appear in timeline as SpecEvents. Model status updates based on actual API interactions.

##### M5-B.3c: Spec Artifact View
**Spec:** `SPEC-m5b3c-spec-artifact.md`

Build the SpecPreview artifact view (right pane) and phase transitions.

**Deliverables:**
- SpecPreview widget showing extracted spec
- Markdown rendering (headers, code, lists, checkboxes)
- Live updates as AI refines spec
- Phase transitions: Drafting → Assessing → Finalized
- Finalized state shows "Ready to run"
- Artifact actions when focused (`y` copy, `e` edit/revert)

**Exit Criteria:** ✓ Right pane shows spec preview, markdown renders correctly, phase badge displays correctly. Spec copy (`y` key) works.

##### M5-B.3c': Markdown Foundation
**Spec:** `SPEC-m5b3c-prime-markdown.md`

Upgrade markdown rendering to use `pulldown-cmark` and add markdown support to timeline/conversation pane.

**Background:**
M5-B.3c introduced a simple line-by-line markdown parser for SpecPreview. However, AI messages in the timeline render as plain text, showing literal `**bold**` and `# headers` instead of styled text. This hurts UX during testing and will be needed for M5-B.3d (run output with code blocks).

Codex uses `pulldown-cmark` for robust markdown parsing. We should adopt the same approach and create a shared text rendering module.

**Deliverables:**
- Replace simple `context/markdown.rs` with `pulldown-cmark` based parser
- Create shared `text/` module for markdown rendering
- Add markdown rendering to ConversationPane for AI/assistant messages
- Keep user messages as plain styled text (like Codex)
- SpecPreview uses the new shared renderer

**Architecture:**
```
crates/ralf-tui/src/
├── text/
│   ├── mod.rs
│   ├── markdown.rs      # pulldown-cmark based renderer
│   └── styles.rs        # MarkdownStyles struct
├── context/
│   └── spec_preview.rs  # Uses text::markdown
├── conversation/
│   └── widget.rs        # Uses text::markdown for AI messages
```

**Exit Criteria:** AI messages in timeline render with markdown styling (headers bold, code highlighted, lists formatted). SpecPreview continues to work. Single markdown implementation shared across components.

##### M5-B.3e: Workflow UX & Phase Guidance
**Spec:** `WORKFLOW_UX.md`

Add user guidance for phase transitions and implement the `/status` command.

**Background:**
Users need to understand where they are in the workflow and how to progress. This implements the Coordinator + Collaborator model for spec review and adds visual guidance throughout.

**Deliverables:**
- Phase-aware footer hints showing next actions (`/accept when ready`)
- Status bar "next action" hint
- Spec pane badge with guidance text (`[Drafting] /accept when ready`)
- `/status` command showing workflow diagram with current position
- Phase transition commands wired to engine:
  - `/accept` - Drafting → Assessing
  - `/approve` - Assessing → Finalized
  - `/edit` - Return to Drafting
  - `/finalize` - Skip assessment, go to Finalized
- System events for phase transitions in timeline

**Exit Criteria:** User can see where they are in workflow, knows what command to use next, `/status` shows visual workflow diagram.

##### M5-B.3f: Model Role Assignment
**Spec:** `WORKFLOW_UX.md` (Model Role Assignment section)

Redesign the Models panel to support per-phase coordinator/collaborator assignment.

**Background:**
The Coordinator + Collaborator model requires users to configure which models serve which roles. Different phases (Spec, Implementation, Finalization) may want different coordinators and collaborators.

**Deliverables:**
- Models panel redesign:
  - Status section (existing model status indicators)
  - Role Configuration section showing per-phase assignments
  - j/k navigation, Enter to configure
  - Mouse click support
- Per-phase configuration:
  - Coordinator (single model dropdown)
  - Collaborators (multi-select checkboxes)
  - Three major phases: Spec, Implementation, Finalization
- Dynamic switching:
  - `/model <name>` command for temporary coordinator override
  - Status bar indicator when overridden (asterisk)
  - Session-only, not persisted
- Configuration persistence to `~/.ralf/config.toml`
- System prompts for each role (engine integration)

**Exit Criteria:** Can configure coordinator/collaborator for each major phase from Models panel. Can temporarily switch models with `/model` command. Configuration persists across sessions.

##### M5-B.3d: Run Artifact Views
**Spec:** `SPEC-m5b3d-run-artifacts.md`

Build the artifact views for the run phase.

**Deliverables:**
- RunOutput widget (streaming output + criteria checklist)
- Run events appear in timeline
- Summary widget (files changed, criteria results)
- Artifact actions (`c` cancel during run)

**Exit Criteria:** Can start a run, see output in artifact pane, run events in timeline, summary after completion.

#### M5-B.4: Advanced Context Views
**Spec:** `SPEC-m5b4-advanced-views.md`

Build remaining context views for full workflow support.

**Deliverables:**
- **PreflightResults** - Check list with pass/fail + actions (Preflight, PreflightFailed)
- **RunConfig** - Model selection, iteration limit, verifiers (Configuring)
- **DecisionPrompt** - Options with numbered keys (Paused, Stuck)
- **DiffViewer** - File-by-file diff with navigation (PendingReview, Approved)
- **CommitView** - Commit message editor + summary (ReadyToCommit, Done)
- **ModelsPanel navigation** - j/k to navigate model list, Enter to enable/disable models
- Timeline filtering by event type

**Exit Criteria:** Can walk through entire workflow (Draft → Run → Review → Commit) with appropriate views at each phase. Can enable/disable models from Models panel.

#### M5-B.5: Thread Management
**Spec:** `SPEC-m5b5-thread-management.md`

Complete thread lifecycle management for the TUI and CLI.

**Background:**
Users need to manage multiple threads (tasks) over time - resuming previous work, organizing threads by name, cleaning up completed work. Claude Code provides `-c` (continue last) and `-r` (recent picker). ralf improves on this with named threads and richer management.

**CLI Deliverables:**
- `ralf shell -c` - Continue last active thread
- `ralf shell -r` - Show thread picker (interactive)
- `ralf shell <name>` - Resume thread by name
- `ralf shell --new [name]` - Create new thread with optional name
- `ralf threads` - List all threads (non-interactive)
- `ralf threads rm <name>` - Delete thread

**TUI Deliverables:**
- Thread picker overlay (`Ctrl+T` or `/threads`)
- No-thread welcome screen with recent threads list
- `/resume [name]` - Resume by name or show picker
- `/new [name]` - Create new thread
- `/rename <name>` - Rename current thread
- `/close` - Close current thread (return to welcome)
- Thread name in status bar
- Thread auto-naming from first message (can override)

**Thread Features:**
- Named threads (user-provided or auto-generated)
- Thread states: Active, Suspended, Completed, Abandoned
- Thread persistence to `.ralf/threads/`
- Thread search/filter in picker
- Crash recovery (resume interrupted thread)

**Exit Criteria:** Can launch ralf and resume a previous thread by name. Can manage threads from CLI without entering TUI. Thread picker shows filterable list with names and status.

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
- Mouse text selection (allow selecting text in timeline/canvas for copy)
- Error state handling

**Exit Criteria:** Run an autonomous loop and feel the activity. New user can learn the UI without docs.

---

## Dependencies

```
M5-A (Shell) ✓
  ├── M5-A.1 (Model Probing) ✓
  │
  ▼
M5-B (Conversation & Artifacts)
  ├── M5-B.1 (Timeline Foundation) ✓
  ├── M5-B.2 (Phase Router & Dynamic Status) ✓
  ├── M5-B.3 (Conversation & Spec Flow)
  │   ├── M5-B.3a (Timeline Input) ✓
  │   ├── M5-B.3a' (Slash Commands) ✓
  │   ├── M5-B.3a'' (Focus Model & Layout Rework) ✓
  │   ├── M5-B.3b (Chat Integration) ✓
  │   ├── M5-B.3c (Spec Artifact View) ✓
  │   ├── M5-B.3c' (Markdown Foundation) ✓
  │   ├── M5-B.3e (Workflow UX & Phase Guidance) ← NEXT
  │   ├── M5-B.3f (Model Role Assignment)
  │   └── M5-B.3d (Run Artifact Views)
  ├── M5-B.4 (Advanced Artifact Views)
  └── M5-B.5 (Thread Management)
  │
  ▼
M5-C (Activity & Polish)
```

Each major phase builds on the previous. No parallel development between major phases.

**Within M5-B**, subphases should be completed sequentially:
- M5-B.1 → M5-B.2: Phase router needs timeline events to display ✓
- M5-B.2 → M5-B.3: Conversation layer needs router infrastructure ✓
- M5-B.3a → M5-B.3a': Slash commands refine input handling ✓
- M5-B.3a' → M5-B.3a'': Focus model rework addresses UX issues from testing
- M5-B.3a'' → M5-B.3b: Chat integration needs stable focus/input model
- M5-B.3b → M5-B.3c: Spec artifact needs chat to produce content
- M5-B.3c → M5-B.3c': Markdown foundation improves UX and provides shared renderer
- M5-B.3c' → M5-B.3e: Workflow UX adds phase guidance and /status command
- M5-B.3e → M5-B.3f: Model role assignment builds on workflow phase concepts
- M5-B.3f → M5-B.3d: Run artifacts need model roles configured for implementation loop
- M5-B.3 and M5-B.4 could potentially overlap once conversation layer is ready
- M5-B.4 → M5-B.5: Thread management needs all views in place
- M5-B.5 could start earlier for CLI-only features (ralf threads)

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
- **Clipboard integration** - Copy event content for sharing/debugging
  - `y` (vim yank) or `Ctrl+C` to copy selected event
  - Cross-platform clipboard via `arboard` crate
  - Essential for extracting error messages, model output, etc.

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
| 2026-01-09 | Broke M5-B.3 into subphases: B.3a SpecEditor, B.3b RunOutput, B.3c Summary. Marked B.1, B.2 complete. |
| 2026-01-09 | Completed M5-B.3a (Timeline Input). Added M5-B.3a' (Slash Commands) after UX analysis revealed input-first model needed. Updated TUI_UX_PRINCIPLES.md with slash command system design. |
| 2026-01-09 | Added M5-B.5 (Thread Management) for CLI flags (-c, -r) and TUI thread picker. Improves on Claude Code with named threads. |
| 2026-01-09 | Added M5-B.3a'' (Focus Model & Layout Rework) based on manual testing feedback. Decisions: full-width input bar, status bar at bottom replacing footer hints, explicit focus model with pane-specific keybindings. |
| 2026-01-09 | Added Kitty keyboard protocol as future consideration in M5-B.3a'' for Command+1/2/3 on Mac and enhanced modifier detection. |
| 2026-01-09 | Completed M5-B.3a'' (Focus Model & Layout Rework): three-way focus, full-width input, pane-specific keybindings, toast notifications. Added mouse text selection to M5-C polish items. |
| 2026-01-10 | Completed M5-B.3b (Chat Integration): async AI invocation, user/AI messages in timeline, model status updates, panic hook for terminal restoration, integration tests and testing documentation. |
| 2026-01-10 | Added M5-B.3e (Workflow UX & Phase Guidance): phase-aware hints, /status command, Coordinator/Collaborator model. Created WORKFLOW_UX.md documenting the user experience flow. |
| 2026-01-10 | Completed M5-B.3c' (Markdown Foundation): pulldown-cmark renderer, markdown in timeline, text wrapping, unicode support. |
| 2026-01-10 | Added M5-B.3f (Model Role Assignment): per-phase coordinator/collaborator config, Models panel redesign, dynamic switching. Updated WORKFLOW_UX.md with model role assignment design. |
