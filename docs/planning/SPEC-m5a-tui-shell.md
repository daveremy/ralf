# M5-A: TUI Core Shell

## Promise

Build the foundational TUI shell with two-pane layout, status bar, footer hints, focus management, and screen modes. This is the skeleton that all content will render into.

## Context

This is Phase 1 of the TUI rebuild. We're starting fresh with a clean architecture based on:
- [TUI_UX_PRINCIPLES.md](TUI_UX_PRINCIPLES.md) - Layout and interaction patterns
- [TUI_STYLE_GUIDE.md](TUI_STYLE_GUIDE.md) - Colors, icons, typography

The existing TUI code (Milestones 2-4) may be referenced for patterns but will not be migrated.

## Deliverables

### File Structure

```
crates/ralf-tui/src/
├── app.rs              # App struct, main loop (rewrite)
├── event.rs            # Event handling (keep/adapt)
├── lib.rs              # Public API (rewrite)
├── headless.rs         # Test infrastructure (keep/adapt)
│
├── layout/
│   ├── mod.rs
│   ├── shell.rs        # Main 5-region layout
│   └── screen_modes.rs # Split, TimelineFocus, ContextFocus
│
├── widgets/
│   ├── mod.rs
│   ├── status_bar.rs   # Top status bar
│   ├── footer_hints.rs # Bottom keybinding hints
│   └── pane.rs         # Generic pane with border/title
│
└── theme/
    ├── mod.rs
    ├── colors.rs       # Catppuccin Mocha palette
    ├── icons.rs        # Nerd/Unicode/ASCII icon sets
    └── borders.rs      # Border sets for NO_COLOR/ASCII fallback
```

### Core Types

```rust
/// Main application state
pub struct App {
    /// Current screen mode
    pub screen_mode: ScreenMode,

    /// Which pane has focus (for split mode)
    pub focused_pane: Pane,

    /// UI configuration
    pub ui_config: UiConfig,

    /// Theme colors
    pub theme: Theme,

    /// Icon set based on config
    pub icons: IconSet,

    /// Border set based on icon mode (for NO_COLOR/ASCII fallback)
    pub borders: BorderSet,

    /// Current terminal size (updated on resize)
    pub terminal_size: (u16, u16),

    /// Should the app quit?
    pub should_quit: bool,
}

/// Screen display modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScreenMode {
    #[default]
    Split,          // Timeline (40%) | Context (60%)
    TimelineFocus,  // Timeline (100%)
    ContextFocus,   // Context (100%)
}

/// Which pane has keyboard focus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pane {
    #[default]
    Timeline,
    Context,
}

/// UI configuration (from config file or environment)
#[derive(Debug, Clone)]
pub struct UiConfig {
    pub theme: ThemeName,
    pub icons: IconMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconMode {
    #[default]
    Nerd,
    Unicode,
    Ascii,  // Also used when NO_COLOR is set
}

impl UiConfig {
    /// Create config, respecting NO_COLOR environment variable
    pub fn from_env() -> Self {
        let icons = if std::env::var("NO_COLOR").is_ok() {
            IconMode::Ascii
        } else {
            IconMode::Nerd // default
        };
        Self {
            theme: ThemeName::Mocha,
            icons,
        }
    }
}
```

### Layout Components

#### Shell Layout (`layout/shell.rs`)

The shell has **4 regions** in split mode:
1. **Status Bar** (top, 1 line)
2. **Timeline Pane** (left, 40%)
3. **Context Pane** (right, 60%)
4. **Footer Hints** (bottom, 1 line) - This serves as the Input/Action Bar placeholder per UX Principles

