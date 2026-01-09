# SPEC-m5b3a: Timeline Input

## Promise

Add the input area to the conversation pane (left), making the timeline interactive. This lays the foundation for the Conversation + Artifact architecture where the left pane is the persistent interaction surface and the right pane shows phase-specific artifacts.

After this milestone:
1. Users can type in an input area at the bottom of the timeline pane
2. Tab switches focus between conversation and artifact panes
3. Input placeholder text changes based on the current phase
4. Enter triggers a "submit" action (actual chat integration is M5-B.3b)

## Background

The TUI follows a **Conversation + Artifact** architecture:
- **Left pane (Conversation)**: Timeline events + Input area (persistent across phases)
- **Right pane (Artifact)**: Phase-specific view (SpecPreview, RunOutput, etc.)

This milestone implements the input area component and focus management. The input will later be wired to the chat system in M5-B.3b.

**Existing components we build on:**
- `TextInputState` / `TextInput` - cursor movement, history navigation, multiline support
- `TimelineState` / `TimelineWidget` - scrollable event history
- `FocusedPane` enum - currently `Timeline` or `Context`
- `ShellApp` - main application state

## Deliverables

### 1. ConversationPane Widget

**File:** `crates/ralf-tui/src/conversation/widget.rs`

A composite widget that combines the timeline with an input area:

```
┌─ Conversation ──────────────────────┐
│                                      │
│  [SpecEvent] User: I want to build  │
│              a CLI that converts... │
│                                      │
│  [SpecEvent] claude: Here's a       │
│              draft specification... │
│                                      │
│  [RunEvent] Iteration 1 started...  │
│                                      │
├──────────────────────────────────────┤
│ > Type your message...            │
└──────────────────────────────────────┘
```

The widget renders:
1. **Timeline area** (top, flexible height): Scrollable event history
2. **Divider** (1 line): Separates history from input
3. **Input area** (bottom, 3 lines): Text input with phase-aware placeholder

```rust
pub struct ConversationPane<'a> {
    timeline: &'a TimelineState,
    input: &'a TextInputState,
    phase: Option<PhaseKind>,
    theme: &'a Theme,
    focused: bool,
}

impl<'a> ConversationPane<'a> {
    pub fn new(
        timeline: &'a TimelineState,
        input: &'a TextInputState,
        theme: &'a Theme,
    ) -> Self;

    pub fn phase(mut self, phase: Option<PhaseKind>) -> Self;
    pub fn focused(mut self, focused: bool) -> Self;
}

impl Widget for ConversationPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer);
}
```

### 2. Phase-Aware Placeholder Text

**File:** `crates/ralf-tui/src/conversation/placeholder.rs`

The input placeholder changes based on the current phase:

```rust
/// Get placeholder text for the input area based on phase.
pub fn input_placeholder(phase: Option<PhaseKind>) -> &'static str {
    match phase {
        None => "Start typing to create a thread...",
        Some(PhaseKind::Drafting) => "Describe your task...",
        Some(PhaseKind::Assessing) => "Refine your specification...",
        Some(PhaseKind::Finalized) => "Type to edit, or press [r] to run...",
        Some(PhaseKind::Preflight) => "Waiting for preflight checks...",
        Some(PhaseKind::PreflightFailed) => "Fix issues or type to retry...",
        Some(PhaseKind::Configuring) => "Confirm settings...",
        Some(PhaseKind::Running | PhaseKind::Verifying) => "Type to cancel or direct...",
        Some(PhaseKind::Paused | PhaseKind::Stuck) => "Provide direction...",
        Some(PhaseKind::Implemented) => "Continue to review...",
        Some(PhaseKind::Polishing) => "Add docs, tests, or continue...",
        Some(PhaseKind::PendingReview) => "Comment or approve...",
        Some(PhaseKind::Approved) => "Proceed to commit...",
        Some(PhaseKind::ReadyToCommit) => "Edit commit message...",
        Some(PhaseKind::Done | PhaseKind::Abandoned) => "Start a new thread...",
    }
}
```

