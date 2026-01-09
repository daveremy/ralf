# M5-B.1: Timeline Foundation

## Promise

Build the timeline event system that forms the backbone of the TUI. Users see a scrollable list of events showing the history of their thread - spec changes, model invocations, verification results, and system messages.

## Context

The M5-A shell provides the two-pane layout with placeholder content. This subphase replaces the Timeline pane placeholder with a real event system.

**Dependencies:**
- M5-A shell (complete)
- M5-A.1 model probing (complete)

**References:**
- [TUI_DEV_PLAN.md](TUI_DEV_PLAN.md) - Phase Views section
- [TUI_UX_PRINCIPLES.md](TUI_UX_PRINCIPLES.md) - Timeline interaction patterns
- [TUI_STYLE_GUIDE.md](TUI_STYLE_GUIDE.md) - Event badges and colors

## Deliverables

### 1. TimelineEvent Data Model

Four event types covering all thread activity:

```rust
/// A timeline event representing thread activity.
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    /// Unique event ID.
    pub id: u64,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Event type and content.
    pub kind: EventKind,
    /// Whether the event is collapsed (for multi-line content).
    pub collapsed: bool,
}

#[derive(Debug, Clone)]
pub enum EventKind {
    /// Spec-related events (user input, spec changes).
    Spec(SpecEvent),
    /// Run-related events (model invocations, iterations).
    Run(RunEvent),
    /// Review-related events (verification, approval).
    Review(ReviewEvent),
    /// System events (model status, errors).
    System(SystemEvent),
}

#[derive(Debug, Clone)]
pub struct SpecEvent {
    /// User message or spec update.
    pub content: String,
    /// Whether this is user input vs system-generated.
    pub is_user: bool,
}

#[derive(Debug, Clone)]
pub struct RunEvent {
    /// Which model produced this.
    pub model: String,
    /// Iteration number (1-based).
    pub iteration: u32,
    /// Event content (file change, command output, etc.).
    pub content: String,
    /// Optional file path if this is a file change.
    pub file: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReviewEvent {
    /// Criterion being verified.
    pub criterion: String,
    /// Verification result.
    pub result: ReviewResult,
    /// Optional details.
    pub details: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewResult {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone)]
pub struct SystemEvent {
    /// System message (model ready, error, etc.).
    pub message: String,
    /// Severity level.
    pub level: SystemLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemLevel {
    Info,
    Warning,
    Error,
}
```

### 2. Timeline Pane Widget

Replace the placeholder Timeline pane with a scrollable event list:

```
┏ Timeline ━━━━━━━━━━━━━━━━━━━━┓
┃ 10:23  [SPEC] User           ┃
┃        Add login endpoint... ┃
┃                              ┃
┃ 10:24  [RUN] claude #1       ┃
┃      ▸ src/auth.rs +47       ┃
┃                              ┃
┃ 10:25  [RUN] claude #1       ┃
┃        Running tests...      ┃
┃                              ┃
┃►10:26  [REVIEW] ✓            ┃  ← Selected event
┃        Tests pass            ┃
┃                              ┃
┃ 10:26  [SYS] ●               ┃
┃        gemini ready          ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

**Event rendering:**
- Timestamp (HH:MM format, muted color)
- Badge: `[SPEC]`, `[RUN]`, `[REVIEW]`, `[SYS]`
- Attribution: model name for Run events, "User" for Spec events
- Content: single line or multi-line with collapse indicator

**Badge colors (from TUI_STYLE_GUIDE.md):**
- SPEC: `primary` (lavender)
- RUN: model color (claude=peach, gemini=blue, codex=green)
- REVIEW: `success`/`error` based on result
- SYS: `info`/`warning`/`error` based on level

### 3. Keyboard Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `g` | Jump to first event |
| `G` | Jump to last event |
| `Enter` | Toggle collapse for selected event |
| `PageDown` | Scroll down one page |
| `PageUp` | Scroll up one page |

**Selection behavior:**
- Selection indicator: `►` prefix on selected event
- Auto-scroll to keep selection visible
- Selection stops at boundaries (no wrap)

### 3a. Mouse Support

Mouse support enabled when terminal supports it (crossterm handles detection).

| Mouse Action | Effect |
|--------------|--------|
| Scroll wheel up | Scroll timeline up (disables follow) |
| Scroll wheel down | Scroll timeline down |
| Left click on event | Select that event |
| Double-click on event | Toggle collapse |

**Mouse behavior notes:**
- Scroll wheel scrolls by 3 events per tick (configurable)
- Click selection works even when timeline pane is not focused
- Mouse support degrades gracefully (keyboard always works)

### 4. Collapsible Events

Events with multi-line content can be collapsed:

```
▸ src/auth.rs +47        (collapsed - shows first line only)
▾ src/auth.rs +47        (expanded - shows all lines)
   + pub fn login() {
   +     // implementation
   + }
```

**Collapse rules:**
- Events with > 1 line of content are collapsible
- Default state: collapsed for Run events, expanded for others
- `Enter` toggles collapse state
- Collapse indicator: `▸` (collapsed) / `▾` (expanded)

### 5. Timeline State

**Design Decision: Event-based scrolling with fixed display height**

To simplify scroll management, each event has a **fixed display height** regardless of content:
- Collapsed events: 2 lines (timestamp+badge line, content preview line)
- Expanded events: 2 + min(content_lines, MAX_EXPANDED_LINES) where MAX_EXPANDED_LINES = 10

This avoids the complexity of line-based scrolling while still allowing useful expansion.

```rust
/// Timeline pane state.
#[derive(Debug, Default)]
pub struct TimelineState {
    /// All events in chronological order.
    pub events: Vec<TimelineEvent>,
    /// Index of selected event (if any).
    pub selected: Option<usize>,
    /// Index of first visible event.
    pub scroll_offset: usize,
    /// Whether to auto-follow new events.
    pub follow: bool,
}