```rust
/// Render the main shell layout
pub fn render_shell(frame: &mut Frame, app: &App) {
    // Divide into: StatusBar | MainArea | FooterHints
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),      // Status bar
            Constraint::Min(0),         // Main area (expands)
            Constraint::Length(1),      // Footer hints (Input/Action Bar placeholder)
        ])
        .split(frame.area());

    // Status bar with placeholder content for M5-A
    let status_content = StatusBarContent::placeholder();
    render_status_bar(frame, chunks[0], &status_content, &app.theme);

    // Main pane area
    render_main_area(frame, chunks[1], app);

    // Footer with keybinding hints (includes help per UX guidance)
    let hints = get_footer_hints(app);
    render_footer_hints(frame, chunks[2], &hints, &app.theme);
}

/// Render the main two-pane area based on screen mode
fn render_main_area(frame: &mut Frame, area: Rect, app: &App) {
    match app.screen_mode {
        ScreenMode::Split => {
            // 40% Timeline | 60% Context
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Percentage(60),
                ])
                .split(area);

            render_timeline_pane(frame, chunks[0], app, app.focused_pane == Pane::Timeline);
            render_context_pane(frame, chunks[1], app, app.focused_pane == Pane::Context);
        }
        ScreenMode::TimelineFocus => {
            // Focus mode: only timeline visible, always focused
            render_timeline_pane(frame, area, app, true);
        }
        ScreenMode::ContextFocus => {
            // Focus mode: only context visible, always focused
            render_context_pane(frame, area, app, true);
        }
    }
}

/// Get footer hints for current state (always includes help and quit per UX)
fn get_footer_hints(app: &App) -> Vec<KeyHint> {
    vec![
        KeyHint { key: "Tab".into(), action: "Focus".into() },
        KeyHint { key: "Ctrl+1/2/3".into(), action: "Modes".into() },
        KeyHint { key: "?".into(), action: "Help".into() },
        KeyHint { key: "Ctrl+Q".into(), action: "Quit".into() },
    ]
}
```

#### Status Bar (`widgets/status_bar.rs`)

```rust
/// Status bar content
pub struct StatusBarContent {
    pub phase: String,          // "Drafting", "Running", etc.
    pub title: String,          // Thread title
    pub model: Option<String>,  // Current model
    pub file: Option<String>,   // Current file:line
    pub metric: Option<String>, // "2/5 criteria"
    pub hint: Option<String>,   // "→ Press Enter to send"
}

impl StatusBarContent {
    /// Placeholder content for M5-A (will be replaced with real data in M5-B)
    pub fn placeholder() -> Self {
        Self {
            phase: "Drafting".into(),
            title: "New Thread".into(),
            model: Some("claude".into()),
            file: None,
            metric: None,
            hint: None,
        }
    }
}

/// Render status bar
/// Format: ● Phase │ "Title" │ model │ file:line │ metric │ → hint
pub fn render_status_bar(frame: &mut Frame, area: Rect, content: &StatusBarContent, theme: &Theme) {
    let spans = vec![
        Span::styled(
            format!("● {} ", content.phase),
            Style::default().fg(theme.primary),
        ),
        Span::raw("│ "),
        Span::styled(
            format!("\"{}\" ", content.title),
            Style::default().fg(theme.text),
        ),
    ];
    // Add optional fields if present...

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.surface));
    frame.render_widget(paragraph, area);
}
```

#### Footer Hints (`widgets/footer_hints.rs`)

```rust
/// A single keybinding hint
pub struct KeyHint {
    pub key: String,    // "Ctrl+P"
    pub action: String, // "Commands"
}

/// Render footer with keybinding hints
/// Format: [Ctrl+P] Commands │ [Tab] Focus │ [?] Help │ [Ctrl+Q] Quit
pub fn render_footer_hints(frame: &mut Frame, area: Rect, hints: &[KeyHint], theme: &Theme);
```

#### Pane Widget (`widgets/pane.rs`)

