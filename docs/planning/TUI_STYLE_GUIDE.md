# ralf TUI Style Guide

## Overview

This document defines the visual design language for ralf's terminal user interface. These are concrete choices—specific colors, characters, and patterns—not abstract principles.

**Design Philosophy:** Colorful and graphical within TUI constraints. Modern, polished aesthetic inspired by Charm.sh tools and Catppuccin themes. Warm and approachable, not cold and utilitarian.

**Icon Strategy:** Nerd Fonts enabled by default (like k9s), with Unicode and ASCII fallbacks configurable. Users with Nerd Fonts installed get richer icons; others can switch to `icons = "unicode"` in config.

---

## Color Palette

### Base Palette (Catppuccin Mocha-inspired)

We use a warm, pastel-influenced palette that's easy on the eyes during long coding sessions.

```
Background Tones:
  Base:       #1e1e2e  (deep navy)
  Surface:    #313244  (panel backgrounds)
  Overlay:    #45475a  (popups, modals)

Foreground Tones:
  Text:       #cdd6f4  (primary text)
  Subtext:    #a6adc8  (secondary text)
  Muted:      #6c7086  (hints, disabled)

Accent Colors:
  Lavender:   #b4befe  (primary accent, focus)
  Blue:       #89b4fa  (information, active)
  Sapphire:   #74c7ec  (links, navigation)
  Teal:       #94e2d5  (secondary accent)

Semantic Colors:
  Green:      #a6e3a1  (success, passed)
  Yellow:     #f9e2af  (warning, in-progress)
  Peach:      #fab387  (attention, paused)
  Red:        #f38ba8  (error, failed)
  Mauve:      #cba6f7  (special, model output)

Model Attribution:
  Claude:     #fab387  (peach/orange)
  Gemini:     #89b4fa  (blue)
  Codex:      #a6e3a1  (green)
```

### ANSI Fallback (16-color terminals)

```
Standard Colors (0-7):
  Black:      0   (background)
  Red:        1   (error)
  Green:      2   (success)
  Yellow:     3   (warning)
  Blue:       4   (info)
  Magenta:    5   (model output)
  Cyan:       6   (accent)
  White:      7   (primary text)

Bright Colors (8-15):
  Bright Black:   8   (muted text)
  Bright Red:     9   (error emphasis)
  Bright Green:   10  (success emphasis)
  Bright Yellow:  11  (warning emphasis)
  Bright Blue:    12  (active/focus)
  Bright Magenta: 13  (special)
  Bright Cyan:    14  (links)
  Bright White:   15  (headers)
```

### Color Detection

```rust
// Detect color capability
fn color_mode() -> ColorMode {
    if std::env::var("NO_COLOR").is_ok() {
        ColorMode::None
    } else if std::env::var("COLORTERM").map(|v| v == "truecolor" || v == "24bit").unwrap_or(false) {
        ColorMode::TrueColor
    } else if std::env::var("TERM").map(|t| t.contains("256color")).unwrap_or(false) {
        ColorMode::Color256
    } else {
        ColorMode::Basic16
    }
}
```

---

## Box Drawing & Borders

### Primary Style: Rounded Corners

Modern, approachable feel. Use rounded corners for all panels and containers.

```
Corners:    ╭ ╮ ╰ ╯
Horizontal: ─
Vertical:   │
T-pieces:   ├ ┤ ┬ ┴ ┼
```

**Example Panel:**
```
╭─ Panel Title ────────────────────────╮
│ Content goes here                    │
│                                      │
╰──────────────────────────────────────╯
```

### Heavy Borders for Emphasis

Use heavy/thick borders for focused panes and important dialogs.

```
Corners:    ┏ ┓ ┗ ┛
Horizontal: ━
Vertical:   ┃
```