impl TimelineState {
    /// Add a new event to the timeline.
    /// If `follow` is true, auto-scrolls to show the new event.
    pub fn push(&mut self, event: TimelineEvent) { ... }

    /// Move selection up. Stops at first event (no wrap).
    /// Disables follow mode.
    pub fn select_prev(&mut self) { ... }

    /// Move selection down. Stops at last event (no wrap).
    pub fn select_next(&mut self) { ... }

    /// Jump to first event. Disables follow mode.
    pub fn jump_to_start(&mut self) { ... }

    /// Jump to last event. Enables follow mode.
    pub fn jump_to_end(&mut self) { ... }

    /// Toggle collapse for selected event.
    pub fn toggle_collapse(&mut self) { ... }

    /// Ensure selected event is visible, adjusting scroll_offset if needed.
    pub fn ensure_selection_visible(&mut self, visible_count: usize) { ... }

    /// Calculate how many events fit in the given height.
    pub fn events_per_page(&self, height: usize) -> usize { ... }
}
```

**Follow mode behavior:**
- `follow = true`: New events auto-scroll timeline to bottom
- `follow = false`: Timeline stays at current scroll position
- Scrolling up (k/↑, PageUp) or jumping to start (g) → disables follow
- Jumping to end (G) → enables follow
- New events when `follow = true` → scroll to show them

**Selection behavior:**
- Selection stops at boundaries (no wrap)
- PageUp/PageDown moves selection by page, clamped to bounds
- Selection always stays visible (auto-scroll if needed)

### 6. Content Display Rules

**Timestamp format:**
- Display: `HH:MM` in local time (converted from UTC storage)
- Same-minute events distinguished by order in list

**Content truncation:**
- First line: truncate at pane width - 4 chars (for padding), add `…` if truncated
- Expanded content: wrap at pane width, max 10 lines, add `[+N more]` if truncated

**Run event summary:**
- If `file` is set: show `▸ {file} {change_summary}` (e.g., `▸ src/auth.rs +47`)
- If no file: show first line of `content`

## Technical Approach

### Integration Points

1. **ShellApp** gains `timeline: TimelineState`
2. **render_timeline_pane** renders from `TimelineState`
3. **Key handling** routes j/k/g/G/Enter to TimelineState methods
4. **Event generation** (future): engine callbacks populate timeline

### File Structure

```
crates/ralf-tui/src/
├── timeline/
│   ├── mod.rs           # Module exports
│   ├── event.rs         # TimelineEvent, EventKind, etc.
│   ├── state.rs         # TimelineState
│   └── widget.rs        # Timeline pane widget
├── shell.rs             # Add TimelineState, route keys
└── layout/
    └── shell.rs         # Update render_timeline_pane
```

### Rendering Strategy

1. Calculate visible height (pane height - border)
2. Determine visible event range from scroll_offset
3. For each visible event:
   - Render timestamp + badge + attribution on first line
   - Render content (respecting collapse state)
   - Highlight if selected
4. Handle partial events at boundaries

## Acceptance Criteria

- [ ] Timeline pane shows events (not placeholder text)
- [ ] Events render with timestamp, badge, attribution, content
- [ ] Badges have correct colors per event type
- [ ] j/k (or Up/Down) moves selection
- [ ] g/G jumps to first/last event
- [ ] Enter toggles collapse for collapsible events
- [ ] Selection stays visible when scrolling
- [ ] PageUp/PageDown scrolls by page
- [ ] Mouse scroll wheel scrolls timeline
- [ ] Mouse click selects event
- [ ] Mouse double-click toggles collapse
- [ ] Empty timeline shows helpful message ("No events yet")

## Testing

### Unit Tests
- `TimelineEvent` creation for each event type
- `TimelineState` navigation (select_prev, select_next, bounds)
- `TimelineState` collapse toggling
- Visible events calculation

### Snapshot Tests
- Timeline with various event types
- Timeline with selected event
- Timeline with collapsed/expanded events
- Empty timeline

### Manual Testing
- Add events, verify scrolling
- Test keyboard navigation
- Test collapse/expand
- Test with narrow terminal

## Out of Scope

- **Event filtering by type** - Deferred to M5-B.4
- **Event search** - Future enhancement
- **Event details panel** - Context pane shows details (M5-B.3+)
- **Real event generation** - For now, use mock/test events

## Resolved Questions

1. **Selection wrap behavior:** Stop at boundaries (less surprising). ✓

2. **Timestamp format:** HH:MM in local time for compactness. ✓

3. **Event ID generation:** Sequential u64 for simplicity, engine can change later. ✓

4. **Scroll model:** Event-based scrolling with fixed display height per event (2 lines collapsed, up to 12 expanded). This avoids complexity of line-based scrolling. ✓

5. **Follow mode:** Explicit rules - scrolling up disables, G re-enables. ✓

## Open Questions

1. **Unknown model colors:** If a model name isn't in theme (e.g., new CLI tool), what color? Suggest: use `info` color as fallback.

## Estimated Scope

- ~400-500 lines of new code
- 4 new files in `timeline/` module
- Modifications to shell.rs, layout/shell.rs
