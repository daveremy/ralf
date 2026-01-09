# M5-A.1: Model Probing Integration

## Promise

Integrate real model discovery and probing into the M5-A shell, replacing placeholder content with live model status. Users see which AI CLI tools are available, which need attention, and can refresh status on demand.

## Context

The M5-A shell renders a placeholder status bar and empty panes. This subphase wires up the existing engine's model discovery (`discover_models`, `probe_model`) to show real status.

**Dependencies:**
- M5-A shell (complete)
- `ralf-engine` discovery module (exists)

**References:**
- [TUI_DEV_PLAN.md](TUI_DEV_PLAN.md) - CLI-First Model Architecture section
- `crates/ralf-engine/src/discovery.rs` - Existing probe logic

## Deliverables

### 1. Model Status in Status Bar

Replace the placeholder status bar with real model status:

```
● Drafting │ "New Thread" │ claude ● gemini ○ codex ○
```

**Status indicators:**
- `●` (ready) - Model probed successfully
- `◐` (cooldown) - Model rate-limited (forward-looking, not implemented this phase)
- `○` (unavailable) - Not found, auth error, or probe failed

**Narrow terminal handling:** On terminals < 60 chars wide, show only count: `2/3 models`

### 2. Models Panel in Context Pane

When no thread is loaded (initial state), the Context pane shows a Models panel:

```
┏ Models ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃                                                      ┃
┃  claude    ● Ready         v1.2.3                    ┃
┃  gemini    ○ Not found     Install: gemini.google... ┃
┃  codex     ○ Auth needed   Run: codex auth login     ┃
┃                                                      ┃
┃  [r] Refresh                                         ┃
┃                                                      ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

**Features:**
- Model name, status indicator, status text
- Version (if available from `--version`)
- Actionable guidance for unavailable models
- `r` key to refresh/re-probe all models (only when Models panel visible)

### 3. Startup Probe Sequence

On shell launch:
1. Show "Checking models..." in status bar
2. Probe all known models **in parallel** using `std::thread::spawn` (10s timeout each, 15s overall deadline)
3. Update status bar with results as they arrive
4. Populate Models panel with details

### 4. Timeline Events for Model Changes (Stretch)

When model status changes (e.g., recovers from cooldown):
- Add system event to Timeline: "claude ready"
- This prepares infrastructure for M5-B timeline

## Technical Approach

### Type Mapping

The TUI defines its own `ModelStatus` type that wraps engine types:

```rust
// Engine types (in ralf-engine/src/discovery.rs):
// - ModelInfo { name, found, callable, path, version, issues }
// - ProbeResult { name, success, needs_auth, response_time_ms, issues, suggestions }

// TUI type (combines both for display):
#[derive(Debug, Clone)]
pub struct ModelStatus {
    pub name: String,
    pub state: ModelState,
    pub version: Option<String>,
    pub message: Option<String>,  // Error or guidance
}

