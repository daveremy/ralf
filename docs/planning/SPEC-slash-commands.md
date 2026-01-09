# SPEC: Slash Command Infrastructure (M5-B.3a')

## Promise

Implement an input-first command system where all typing goes directly to the input area, and actions are invoked via slash commands (`/help`, `/quit`) or modifier keybindings (`Ctrl+1`, `Escape`). This eliminates the UX conflict where reserved keys blocked free typing.

## Background

The M5-B.3a implementation revealed a fundamental UX issue: when users see a cursor in the input area, they expect to type freely. But reserved keys like `q` (quit), `1/2/3` (screen modes), and `j/k` (navigation) blocked text entry.

Analysis (with Gemini review) led to adopting Claude Code's input-first model:
- All character keys go to input (no reserved keys)
- Actions via slash commands (discoverable, self-documenting)
- Fast keybindings as alternates for power users
- Escape cascade: clear input → cancel operation → quit

## Deliverables

### 1. Command Parser & Registry

```rust
// crates/ralf-tui/src/commands/mod.rs

/// A parsed command from user input
pub enum Command {
    // Global commands
    Help,
    Quit,
    Split,
    Focus,
    Canvas,
    Refresh,
    Clear,
    Search(Option<String>),  // /search [query] - future
    Model(Option<String>),   // /model [name]
    Copy,
    Editor,

    // Phase-specific (stubs - implemented in later milestones)
    Approve,
    Reject(Option<String>),  // /reject [feedback]
    Pause,
    Resume,
    Cancel,

    // Unknown command
    Unknown(String),
}

/// Parse a slash command from input text
pub fn parse_command(input: &str) -> Option<Command>;

/// Check if input starts with '/'
pub fn is_command(input: &str) -> bool;

/// Get command completions for autocomplete
pub fn get_completions(partial: &str, phase: Option<PhaseKind>) -> Vec<CommandInfo>;

/// Command metadata for help display
pub struct CommandInfo {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub keybinding: Option<&'static str>,
    pub phase_specific: bool,
}
```

### 2. Global Commands

| Command | Aliases | Keybinding | Description |
|---------|---------|------------|-------------|
| `/help` | `/?` | `F1` | Show available commands |
| `/quit` | `/q`, `/exit` | `Escape` (when empty) | Exit ralf |
| `/split` | `/1` | `Ctrl+1` | Split view mode |
| `/focus` | `/2` | `Ctrl+2` | Focus conversation mode |
| `/canvas` | `/3` | `Ctrl+3` | Focus canvas mode |
| `/refresh` | — | `Ctrl+R` | Refresh model status |
| `/clear` | — | `Ctrl+L` | Clear conversation |
| `/search` | `/find` | `Ctrl+F` | Search timeline (future) |
| `/model` | — | — | Switch active model (takes argument) |
| `/copy` | — | — | Copy last response to clipboard |
| `/editor` | — | — | Open in $EDITOR |

### 3. Keybinding Layer

Update `ShellApp::handle_key_event` to use modifier keys:

```rust
// Remove old reserved key logic
// All character keys go to input when conversation pane focused

match key.code {
    // Modifier keybindings (always work)
    KeyCode::F(1) => self.show_help(),
    KeyCode::Esc => self.escape_cascade(),
    KeyCode::Char('1') if key.modifiers.contains(KeyModifiers::CONTROL) => {
        self.set_screen_mode(ScreenMode::Split);
    }
    KeyCode::Char('2') if key.modifiers.contains(KeyModifiers::CONTROL) => {
        self.set_screen_mode(ScreenMode::TimelineFocus);
    }
    KeyCode::Char('3') if key.modifiers.contains(KeyModifiers::CONTROL) => {
        self.set_screen_mode(ScreenMode::ContextFocus);
    }
    KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
        self.refresh_models();
    }
    KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
        self.clear_conversation();
    }
    KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
        self.start_search();  // Future: timeline search
    }
    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
        self.graceful_pause();  // Trap SIGINT, don't kill process
    }

    // Tab for focus (no modifier needed - not a character)
    KeyCode::Tab => self.cycle_focus(),
    KeyCode::BackTab => self.cycle_focus_reverse(),

    // Page keys for timeline scroll
    KeyCode::PageUp => self.timeline.scroll_up(),
    KeyCode::PageDown => self.timeline.scroll_down(),

    // Alt+j/k for vim-style scroll
    KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::ALT) => {
        self.timeline.scroll_down();
    }
    KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::ALT) => {
        self.timeline.scroll_up();
    }

    // All other keys go to input when conversation focused
    _ if self.focused_pane == FocusedPane::Timeline => {
        self.handle_input_key(key);
    }
}
```

### 4. Escape Cascade

