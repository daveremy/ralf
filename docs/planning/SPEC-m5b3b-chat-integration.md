# SPEC-m5b3b: Chat Integration

## Promise

Wire the TUI input to the engine's chat system. Users can send messages, receive AI responses in the timeline, and threads persist across sessions.

**After this milestone:**
1. Type message → press Enter → see AI response in timeline
2. Thread created on first message, persists to `.ralf/threads/`
3. Model status updates based on chat outcomes (ready/rate-limited/error)

---

## Existing Engine APIs

These already exist in `ralf-engine` - we integrate with them, not rebuild them:

| API | Location | Purpose |
|-----|----------|---------|
| `invoke_chat(model, context, timeout)` | `chat.rs:227` | Async CLI invocation |
| `ChatContext::build_prompt()` | `chat.rs:95` | Builds prompt with system message + history |
| `SPEC_STUDIO_SYSTEM_PROMPT` | `chat.rs:156` | System prompt for Drafting phase |
| `Thread` | `chat.rs:300` | Persistence with `save()`/`load()` |
| `extract_spec_from_response()` | `chat.rs:493` | Extract spec from AI output |
| `draft_has_promise()` | `chat.rs:477` | Check for `<promise>` tag |
| `ModelConfig::default_for()` | `config.rs:199` | CLI commands with bypass flags |

**CLI invocation is handled** - includes `--dangerously-skip-permissions` (claude), `--dangerously-bypass-approvals-and-sandbox` (codex).

---

## Deliverables

### 1. Wire Input to Chat

**File:** `crates/ralf-tui/src/shell.rs`

Update `submit_input()` to invoke chat when not a slash command:

```rust
fn submit_input(&mut self) -> Option<ShellAction> {
    let content = self.input.submit();

    // Handle slash commands (existing)
    if content.starts_with('/') {
        return self.execute_command(&content);
    }

    // Handle chat message (new)
    if !content.trim().is_empty() {
        self.send_chat_message(content);
    }
    None
}

fn send_chat_message(&mut self, content: String) {
    // Block input while waiting for response
    if self.chat_loading {
        self.show_toast("Waiting for response...");
        return;
    }

    // Create thread if needed
    if self.thread.is_none() {
        self.thread = Some(Thread::new());
    }

    // Add user message to timeline
    self.timeline.push(EventKind::Spec(SpecEvent::user(&content)));

    // Add to thread
    let thread = self.thread.as_mut().unwrap();
    thread.add_message(ChatMessage::user(&content));

    // Get model config
    let Some(model_config) = self.get_available_model() else {
        self.show_toast("No model available");
        return;
    };

    // Spawn async chat
    let context = thread.to_context();
    let (tx, rx) = mpsc::unbounded_channel();
    self.chat_rx = Some(rx);
    self.chat_loading = true;

    let model = model_config.clone();
    tokio::spawn(async move {
        let result = invoke_chat(&model, &context, model.timeout_seconds).await;
        let _ = tx.send(result);
    });
}
```

### 2. Model Selection Bridge

**File:** `crates/ralf-tui/src/shell.rs`

Map TUI `ModelStatus` to engine `ModelConfig`:

```rust
fn get_available_model(&self) -> Option<ModelConfig> {
    // Find first ready model from probed status
    let ready = self.models.iter().find(|m| m.is_ready())?;

    // Get config (has CLI command + flags)
    Some(ModelConfig::default_for(&ready.name))
}
```

### 3. Async Response Handling

**File:** `crates/ralf-tui/src/shell.rs`

Poll for chat results in event loop:

```rust
// In run() loop, before event polling:
self.poll_chat_response();

fn poll_chat_response(&mut self) {
    let Some(rx) = self.chat_rx.as_mut() else { return };

    match rx.try_recv() {
        Ok(Ok(result)) => {
            self.chat_loading = false;

            // Add AI response to timeline
            self.timeline.push(EventKind::Spec(SpecEvent::assistant(
                &result.content,
                &result.model,
            )));

            // Update thread
            if let Some(thread) = self.thread.as_mut() {
                thread.add_message(ChatMessage::assistant(&result.content, &result.model));

                // Extract and store draft
                if let Some(spec) = extract_spec_from_response(&result.content) {
                    thread.draft = spec;
                }

                // Save thread
                let _ = thread.save(&self.ralf_dir());
            }

            // Update model status to Ready
            self.update_model_status(&result.model, Ok(()));

            // Update status bar
            self.update_thread_display();
        }
        Ok(Err(e)) => {
            self.chat_loading = false;
            self.timeline.push(EventKind::System(SystemEvent::error(&e.to_string())));

            // Update model status based on error
            if let Some(model_name) = self.last_chat_model.as_ref() {
                self.update_model_status(model_name, Err(&e));
            }
        }
        Err(TryRecvError::Empty) => {} // Still waiting
        Err(TryRecvError::Disconnected) => {
            self.chat_rx = None;
            self.chat_loading = false;
        }
    }
}
```