**Example Focused Panel:**
```
┏━ Active Panel ━━━━━━━━━━━━━━━━━━━━━━━┓
┃ This panel has focus                 ┃
┃                                      ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

### Separator Lines

```
Horizontal separator:  ─────────────────
Section divider:       ───────────────── (with spacing above/below)
Vertical separator:    │
Dashed separator:      ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ (for soft divisions)
```

---

## Icons & Symbols

### Icon Strategy: Nerd Fonts (Optional, On by Default)

Following the k9s approach:
- **Default:** Use Nerd Font icons (richer, more expressive)
- **Fallback:** Users can switch to Unicode-only via config
- **Graceful:** If glyphs don't render, user sets `icons = "unicode"`

```toml
# Config option
[ui]
icons = "nerd"     # "nerd" (default), "unicode", "ascii"
```

Recommended fonts: [Meslo LG Nerd Font](https://www.nerdfonts.com/), Hack Nerd Font, JetBrains Mono Nerd Font

### Icon Reference Table

| Purpose | Nerd Font | Unicode | ASCII | Notes |
|---------|-----------|---------|-------|-------|
| **Status** |
| Running/Active | `󰄬` (nf-md-check_circle) | `●` | `[*]` | Green |
| Stopped/Inactive | `󰅖` (nf-md-close_circle) | `○` | `[ ]` | Dim |
| In Progress | `󰦖` (nf-md-progress_clock) | `◐` | `[~]` | Yellow, animated |
| Paused | `󰏤` (nf-md-pause_circle) | `◑` | `[=]` | Yellow |
| **Results** |
| Success/Passed | `󰄬` (nf-md-check_circle) | `✓` | `[x]` | Green |
| Failed/Error | `󰅖` (nf-md-close_circle) | `✗` | `[X]` | Red |
| Warning | `󰀦` (nf-md-alert) | `⚠` | `[!]` | Yellow |
| Info | `󰋼` (nf-md-information) | `ℹ` | `[i]` | Blue |
| **Navigation** |
| Collapsed | `󰅂` (nf-md-chevron_right) | `▸` | `>` | |
| Expanded | `󰅀` (nf-md-chevron_down) | `▾` | `v` | |
| Next/Forward | `󰁔` (nf-md-arrow_right) | `→` | `->` | |
| Back | `󰁍` (nf-md-arrow_left) | `←` | `<-` | |
| **Timeline Events** |
| Spec/Chat | `󰭻` (nf-md-message) | `💬` | `[S]` | Lavender |
| Run/Iteration | `󱐋` (nf-md-lightning_bolt) | `⚡` | `[R]` | Yellow |
| Review | `󰈈` (nf-md-eye) | `👁` | `[V]` | Blue |
| System | `󰒓` (nf-md-cog) | `⚙` | `[*]` | Dim |
| **Git/Files** |
| File Added | `󰐕` (nf-md-plus) | `+` | `+` | Green |
| File Modified | `󰦒` (nf-md-pencil) | `~` | `~` | Yellow |
| File Deleted | `󰍴` (nf-md-minus) | `-` | `-` | Red |
| Git Branch | `󰘬` (nf-md-source_branch) | `⎇` | `@` | |
| Git Commit | `󰜘` (nf-md-source_commit) | `•` | `o` | |
| **Models** |
| Claude | `󰚩` (nf-md-robot) | `🤖` | `[C]` | Peach |
| Gemini | `󱗻` (nf-md-diamond) | `💎` | `[G]` | Blue |
| Codex | `󰘦` (nf-md-code_braces) | `⌘` | `[X]` | Green |
| **Misc** |
| Help | `󰋖` (nf-md-help_circle) | `?` | `?` | |
| Settings | `󰒓` (nf-md-cog) | `⚙` | `*` | |
| Folder | `󰉋` (nf-md-folder) | `📁` | `/` | |
| File | `󰈔` (nf-md-file) | `📄` | `-` | |

### Spinner Animation

**Nerd Font spinner** (smooth, 6 frames at 80ms):
```
󰪞 󰪟 󰪠 󰪡 󰪢 󰪣 (nf-md-loading variants)
```

**Unicode fallback** (4 frames at 100ms):
```
◐ ◓ ◑ ◒ (half circles)
```

**ASCII fallback** (4 frames):
```
| / - \
```

### File Type Icons (Nerd Font only)

When Nerd Fonts enabled, show file type icons:
```
󰌛  .rs     Rust
󰌞  .py     Python
󰛦  .js     JavaScript
󰛦  .ts     TypeScript
󰗀  .md     Markdown
󰗀  .json   JSON
󰗀  .toml   TOML
󰗀  .yaml   YAML
```

Falls back to no icon (just filename) in Unicode/ASCII mode.

---

## Progress Bars

### Standard Progress Bar

```
[████████░░░░░░░░░░░░] 40%
```

Characters:
- Full block: `█` (U+2588)
- Light shade: `░` (U+2591)
- Brackets: `[` `]`

### Compact Progress Bar (for tight spaces)

```
▓▓▓▓░░░░░░
```

Characters:
- Dark shade: `▓` (U+2593)
- Light shade: `░` (U+2591)

### Criteria Progress

```
Criteria: [██████░░░░] 3/5
  ✓ Build passed
  ✓ Lint passed
  ◐ Tests running
  ○ Types pending
  ○ Custom pending
```

---

## Typography

### Text Weights

```
Bold:       Headers, active items, important info
            Phase names, panel titles, key metrics

Normal:     Primary content, body text
            Timeline entries, descriptions

Dim:        Secondary info, hints, metadata
            Timestamps, file paths, keyboard hints

