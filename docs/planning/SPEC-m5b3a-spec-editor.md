# SPEC-m5b3a: SpecEditor Context View

## Promise

Build the SpecEditor context view - the entry point where users draft and refine task specifications through chat with AI. This enables the core Drafting → Assessing → Finalized workflow in the TUI.

After this milestone, users can:
1. Create a new thread from the TUI
2. Chat with AI to develop a task specification
3. See the extracted spec preview update as AI responds
4. Finalize the spec when it contains a `<promise>` tag
5. See phase transitions reflected in status bar and footer hints

## Background

The SpecEditor is shown when `ContextView::from_phase()` returns `SpecEditor` (phases: Drafting, Assessing, Finalized). Currently this renders a placeholder. This milestone implements the actual view.

**Engine APIs available:**
- `invoke_chat(model, context, timeout)` - async chat invocation
- `ChatContext` - conversation history + draft
- `Thread` - persistence (save/load)
- `extract_spec_from_response()` - extract spec from AI response
- `draft_has_promise()` - check if spec is finalized

**Existing TUI components:**
- `TextInputState` / `TextInput` - multiline input with cursor, history
- `ThreadDisplay` - UI-friendly thread state (M5-B.2)
- Phase routing infrastructure (M5-B.2)

## Deliverables

### 1. SpecEditor Widget

**File:** `crates/ralf-tui/src/context/spec_editor.rs`

A two-part view:
1. **Top: Message History** - Scrollable list of chat messages
2. **Bottom: Input Area** - Text input for user messages

```
┌─ Spec ─────────────────────────────────────────────┐
│                                                     │
│  You: I want to build a CLI that converts          │
│       markdown to HTML                             │
│                                                     │
│  claude: I'll help you create a spec for that.    │
│          Here's a draft:                           │
│                                                     │
│          # Markdown to HTML CLI                    │
│          ## Goal                                   │
│          Build a CLI tool that...                  │
│                                                     │
│          What input formats should it support?     │
│                                                     │
├─────────────────────────────────────────────────────┤
│ > Type your message...                          │
│   [Enter] Send  [Shift+Enter] Newline              │
└─────────────────────────────────────────────────────┘
```

**Message Display:**
- User messages: Right-aligned or prefixed with "You:"
- Assistant messages: Prefixed with model name (e.g., "claude:")
- System messages: Styled differently (muted)
- Timestamps shown on hover/expand (future)
- Auto-scroll to bottom on new messages

**Input Behavior:**
- Enter: Send message (if non-empty)
- Shift+Enter: Insert newline
- Up/Down: Navigate input history (when at start/end of input)
- Esc: Clear input (if non-empty) or blur focus

```rust
pub struct SpecEditor<'a> {
    messages: &'a [ChatMessage],
    input_state: &'a TextInputState,
    spec_preview: Option<&'a str>,
    phase: PhaseKind,
    theme: &'a Theme,
    focused: bool,
}

impl<'a> SpecEditor<'a> {
    pub fn new(
        messages: &'a [ChatMessage],
        input_state: &'a TextInputState,
        theme: &'a Theme,
    ) -> Self;

    pub fn spec_preview(mut self, preview: Option<&'a str>) -> Self;
    pub fn phase(mut self, phase: PhaseKind) -> Self;
    pub fn focused(mut self, focused: bool) -> Self;
}

impl Widget for SpecEditor<'_> {
    fn render(self, area: Rect, buf: &mut Buffer);
}
```

### 2. Spec Preview Panel (Optional Split)

When the AI has produced a spec (detected via `extract_spec_from_response`), show a preview panel. This can be:
- **Option A:** Inline in message history (spec portion highlighted)
- **Option B:** Split view with messages left, spec preview right
- **Option C:** Togglable with a keybind (e.g., `Ctrl+P` for preview)

**Recommendation:** Start with Option A (inline highlighting) for simplicity. The spec portion of AI responses is already visually distinct (markdown headers). Add Option B/C in polish phase if needed.

### 3. Chat State Management