impl ModelStatus {
    /// Create from engine discovery + probe results
    pub fn from_engine(info: &ModelInfo, probe: Option<&ProbeResult>) -> Self {
        let state = match (info.callable, probe) {
            (false, _) => ModelState::Unavailable,
            (true, Some(p)) if p.success => ModelState::Ready,
            (true, Some(p)) if p.needs_auth => ModelState::Unavailable,
            (true, Some(_)) => ModelState::Unavailable,
            (true, None) => ModelState::Probing,
        };
        // ... build message from issues/suggestions
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelState {
    Probing,       // Currently checking
    Ready,         // Probe succeeded
    Cooldown(u64), // Rate-limited, seconds remaining (forward-looking, not used this phase)
    Unavailable,   // Not found or error
}
```

### Error Message Mapping

Map engine results to user-friendly messages per TUI_DEV_PLAN.md:

| Condition | State | Message |
|-----------|-------|---------|
| `!info.found` | Unavailable | "{name} not found. Install: {url}" |
| `probe.needs_auth` | Unavailable | "{name} needs auth. Run: `{cmd} auth login`" |
| `!probe.success && timeout` | Unavailable | "{name} not responding (10s timeout)" |
| `!probe.success` | Unavailable | "{name} error: {first issue}" |
| `probe.success` | Ready | "Ready" |

**Install URLs:**
- claude: `https://docs.anthropic.com/claude/docs/claude-code`
- codex: `https://github.com/openai/codex`
- gemini: `https://github.com/google/generative-ai-cli`

### ShellApp Changes

```rust
pub struct ShellApp {
    // ... existing fields ...

    /// Model status from probing
    pub models: Vec<ModelStatus>,

    /// Whether initial probe is complete
    pub probe_complete: bool,
}
```

### Parallel Probing Implementation

```rust
use std::thread;
use std::time::Duration;

fn probe_models_parallel(timeout: Duration) -> Vec<ModelStatus> {
    let discovery = discover_models();
    let handles: Vec<_> = discovery.models.iter().map(|info| {
        let name = info.name.clone();
        let info_clone = info.clone();
        thread::spawn(move || {
            let probe = probe_model(&name, timeout);
            ModelStatus::from_engine(&info_clone, Some(&probe))
        })
    }).collect();

    handles.into_iter()
        .map(|h| h.join().unwrap_or_else(|_| /* error status */))
        .collect()
}
```

### Integration Points

1. **Startup:** Call `probe_models_parallel()` before entering render loop
2. **Status bar:** Render model indicators from `self.models`
3. **Context pane:** New `ModelsPanel` widget when in initial state
4. **Refresh:** `r` key triggers re-probe (when Models panel visible)

### File Changes

```
crates/ralf-tui/src/
├── shell.rs           # Add ModelStatus, probing logic, handle 'r' key
├── widgets/
│   ├── mod.rs         # Export ModelsPanel
│   ├── models_panel.rs # NEW: Models panel widget
│   └── status_bar.rs  # Update to show model indicators
└── layout/
    └── shell.rs       # Render ModelsPanel in context area
```

## Acceptance Criteria

- [ ] Status bar shows real model status (not placeholder)
- [ ] Models panel displays in Context pane on startup
- [ ] Each model shows: name, state indicator, status message
- [ ] Unavailable models show actionable guidance (install URL or auth command)
- [ ] `r` key refreshes model status
- [ ] Startup probe completes within 15s (parallel probing)
- [ ] Shell remains responsive during probing
- [ ] Works with NO_COLOR (ASCII indicators: `[x]`=ready, `[ ]`=unavailable)

## Testing

### Unit Tests
- `ModelStatus::from_engine()` mapping logic
- Status bar rendering with various model states
- Models panel rendering

### Snapshot Tests
- Shell with all models ready
- Shell with mixed model states
- Shell with all models unavailable

### Test Mocking Strategy

For unit tests, create mock data directly:
```rust
fn mock_model_status(name: &str, state: ModelState) -> ModelStatus {
    ModelStatus { name: name.into(), state, version: None, message: None }
}
```

For integration tests that need real probing, use `#[ignore]` and run manually.

### Manual Testing
- Launch with claude installed, others not
- Launch with no models (should show all unavailable)
- Press `r` to refresh and verify re-probe
- Test on narrow terminal (< 60 chars)

## Out of Scope

- **Cooldown timer display** - `ModelState::Cooldown(u64)` is defined for forward compatibility but not populated this phase (needs engine cooldown state integration)
- **Enable/disable persistence** - needs config changes
- **Timeline events** - stretch goal, defer to M5-B
- **Model variant selection** - open question in TUI_DEV_PLAN.md

## Resolved Questions

1. **Parallel probing:** Use `std::thread::spawn` for parallel blocking probes at startup. Each probe has 10s timeout, overall 15s deadline. This keeps the implementation simple while achieving parallelism.

2. **Probe caching:** No caching for now. Always probe fresh on launch and on `r` refresh.

3. **Keybindings:** `r` for refresh does not conflict with existing keys (1/2/3, Tab, q, Esc). The `r` key only works when Models panel is visible.

## Estimated Scope

- ~200-300 lines of new code
- 1-2 new files (models_panel.rs, possibly models.rs for types)
- Modifications to shell.rs, status_bar.rs, layout/shell.rs