```rust
/// Generic pane with border and optional title
pub struct PaneWidget<'a> {
    pub title: Option<&'a str>,
    pub focused: bool,
    pub content: Option<&'a str>,  // Placeholder text for M5-A
    pub theme: &'a Theme,
    pub borders: &'a BorderSet,
}

impl<'a> PaneWidget<'a> {
    pub fn new(theme: &'a Theme, borders: &'a BorderSet) -> Self {
        Self {
            title: None,
            focused: false,
            content: None,
            theme,
            borders,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn content(mut self, content: &'a str) -> Self {
        self.content = Some(content);
        self
    }
}

impl Widget for PaneWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Select border set based on focus and icon mode
        let border_set = if self.focused {
            self.borders.focused()
        } else {
            self.borders.normal()
        };

        let border_style = if self.focused {
            Style::default().fg(self.theme.primary)
        } else {
            Style::default().fg(self.theme.border)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(border_set)
            .border_style(border_style)
            .title(self.title.unwrap_or_default());

        let inner = block.inner(area);
        block.render(area, buf);

        // Render placeholder content
        if let Some(text) = self.content {
            let paragraph = Paragraph::new(text)
                .style(Style::default().fg(self.theme.subtext));
            paragraph.render(inner, buf);
        }
    }
}
```

#### Border Set (`theme/borders.rs`)

```rust
/// Border characters based on icon mode (supports NO_COLOR/ASCII fallback)
pub struct BorderSet {
    mode: IconMode,
}

impl BorderSet {
    pub fn new(mode: IconMode) -> Self {
        Self { mode }
    }

    /// Normal (unfocused) borders - rounded for Unicode, ASCII for fallback
    pub fn normal(&self) -> symbols::border::Set {
        match self.mode {
            IconMode::Nerd | IconMode::Unicode => symbols::border::ROUNDED,
            IconMode::Ascii => symbols::border::PLAIN,  // +--+ ASCII corners
        }
    }

    /// Focused borders - thick for Unicode, double for ASCII
    pub fn focused(&self) -> symbols::border::Set {
        match self.mode {
            IconMode::Nerd | IconMode::Unicode => symbols::border::THICK,
            IconMode::Ascii => symbols::border::DOUBLE,  // Best ASCII emphasis
        }
    }
}
```

### Theme Implementation (`theme/colors.rs`)

```rust
/// Catppuccin Mocha color palette
pub struct Theme {
    // Backgrounds
    pub base: Color,        // #1e1e2e
    pub surface: Color,     // #313244
    pub overlay: Color,     // #45475a

    // Foregrounds
    pub text: Color,        // #cdd6f4
    pub subtext: Color,     // #a6adc8
    pub muted: Color,       // #6c7086

    // Accents
    pub primary: Color,     // #b4befe (lavender)
    pub secondary: Color,   // #94e2d5 (teal)

    // Semantic
    pub success: Color,     // #a6e3a1
    pub warning: Color,     // #f9e2af
    pub error: Color,       // #f38ba8
    pub info: Color,        // #89b4fa

    // Models
    pub claude: Color,      // #fab387
    pub gemini: Color,      // #89b4fa
    pub codex: Color,       // #a6e3a1

    // Borders
    pub border: Color,      // #45475a
    pub border_focused: Color, // #b4befe
}

impl Theme {
    pub fn mocha() -> Self { /* Catppuccin Mocha */ }
    pub fn latte() -> Self { /* Catppuccin Latte (light) */ }
    pub fn high_contrast() -> Self { /* Maximum contrast */ }
}
```

### Icon Implementation (`theme/icons.rs`)

All icons from TUI_STYLE_GUIDE.md are implemented here.