**File:** `crates/ralf-tui/src/context/chat_state.rs`

Manage the chat session state in the TUI:

```rust
/// State for an active chat session.
pub struct ChatState {
    /// Thread being edited.
    pub thread: Thread,
    /// Text input state.
    pub input: TextInputState,
    /// Extracted spec preview (updated after each AI response).
    pub spec_preview: Option<String>,
    /// Whether AI is currently responding.
    pub loading: bool,
    /// Error from last invocation, if any.
    pub error: Option<String>,
    /// Scroll offset for message history.
    pub scroll_offset: usize,
}

impl ChatState {
    /// Create a new chat state with an empty thread.
    pub fn new() -> Self;

    /// Create from an existing thread (for resume).
    pub fn from_thread(thread: Thread) -> Self;

    /// Add a user message and prepare for AI response.
    pub fn send_message(&mut self) -> Option<String>;

    /// Handle AI response.
    ///
    /// This method:
    /// 1. Adds assistant message to thread
    /// 2. Extracts spec from response via `extract_spec_from_response()`
    /// 3. Updates `thread.draft` with extracted spec (critical for feedback loop!)
    /// 4. Updates `spec_preview` for display
    /// 5. Saves thread to disk
    pub fn receive_response(&mut self, response: ChatResult);

    /// Handle AI error.
    pub fn receive_error(&mut self, error: String);

    /// Update spec preview and thread.draft from latest AI message.
    ///
    /// IMPORTANT: Must update `thread.draft` so that `ChatContext::build_prompt()`
    /// includes the current draft in the next invocation. Without this, the AI
    /// won't see its previous spec suggestions.
    fn update_spec_preview(&mut self);

    /// Check if spec is finalized (has promise tag).
    pub fn is_finalized(&self) -> bool;

    /// Get messages for display.
    pub fn messages(&self) -> &[ChatMessage];
}
```

### 4. Async Chat Integration

**File:** `crates/ralf-tui/src/context/chat_handler.rs`

Handle async chat invocation from the TUI event loop:

```rust
use tokio::sync::mpsc;

/// Message types for chat async operations.
pub enum ChatEvent {
    /// AI response received.
    Response(ChatResult),
    /// AI invocation failed.
    Error(String),
}

/// Spawn a chat invocation task.
pub fn spawn_chat_invocation(
    model: ModelConfig,
    context: ChatContext,
    timeout_secs: u64,
    tx: mpsc::UnboundedSender<ChatEvent>,
);
```

The main event loop receives `ChatEvent` and updates `ChatState` accordingly.

### 5. Thread Creation Flow

When no thread is active and user starts typing, create a new thread:

1. User presses a key in SpecEditor
2. If no active thread, create `Thread::new()`
3. Thread is in Drafting phase
4. First message sets thread title (first 50 chars)

**Thread Lifecycle:**
- Created on first input
- Saved after each AI response
- Loaded on resume (Ctrl+T picker - future milestone)

### 6. Phase Transitions

The SpecEditor handles three phases with different behaviors:

| Phase | Behavior |
|-------|----------|
| Drafting | Chat enabled, AI responds with spec suggestions |
| Assessing | Chat enabled, AI reviews/refines spec |
| Finalized | Chat disabled (read-only), show "Ready to run" prompt |

**Transition Logic:**
- Drafting → Assessing: When AI produces first spec draft (contains `#` header)
- Assessing → Finalized: When spec contains `<promise>COMPLETE</promise>`
- Finalized → Drafting: User presses `e` to edit (reverts to drafting)

```rust
impl ChatState {
    /// Determine current phase based on spec state.
    pub fn current_phase(&self) -> PhaseKind {
        if self.is_finalized() {
            PhaseKind::Finalized
        } else if self.spec_preview.is_some() {
            PhaseKind::Assessing
        } else {
            PhaseKind::Drafting
        }
    }
}
```

### 7. Integration with Shell

Update `ShellApp` to manage chat state:

