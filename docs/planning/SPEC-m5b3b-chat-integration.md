# SPEC-m5b3b: Chat Integration

## Promise

Wire the conversation input to the chat system, enabling real AI conversations. User messages and AI responses appear in the timeline as SpecEvents, and the Thread persists across sessions.

After this milestone:
1. Users can send messages and receive AI responses
2. Messages appear in the timeline as SpecEvents
3. A Thread is created on first message
4. The Thread persists to disk (`.ralf/threads/`)
5. The draft feedback loop works (AI sees its previous spec suggestions)

## Background

This milestone connects the input area (M5-B.3a) to the engine's chat system:
- `invoke_chat(model, context, timeout)` - async chat invocation
- `ChatContext` - conversation history + current draft
- `Thread` - persistence (from `ralf_engine::chat`)
- `extract_spec_from_response()` - extract spec from AI response

**Architectural note:** With the Conversation + Artifact split:
- Chat messages go into the **timeline** (left pane) as `SpecEvent` entries
- The **artifact** pane (right) will show the extracted spec (M5-B.3c)
- This milestone focuses on timeline events, not artifact rendering

## Deliverables

### 1. ChatState Management

**File:** `crates/ralf-tui/src/chat/state.rs`

Manages the active chat session state:

```rust
use ralf_engine::chat::{Thread, ChatMessage, ChatContext, ChatResult, extract_spec_from_response};
use ralf_engine::thread::PhaseKind;

/// State for an active chat session.
pub struct ChatState {
    /// Thread being edited (chat persistence).
    pub thread: Thread,
    /// Extracted spec preview from latest AI response.
    pub spec_preview: Option<String>,
    /// Whether AI is currently responding.
    pub loading: bool,
    /// Error from last invocation, if any.
    pub error: Option<String>,
}

impl ChatState {
    /// Create a new chat state (creates a new Thread).
    pub fn new() -> Self {
        Self {
            thread: Thread::new(),
            spec_preview: None,
            loading: false,
            error: None,
        }
    }

    /// Create from an existing thread (for resume).
    pub fn from_thread(thread: Thread) -> Self {
        // Extract spec preview from last assistant message
        let spec_preview = thread.messages.iter()
            .rev()
            .find(|m| m.role == Role::Assistant)
            .and_then(|m| extract_spec_from_response(&m.content));

        Self {
            thread,
            spec_preview,
            loading: false,
            error: None,
        }
    }

    /// Prepare to send a user message.
    /// Returns the message content if non-empty.
    pub fn prepare_send(&mut self, content: String) -> Option<String> {
        let content = content.trim().to_string();
        if content.is_empty() {
            return None;
        }
        Some(content)
    }

    /// Record that a user message was sent.
    pub fn record_user_message(&mut self, content: &str) {
        self.thread.add_message(ChatMessage::user(content));
        self.loading = true;
        self.error = None;
    }

    /// Handle AI response.
    ///
    /// This method:
    /// 1. Adds assistant message to thread
    /// 2. Extracts spec from response
    /// 3. Updates thread.draft (critical for feedback loop!)
    /// 4. Updates spec_preview for display
    pub fn receive_response(&mut self, result: ChatResult) {
        self.loading = false;

        // Add assistant message
        self.thread.add_message(ChatMessage::assistant(&result.content, &result.model));

        // Extract and update spec
        if let Some(spec) = extract_spec_from_response(&result.content) {
            self.thread.draft = spec.clone();
            self.spec_preview = Some(spec);
        }
    }

    /// Handle AI error.
    pub fn receive_error(&mut self, error: String) {
        self.loading = false;
        self.error = Some(error);
    }

    /// Build chat context for model invocation.
    pub fn build_context(&self) -> ChatContext {
        self.thread.to_context()
    }

    /// Check if spec is finalized (has promise tag).
    pub fn is_finalized(&self) -> bool {
        ralf_engine::chat::draft_has_promise(&self.thread.draft)
    }

    /// Determine current phase based on chat state.
    pub fn current_phase(&self) -> PhaseKind {
        if self.is_finalized() {
            PhaseKind::Finalized
        } else if self.spec_preview.is_some() {
            PhaseKind::Assessing
        } else {
            PhaseKind::Drafting
        }
    }

    /// Get messages for display.
    pub fn messages(&self) -> &[ChatMessage] {
        &self.thread.messages
    }

    /// Save thread to disk.
    pub fn save(&self, spec_dir: &Path) -> Result<(), ChatError> {
        self.thread.save(spec_dir)
    }
}
```

