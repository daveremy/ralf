# Testing Strategy

This document outlines the testing approach for ralf, with particular focus on TUI testing best practices.

## Test Pyramid

```
         /\
        /  \  E2E (PTY)
       /----\
      /      \  Integration
     /--------\
    /          \  Unit Tests
   /--------------\
```

### Unit Tests
- Test individual functions and methods in isolation
- Fast, deterministic, no external dependencies
- Located in `mod tests {}` blocks within each module

### Integration Tests
- Test interactions between components
- Event sequences (type → enter → state change)
- Async flows with `#[tokio::test]`
- Located in dedicated test sections within modules

### E2E Tests
- Test the full binary with PTY (pseudo-terminal)
- Verify actual terminal rendering and input handling
- Located in `pty_e2e_tests` module

## TUI-Specific Testing

### State-Based Testing
Test that actions produce expected state changes:

```rust
#[test]
fn test_escape_clears_input() {
    let mut app = ShellApp::new();
    app.input.insert('x');

    app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert!(app.input.is_empty());
}
```

### Event Sequence Testing
Test sequences of user interactions:

```rust
#[tokio::test]
async fn test_integration_type_and_submit() {
    let mut app = ShellApp::new();
    app.models[0].state = ModelState::Ready;

    // Type "hello" character by character
    for c in "hello".chars() {
        app.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }

    // Press Enter to submit
    app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    // Verify results
    assert!(app.input.is_empty());
    assert!(app.chat_loading);
}
```

### Async Runtime Testing
When code uses `tokio::spawn`, tests MUST use `#[tokio::test]`:

```rust
// BAD: Will pass but doesn't actually test the spawn
#[test]
fn test_chat_blocks_when_loading() {
    let mut app = ShellApp::new();
    app.chat_loading = true;
    app.send_chat_message("test"); // Returns early, never spawns
}

// GOOD: Actually exercises the tokio::spawn path
#[tokio::test]
async fn test_send_chat_spawns_task() {
    let mut app = ShellApp::new();
    app.models[0].state = ModelState::Ready;

    app.send_chat_message("test"); // Actually spawns async task

    assert!(app.chat_loading);
    assert!(app.chat_rx.is_some());
}
```

### Snapshot Testing
Visual regression testing for rendered output:

```rust
#[test]
fn test_snapshot_shell_split_mode() {
    let result = render_shell_to_string(
        ScreenMode::Split,
        FocusedPane::Timeline,
        80, 24,
    );
    assert_snapshot!("shell_split_mode", result);
}
```

Update snapshots with: `cargo insta review`

## Test Categories

### Required for New Features
1. **Unit tests** for new functions/methods
2. **Integration tests** for user-facing flows
3. **Async tests** (`#[tokio::test]`) if using spawn/channels
4. **Snapshot tests** if changing visual output

### When to Add E2E Tests
- Critical user flows (startup, quit, basic input)
- Cross-cutting concerns (terminal size, mouse events)
- Bugs that escaped other test layers

## Running Tests

```bash
# All tests
cargo test

# Single crate
cargo test -p ralf-tui

# Specific test
cargo test -p ralf-tui test_integration_type_and_submit

# With output
cargo test -p ralf-tui -- --nocapture

# Update snapshots
cargo insta review
```

## Terminal Restoration

TUI applications modify terminal state (raw mode, alternate screen). If the app crashes, the terminal must be restored to prevent the user from seeing garbled output.

### Panic Hook

We install a panic hook that restores the terminal before printing the panic message:

```rust
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), DisableMouseCapture, LeaveAlternateScreen, ShowCursor);
}

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));
}
```

This is called at the start of `run_shell_tui()` and `run_tui()`.

### RAII Guard

For normal exits, we use an RAII guard that calls `restore_terminal()` on drop:

```rust
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}
```

## Common Pitfalls

### 1. Missing Tokio Runtime
```rust
// This will PANIC at runtime if no tokio runtime exists
tokio::spawn(async { ... });

// Solution: Ensure run_shell_tui creates a runtime
let rt = tokio::runtime::Runtime::new()?;
let _guard = rt.enter();
```

### 2. Testing Early-Return Paths Only
```rust
// Only tests the "no model" path, not the actual spawn
#[test]
fn test_send_chat() {
    let mut app = ShellApp::new(); // All models probing
    app.send_chat_message("test"); // Returns early!
}

// Better: Test all paths including the spawn
#[tokio::test]
async fn test_send_chat_with_ready_model() {
    let mut app = ShellApp::new();
    app.models[0].state = ModelState::Ready; // Now it will spawn
    app.send_chat_message("test");
}
```

### 3. Not Testing State Preservation
```rust
// Test that unrelated actions don't break state
#[tokio::test]
async fn test_focus_cycle_during_chat() {
    let mut app = ShellApp::new();
    app.send_chat_message("test");

    // Focus change shouldn't affect chat
    app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

    assert!(app.chat_loading); // Still loading!
}
```

## Test Coverage Goals

| Component | Target | Current |
|-----------|--------|---------|
| shell.rs | 80%+ | ~75% |
| models.rs | 90%+ | ~90% |
| timeline/ | 80%+ | ~80% |
| widgets/ | 70%+ | ~70% |

## Adding New Tests Checklist

- [ ] Unit test for the new function/method
- [ ] Integration test if it's a user-facing feature
- [ ] `#[tokio::test]` if using async/spawn
- [ ] Snapshot test if changing rendering
- [ ] Error path tests (not just happy path)
- [ ] State preservation tests (actions don't break unrelated state)
