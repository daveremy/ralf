# SPEC: M5-B.3c'' Compact Timeline Format

## Promise

Redesign timeline event rendering to be compact and conversation-focused, inspired by Claude Code's minimal approach. Messages will start on a single line with symbol-based speaker identification, eliminating verbose badge lines and reducing vertical space consumption.

## Background

### Current Format (2+ lines per message)
```
  [SPEC] user
       ▸ Help me implement a feature that adds pagination...

  [SPEC] claude
       ▾ I'll help you implement that feature.
         Here's what I understand:
         - Point 1
         - Point 2
```

**Issues:**
- Badge line (`[SPEC] user`) wastes a full line
- Phase label (`[SPEC]`) is redundant - status bar already shows phase
- Verbose labels ("user", "claude") when context makes speaker obvious
- Doesn't feel like a natural conversation

### Claude Code's Approach
```
> Help me implement a feature...

I'll help you implement that feature.
Here's what I understand:
```

**Why it works:**
- No labels needed - context makes speaker obvious
- `>` clearly marks user input
- AI responses just flow naturally
- Minimal, content-focused

### ralf's Complications
1. **Multiple AI models** - Need to attribute responses (claude, gemini, codex)
2. **Collaborator reviews** - Multiple AI responses in sequence
3. **Event types** - Spec, Run, Review, System all render differently

## Design

### New Compact Format

**User message:**
```
▸ › Help me implement a feature that adds pagination...
```

**AI coordinator message (expanded):**
```
▾ ● I'll help you implement that feature.              claude
    Here's what I understand:
    - Point 1
    - Point 2
```

**AI coordinator message (collapsed):**
```
▸ ● I'll help you implement that feature...            claude
```

**Collaborator review:**
```
▸ ◦ Consider adding error handling for edge cases.     gemini
```

**System message:**
```
  ! Model claude rate limited, cooling down 60s
```

### Symbol Legend

| Symbol | Meaning | Color | Notes |
|--------|---------|-------|-------|
| `›` | User message | `theme.text` | Like Claude Code's `>` |
| `●` | Coordinator AI | Model color | Filled circle |
| `◦` | Collaborator AI | Model color | Hollow circle (review) |
| `!` | System message | `theme.warning` | Alert/info |
| `▸` | Collapsed | `theme.muted` | Can expand |
| `▾` | Expanded | `theme.muted` | Can collapse |

### Model Colors
- `claude` → `theme.claude` (purple)
- `gemini` → `theme.gemini` (blue)
- `codex` → `theme.codex` (green)
- Unknown → `theme.info`

### Layout Structure

```
┌─ Position ──────────────────────────────────────────────┐
│ Col 0-1: Selection indicator (▸ or space)               │
│ Col 2:   Speaker symbol (›, ●, ◦, !)                    │
│ Col 3:   Space                                          │
│ Col 4+:  Content (wrapped)                              │
│ Right:   Model attribution (AI only)                    │
└─────────────────────────────────────────────────────────┘
```

**Example with columns:**
```
▸ › Help me implement a feature that adds...
││ │ └─ Content starts col 4
│└─┴─ Speaker symbol col 2
└─ Selection/collapse col 0-1

▾ ● I'll help you implement that.              claude
││ │ └─ Content                                └─ Right-aligned
│└─┴─ Speaker symbol (colored)
└─ Collapse indicator
```

### Selection Behavior

When an event is selected:
- Highlight the entire first line (or use `▸` prefix)
- Show collapse indicator based on state
- `Enter` toggles collapse
- `y` copies content

### Phase Context

**Remove from timeline events.** Phase is shown in:
- Status bar: `● Drafting │ "My Feature" │ claude ●`
- Canvas badge: `[Drafting] /accept when ready`

This eliminates redundant `[SPEC]`, `[RUN]`, `[REVIEW]` badges per message.

### Event Type Rendering

| Event Type | Symbol | Attribution | Collapsible |
|------------|--------|-------------|-------------|
| Spec (user) | `›` | None | Yes (if long) |
| Spec (AI) | `●` | Model name | Yes |
| Run | `●` | Model name | Yes |
| Review | `◦` | Model name | Yes |
| System | `!` | None | No |

### Expanded Content

For expanded events, continuation lines are indented to align with content start:

```
▾ ● I'll help you implement that feature.              claude
    Here's what I understand:
    - You want pagination for the user list
    - Maximum 20 items per page

    Here's my plan:
    1. Add `page` parameter to API
    2. Update frontend to handle pagination
```

Indent = 4 spaces (to align with content after `▾ ● `).

## Deliverables

### Files to Modify

1. **`crates/ralf-tui/src/timeline/widget.rs`**
   - Rewrite `render_event()` for compact format
   - Remove badge line rendering
   - Add right-aligned model attribution
   - Update selection highlighting

2. **`crates/ralf-tui/src/timeline/event.rs`**
   - Add `speaker_symbol()` method
   - Update `badge()` to return symbol instead of text
   - May need `is_coordinator()` vs `is_collaborator()` distinction

3. **`crates/ralf-tui/src/theme/icons.rs`** (if needed)
   - Add speaker symbols to IconSet
   - ASCII fallbacks: `>` for `›`, `*` for `●`, `o` for `◦`

4. **Snapshot tests**
   - Update any timeline snapshots to match new format

### Acceptance Criteria

- [ ] User messages render as `▸ › content...` (no "user" label)
- [ ] AI messages render as `▸ ● content...              model`
- [ ] Model name is right-aligned on first line
- [ ] Collaborator reviews use hollow circle `◦`
- [ ] System messages use `!` symbol
- [ ] No `[SPEC]`, `[RUN]`, `[REVIEW]` badges per message
- [ ] Expanded content indented 4 spaces
- [ ] Selection highlights work correctly
- [ ] Collapse/expand with Enter works
- [ ] Copy with `y` works
- [ ] ASCII mode has appropriate fallbacks

## Non-Goals

- Changing the underlying event data model
- Adding new event types
- Modifying how events are stored/persisted
- Changing the canvas/spec preview rendering

## Testing

1. **Visual testing** - Run TUI and verify appearance
2. **Snapshot tests** - Update existing snapshots
3. **Unit tests** - Test `speaker_symbol()` and related methods
4. **Edge cases**:
   - Very long first lines (truncation + attribution)
   - Empty content
   - Unicode/emoji in content
   - Narrow terminal widths

## Clarifications (from Gemini review)

1. **Review result colors** - Review events currently color by result (green=pass, red=fail). Keep this behavior - the `◦` symbol color indicates pass/fail status, not model. Model attribution still appears right-aligned.

2. **Run iterations** - Preserve iteration numbers in attribution: `claude #2` not just `claude`. This helps track progress through implementation loops.

3. **Attribution width** - Reserve ~15 chars for attribution (`claude #99` = 10 chars + padding). Truncate content to fit, never truncate attribution.

## Open Questions

1. **Review distinction** - Is `◦` (hollow) enough to distinguish collaborator reviews, or do we need more?
   - *Proposed:* Start with hollow circle, iterate based on feedback

2. **Selection indicator** - Use `▸` prefix or full-line highlight?
   - *Proposed:* `▸` prefix (current approach) - simpler, works well

3. **Attribution truncation** - What if model name + content don't fit?
   - *Resolved:* Truncate content, reserve fixed width for attribution