### 4. Model Status Updates

**File:** `crates/ralf-tui/src/models.rs`

Update status based on chat outcomes:

```rust
impl ModelStatus {
    pub fn update_from_result(&mut self, result: Result<(), &RunnerError>) {
        match result {
            Ok(()) => {
                self.state = ModelState::Ready;
                self.message = None;
            }
            Err(RunnerError::Timeout(_)) => {
                self.state = ModelState::Unavailable;
                self.message = Some("Timeout".into());
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("429") || msg.contains("rate limit") {
                    self.state = ModelState::Cooldown(900); // 15 min default
                    self.message = Some("Rate limited".into());
                } else if msg.contains("401") || msg.contains("auth") {
                    self.state = ModelState::Unavailable;
                    self.message = Some("Auth required".into());
                } else {
                    self.state = ModelState::Unavailable;
                    self.message = Some(msg);
                }
            }
        }
    }
}
```

### 5. Status Cache

**File:** `crates/ralf-tui/src/models.rs`

Cache model status to skip probing on startup:

```rust
pub fn save_cache(models: &[ModelStatus], ralf_dir: &Path) -> io::Result<()> {
    let path = ralf_dir.join("models.json");
    let json = serde_json::to_string_pretty(models)?;
    fs::write(path, json)
}

pub fn load_cache(ralf_dir: &Path) -> io::Result<Vec<ModelStatus>> {
    let path = ralf_dir.join("models.json");
    let json = fs::read_to_string(path)?;
    serde_json::from_str(&json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
```

### 6. Loading Indicator

Show loading state in timeline while waiting:

```rust
// In ShellApp
pub chat_loading: bool,

// In timeline render, if chat_loading:
// Show "● Waiting for {model}..." at bottom
```

### 7. SpecEvent Updates

**File:** `crates/ralf-tui/src/timeline/event.rs`

Add constructors for chat messages:

```rust
impl SpecEvent {
    pub fn user(content: &str) -> Self {
        Self { content: content.into(), is_user: true }
    }

    pub fn assistant(content: &str, model: &str) -> Self {
        Self {
            content: format!("[{}] {}", model, content),
            is_user: false
        }
    }
}
```

---

## New ShellApp Fields

```rust
pub struct ShellApp {
    // ... existing fields ...

    /// Active thread (None until first message)
    pub thread: Option<Thread>,

    /// Channel for receiving chat results
    chat_rx: Option<mpsc::UnboundedReceiver<Result<ChatResult, RunnerError>>>,

    /// Whether waiting for AI response
    pub chat_loading: bool,

    /// Last model used (for error attribution)
    last_chat_model: Option<String>,
}
```

---

## Acceptance Criteria

### Chat Flow
- [ ] Type message, press Enter, see AI response in timeline
- [ ] User message appears immediately (before AI responds)
- [ ] Loading indicator shows while waiting
- [ ] Input blocked while waiting (shows toast if user tries to send)
- [ ] AI response shows with model name prefix
- [ ] Error messages appear as system events

### Thread Persistence
- [ ] Thread created on first message
- [ ] Thread title set from first user message
- [ ] Thread saved to `.ralf/threads/{id}.jsonl` after AI response
- [ ] Draft extracted and stored in thread

### Model Status
- [ ] Status updates to Ready on successful chat
- [ ] Status updates to Cooldown on rate limit
- [ ] Status updates to Unavailable on auth/timeout error
- [ ] Status cached to `.ralf/models.json`
- [ ] Cached status loaded on startup

### Status Bar
- [ ] Shows thread title when thread exists
- [ ] Shows phase (Drafting → Assessing when draft extracted)
- [ ] Shows Finalized when draft has `<promise>` tag

---

## Non-Goals

- **Spec preview in artifact pane** - M5-B.3c
- **Streaming responses** - Future enhancement
- **Thread picker UI** - M5-B.5
- **Model selection UI** - Use first available for now
- **Cancellation** - Future enhancement

---

## Testing

### Unit Tests
- `SpecEvent::user()` / `SpecEvent::assistant()` constructors
- `ModelStatus::update_from_result()` state transitions
- Status cache save/load round-trip

### Integration Tests
- Mock `invoke_chat` to test full flow
- Verify timeline events in correct order
- Verify thread persistence

### Manual Testing
- Real chat with each CLI (claude, codex, gemini)
- Rate limit handling (if triggerable)
- Network error handling