### 3. Updated Focus Management

**File:** Update `crates/ralf-tui/src/shell.rs`

Extend focus handling to support input within the conversation pane:

```rust
impl ShellApp {
    /// Handle key event when conversation pane is focused.
    /// Returns true if the event was handled.
    pub fn handle_conversation_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            // Text input handling
            KeyCode::Char(c) if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
                self.input.insert(c);
                true
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.input.insert('\n');
                true
            }
            KeyCode::Enter => {
                self.submit_input();
                true
            }
            KeyCode::Backspace => {
                self.input.backspace();
                true
            }
            KeyCode::Delete => {
                self.input.delete();
                true
            }
            KeyCode::Left => {
                self.input.move_left();
                true
            }
            KeyCode::Right => {
                self.input.move_right();
                true
            }
            KeyCode::Home => {
                self.input.move_home();
                true
            }
            KeyCode::End => {
                self.input.move_end();
                true
            }
            KeyCode::Up => {
                // Navigate history when at start of input
                if self.input.cursor == 0 || self.input.is_empty() {
                    self.input.history_prev();
                }
                true
            }
            KeyCode::Down => {
                // Navigate history when at end of input
                if self.input.cursor == self.input.content.len() {
                    self.input.history_next();
                }
                true
            }
            KeyCode::Esc => {
                if !self.input.is_empty() {
                    self.input.clear();
                }
                true
            }
            _ => false,
        }
    }

    /// Submit the current input (placeholder - actual chat in M5-B.3b).
    fn submit_input(&mut self) {
        let content = self.input.submit();
        if content.trim().is_empty() {
            return;
        }
        // For now, just add a system event to show it worked
        // Chat integration will replace this in M5-B.3b
        self.timeline.add_event(EventKind::System(SystemEvent {
            message: format!("[Input received: {} chars]", content.len()),
        }));
    }
}
```

### 4. TextInputState Integration

**File:** Update `crates/ralf-tui/src/shell.rs`

Add `TextInputState` to `ShellApp`:

```rust
pub struct ShellApp {
    // ... existing fields ...

    /// Text input state for the conversation pane.
    pub input: TextInputState,
}

impl ShellApp {
    pub fn new() -> Self {
        // ... existing initialization ...
        Self {
            // ... existing fields ...
            input: TextInputState::new(),
        }
    }
}
```

### 5. Layout Integration

**File:** Update `crates/ralf-tui/src/layout/shell.rs`

Modify `render_shell` to use `ConversationPane` for the timeline pane:

```rust
pub fn render_shell<B: Backend>(
    frame: &mut Frame,
    app: &mut ShellApp,
) {
    // ... existing layout code ...

    // Render conversation pane (left)
    let phase = app.current_thread.as_ref().map(|t| t.phase_kind);
    let conversation = ConversationPane::new(&app.timeline, &app.input, &app.theme)
        .phase(phase)
        .focused(app.focused_pane == FocusedPane::Timeline);
    frame.render_widget(conversation, timeline_area);

    // ... rest of rendering ...
}
```

### 6. Footer Hints Update

**File:** Update `crates/ralf-tui/src/widgets/footer_hints.rs`

Add conversation-focused hints:

```rust
/// Get hints when conversation pane is focused.
pub fn conversation_hints(phase: Option<PhaseKind>) -> Vec<Hint> {
    let mut hints = vec![
        Hint::new("Enter", "Send"),
        Hint::new("Shift+Enter", "Newline"),
    ];

    // Phase-specific hints
    match phase {
        Some(PhaseKind::Finalized) => {
            hints.push(Hint::new("r", "Run"));
        }
        Some(PhaseKind::Running | PhaseKind::Verifying) => {
            hints.push(Hint::new("c", "Cancel"));
        }
        _ => {}
    }

    hints.push(Hint::new("Tab", "Switch pane"));
    hints.push(Hint::new("?", "Help"));
    hints
}
```

## Non-Goals