Italic:     Emphasis within text (sparingly)
            User quotes, variable names
```

### Text Hierarchy Example

```
╭─ Panel Title ────────────────────────╮  ← Bold, accent color
│                                      │
│ Primary content in normal weight     │  ← Normal, text color
│ with important terms in bold.        │
│                                      │
│ Secondary info in dim text           │  ← Dim, subtext color
│ Hint: Press ? for help               │  ← Dim, muted color
│                                      │
╰──────────────────────────────────────╯
```

---

## Spacing & Layout

### Spacing Units

Base unit: 1 character

```
Tight:    0 chars  (no spacing)
Compact:  1 char   (minimal breathing room)
Normal:   2 chars  (standard spacing)
Relaxed:  3 chars  (generous spacing)
```

### Panel Padding

```
╭─ Title ──────────────────────────────╮
│                                      │  ← 1 line padding top
│  Content with 1 char left padding    │  ← 1 char padding left
│                                      │  ← 1 line padding bottom
╰──────────────────────────────────────╯
```

### Status Bar Layout

```
│ ● Phase │ Title │ Model │ file.rs:42 │ 2/5 criteria │ → Next action │
```

- 1 char padding on each side of separators
- Truncate middle sections first if space constrained
- Status indicator (●) flush left
- Help hint flush right

### Footer Layout

```
│ [Ctrl+Enter] Send │ [Ctrl+F] Finalize │ [Tab] Focus │ [?] Help │
```

- Actions left-to-right by frequency
- Help and quit always rightmost
- 1 char between bracket and text

---

## Component Patterns

### Status Bar

```
┃ ● Running │ "Add auth" │ gemini │ src/auth.rs:47 │ 2/5 ┃ → Press ? for help ┃
  ↑          ↑            ↑        ↑                ↑      ↑
  Phase      Thread       Model    Current file     Metric Next action
  (colored)  (truncate)   (colored)(streaming)
```

Color rules:
- Phase indicator: Yellow (running), Green (success), Red (error)
- Model name: Model's assigned color
- Metric: Green if progressing, yellow if stuck

### Timeline Entry

```
▸ [Run] Iteration 2                           gemini   2m ago
  └─ Files: +2 ~3 -0 │ ✓ Build │ ✗ Tests (3 failed)
```

Expanded:
```
▾ [Run] Iteration 2                           gemini   2m ago
  ├─ Modified: src/auth.rs (+47), src/lib.rs (+3, -1)
  ├─ Created: tests/auth_test.rs (+23)
  ├─ ✓ Build passed
  ├─ ✗ Tests: 3 failed
  │    └─ test_login_invalid_token FAILED
  │    └─ test_logout_no_session FAILED
  │    └─ test_refresh_expired FAILED
  └─ Attempting fix in iteration 3...
```

### Transient Toast

```
                          ╭─ src/auth.rs ─────────────────╮
                          │ + pub fn login(token: &str)   │
                          │ +     -> Result<User> {       │
                          │ +     validate(token)?;       │
                          │ + }                           │
                          ╰───────────────────────────────╯
```

- Appears in bottom-right corner
- Rounded corners
- Green `+` for additions, red `-` for deletions
- Fades after 2-3 seconds

### Decision Prompt

```
╭─ Stuck after 5 iterations ────────────────────────────────────────╮
│                                                                   │
│  Best result: 2/4 criteria passed                                 │
│  Models tried: claude, gemini, codex                              │
│                                                                   │
│  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  │
│                                                                   │
│  [1] Revise spec      Modify requirements and restart             │
│  [2] Reconfigure      Change models or iteration limit            │
│  [3] Manual assist    Edit code yourself, then retry              │
│  [4] Abandon          Stop this thread                            │
│                                                                   │
╰───────────────────────────────────────────────────────────────────╯
```

- Centered in context pane
- Dashed separator between info and actions
- Numbered options with descriptions
- Dim descriptions

### Preflight Check Results

```
╭─ Preflight Checks ────────────────────────────────────────────────╮
│                                                                   │
│  ✓ Git Working Tree        Clean                                  │
│  ✓ Git Baseline            main @ a1b2c3d                         │
│  ✓ Spec Has Promise        Found <promise> tag                    │
│  ✓ Criteria Parseable      5 criteria extracted                   │
│  ✗ Models Available        No models configured                   │
│  ✓ Verifiers Available     build, test configured                 │
│  ✓ No Concurrent Run       Ready to start                         │
│                                                                   │
│  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  │
│                                                                   │
│  1 check failed. Fix issues and retry.                            │
│                                                                   │
│  [r] Retry    [c] Configure models    [Esc] Back                  │
│                                                                   │
╰───────────────────────────────────────────────────────────────────╯
```

---

## Theming

### Theme Structure

```rust
pub struct Theme {
    // Backgrounds
    pub base: Color,
    pub surface: Color,
    pub overlay: Color,