```rust
// In shell.rs
pub struct ShellApp {
    // ... existing fields ...

    /// Active chat state (when in spec editing phases).
    pub chat_state: Option<ChatState>,

    /// Channel for receiving chat events.
    chat_rx: Option<mpsc::UnboundedReceiver<ChatEvent>>,
}

impl ShellApp {
    /// Start a new chat session.
    pub fn start_new_thread(&mut self);

    /// Handle chat-related key events.
    pub fn handle_chat_key(&mut self, key: KeyEvent) -> bool;

    /// Poll for chat events (called in event loop).
    pub fn poll_chat_events(&mut self);
}
```

### 8. Footer Hints Updates

Update `hints_for_state` (from M5-B.2) to show chat-specific hints:

| Phase | Context Focused | Hints |
|-------|-----------------|-------|
| Drafting | Yes | `[Enter] Send` `[Shift+Enter] Newline` `[?] Help` |
| Assessing | Yes | `[Enter] Send` `[Ctrl+F] Finalize` `[?] Help` |
| Finalized | Yes | `[r] Run` `[e] Edit` `[?] Help` |

### 9. Basic Markdown Rendering

**File:** `crates/ralf-tui/src/widgets/markdown.rs`

AI responses are markdown. Render basic elements for readability:

| Element | Rendering |
|---------|-----------|
| `# Header` | Bold + primary color |
| `## Header` | Bold + secondary color |
| `### Header` | Bold |
| `**bold**` | Bold style |
| `*italic*` | Italic style (or dim) |
| `` `code` `` | Surface background color |
| ```` ``` ```` code blocks | Surface background, monospace |
| `- list item` | Proper indentation with bullet |
| `1. numbered` | Proper indentation with number |
| `> quote` | Muted color, indented |
| `[text](url)` | Underlined, show text only |
| `- [ ] checkbox` | Render as `☐` or `[ ]` |
| `- [x] checkbox` | Render as `☑` or `[x]` |

**Implementation approach:**

Use `pulldown-cmark` for parsing (already handles edge cases), then convert events to styled `ratatui::text::Line` spans:

```rust
use pulldown_cmark::{Parser, Event, Tag};
use ratatui::text::{Line, Span};

pub struct MarkdownRenderer<'a> {
    theme: &'a Theme,
}