```rust
/// Icon set based on configuration
pub struct IconSet {
    pub mode: IconMode,
}

impl IconSet {
    pub fn new(mode: IconMode) -> Self { Self { mode } }

    // === Status Icons ===

    pub fn running(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰄬",
            IconMode::Unicode => "●",
            IconMode::Ascii => "[*]",
        }
    }

    pub fn stopped(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰅖",
            IconMode::Unicode => "○",
            IconMode::Ascii => "[ ]",
        }
    }

    pub fn in_progress(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰦖",
            IconMode::Unicode => "◐",
            IconMode::Ascii => "[~]",
        }
    }

    pub fn paused(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰏤",
            IconMode::Unicode => "◑",
            IconMode::Ascii => "[=]",
        }
    }

    // === Result Icons ===

    pub fn success(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰄬",
            IconMode::Unicode => "✓",
            IconMode::Ascii => "[x]",
        }
    }

    pub fn error(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰅖",
            IconMode::Unicode => "✗",
            IconMode::Ascii => "[X]",
        }
    }

    pub fn warning(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰀦",
            IconMode::Unicode => "⚠",
            IconMode::Ascii => "[!]",
        }
    }

    pub fn info(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰋼",
            IconMode::Unicode => "ℹ",
            IconMode::Ascii => "[i]",
        }
    }

    // === Navigation Icons ===

    pub fn collapsed(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰅂",
            IconMode::Unicode => "▸",
            IconMode::Ascii => ">",
        }
    }

    pub fn expanded(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰅀",
            IconMode::Unicode => "▾",
            IconMode::Ascii => "v",
        }
    }

    pub fn arrow_right(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰁔",
            IconMode::Unicode => "→",
            IconMode::Ascii => "->",
        }
    }

    pub fn arrow_left(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰁍",
            IconMode::Unicode => "←",
            IconMode::Ascii => "<-",
        }
    }

    // === Timeline Event Icons ===

    pub fn event_spec(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰭻",
            IconMode::Unicode => "💬",
            IconMode::Ascii => "[S]",
        }
    }

    pub fn event_run(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󱐋",
            IconMode::Unicode => "⚡",
            IconMode::Ascii => "[R]",
        }
    }

    pub fn event_review(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰈈",
            IconMode::Unicode => "👁",
            IconMode::Ascii => "[V]",
        }
    }

    pub fn event_system(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰒓",
            IconMode::Unicode => "⚙",
            IconMode::Ascii => "[.]",  // Changed from [*] to avoid collision with Running
        }
    }

    // === Git/File Icons ===

    pub fn file_added(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰐕",
            IconMode::Unicode => "+",
            IconMode::Ascii => "+",
        }
    }

    pub fn file_modified(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰦒",
            IconMode::Unicode => "~",
            IconMode::Ascii => "~",
        }
    }

    pub fn file_deleted(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰍴",
            IconMode::Unicode => "-",
            IconMode::Ascii => "-",
        }
    }

    pub fn git_branch(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰘬",
            IconMode::Unicode => "⎇",
            IconMode::Ascii => "@",
        }
    }

    pub fn git_commit(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰜘",
            IconMode::Unicode => "•",
            IconMode::Ascii => "o",
        }
    }

    // === Model Icons ===

    pub fn model_claude(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰚩",
            IconMode::Unicode => "🤖",
            IconMode::Ascii => "[C]",
        }
    }

    pub fn model_gemini(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󱗻",
            IconMode::Unicode => "💎",
            IconMode::Ascii => "[G]",
        }
    }

    pub fn model_codex(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰘦",
            IconMode::Unicode => "⌘",
            IconMode::Ascii => "[X]",
        }
    }

    // === Misc Icons ===

    pub fn help(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰋖",
            IconMode::Unicode => "?",
            IconMode::Ascii => "?",
        }
    }

    pub fn settings(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰒓",
            IconMode::Unicode => "⚙",
            IconMode::Ascii => "*",
        }
    }

    pub fn folder(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰉋",
            IconMode::Unicode => "📁",
            IconMode::Ascii => "/",
        }
    }

    pub fn file(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "󰈔",
            IconMode::Unicode => "📄",
            IconMode::Ascii => "-",
        }
    }

    // === Spinner Frames (for animation) ===

    pub fn spinner_frames(&self) -> &'static [&'static str] {
        match self.mode {
            IconMode::Nerd => &["󰪞", "󰪟", "󰪠", "󰪡", "󰪢", "󰪣"],
            IconMode::Unicode => &["◐", "◓", "◑", "◒"],
            IconMode::Ascii => &["|", "/", "-", "\\"],
        }
    }
}
```

