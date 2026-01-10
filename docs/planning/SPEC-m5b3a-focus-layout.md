# M5-B.3a'' Focus Model & Layout Rework

## Promise

Rework the TUI layout and focus model to provide a clean, consistent UX where input is always accessible and pane navigation is intuitive.

## Background

Manual testing of M5-B.3a' revealed several UX issues:
1. Can't type in canvas mode (input hidden when context pane is fullscreen)
2. Timeline navigation confusing (Alt+j/k awkward, unclear when it applies)
3. Footer hints cluttered and not like Claude Code's minimal style
4. Need explicit focus states with pane-specific keybindings

## Design

### New Layout

```
┌─────────────────────────────────────────────────────┐
│ Status Bar (phase, title, model)                    │
├────────────────────────┬────────────────────────────┤
│ Timeline               │ Canvas/Context             │
│                        │                            │
│  [events...]           │  [phase-specific view]     │
│                        │                            │
├────────────────────────┴────────────────────────────┤
│ > Input area (full width, always visible)           │
├─────────────────────────────────────────────────────┤
│ Split │ Timeline focus │ [hints]      (status bar)  │
└─────────────────────────────────────────────────────┘
```

### Three-Way Focus Model

Focus cycles through three targets:
1. **Timeline** - Event list navigation
2. **Canvas** - Context pane interaction
3. **Input** - Text entry

**Tab** cycles: Timeline → Canvas → Input → Timeline...

**Tab Priority:**
- When autocomplete popup is open, Tab navigates/accepts autocomplete (existing behavior)
- When no autocomplete, Tab cycles focus

**Single-Pane Modes:**
- In TimelineFocus mode: Tab cycles Timeline → Input → Timeline (Canvas skipped)
- In ContextFocus mode: Tab cycles Canvas → Input → Canvas (Timeline skipped)

### Pane-Specific Keybindings

When **Timeline** is focused:
- `j/k` or `↓/↑` - Navigate events
- `Enter` - Toggle collapse
- `y` - Copy selected event
- `g` - Jump to top
- `G` - Jump to bottom

When **Canvas** is focused:
- Context-specific (TBD per phase)

When **Input** is focused:
- Normal typing
- `Enter` - Submit
- `Shift+Enter` - Newline
- `/` - Start command

### Focus Indicators

- Focused pane has highlighted border (theme.primary)
- Unfocused panes have muted border (theme.muted)
- Input area shows cursor when focused

### Bottom Status Bar

Replace verbose footer hints with minimal status:
```
Split │ Timeline │ Drafting            [Tab] focus │ [?] help
```

Components:
- Screen mode (Split/Timeline/Canvas)
- Focused pane name
- Thread phase (if any)
- Minimal hints (Tab, ?)

## Deliverables

- [ ] Full-width input bar visible in all screen modes
- [ ] Remove footer hints widget
- [ ] Add minimal bottom status bar
- [ ] Three-way focus cycling (Timeline/Canvas/Input)
- [ ] Pane-specific keybindings for Timeline (j/k, y, Enter)
- [ ] Visual focus indicators on pane borders
- [ ] Update screen modes to show input in all modes

## Non-Goals

- Canvas-specific keybindings (defer to phase implementations)
- Input history (separate feature)
- Kitty keyboard protocol (future enhancement)

## Acceptance Criteria

1. Input bar visible in Split, Timeline, and Canvas modes
2. Tab cycles focus through Timeline → Canvas → Input
3. j/k navigates timeline when Timeline is focused
4. j/k types "jk" when Input is focused
5. Focus state clearly visible via border colors
6. Bottom status bar shows mode, focus, and phase
7. All existing slash commands still work