impl<'a> MarkdownRenderer<'a> {
    pub fn new(theme: &'a Theme) -> Self;

    /// Render markdown text to styled lines.
    pub fn render(&self, markdown: &str) -> Vec<Line<'static>>;
}
```

**Fallback:** If `pulldown-cmark` adds too much complexity, implement a simple line-by-line regex-based renderer for the most common patterns (headers, bold, code blocks, lists).

## Non-Goals

- **Thread picker (Ctrl+T)**: Future milestone - for now, only one thread at a time
- **Model selection in chat**: Use default/available model for now
- **Streaming responses**: Show loading state, then full response (streaming is M5-C)
- **Full markdown rendering**: Complex elements like tables, images, HTML. Basic elements supported (see Deliverable 9).
- **Spec validation**: Beyond promise tag detection
- **Offline mode**: Requires available model to chat
- **Manual spec editing**: Direct editing of the generated spec (without AI). Users refine specs through chat. If manual editing is needed, add in polish phase.

## Acceptance Criteria

### Functional
- [ ] SpecEditor renders in context pane when phase is Drafting/Assessing/Finalized
- [ ] Can type message and send with Enter
- [ ] Shift+Enter inserts newline in input
- [ ] User messages appear in history immediately
- [ ] AI response appears after invocation completes
- [ ] Loading indicator shown during AI invocation
- [ ] Error message shown if invocation fails
- [ ] Spec preview extracted from AI responses
- [ ] Phase transitions: Drafting → Assessing → Finalized
- [ ] Status bar shows current phase
- [ ] Footer hints update based on phase
- [ ] Thread created on first message
- [ ] Thread saved after AI responses
- [ ] Can finalize spec with `<promise>` tag
- [ ] Finalized phase shows "Ready to run" state

### Navigation
- [ ] Up/Down arrows navigate input history
- [ ] Message history scrolls when exceeds viewport
- [ ] j/k scroll message history when input not focused
- [ ] Esc clears input or blurs focus

### Edge Cases
- [ ] Empty input: Enter does nothing
- [ ] Very long messages: Word wrap in display
- [ ] No available models: Show error, disable send
- [ ] AI timeout: Show timeout error, allow retry
- [ ] Rate limited: Show rate limit message

### Markdown Rendering
- [ ] Headers (`#`, `##`, `###`) render with appropriate styling
- [ ] Bold and italic text render with correct styles
- [ ] Inline code renders with background color
- [ ] Code blocks render with background color and preserve formatting
- [ ] Lists (bulleted and numbered) render with proper indentation
- [ ] Checkboxes render as `☐`/`☑` or `[ ]`/`[x]`
- [ ] Blockquotes render indented and muted
- [ ] Links show text without raw URL clutter

### Tests
- [ ] Unit tests for `ChatState` transitions
- [ ] Unit tests for spec preview extraction
- [ ] Unit tests for phase determination
- [ ] Unit tests for `MarkdownRenderer` (headers, code, lists)
- [ ] Snapshot test for SpecEditor layout
- [ ] Snapshot test for markdown rendering
- [ ] Integration test for send/receive flow (mocked)

## Technical Notes

### Async Pattern

The TUI uses a synchronous event loop with `crossterm`. For async chat invocations:

1. Spawn tokio task for `invoke_chat`
2. Send result via `mpsc::unbounded_channel`
3. Poll channel in event loop (non-blocking)
4. Update state when response received

```rust
// In event loop
if let Ok(event) = self.chat_rx.try_recv() {
    match event {
        ChatEvent::Response(result) => {
            self.chat_state.receive_response(result);
        }
        ChatEvent::Error(e) => {
            self.chat_state.receive_error(e);
        }
    }
}
```

### Model Selection

For this milestone, use the first available (ready) model:

```rust
fn select_chat_model(models: &[ModelStatus]) -> Option<&ModelStatus> {
    models.iter().find(|m| m.is_ready())
}
```

**ModelStatus → ModelConfig bridging:**

`ModelStatus` (TUI) tracks availability. `ModelConfig` (engine) contains command arguments for invocation. Bridge them via the engine's config:

```rust
// Get ModelConfig from engine config by model name
let config = ralf_engine::Config::load()?;
let model_config = config.models.iter()
    .find(|m| m.name == model_status.name)
    .cloned()
    .unwrap_or_else(|| ModelConfig::default_for(&model_status.name));
```

Future: Model preference in config, Ctrl+M to switch.

### Thread Persistence

Threads are saved to `.ralf/threads/{id}.jsonl` using the engine's `Thread::save()`. This happens:
- After each AI response
- Before switching threads (future)
- On graceful exit

### Layout

```
┌─────────────────────────────────────────────────────┐
│ Message History (flex, min 3 lines)                 │
│ - Scrollable                                        │
│ - Shows all ChatMessages                            │
│                                                     │
├─────────────────────────────────────────────────────┤
│ Input Area (fixed, 3-5 lines)                       │
│ - TextInput widget                                  │
│ - Hint line below                                   │
└─────────────────────────────────────────────────────┘
```

## Dependencies

- M5-B.2 (Phase Router) - for `ContextView::SpecEditor` routing
- Engine chat module - `invoke_chat`, `Thread`, `ChatContext`
- Existing `TextInputState` - input handling
- `pulldown-cmark` crate - markdown parsing (add to ralf-tui Cargo.toml)

## Risks

1. **Async complexity**: First async operation in TUI. Keep pattern simple.
2. **State sync**: Thread state must stay consistent with engine Thread.
3. **Large responses**: AI might produce very long specs. Need scroll handling.

## Open Questions

1. **Input height**: Fixed 3 lines or auto-expand? Recommendation: Fixed 3, scroll internally.
2. **Spec preview display**: Inline vs split? Start inline, add split if needed.
3. **Auto-save frequency**: After each AI response sufficient? Yes for now.