```rust
fn escape_cascade(&mut self) {
    if !self.input.is_empty() {
        // Step 1: Clear input
        self.input.clear();
    } else if self.has_active_operation() {
        // Step 2: Cancel operation
        self.cancel_operation();
    } else {
        // Step 3: Quit
        self.should_quit = true;
    }
}
```

### 4a. Slash Escaping

To send a message that starts with `/` (e.g., discussing `/etc/config`), use `//` at the start:

| Input | Result |
|-------|--------|
| `/help` | Executes help command |
| `//etc/config` | Sends message "/etc/config" |
| `hello /world` | Sends message "hello /world" (no escape needed) |

```rust
fn submit_input(&mut self) {
    let content = self.input.content().trim();

    if content.starts_with("//") {
        // Escaped slash - send as message with single /
        let message = format!("/{}", &content[2..]);
        self.send_message(&message);
    } else if is_command(content) {
        // Execute command
        self.execute_command(parse_command(content));
    } else {
        // Regular message
        self.send_message(content);
    }
    self.input.clear();
}
```

### 4b. Signal Handling (Ctrl+C)

`Ctrl+C` must be trapped to prevent SIGINT from killing the process without saving state:

```rust
// In main.rs or shell setup
fn setup_signal_handlers() {
    // Use ctrlc crate or tokio::signal
    ctrlc::set_handler(move || {
        // Send graceful pause signal to app
        // Don't call std::process::exit()
    }).expect("Error setting Ctrl-C handler");
}
```

**Behavior:**
- During active operation: Graceful pause (finish current step, save state)
- When idle: Same as `/quit` (prompt if unsaved changes)
- Never abruptly terminates without cleanup

### 5. Help Overlay

When `/help` is invoked (or `F1`), show a modal overlay:

```
┌─ Commands ──────────────────────────────────────────────────────┐
│                                                                 │
│ Global:                                                         │
│   /help, /?        Show this help                          [F1] │
│   /quit, /q, /exit Exit ralf                               [Esc]│
│   /split, /1       Split view mode                      [Ctrl+1]│
│   /focus, /2       Focus conversation                   [Ctrl+2]│
│   /canvas, /3      Focus canvas                         [Ctrl+3]│
│   /refresh         Refresh model status                 [Ctrl+R]│
│   /clear           Clear conversation                   [Ctrl+L]│
│   /model [name]    Switch active model                          │
│   /copy            Copy last response                           │
│   /editor          Open in $EDITOR                              │
│                                                                 │
│ Current phase (Drafting):                                       │
│   /finalize        Finalize the spec                            │
│   /assess          Request AI assessment                        │
│                                                                 │
│ Navigation:                                                     │
│   Tab              Switch panes                                 │
│   PageUp/Down      Scroll timeline                              │
│   Enter            Submit message                               │
│   Shift+Enter      Insert newline                               │
│                                                                 │
│                                              [Esc] Close        │
└─────────────────────────────────────────────────────────────────┘
```

### 6. Autocomplete Popup

When user types `/`, show autocomplete popup that filters as they type:

```
> /mo█
  ┌──────────────────────────┐
  │ /model [name]  Switch... │
  └──────────────────────────┘
```

**Behavior:**
- Popup appears immediately on `/`
- Filters as user types
- Up/Down to navigate options
- Tab or Enter to complete
- Escape to dismiss
- Shows only commands valid for current phase

### 7. Focus Trap Escape

Typing `/` from any pane should jump focus to the input and insert `/`:

```rust
// In handle_key_event, before pane-specific handling
if key.code == KeyCode::Char('/') && !key.modifiers.contains(KeyModifiers::CONTROL) {
    self.focused_pane = FocusedPane::Timeline;  // Jump to conversation
    self.input.insert('/');
    return;
}
```

### 8. Footer Hints Update

Update `hints_for_state` to show slash command style hints:

```rust
// Before:
// hints.push(KeyHint::new("q", "Quit"));
// hints.push(KeyHint::new("1", "Split"));

// After:
hints.push(KeyHint::new("Tab", "Switch"));
hints.push(KeyHint::new("Enter", "Send"));
hints.push(KeyHint::new("/", "Commands"));
hints.push(KeyHint::new("Esc", "Quit"));

// Phase-specific slash commands in hints
if phase == Some(PhaseKind::PendingReview) {
    hints.push(KeyHint::new("/approve", ""));
    hints.push(KeyHint::new("/reject", ""));
}
```

### 9. Command Execution