### Event Handling

```rust
/// Handle keyboard input
pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    match key.code {
        // Screen modes
        KeyCode::Char('1') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.screen_mode = ScreenMode::Split;
        }
        KeyCode::Char('2') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.screen_mode = ScreenMode::TimelineFocus;
        }
        KeyCode::Char('3') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.screen_mode = ScreenMode::ContextFocus;
        }

        // Focus management - only effective in Split mode
        // In focus modes (TimelineFocus/ContextFocus), Tab is a no-op since only one pane is visible
        KeyCode::Tab => {
            if app.screen_mode == ScreenMode::Split {
                app.focused_pane = match app.focused_pane {
                    Pane::Timeline => Pane::Context,
                    Pane::Context => Pane::Timeline,
                };
            }
            // In non-Split modes, Tab does nothing (single visible pane is always focused)
        }

        // Help overlay (placeholder for M5-A, implemented in M5-C)
        KeyCode::Char('?') => {
            // TODO: Show help overlay in M5-C
        }

        // Quit
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        _ => {}
    }
}

/// Handle terminal resize events
pub fn handle_resize_event(app: &mut App, width: u16, height: u16) {
    // ratatui handles layout recalculation automatically on next draw
    // This hook exists for future use (e.g., adjusting minimum size warnings)
    app.terminal_size = (width, height);
}
```

**Note on Ctrl+\\ (Swap Panes):** Per UX Principles, Ctrl+\\ swaps the Timeline and Context pane positions. This is deferred to M5-B as it requires tracking swap state and affects rendering logic beyond the core shell.

### Main Loop

```rust
pub fn run_app(terminal: &mut Terminal<impl Backend>) -> io::Result<()> {
    let ui_config = UiConfig::from_env();  // Respects NO_COLOR
    let theme = Theme::mocha();
    let icons = IconSet::new(ui_config.icons);
    let borders = BorderSet::new(ui_config.icons);

    let mut app = App {
        screen_mode: ScreenMode::default(),
        focused_pane: Pane::default(),
        ui_config,
        theme,
        icons,
        borders,
        should_quit: false,
        terminal_size: terminal.size().map(|s| (s.width, s.height)).unwrap_or((80, 24)),
    };

    loop {
        // Render
        terminal.draw(|frame| {
            render_shell(frame, &app);
        })?;

        // Handle events (keyboard, resize, etc.)
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => {
                    handle_key_event(&mut app, key);
                }
                Event::Resize(width, height) => {
                    handle_resize_event(&mut app, width, height);
                }
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
```

### Headless Testing

```rust
/// Create a test terminal with fixed size
pub fn create_test_terminal(width: u16, height: u16) -> Terminal<TestBackend> {
    let backend = TestBackend::new(width, height);
    Terminal::new(backend).unwrap()
}

/// Render app to string for assertions
pub fn render_to_string(app: &App, width: u16, height: u16) -> String {
    let mut terminal = create_test_terminal(width, height);
    terminal.draw(|frame| render_shell(frame, app)).unwrap();

    let buffer = terminal.backend().buffer();
    buffer_to_string(buffer)
}

/// Snapshot test helper
#[cfg(test)]
fn assert_snapshot(name: &str, content: &str) {
    insta::assert_snapshot!(name, content);
}
```

## Acceptance Criteria

### Layout
- [ ] Shell renders with 4 regions in split mode (status bar, timeline pane, context pane, footer hints)
- [ ] Split mode shows Timeline (40%) and Context (60%) panes
- [ ] Timeline Focus mode shows only Timeline pane (100%)
- [ ] Context Focus mode shows only Context pane (100%)
- [ ] Layout adapts to terminal resize (handles `Event::Resize`)
- [ ] Terminals smaller than 40x12 show "Terminal too small" warning without crashing