### 2. Async Chat Handler

**File:** `crates/ralf-tui/src/chat/handler.rs`

Handle async chat invocation from the synchronous TUI event loop:

```rust
use tokio::sync::mpsc;
use ralf_engine::chat::{invoke_chat, ChatContext, ChatResult};
use ralf_engine::config::ModelConfig;

/// Events from async chat operations.
pub enum ChatEvent {
    /// AI response received.
    Response(ChatResult),
    /// AI invocation failed.
    Error(String),
}

/// Spawn a chat invocation task.
///
/// The result will be sent via the provided channel.
pub fn spawn_chat_invocation(
    model: ModelConfig,
    context: ChatContext,
    timeout_secs: u64,
    tx: mpsc::UnboundedSender<ChatEvent>,
) {
    tokio::spawn(async move {
        match invoke_chat(&model, &context, timeout_secs).await {
            Ok(result) => {
                let _ = tx.send(ChatEvent::Response(result));
            }
            Err(e) => {
                let _ = tx.send(ChatEvent::Error(e.to_string()));
            }
        }
    });
}
```

### 3. SpecEvent Timeline Integration

**File:** Update `crates/ralf-tui/src/timeline/event.rs`

Extend SpecEvent to carry chat messages:

```rust
/// Events during spec creation phase.
#[derive(Debug, Clone)]
pub struct SpecEvent {
    /// Message role.
    pub role: SpecEventRole,
    /// Message content.
    pub content: String,
    /// Model name (for assistant messages).
    pub model: Option<String>,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecEventRole {
    /// User message.
    User,
    /// AI assistant response.
    Assistant,
    /// System notification.
    System,
}

impl SpecEvent {
    /// Create from a chat message.
    pub fn from_chat_message(msg: &ChatMessage) -> Self {
        Self {
            role: match msg.role {
                Role::User => SpecEventRole::User,
                Role::Assistant => SpecEventRole::Assistant,
                Role::System => SpecEventRole::System,
            },
            content: msg.content.clone(),
            model: msg.model.clone(),
            timestamp: msg.timestamp,
        }
    }
}
```

### 4. ShellApp Integration

**File:** Update `crates/ralf-tui/src/shell.rs`

Add chat state and event handling:

```rust
use tokio::sync::mpsc;
use crate::chat::{ChatState, ChatEvent, spawn_chat_invocation};

pub struct ShellApp {
    // ... existing fields ...

    /// Active chat state (when in spec editing phases).
    pub chat_state: Option<ChatState>,

    /// Channel for receiving chat events.
    chat_rx: Option<mpsc::UnboundedReceiver<ChatEvent>>,

    /// Tokio runtime handle for spawning tasks.
    runtime: tokio::runtime::Handle,
}

impl ShellApp {
    /// Start a new chat session.
    pub fn start_new_thread(&mut self) {
        self.chat_state = Some(ChatState::new());
    }

    /// Submit input as a chat message.
    pub fn submit_chat_message(&mut self) {
        let content = self.input.submit();

        // Create thread if needed
        if self.chat_state.is_none() {
            self.start_new_thread();
        }

        let chat = self.chat_state.as_mut().unwrap();

        // Prepare and validate message
        let Some(content) = chat.prepare_send(content) else {
            return;
        };

        // Record user message
        chat.record_user_message(&content);

        // Add to timeline
        self.timeline.add_event(EventKind::Spec(SpecEvent {
            role: SpecEventRole::User,
            content: content.clone(),
            model: None,
            timestamp: Utc::now(),
        }));

        // Find available model
        let Some(model_config) = self.get_chat_model() else {
            chat.receive_error("No model available".into());
            return;
        };

        // Create channel for response
        let (tx, rx) = mpsc::unbounded_channel();
        self.chat_rx = Some(rx);

        // Spawn chat invocation
        let context = chat.build_context();
        spawn_chat_invocation(model_config, context, 120, tx);
    }

    /// Poll for chat events (called in event loop).
    pub fn poll_chat_events(&mut self) {
        let Some(rx) = self.chat_rx.as_mut() else {
            return;
        };

        // Non-blocking receive
        while let Ok(event) = rx.try_recv() {
            match event {
                ChatEvent::Response(result) => {
                    if let Some(chat) = self.chat_state.as_mut() {
                        // Add to timeline
                        self.timeline.add_event(EventKind::Spec(SpecEvent {
                            role: SpecEventRole::Assistant,
                            content: result.content.clone(),
                            model: Some(result.model.clone()),
                            timestamp: Utc::now(),
                        }));

                        // Update chat state
                        chat.receive_response(result);

                        // Update thread display for status bar
                        self.update_thread_display();

                        // Save thread
                        if let Err(e) = chat.save(&self.spec_dir()) {
                            self.show_toast(format!("Save failed: {e}"));
                        }
                    }
                }
                ChatEvent::Error(e) => {
                    if let Some(chat) = self.chat_state.as_mut() {
                        chat.receive_error(e.clone());
                    }
                    self.timeline.add_event(EventKind::System(SystemEvent {
                        message: format!("Chat error: {e}"),
                    }));
                }
            }
        }
    }

    /// Get a model config for chat.
    fn get_chat_model(&self) -> Option<ModelConfig> {
        // Find first ready model
        let ready_model = self.models.iter().find(|m| m.is_ready())?;

        // Load config to get ModelConfig
        let config = ralf_engine::Config::load().ok()?;
        config.models.iter()
            .find(|m| m.name == ready_model.name)
            .cloned()
    }

    /// Update ThreadDisplay from chat state.
    fn update_thread_display(&mut self) {
        if let Some(chat) = &self.chat_state {
            let phase = chat.current_phase();
            self.current_thread = Some(ThreadDisplay {
                id: chat.thread.id.clone(),
                title: chat.thread.title.clone(),
                phase_kind: phase,
                phase_display: format!("{:?}", phase),
                iteration: None,
                max_iterations: 5,
                failure_reason: None,
            });
        }
    }
}
```

### 5. Event Loop Integration

**File:** Update `crates/ralf-tui/src/shell.rs` (run method)

Poll for chat events in the main loop:

```rust
impl ShellApp {
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            // Poll for chat events
            self.poll_chat_events();

            // Render
            terminal.draw(|frame| render_shell(frame, self))?;

            // Handle input events with timeout
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if self.handle_key(key) {
                        // Event handled
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }
}
```

### 6. Loading Indicator

Show loading state in the timeline while waiting for AI:

```rust
impl TimelineState {
    /// Add a loading indicator event.
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }
}

// In TimelineWidget render:
if self.state.loading {
    // Show loading indicator at bottom
    let loading_line = Line::from(vec![
        Span::styled("● ", Style::default().fg(self.theme.info)),
        Span::styled("Waiting for AI...", Style::default().fg(self.theme.subtext)),
    ]);
    // Render at appropriate position
}
```

### 7. Model Selection Bridge

**File:** `crates/ralf-tui/src/chat/model_bridge.rs`

Bridge between TUI `ModelStatus` and engine `ModelConfig`:

```rust
use crate::models::ModelStatus;
use ralf_engine::config::ModelConfig;

/// Get ModelConfig for a ready model.
pub fn get_model_config(model_status: &ModelStatus) -> Option<ModelConfig> {
    if !model_status.is_ready() {
        return None;
    }

    // Load engine config
    let config = ralf_engine::Config::load().ok()?;

    // Find matching model config
    config.models.iter()
        .find(|m| m.name == model_status.name)
        .cloned()
}
```

## Non-Goals

- **Spec preview rendering**: The artifact pane (M5-B.3c) handles spec display
- **Markdown rendering**: M5-B.3c handles formatting
- **Streaming responses**: Future enhancement
- **Thread picker**: Loading existing threads is future work
- **Model selection UI**: Use first available model for now

