# Milestone 2: TUI Foundation

## Context

You are implementing the TUI foundation for `ralf`, a multi-model autonomous loop engine. Milestone 1 (Engine Core) is complete - we have working config, state, discovery, model invocation, verification, and a headless `ralf run` command.

Now we need to build the **beautiful TUI shell** that will host the Spec Studio and Run Dashboard in future milestones.

## Stack

- **ratatui** (0.29) - TUI framework
- **crossterm** (0.28) - Terminal backend
- **ralf-engine** - Already implemented engine APIs

## Scope

### 1. App Shell & Event Loop

Create the main TUI application structure:

```
crates/ralf-tui/src/
├── lib.rs           # Public API (run_tui)
├── app.rs           # App state and update logic
├── event.rs         # Event handling (keyboard, tick)
├── ui/
│   ├── mod.rs       # UI module
│   ├── theme.rs     # Colors, styles
│   ├── layout.rs    # Layout helpers
│   └── widgets/     # Reusable widgets
│       ├── mod.rs
│       ├── tabs.rs
│       ├── log_viewer.rs
│       └── status_bar.rs
└── screens/
    ├── mod.rs       # Screen trait and routing
    ├── welcome.rs   # Screen 0: Welcome/repo detection
    └── setup.rs     # Screen 1: Setup/probe/config
```

### 2. Theme & Styling

Create a consistent visual theme:
- Color palette (background, foreground, accent, success, warning, error)
- Border styles (rounded preferred)
- Status indicators (✅ ⚠ ❌ or ASCII equivalents)
- Consistent spacing and alignment

### 3. Event Loop

Implement a non-blocking event loop:
- Keyboard input handling
- Tick events for updates (e.g., 4 Hz)
- Graceful shutdown (Ctrl+C, 'q')
- No UI flicker

### 4. Core Widgets

**Tabs widget:**
- Horizontal tab bar
- Keyboard navigation (Tab, Shift+Tab, number keys)
- Visual indicator for active tab

**Log viewer widget:**
- Scrollable text area
- Auto-scroll to bottom (tail mode)
- Manual scroll with j/k or arrows
- Line wrapping

**Status bar:**
- Current screen/mode
- Key hints
- Model status indicators

### 5. Screen 0: Welcome

Display on first run or when no config exists:
```
┌─ ralf ───────────────────────────────────────────────────────────────┐
│                                                                      │
│  Welcome to ralf — multi-model autonomous loop engine                │
│                                                                      │
│  Repository: /path/to/repo                                          │
│  Git status: clean (branch: main)                                   │
│                                                                      │
│  Models:  claude ✓   codex ✓   gemini ⚠ (needs auth)                │
│                                                                      │
│  [s] Setup   [q] Quit                                               │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

Wire to engine:
- `discover_models()` for model detection
- Git info from `get_git_info()`
- Check for `.ralf/config.json` existence

### 6. Screen 1: Setup

Interactive setup flow:
```
┌─ Setup ──────────────────────────────────────────────────────────────┐
│                                                                      │
│  Probing models...                                                  │
│                                                                      │
│  claude   [████████████████████] ✓ ready (1.2s)                     │
│  codex    [████████████████████] ✓ ready (0.8s)                     │
│  gemini   [██████              ] ⚠ timeout - may need auth          │
│                                                                      │
│  ─────────────────────────────────────────────────────────────────  │
│                                                                      │
│  Model selection: (•) Round-robin  ( ) Priority                     │
│  Promise tag: COMPLETE                                               │
│                                                                      │
│  [Enter] Save config   [d] Disable gemini   [r] Retry   [q] Back    │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

Features:
- Async model probing with progress indication
- Probe results with response times
- Option to disable problematic models
- Model selection strategy toggle
- Save config to `.ralf/config.json`

Wire to engine:
- `probe_model()` for each detected model
- `Config::save()` to write config

### 7. Help Overlay

Press `?` to show keyboard shortcuts:
```
┌─ Help ───────────────────────────────────────────────────────────────┐
│                                                                      │
│  Navigation                                                         │
│    Tab / Shift+Tab   Next/prev section                              │
│    j/k or ↑/↓        Scroll                                         │
│    Enter             Select/confirm                                  │
│    Esc               Back/cancel                                     │
│    q                 Quit                                            │
│    ?                 Toggle this help                                │
│                                                                      │
│  [Press any key to close]                                           │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

## Acceptance Criteria

1. `ralf` (no args) opens the TUI
2. Welcome screen shows repo info and detected models
3. Setup screen probes models with visual progress
4. Config is saved to `.ralf/config.json` after setup
5. Help overlay works with `?` key
6. Clean exit with `q` or Ctrl+C
7. No flicker or visual artifacts
8. All existing CLI commands still work

## Testing Strategy

- Unit tests for widget rendering (ratatui test utilities)
- Manual testing of keyboard navigation
- Verify config file is written correctly after setup

## Dependencies to Add

```toml
# In workspace Cargo.toml [workspace.dependencies]
tokio = { version = "1.0", features = ["rt-multi-thread", "macros", "process", "time", "io-util", "fs", "sync"] }

# In ralf-tui/Cargo.toml
tokio.workspace = true
```

## Notes

- Keep engine logic in ralf-engine, TUI is presentation only
- Use async for probe operations to keep UI responsive
- Consider terminal size constraints (minimum 80x24)
- Prefer ASCII fallbacks for symbols when possible

## Completion

When all acceptance criteria are met, output:

<promise>COMPLETE</promise>