```rust
fn execute_command(&mut self, cmd: Command) -> CommandResult {
    match cmd {
        Command::Help => {
            self.show_help_overlay = true;
            CommandResult::Handled
        }
        Command::Quit => {
            self.should_quit = true;
            CommandResult::Handled
        }
        Command::Split => {
            self.screen_mode = ScreenMode::Split;
            CommandResult::Handled
        }
        Command::Focus => {
            self.screen_mode = ScreenMode::TimelineFocus;
            CommandResult::Handled
        }
        Command::Canvas => {
            self.screen_mode = ScreenMode::ContextFocus;
            CommandResult::Handled
        }
        Command::Refresh => {
            if self.can_refresh_models() {
                CommandResult::Action(ShellAction::RefreshModels)
            } else {
                CommandResult::Error("Cannot refresh while probing")
            }
        }
        Command::Clear => {
            self.timeline.clear();
            self.input.clear();
            CommandResult::Handled
        }
        Command::Model(name) => {
            // TODO: Implement in later milestone
            CommandResult::Error("Model switching not yet implemented")
        }
        Command::Copy => {
            // TODO: Implement clipboard copy
            CommandResult::Error("Copy not yet implemented")
        }
        Command::Editor => {
            // TODO: Implement $EDITOR integration
            CommandResult::Error("Editor not yet implemented")
        }
        // Phase-specific commands return NotAvailable if wrong phase
        Command::Approve | Command::Reject(_) => {
            if self.current_phase() == Some(PhaseKind::PendingReview) {
                CommandResult::Error("Phase commands not yet implemented")
            } else {
                CommandResult::NotAvailable("Only available in review phase")
            }
        }
        Command::Unknown(cmd) => {
            CommandResult::Error(format!("Unknown command: /{}", cmd))
        }
        _ => CommandResult::NotAvailable("Command not available in current phase")
    }
}
```

### 10. Input Submission Flow

Update input submission to check for commands:

```rust
fn submit_input(&mut self) {
    let content = self.input.content().trim();

    if content.is_empty() {
        return;
    }

    if is_command(content) {
        if let Some(cmd) = parse_command(content) {
            let result = self.execute_command(cmd);
            match result {
                CommandResult::Error(msg) => {
                    self.show_toast(&msg, ToastLevel::Error);
                }
                CommandResult::NotAvailable(msg) => {
                    self.show_toast(&msg, ToastLevel::Warning);
                }
                _ => {}
            }
        }
        self.input.clear();
    } else {
        // Regular message - send to chat (M5-B.3b)
        self.send_message(content);
        self.input.clear();
    }
}
```

## File Structure

```
crates/ralf-tui/src/
├── commands/
│   ├── mod.rs          # Command enum, parser, registry
│   ├── completions.rs  # Autocomplete logic
│   └── help.rs         # Help overlay widget
├── shell.rs            # Updated key handling, command execution
└── widgets/
    └── autocomplete.rs # Autocomplete popup widget
```

## Acceptance Criteria

1. **Input-first**: All character keys go to input when conversation pane is focused
2. **Slash commands work**: `/quit`, `/help`, `/split`, `/focus`, `/canvas`, `/refresh`, `/clear` execute correctly
3. **Keybindings work**: `Ctrl+1/2/3`, `F1`, `Ctrl+R`, `Ctrl+L`, `Ctrl+C`, `Escape` work as alternates
4. **Help overlay**: `/help` or `F1` shows context-aware command list
5. **Autocomplete**: Typing `/` shows completion popup
6. **Escape cascade**: Escape clears input → cancels → quits (in order)
7. **Focus trap**: `/` from any pane jumps to input
8. **Footer hints**: Show slash command style (`Tab:Switch  /:Commands  Esc:Quit`)
9. **Unknown commands**: Show error toast for unrecognized commands
10. **Phase commands**: Show "not available" for wrong-phase commands
11. **Slash escaping**: `//foo` sends message "/foo" (escaped slash)
12. **Signal handling**: `Ctrl+C` triggers graceful pause, not process termination

## Non-Goals

- Full implementation of phase-specific commands (stubs only)
- Model switching implementation (`/model` shows "not implemented")
- Clipboard integration (`/copy` shows "not implemented")
- $EDITOR integration (`/editor` shows "not implemented")
- Timeline search implementation (`/search` shows "not implemented")
- Command history (up arrow for previous commands)
- Vim-style `/` search (conflicts with slash commands; use `Ctrl+F` or `/search`)

These are deferred to M5-B.3b and beyond.

## Testing Strategy

1. **Unit tests** for command parser
2. **Unit tests** for keybinding handling
3. **Integration tests** for escape cascade
4. **Snapshot tests** for help overlay
5. **Manual testing** for autocomplete UX

## Dependencies

- M5-B.3a (Timeline Input) - provides ConversationPane and input widget

## Risks

1. **Terminal compatibility**: `Ctrl+1/2/3` may not work in all terminals
   - Mitigation: Slash commands are always available as fallback

2. **Autocomplete complexity**: Popup rendering in TUI can be tricky
   - Mitigation: Start with simple list, enhance later

## References

- TUI_UX_PRINCIPLES.md - "Input Model & Command System" section
- Claude Code - slash command patterns
- Gemini review feedback on hybrid keybinding approach