## Acceptance Criteria

### Chat Flow
- [ ] Typing message and pressing Enter sends to AI
- [ ] User message appears in timeline immediately
- [ ] Loading indicator shows while waiting for AI
- [ ] AI response appears in timeline when received
- [ ] Error message shows if AI invocation fails

### Thread Management
- [ ] New Thread created on first message
- [ ] Thread title set from first user message (first 50 chars)
- [ ] Thread saved to `.ralf/threads/{id}.jsonl` after AI response
- [ ] Thread contains all messages

### Draft Feedback Loop
- [ ] `extract_spec_from_response()` called on AI responses
- [ ] Extracted spec stored in `thread.draft`
- [ ] `ChatContext.build_prompt()` includes draft in next invocation
- [ ] AI sees previous spec suggestions in conversation

### Phase Transitions
- [ ] Starts in Drafting phase
- [ ] Transitions to Assessing when spec_preview is set
- [ ] Transitions to Finalized when draft has promise tag
- [ ] Status bar reflects phase changes

### Timeline Display
- [ ] User messages show with "You:" prefix
- [ ] Assistant messages show with model name prefix
- [ ] System messages styled differently (muted)
- [ ] Timeline auto-scrolls to show new messages

### Error Handling
- [ ] Timeout shows timeout error
- [ ] Network errors show appropriate message
- [ ] No model available shows error
- [ ] Errors don't crash the app

### Tests
- [ ] Unit tests for `ChatState` transitions
- [ ] Unit tests for `prepare_send` validation
- [ ] Unit tests for `receive_response` spec extraction
- [ ] Unit tests for phase determination
- [ ] Integration test for send/receive flow (mocked model)
- [ ] Test thread persistence round-trip

## Technical Notes

### Async Pattern

The TUI uses a synchronous crossterm event loop. We integrate async chat via:

1. Spawn tokio task for `invoke_chat`
2. Send result through `mpsc::unbounded_channel`
3. Poll channel in event loop (non-blocking `try_recv`)
4. Update state when message received

```
User presses Enter
    │
    ├── input.submit() → content
    ├── chat.record_user_message(content)
    ├── timeline.add_event(SpecEvent::User)
    ├── spawn_chat_invocation(model, context, tx)
    └── chat.loading = true

Event loop iteration
    │
    └── poll_chat_events()
        │
        └── rx.try_recv()
            │
            ├── Response(result)
            │   ├── timeline.add_event(SpecEvent::Assistant)
            │   ├── chat.receive_response(result)
            │   └── chat.save()
            │
            └── Error(e)
                └── chat.receive_error(e)
```

### Tokio Runtime

The TUI needs a tokio runtime for async operations. Options:

1. **Runtime in ShellApp**: Create runtime at startup, store handle
2. **Global runtime**: Use `tokio::runtime::Runtime::new()`
3. **Main is async**: Make `main()` async with `#[tokio::main]`

Recommendation: Option 1 - create runtime in `ShellApp::new()`, avoids global state.

### Thread Persistence Path

Threads save to `.ralf/threads/{uuid}.jsonl`:
```
.ralf/
└── threads/
    ├── abc123-def456.jsonl
    └── xyz789-uvw012.jsonl
```

The `.ralf` directory is created in the current working directory (project root).

## Dependencies

- M5-B.3a (Timeline Input) - input handling, focus management
- `ralf_engine::chat` - `invoke_chat`, `Thread`, `ChatContext`, `ChatMessage`
- `ralf_engine::config` - `ModelConfig`
- `tokio` runtime (already a dependency)

## Risks

1. **Async complexity**: First async integration in TUI - keep pattern simple
2. **State sync**: Thread state must stay consistent between TUI and engine
3. **Model unavailable**: Handle gracefully when no models are ready
4. **Large responses**: AI might produce very long responses - timeline should handle

## Open Questions

1. **Chat timeout**: 120 seconds default - is this appropriate? *Recommendation: Yes, models can be slow*
2. **Auto-save frequency**: After each AI response? *Recommendation: Yes, prevents data loss*
3. **Error recovery**: Retry on timeout? *Recommendation: No auto-retry, let user decide*