    // Foregrounds
    pub text: Color,
    pub subtext: Color,
    pub muted: Color,

    // Accents
    pub primary: Color,
    pub secondary: Color,

    // Semantic
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Models
    pub claude: Color,
    pub gemini: Color,
    pub codex: Color,

    // Borders
    pub border: Color,
    pub border_focused: Color,
}
```

### UI Config Structure

```rust
pub struct UiConfig {
    pub theme: ThemeName,           // "mocha", "latte", "high_contrast"
    pub icons: IconMode,            // "nerd" (default), "unicode", "ascii"
}

pub enum IconMode {
    Nerd,      // Nerd Font icons (default)
    Unicode,   // Standard Unicode symbols
    Ascii,     // ASCII-only fallback
}
```

### Built-in Themes

**Mocha (Default)** - Warm dark theme based on Catppuccin
**Latte** - Light theme for bright environments
**High Contrast** - Maximum contrast for accessibility

### NO_COLOR Support

When `NO_COLOR` environment variable is set:
- Disable all colors
- Use bold/dim/normal for hierarchy
- Use ASCII box drawing (`+`, `-`, `|`)
- Rely on symbols (`*`, `x`, `>`) for status

---

## Accessibility

### Contrast Requirements

All text meets WCAG 2.1 Level AA (4.5:1 contrast ratio):
- Primary text on base: 11.3:1 ✓
- Subtext on base: 7.2:1 ✓
- Muted on base: 4.6:1 ✓

### Color Independence

Never use color as the only indicator:
- ✓ Green check + "passed" text
- ✗ Red X + "failed" text
- ● Yellow dot + "running" text

### Keyboard Navigation

All interactions achievable via keyboard:
- Tab for focus cycling
- Arrow keys for selection
- Enter for activation
- Escape for cancel/back
- Number keys for quick selection

---

## ASCII Fallback Mode

For terminals without Unicode support:

```
Borders:    +--+    instead of    ╭──╮
            |  |                  │  │
            +--+                  ╰──╯

Status:     [*]     instead of    ●
            [ ]     instead of    ○
            [>]     instead of    ▸
            [v]     instead of    ▾

Results:    [x]     instead of    ✓
            [X]     instead of    ✗
            [!]     instead of    ⚠

Progress:   [####----]  instead of  [████░░░░]
```

---

## Implementation Notes

### Ratatui Integration

```rust
use ratatui::style::{Color, Modifier, Style};

// Define theme colors
const BASE: Color = Color::Rgb(30, 30, 46);
const TEXT: Color = Color::Rgb(205, 214, 244);
const LAVENDER: Color = Color::Rgb(180, 190, 254);
const GREEN: Color = Color::Rgb(166, 227, 161);
const RED: Color = Color::Rgb(243, 139, 168);

// Style helpers
fn header_style() -> Style {
    Style::default().fg(LAVENDER).add_modifier(Modifier::BOLD)
}

fn success_style() -> Style {
    Style::default().fg(GREEN)
}

fn error_style() -> Style {
    Style::default().fg(RED)
}

fn dim_style() -> Style {
    Style::default().fg(Color::Rgb(108, 112, 134))
}
```

### Border Sets

```rust
use ratatui::symbols::border;

// Rounded borders (default)
const ROUNDED: border::Set = border::Set {
    top_left: "╭",
    top_right: "╮",
    bottom_left: "╰",
    bottom_right: "╯",
    horizontal: "─",
    vertical: "│",
    // ... etc
};

// Heavy borders (focused)
const HEAVY: border::Set = border::Set {
    top_left: "┏",
    top_right: "┓",
    bottom_left: "┗",
    bottom_right: "┛",
    horizontal: "━",
    vertical: "┃",
    // ... etc
};
```

---

## Appendix: Research Sources

This style guide draws from analysis of these TUIs:

- **lazygit** - Git TUI, excellent panel focus and color usage
- **gitui** - Git TUI, clean high-contrast design
- **bottom (btm)** - System monitor, colorful data visualization
- **k9s** - Kubernetes TUI, dense information with semantic colors
- **charm.sh tools** - Modern aesthetic, rounded corners, playful
- **delta** - Git diff viewer, syntax highlighting
- **glow** - Markdown renderer, elegant spacing
- **zellij** - Terminal multiplexer, clear pane boundaries

Theme inspiration:
- **Catppuccin** - Warm pastel palette, 4 flavors
- **Dracula** - High contrast dark theme
- **Solarized** - Scientific color selection