### Focus Management
- [ ] Tab cycles focus between Timeline and Context panes (in Split mode only)
- [ ] Tab is no-op in TimelineFocus and ContextFocus modes (single visible pane is always focused)
- [ ] Focused pane has highlighted border (primary color, thick/double for ASCII)
- [ ] Unfocused pane has dim border (rounded/plain for ASCII)
- [ ] Focus indicator visible in both panes

### Screen Modes
- [ ] Ctrl+1 switches to Split mode
- [ ] Ctrl+2 switches to Timeline Focus mode
- [ ] Ctrl+3 switches to Context Focus mode
- [ ] Current mode persists until changed

### Status Bar
- [ ] Renders in top row
- [ ] Shows placeholder content: "● Phase │ Title │ model"
- [ ] Uses theme colors

### Footer Hints
- [ ] Renders in bottom row
- [ ] Shows keybinding hints: "[Tab] Focus │ [Ctrl+1/2/3] Modes │ [?] Help │ [Ctrl+Q] Quit"
- [ ] Always includes help and quit hints (per UX Principles)
- [ ] Uses consistent formatting

### Theme
- [ ] Catppuccin Mocha colors implemented
- [ ] Theme struct with all colors from TUI_STYLE_GUIDE.md
- [ ] Colors applied consistently

### Icons
- [ ] IconSet with Nerd/Unicode/ASCII modes
- [ ] All icons from TUI_STYLE_GUIDE.md reference table implemented
- [ ] Default to Nerd mode (unless NO_COLOR is set)

### NO_COLOR Support
- [ ] Respects NO_COLOR environment variable
- [ ] When NO_COLOR is set, uses ASCII icons and plain borders
- [ ] BorderSet switches between ROUNDED/THICK (Unicode) and PLAIN/DOUBLE (ASCII)

### Quit
- [ ] Ctrl+Q quits the application
- [ ] Clean terminal restoration on exit

### Testing
- [ ] Headless test infrastructure working
- [ ] At least 5 snapshot tests for layout variations
- [ ] Unit tests for screen mode transitions
- [ ] Unit tests for focus cycling (including Tab no-op in focus modes)
- [ ] Test for NO_COLOR mode rendering

### Build
- [ ] `cargo build -p ralf-tui` succeeds
- [ ] `cargo clippy -p ralf-tui` has no warnings
- [ ] `cargo test -p ralf-tui` passes

## Non-Goals (for M5-A)

- Actual timeline content (just placeholder "Timeline Pane")
- Actual context content (just placeholder "Context Pane")
- Thread loading or persistence
- Phase-specific behavior
- Command palette (Ctrl+P) - deferred to M5-C
- Activity indicators (heartbeat row, toasts) - deferred to M5-C
- Swap panes position (Ctrl+\\) - deferred to M5-B (requires swap state tracking)
- Help overlay (?) - deferred to M5-C (placeholder key handler only in M5-A)
- Mouse support

## Testing Strategy

### Snapshot Tests

```rust
#[test]
fn test_split_mode_layout() {
    let app = App::default();
    let output = render_to_string(&app, 80, 24);
    assert_snapshot!("split_mode_80x24", output);
}

#[test]
fn test_timeline_focus_layout() {
    let mut app = App::default();
    app.screen_mode = ScreenMode::TimelineFocus;
    let output = render_to_string(&app, 80, 24);
    assert_snapshot!("timeline_focus_80x24", output);
}

#[test]
fn test_context_focus_layout() {
    let mut app = App::default();
    app.screen_mode = ScreenMode::ContextFocus;
    let output = render_to_string(&app, 80, 24);
    assert_snapshot!("context_focus_80x24", output);
}

#[test]
fn test_focused_pane_indicator() {
    let mut app = App::default();
    app.focused_pane = Pane::Context;
    let output = render_to_string(&app, 80, 24);
    assert_snapshot!("context_focused_80x24", output);
}

#[test]
fn test_narrow_terminal() {
    let app = App::default();
    let output = render_to_string(&app, 40, 24);
    assert_snapshot!("split_mode_40x24", output);
}
```