- **Chat invocation**: Actual AI chat happens in M5-B.3b
- **Thread creation**: Creating Thread objects happens in M5-B.3b
- **Spec extraction**: Parsing AI responses for specs is M5-B.3c
- **Markdown rendering**: Rendering formatted messages is M5-B.3c
- **Streaming responses**: Real-time AI output is future work

## Acceptance Criteria

### Input Area
- [ ] Input area renders at bottom of conversation pane
- [ ] Input has a visual divider separating it from timeline
- [ ] Placeholder text shows when input is empty and unfocused
- [ ] Placeholder changes based on current phase
- [ ] Cursor shows when input is focused

### Text Editing
- [ ] Can type characters into input
- [ ] Backspace deletes character before cursor
- [ ] Delete removes character at cursor
- [ ] Left/Right arrows move cursor
- [ ] Home/End move to start/end of input
- [ ] Shift+Enter inserts newline (multiline support)
- [ ] Enter submits input (adds placeholder event for now)
- [ ] Esc clears input

### History Navigation
- [ ] Up arrow navigates to previous history entry (when at start)
- [ ] Down arrow navigates to next history entry (when at end)
- [ ] History preserves submitted inputs

### Focus Management
- [ ] Tab switches focus between conversation and artifact panes
- [ ] Conversation pane border highlights when focused
- [ ] Typing only works when conversation pane is focused
- [ ] Footer hints change based on focused pane

### Layout
- [ ] Input area is fixed height (3 lines)
- [ ] Timeline area takes remaining space
- [ ] Input area scrolls internally for long content
- [ ] Layout works at minimum terminal size (40x12)

### Tests
- [ ] Unit tests for `input_placeholder()` all phases
- [ ] Unit tests for `ConversationPane` rendering
- [ ] Unit tests for key handling in conversation focus
- [ ] Snapshot test for conversation pane layout
- [ ] Integration test for focus switching

## Technical Notes

### Border Composition

The `ConversationPane` wraps both timeline and input in a single border. The existing `TimelineWidget` has its own border - we'll need to either:
- **Option A**: Disable `TimelineWidget`'s border and render just its content
- **Option B**: Create a borderless variant of `TimelineWidget`
- **Option C**: Extract timeline content rendering into a separate function

Recommendation: Option A - modify `TimelineWidget` to accept a `borders: bool` parameter.

### Layout Calculation

The conversation pane layout:
```
total_height = pane_inner_height
input_height = 3 (fixed)
divider_height = 1
timeline_height = total_height - input_height - divider_height
```

### Input Area Height

Fixed at 3 lines to:
- Show enough context for multiline input
- Leave maximum space for timeline
- Keep predictable layout

If input exceeds 3 lines, it scrolls internally (handled by `TextInput` widget).

### Focus State Flow

```
Tab pressed
    │
    ├── Was Conversation → Now Artifact
    │   └── Footer hints → artifact actions
    │
    └── Was Artifact → Now Conversation
        └── Footer hints → input actions
```

### Event Flow (Placeholder)

Until M5-B.3b, Enter creates a system event:
```
User types → input.insert(c)
User presses Enter → input.submit() → SystemEvent → timeline
```

After M5-B.3b, this becomes:
```
User presses Enter → ChatState.send_message() → SpecEvent → timeline → AI invocation
```

## Dependencies

- M5-B.1 (Timeline Foundation) - `TimelineState`, `TimelineWidget`
- M5-B.2 (Phase Router) - `ContextView`, focus management
- Existing `TextInputState` / `TextInput` widgets

## Risks

1. **Input stealing focus**: Must ensure Tab always works to escape input
2. **Small terminals**: 3-line input might be too much on tiny screens
3. **History edge cases**: History navigation when mid-edit could be confusing

## Open Questions

1. **Input height**: Fixed 3 lines or collapse to 1 when empty? *Recommendation: Fixed 3 for consistency*
2. **Scroll sync**: Should timeline auto-scroll when new events appear? *Recommendation: Yes, unless user has scrolled up*