### Unit Tests

```rust
#[test]
fn test_focus_cycling_in_split_mode() {
    let mut app = App::default();
    assert_eq!(app.screen_mode, ScreenMode::Split);
    assert_eq!(app.focused_pane, Pane::Timeline);

    handle_key_event(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.focused_pane, Pane::Context);

    handle_key_event(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.focused_pane, Pane::Timeline);
}

#[test]
fn test_focus_cycling_noop_in_focus_modes() {
    let mut app = App::default();
    app.screen_mode = ScreenMode::TimelineFocus;
    app.focused_pane = Pane::Timeline;

    // Tab should be a no-op in TimelineFocus mode
    handle_key_event(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.focused_pane, Pane::Timeline); // Unchanged

    app.screen_mode = ScreenMode::ContextFocus;
    // Tab should also be a no-op in ContextFocus mode
    handle_key_event(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.focused_pane, Pane::Timeline); // Still unchanged
}

#[test]
fn test_screen_mode_switching() {
    let mut app = App::default();
    assert_eq!(app.screen_mode, ScreenMode::Split);

    handle_key_event(&mut app, KeyEvent::new(KeyCode::Char('2'), KeyModifiers::CONTROL));
    assert_eq!(app.screen_mode, ScreenMode::TimelineFocus);

    handle_key_event(&mut app, KeyEvent::new(KeyCode::Char('3'), KeyModifiers::CONTROL));
    assert_eq!(app.screen_mode, ScreenMode::ContextFocus);

    handle_key_event(&mut app, KeyEvent::new(KeyCode::Char('1'), KeyModifiers::CONTROL));
    assert_eq!(app.screen_mode, ScreenMode::Split);
}

#[test]
fn test_quit() {
    let mut app = App::default();
    assert!(!app.should_quit);

    handle_key_event(&mut app, KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
    assert!(app.should_quit);
}
```

## Dependencies

- `ratatui` - TUI rendering (already in use)
- `crossterm` - Terminal backend (already in use)
- `insta` - Snapshot testing (add to dev-dependencies)

## Resolved Questions

1. **Swap panes position?** → Deferred to M5-B. Requires tracking swap state and affects rendering logic beyond core shell scope.

2. **Minimum terminal size?** → 40x12 minimum. Below this size:
   - Layout still renders but content may be clipped
   - No crash or panic - graceful degradation
   - Status bar shows "Terminal too small" warning if width < 40 or height < 12
   - Snapshot tests cover 80x24 (standard) and 40x24 (narrow) cases

3. **Config file location?** → For M5-A, config comes from environment (NO_COLOR) and defaults. Full config file support (`.ralf/config.toml`) deferred to later milestone.

## Future Improvements

Suggestions from code review (non-blocking, for future milestones):

1. **ASCII snapshot test** - Add a shell snapshot test with `IconMode::Ascii` to verify NO_COLOR rendering matches expectations.

2. **Boundary size test** - Add a test at exactly MIN_WIDTH x MIN_HEIGHT (40x12) to verify the boundary behavior.

3. **Extract keybindings** - For M5-C help overlay, consider extracting keybindings to a central location for single-source-of-truth.

4. **StatusBar hint arrow** - Consider using `icons.arrow_right()` for the "→" in StatusBarContent hint for ASCII compatibility.

5. **Consolidate pane renderers** - `render_timeline_pane` and `render_context_pane` are nearly identical; could be consolidated in M5-B when real content diverges.

6. **ThemeName enum** - Add `ThemeName` enum to `UiConfig` when theme switching is needed (e.g., light mode, high contrast).
